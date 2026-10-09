# 2. Solution design

## Idea

Use the customer's already-authenticated banking app as a second channel. During a call, the bank creates a short-lived session for that customer. Only someone with access to the bank's backend can create it, and the resulting 6-digit code is shown in the customer's app. A genuine agent knows the code. An impersonator does not.

```text
Agent console --> Bank backend --(X-API-Key)--> Ringseal API
                                                   ^
Customer app (Ringseal SDK) --(Bearer JWT from bank)--+
```

## Flow

1. The agent starts a call with the customer.
2. The bank backend calls `POST /api/v1/sessions` with the customer and agent references. Ringseal generates a code, stores it encrypted, and returns the code to the bank backend.
3. The customer opens their bank app. The SDK calls `GET /api/v1/verify` with a short-lived JWT issued by the bank and displays the code.
4. The two sides compare the code. **Recommended direction:** the agent states the code and the customer checks it against the app. The customer should never be asked to read the code to the caller (see the threat model).
5. The agent ends the session, or it expires. The stored ciphertext is then set to NULL.

## Components

| Component | Notes |
|---|---|
| API server | Rust, axum, sqlx, PostgreSQL. Public API on port 8080. Tenant and key management is on a separate internal listener bound to 127.0.0.1 (port 9090). |
| Tenants | One tenant per bank. Sessions, keys and audit entries are scoped by `tenant_id`. |
| Tenant API keys | Random 256-bit keys (`rsk_` prefix), stored only as SHA-256 hashes. Support scopes (`create`, `read`, `end`), expiry, an IP allowlist and an active flag. |
| Customer auth | HS256 JWT issued by the bank (see limitations in the threat model). Requires `exp`; enforces a maximum lifetime when `iat` is present and checks `aud` when present. |
| SDKs | Android (Kotlin), iOS (Swift), Flutter (Dart). Each provides a client and a ready-made verification screen with themes. |
| Demos | `demo/agent-console` and `demo/customer-app`, plain HTML. |
| API contract | `openapi/openapi.yaml`. |

## Code generation and storage

- **Generation:** each digit is drawn from the OS-seeded thread RNG with rejection sampling. Bytes of 250 or more are discarded so every digit is uniform, with no modulo bias.
- **Strength:** a 6-digit code carries about 20 bits. Its security rests on short life, an attempt limit and the fact that an attacker has no oracle to test guesses against, not on entropy alone.
- **At rest:** AES-256-GCM with a random 96-bit nonce. The key comes from `HMAC-SHA256(MASTER_SECRET, "code-encryption")`. The tenant id is bound as associated data, so a ciphertext cannot be moved between tenants.
- **Lifetime:** the ciphertext is set to NULL when a session ends or expires. The plaintext code is not stored in any database column.

## Abuse controls

- One active session per customer per tenant, enforced by a partial unique index.
- A cap on sessions created per customer per time window.
- A cap on how many times a session's code can be fetched (`MAX_ATTEMPTS`, default 3).
- Configurable code expiry. `.env.example` and the sample compose file use 60 seconds.
- A sliding-window rate limiter and its configuration exist, but v0.1 does not yet apply it to incoming requests (see limitations in the threat model).
- Startup checks reject short or known-weak secrets.

## Auditing and webhooks

- Every create, read and view action writes an audit row with actor, authenticated principal, IP and user agent. The application database role has no UPDATE or DELETE on the audit table, and triggers reject modification.
- Each audit row records the hash of the previous one for tamper evidence.
- Tenants can receive webhooks (for example `session.viewed`). Deliveries are HTTPS-only by default, do not follow redirects, time out after 5 seconds, retry with backoff, and carry an HMAC-SHA256 signature.

## Design decisions

- **Out-of-band channel instead of SMS:** the app session is already authenticated, so no new secret is sent to the customer.
- **Bank issues customer tokens:** the bank stays the identity authority; Ringseal never handles customer credentials.
- **Multi-tenant from the start:** isolation is enforced in every query by tenant id rather than added later.
