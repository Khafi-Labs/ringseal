<div align="center">

# Ringseal

**Caller verification for banks.** Let customers confirm, during a live call, that the person on the line is a real bank agent.

[![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![Status](https://img.shields.io/badge/status-v0.1%20pre--release-yellow)

</div>

> **Status:** v0.1, pre-release, not externally audited. Read the [known limitations](research/03-threat-model.md#known-limitations-v01) before deploying to production.

## Overview

Fraudsters impersonate banks by phone, and caller ID, security questions and SMS codes do not reliably stop them. Ringseal turns the customer's already-authenticated banking app into a second channel: while a call is in progress, the bank creates a short-lived session and the customer's app displays a 6-digit code that only a genuine agent can know.

Ringseal is a multi-tenant REST API (Rust, axum, PostgreSQL) with mobile SDKs for Android, iOS and Flutter, an OpenAPI specification and two browser demos. One deployment can serve many banks, with strict separation by tenant.

Read more in [`research/`](research/): the [problem statement](research/01-problem-statement.md), the [solution design](research/02-solution-design.md) and the [threat model](research/03-threat-model.md).

## How it works

```text
+--------------+     +----------------+  X-API-Key   +---------------+
| Agent console| --> |  Bank backend  | -----------> | Ringseal API  |
+--------------+     +-------+--------+              | (multi-tenant)|
                             | issues JWT            +-------+-------+
                             v                               ^
                     +----------------+   Bearer JWT         |
                     | Customer app   | ---------------------+
                     | (Ringseal SDK) |
                     +----------------+
```

1. The agent starts a call. The bank backend creates a session for the customer (`POST /api/v1/sessions`) and receives the code.
2. The customer opens their bank app. The SDK fetches the code (`GET /api/v1/verify`) with a short-lived JWT issued by the bank.
3. The agent states the code and the customer checks it against the app. The customer should never be asked to read the code to the caller.
4. The agent ends the session, or it expires. The stored code is erased.

## Features

- **Out-of-band verification** through the authenticated banking app, with no new secret for the customer to hand over.
- **Short-lived codes**: uniform 6-digit codes (rejection sampling, no modulo bias), configurable expiry, a cap on fetches per session and one active session per customer.
- **Encrypted at rest**: AES-256-GCM, with the key derived from `MASTER_SECRET` and the tenant id bound as associated data. Ciphertext is set to NULL when a session ends or expires.
- **Multi-tenant**: sessions, API keys and audit entries are scoped per tenant.
- **Scoped API keys**: SHA-256 hashed at rest, with `create` / `read` / `end` scopes, expiry, IP allowlist and revocation.
- **Audit log**: append-only table (no UPDATE or DELETE for the application role) with per-tenant hash linking.
- **Webhooks**: HMAC-signed, HTTPS-only by default, no redirects, retry with backoff.
- **Hardened startup**: the server refuses short or known-weak secrets.
- **Mobile SDKs** with ready-made, themeable verification screens. The Android SDK enables screenshot protection by default.

## Quick start

Requirements: Docker and Docker Compose.

```bash
git clone https://github.com/khafiLabs/ringseal.git
cd ringseal

cp .env.example .env
# Edit .env and set:
#   MASTER_SECRET  (openssl rand -hex 32)
#   ADMIN_SECRET   (openssl rand -hex 16)
#   DB_PASSWORD

mkdir -p secrets
printf '%s' "<same value as DB_PASSWORD>" > secrets/db_password.txt

docker-compose up -d
curl http://localhost:8080/api/v1/health
```

Create a tenant. Tenant management runs on an internal listener bound to `127.0.0.1:9090` inside the container, so call it from there:

```bash
docker-compose exec ringseal sh -c 'curl -s -X POST http://127.0.0.1:9090/internal/v1/tenants \
  -H "X-Admin-Secret: $ADMIN_SECRET" \
  -H "Content-Type: application/json" \
  -d "{\"name\": \"Example Bank\"}"'
# -> {"id": "<tenant uuid>", "name": "Example Bank", "api_key": "rsk_..."}
```

Save the `api_key`; it is shown once. Then create a session as the bank backend:

```bash
curl -s -X POST http://localhost:8080/api/v1/sessions \
  -H "X-API-Key: rsk_..." \
  -H "Content-Type: application/json" \
  -d '{"customer_ref": "cust_123", "agent_ref": "agent_456"}'
# -> {"session_id": "...", "code": "482913", "expires_at": "..."}
```

The customer app then calls `GET /api/v1/verify` with a bearer JWT (see below). Run the sample compose file behind a TLS-terminating reverse proxy; it serves plain HTTP.

## API

Full contract: [`openapi/openapi.yaml`](openapi/openapi.yaml). Guide: [`docs/integration-guide.md`](docs/integration-guide.md).

| Endpoint | Method | Auth | Purpose |
|---|---|---|---|
| `/api/v1/sessions` | POST | API key (`create`) | Create a session and get the code |
| `/api/v1/sessions/{id}` | GET | API key (`read`) | Get session details |
| `/api/v1/sessions/active?customer_ref=` | GET | API key (`read`) | Get a customer's active session |
| `/api/v1/sessions/{id}/end` | POST | API key (`end`) | End a session |
| `/api/v1/verify` | GET | Bearer JWT | Fetch the code (customer SDK) |
| `/api/v1/health` | GET | none | Health check |
| `/internal/v1/tenants` | POST | Admin secret | Create a tenant (internal listener) |
| `/internal/v1/tenants/{id}/keys` | GET, POST | Admin secret | List or create API keys (internal listener) |
| `/internal/v1/tenants/{id}/keys/{key_id}/revoke` | POST | Admin secret | Revoke an API key (internal listener) |

### Authentication

| Caller | Mechanism |
|---|---|
| Bank backend | `X-API-Key` header |
| Customer app | `Authorization: Bearer <JWT>`, issued by the bank |
| Operator | `X-Admin-Secret` header, internal listener only |

Customer JWTs are HS256 in v0.1, verified with `MASTER_SECRET`. Claims:

| Claim | Required | Notes |
|---|---|---|
| `tenant_id` | yes | Your tenant UUID |
| `customer_ref` | yes | Must match the reference used when creating sessions |
| `exp` | yes | Keep it at 5 minutes or less |
| `iat` | recommended | The maximum-lifetime check runs only when present |
| `aud` | recommended | Must be `ringseal` if present |

## Configuration

All settings are environment variables; see [`.env.example`](.env.example).

| Variable | Default | Description |
|---|---|---|
| `DATABASE_URL` | required | PostgreSQL connection string |
| `MASTER_SECRET` | required | At least 32 characters. Derives the code-encryption key and verifies customer JWTs |
| `ADMIN_SECRET` | required | At least 16 characters |
| `HOST` / `PORT` | `127.0.0.1` / `8080` | Public API listener |
| `INTERNAL_PORT` | `9090` | Internal listener (always bound to `127.0.0.1`) |
| `CODE_EXPIRY_SECONDS` | `300` in code, `60` in `.env.example` | Code lifetime |
| `MAX_ATTEMPTS` | `3` | Code fetches allowed per session |
| `MAX_SESSIONS_PER_CUSTOMER` / `MAX_SESSIONS_WINDOW_SECONDS` | `3` / `300` | Session creation cap per customer |
| `MAX_JWT_LIFETIME_SECONDS` | `300` | Maximum token age when `iat` is set |
| `TRUSTED_PROXY_COUNT` | `0` | Reverse proxies in front of the service (for `X-Forwarded-For`) |
| `WEBHOOK_REQUIRE_HTTPS` | `true` | Require HTTPS for webhook URLs |
| `RATE_LIMIT_*`, `UNAUTH_RATE_LIMIT_*` | `100`/`60`, `20`/`60` | Reserved. The limiter is not yet applied to requests |
| `RUST_LOG` | `ringseal=info,tower_http=info` | Log filter |

## SDKs

| Platform | Path | Minimum | Docs |
|---|---|---|---|
| Android (Kotlin) | [`sdk/android/ringseal`](sdk/android/ringseal) | minSdk 24 | [README](sdk/android/ringseal/README.md) |
| iOS (Swift, SwiftUI and UIKit) | [`sdk/ios/RingsealSDK`](sdk/ios/RingsealSDK) | iOS 15 | [README](sdk/ios/RingsealSDK/README.md) |
| Flutter (Dart) | [`sdk/flutter/ringseal_sdk`](sdk/flutter/ringseal_sdk) | Flutter 3.10, Dart 3 | [README](sdk/flutter/ringseal_sdk/README.md) |

Demos: open [`demo/agent-console/index.html`](demo/agent-console/index.html) and [`demo/customer-app/index.html`](demo/customer-app/index.html) in a browser to try the agent and customer sides against a local instance.

## Security

Ringseal handles authentication material, so please read [`research/03-threat-model.md`](research/03-threat-model.md). Highlights for v0.1:

- Request rate limiting is not enforced yet; put limits in a reverse proxy or gateway.
- Customer tokens use a shared secret (HS256). Moving to Ed25519 is planned.
- Webhook URL validation and webhook signing need hardening.
- Restrict CORS and run behind TLS in production.

To report a vulnerability, see [SECURITY.md](SECURITY.md). Please do not open public issues for security problems.

## Development

```bash
# Requires a PostgreSQL instance and the variables from .env.example
export DATABASE_URL=postgresql://ringseal:<password>@localhost:5432/ringseal
export MASTER_SECRET=$(openssl rand -hex 32)
export ADMIN_SECRET=$(openssl rand -hex 16)

cargo run
```

The server creates its schema on startup. The files in [`migrations/`](migrations) hold the SQL history for reference and manual deployments.

## Project structure

```text
src/            API server (handlers, auth, code generation, audit, webhooks)
migrations/     SQL reference migrations
openapi/        OpenAPI 3 specification
docs/           Integration guide
research/       Problem statement, solution design, threat model
sdk/            Android, iOS and Flutter SDKs
demo/           Agent console and customer app demos
```

## Roadmap

- Ed25519-signed customer tokens and separate keys per purpose
- Enforced, shared rate limiting
- Per-tenant webhook secrets and the full session event set
- Hardened webhook destination checks
- Required `iat`, `aud` and `jti` with replay protection
- Audit-chain verification tooling
- External security review

## Contributing

Issues and pull requests are welcome. For larger changes, please open an issue first to discuss the approach.

## License

Licensed under the [Apache License, Version 2.0](LICENSE). Copyright 2026 Khafi Labs.
