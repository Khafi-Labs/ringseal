# Ringseal Integration Guide

Welcome to the Ringseal Integration Guide. This document provides everything you need to know to integrate Ringseal into your banking infrastructure.

## 1. Overview and Prerequisites

Ringseal requires:
- A backend server capable of making secure HTTP requests.
- The ability to issue JWTs to your customers within their authenticated session (mobile/web app).
- A Ringseal Tenant API Key.

## 2. Getting Started (Sandbox Setup)

To get started, spin up the local development environment or request a Sandbox API key from our team.

```bash
# Tenant management is served on the internal listener (127.0.0.1:9090 inside the container).
# With the sample compose file, run it from inside the container:
#   docker-compose exec ringseal curl ...
curl -X POST http://127.0.0.1:9090/internal/v1/tenants \
  -H "X-Admin-Secret: $ADMIN_SECRET" \
  -H "Content-Type: application/json" \
  -d '{"name": "My Bank", "webhook_url": "https://mybank.com/webhooks/ringseal"}'
```
Save the returned `api_key`.

## 3. Authentication

### Backend Authentication
Include your API Key in the `X-API-Key` header for all server-to-server requests.

### Customer Authentication (JWT)
Your backend must issue a JWT for the customer's device. The JWT must include a `customer_ref` claim.

Required claims: `tenant_id` (your tenant UUID), `customer_ref` (the same reference your backend sends when creating sessions) and `exp`. Also set `iat` and `aud` (`"ringseal"`): the maximum-lifetime check only runs when `iat` is present. In v0.1 tokens are HS256, verified with the deployment's `MASTER_SECRET`. Keep token lifetimes short (5 minutes or less).

**Python JWT Example:**
```python
import time
import jwt

def generate_customer_jwt(tenant_id, customer_id, secret):
    now = int(time.time())
    payload = {
        "tenant_id": tenant_id,
        "customer_ref": customer_id,
        "aud": "ringseal",
        "iat": now,
        "exp": now + 300,
    }
    return jwt.encode(payload, secret, algorithm="HS256")
```

**Node.js JWT Example:**
```javascript
const jwt = require('jsonwebtoken');

function generateCustomerJwt(tenantId, customerId, secret) {
  return jwt.sign(
    { tenant_id: tenantId, customer_ref: customerId, aud: 'ringseal' },
    secret,
    { algorithm: 'HS256', expiresIn: '5m' } // iat is added automatically
  );
}
```

## 4. Session Lifecycle

1. **Create**: Agent calls backend, backend creates session via `/api/v1/sessions`.
2. **Verify**: Customer SDK fetches the code via `/api/v1/verify` using their JWT.
3. **End/Expire**: Agent ends session, or it automatically expires.

## 5. Implementing the Agent Side

**Create a session:**
```bash
curl -X POST https://api.ringseal.com/api/v1/sessions \
  -H "X-API-Key: your_api_key_here" \
  -H "Content-Type: application/json" \
  -d '{"customer_ref": "cust_123", "agent_ref": "agent_456"}'
```

**End a session:**
```bash
curl -X POST https://api.ringseal.com/api/v1/sessions/123e4567-e89b-12d3-a456-426614174000/end \
  -H "X-API-Key: your_api_key_here"
```

## 6. Implementing the Customer Side (SDK)

The customer application needs to fetch the active session to display the code.

```bash
curl -X GET https://api.ringseal.com/api/v1/verify \
  -H "Authorization: Bearer your_generated_jwt_here"
```

## 7. Webhooks

If a tenant has a `webhook_url`, Ringseal sends a signed `POST` when a customer views a session (`session.viewed`, including IP address and user agent). Other lifecycle events (`session.created`, `session.ended`, `session.expired`) are not emitted in v0.1.

Requests carry `X-Ringseal-Event-Id`, `X-Ringseal-Timestamp` and `X-Ringseal-Signature` (hex HMAC-SHA256 of the raw body). Deliveries require HTTPS by default, do not follow redirects, time out after 5 seconds and retry up to 3 times.

In v0.1 the signature key is the server's `MASTER_SECRET`, not a per-tenant secret, so tenants cannot independently verify signatures yet. Per-tenant webhook secrets are on the roadmap. Until then, treat webhooks as notifications and confirm state through the API.

## 8. Security Best Practices

- Never expose the `X-API-Key` in client applications.
- JWTs should have a short expiration time (e.g., 5-15 minutes).
- Do not trigger sensitive actions from webhooks alone; confirm state through the API.

## 9. Error Handling and Retry Logic

- **401 Unauthorized**: Check API keys and JWT expirations.
- **429 Too Many Requests**: Implement exponential backoff.
- **404 Not Found**: Handle gracefully (e.g., no active session).

## 10. FAQ

**Q: Can a customer have multiple active sessions?**
A: No, only one active session per `customer_ref` is allowed.
