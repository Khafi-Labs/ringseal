# 3. Threat model

Scope: Ringseal API v0.1 as published in this repository. This is a design-level review written by the authors. It is not a third-party audit.

## Assets

1. The verification code of a live session.
2. Tenant API keys and the `MASTER_SECRET`.
3. The integrity of the audit log.
4. Customer-to-session mapping (who is on a call with whom).

## Trust boundaries

- The **bank backend** is trusted to authenticate agents and customers and to issue customer JWTs.
- **Ringseal** is trusted to keep tenants separate and to protect codes at rest.
- The **phone network and the call** are untrusted.
- The **customer's device** is trusted only while the user is logged in to the bank app.

## Attackers considered

| Attacker | Goal |
|---|---|
| A. Remote caller posing as the bank | Make the customer believe the call is genuine. |
| B. Remote caller posing as the customer | Obtain agent actions. |
| C. Eavesdropper on the call | Reuse a spoken code. |
| D. Malicious or compromised tenant | Read or affect another tenant's sessions. |
| E. Network attacker | Steal API keys, tokens or codes in transit. |
| F. Database reader (backup leak, SQL access) | Recover live codes. |
| G. Insider with API access | Create sessions for arbitrary customers. |

## Analysis

| Threat | Mitigation in design | Residual risk |
|---|---|---|
| A. Fake "bank" caller | The code only exists inside a session created through the bank backend, so the caller cannot produce it. Guessing succeeds with probability about 1 in 1,000,000 per guess, and a session lives at most `CODE_EXPIRY_SECONDS`. | Works only if the customer actually checks. If the customer is coached to *read the code out* to the caller, an attacker can harvest it. Procedures and training must fix the direction (agent states, customer compares). |
| B. Fake "customer" caller | Sessions are created for a customer reference chosen by the agent; the code appears only in that customer's authenticated app. | Ringseal does not authenticate the person on the line to the agent beyond that. Banks still need their own identity checks. |
| C. Eavesdropper | Codes are single-session and expire quickly. | A live relay (attacker bridges the two parties during the call) is not prevented. |
| D. Cross-tenant access | Every lookup is scoped by tenant id. Ciphertext carries the tenant id as AEAD associated data. API keys map to exactly one tenant. | Isolation relies on every future query keeping the tenant filter. Add tests for this. |
| E. Transit | The service expects TLS termination in front of it. Webhooks require HTTPS by default. | The sample compose file serves plain HTTP on 8080. Never expose it directly. |
| F. Database reader | Codes are stored only as AES-256-GCM ciphertext, wiped on end or expiry. API keys are stored only as hashes. | A reader who also gets `MASTER_SECRET` can decrypt live codes. Keep the secret outside the database host and backups. |
| G. Insider | Per-key scopes, IP allowlists, key expiry and audit entries with principal names. | A fully privileged bank insider can create sessions. Detection depends on reviewing the audit log. |

## Known limitations (v0.1)

These are listed openly so adopters can plan around them. Fixes are planned.

1. **Rate limiting is not enforced.** A sliding-window limiter and its settings (`RATE_LIMIT_*`, `UNAUTH_RATE_LIMIT_*`) are implemented, but no request path calls it yet. Until it is wired in, put rate limiting in front of the service (reverse proxy or gateway). Session creation per customer is still capped separately, and fetches per session by `MAX_ATTEMPTS`.
2. **Shared-secret customer tokens.** Customer JWTs are HS256, verified with `MASTER_SECRET`. The bank must therefore hold the same secret that decrypts stored codes and signs webhooks. Planned: switch to Ed25519 (EdDSA) so banks sign with a private key and Ringseal only holds a public key, and separate keys per purpose.
3. **Optional token claims.** `iat`, `aud` and `jti` are optional. The maximum-lifetime check runs only when `iat` is present, and there is no replay tracking by `jti`. Only `exp` is required. Planned: require `iat`, `aud` and `jti`, and cache used `jti` values for their lifetime.
4. **Webhook URL validation is string-based.** Private ranges are blocked by hostname prefix. It does not resolve DNS, so hostnames that resolve to private addresses, alternative IP notations and similar tricks may pass. Redirects are disabled, which limits but does not remove the risk. Planned: resolve and validate the connecting IP at dispatch time.
5. **Client IP comes only from `X-Forwarded-For`.** With no trusted proxy configured, requests without the header all share the identity "unknown", and a client-supplied header can influence the address. This affects IP allowlists, rate limits and audit entries. Deploy behind a proxy that sets the header and set `TRUSTED_PROXY_COUNT` correctly.
6. **Rate limiter design.** Once wired in, it is per process and in memory, so limits are not shared between replicas and reset on restart.
7. **Audit chain is not serialized.** The previous hash is read and the new row inserted in separate steps, so concurrent writes for one tenant can fork the chain. No verification tool is included yet.
8. **Attempts count fetches, not wrong guesses.** Each code fetch uses one of `MAX_ATTEMPTS`, so app retries on a weak network can exhaust a session.
9. **Code expiry default.** If `CODE_EXPIRY_SECONDS` is unset the server defaults to 300 seconds. The provided `.env.example` and compose file set 60.
10. **Permissive CORS.** The public API currently allows any origin, method and header. Restrict it for production.
11. **Webhook signing.** Webhooks are signed with `MASTER_SECRET`, not a per-tenant secret, so tenants cannot verify them independently, and only `session.viewed` is emitted. Planned: per-tenant webhook secrets and the full event set.
12. **No external audit or fuzzing yet.**

## What Ringseal does not protect against

- A compromised customer phone or banking app session.
- A customer who knowingly reads the code to an attacker.
- A compromised bank backend or `MASTER_SECRET`.
- Denial of service at the network layer.

## Reporting issues

See [SECURITY.md](../SECURITY.md).
