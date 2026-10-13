# Changelog

## 1.0.2

- Security: remove `rsa` (RUSTSEC-2023-0071, Marvin attack) from the dependency tree. JWT crypto now uses aws-lc-rs, sqlx is limited to PostgreSQL and updated to 0.9.
- Update: rand 0.10, hmac 0.13 with sha2 0.11, tower-http 0.7, reqwest 0.13.

## 1.0.1

Fixes found by the new end-to-end test suite. **Use this instead of 1.0.0.**

- Fix: the server could not start against PostgreSQL, because the schema was run as one prepared statement.
- Fix: every customer token carrying `aud: "ringseal"` (as the integration guide says) was rejected as invalid.
- Fix: the audit-log append-only triggers and the one-live-session-per-customer index were only in `migrations/`; they are now created at startup. Creating a session now ends the customer's previous live session.
- Fix: invalid request bodies return the documented 400 JSON error instead of axum's plain 422.
- Fix: a tenant's `webhook_url` is validated when the tenant is created, not only at delivery.
- Security: upgrade `jsonwebtoken` 9.3.1 to 10.4.0 (GHSA-h395-gr6q-cpjc, claim type confusion). Ringseal required `exp` and parsed claims strictly, so the issue was not exploitable here, but the patched version is now used.
- Add: end-to-end tests (`tests/e2e.py`) run in CI against PostgreSQL, 11 unit tests, strict clippy.
- Add: Dependabot, weekly `cargo audit`, container image build check, CODEOWNERS and issue/PR templates.

## 1.0.0

First tagged release (pre-release). Superseded by 1.0.1.
