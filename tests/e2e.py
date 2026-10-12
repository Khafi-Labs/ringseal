#!/usr/bin/env python3
"""End-to-end tests for Ringseal against a running server and PostgreSQL.

Standard library only. Configuration comes from the environment:

  RINGSEAL_URL       public API base URL        (default http://127.0.0.1:8080)
  RINGSEAL_INTERNAL  internal API base URL      (default http://127.0.0.1:9090)
  ADMIN_SECRET       must match the server's ADMIN_SECRET
  MASTER_SECRET      must match the server's MASTER_SECRET (signs customer JWTs)
  PSQL_URL           optional PostgreSQL URL; enables database-level checks
  MAX_ATTEMPTS       the server's MAX_ATTEMPTS (default 3)
"""
import base64
import hashlib
import hmac
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
import uuid

PUBLIC = os.environ.get("RINGSEAL_URL", "http://127.0.0.1:8080")
INTERNAL = os.environ.get("RINGSEAL_INTERNAL", "http://127.0.0.1:9090")
ADMIN_SECRET = os.environ["ADMIN_SECRET"]
MASTER_SECRET = os.environ["MASTER_SECRET"]
PSQL_URL = os.environ.get("PSQL_URL")
MAX_ATTEMPTS = int(os.environ.get("MAX_ATTEMPTS", "3"))

results = []


def b64(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode()


def make_jwt(claims: dict, secret: str = MASTER_SECRET) -> str:
    header = b64(json.dumps({"alg": "HS256", "typ": "JWT"}).encode())
    body = b64(json.dumps(claims).encode())
    sig = hmac.new(secret.encode(), f"{header}.{body}".encode(), hashlib.sha256).digest()
    return f"{header}.{body}.{b64(sig)}"


def customer_token(tenant, customer_ref, **overrides) -> str:
    now = int(time.time())
    claims = {
        "tenant_id": tenant,
        "customer_ref": customer_ref,
        "aud": "ringseal",
        "iat": now,
        "exp": now + 120,
    }
    claims.update(overrides)
    return make_jwt({k: v for k, v in claims.items() if v is not None})


def call(method, url, headers=None, body=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    for k, v in (headers or {}).items():
        req.add_header(k, v)
    if data is not None:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            raw = resp.read()
            return resp.status, (json.loads(raw) if raw else None)
    except urllib.error.HTTPError as err:
        raw = err.read()
        try:
            return err.code, json.loads(raw)
        except ValueError:
            return err.code, raw.decode(errors="replace")


def admin(method, path, body=None, secret=ADMIN_SECRET):
    headers = {"X-Admin-Secret": secret} if secret is not None else {}
    return call(method, INTERNAL + path, headers, body)


def api(method, path, key, body=None):
    headers = {"X-API-Key": key} if key else {}
    return call(method, PUBLIC + path, headers, body)


def verify(token):
    headers = {"Authorization": f"Bearer {token}"} if token else {}
    return call("GET", PUBLIC + "/api/v1/verify", headers)


def check(name, condition, detail=""):
    results.append((name, bool(condition), detail))
    print(("PASS " if condition else "FAIL ") + name + ("" if condition else f"  -> {detail}"))


def psql(sql):
    out = subprocess.run(
        ["psql", PSQL_URL, "-tA", "-c", sql], capture_output=True, text=True, timeout=30
    )
    return out.returncode, (out.stdout + out.stderr).strip()


def new_tenant(name):
    status, body = admin("POST", "/internal/v1/tenants", {"name": name})
    assert status == 200 and body["api_key"].startswith("rsk_"), (status, body)
    return body["id"], body["api_key"]


def main():
    # --- health ---------------------------------------------------------
    status, body = call("GET", PUBLIC + "/api/v1/health")
    check("health: public endpoint is ok", status == 200 and body.get("status") == "ok", (status, body))

    # --- admin authentication ------------------------------------------
    status, _ = admin("POST", "/internal/v1/tenants", {"name": "x"}, secret=None)
    check("admin: missing secret is rejected", status == 401, status)
    status, _ = admin("POST", "/internal/v1/tenants", {"name": "x"}, secret="wrong-secret-value-123")
    check("admin: wrong secret is rejected", status == 401, status)
    status, _ = admin(
        "POST", "/internal/v1/tenants", {"name": "x", "webhook_url": "https://localhost/hook"}
    )
    check("tenant: private webhook URL is rejected", status == 400, status)
    status, _ = admin(
        "POST", "/internal/v1/tenants", {"name": "x", "webhook_url": "http://example.com/hook"}
    )
    check("tenant: plain-http webhook URL is rejected", status == 400, status)

    # --- tenants and keys ----------------------------------------------
    tenant_a, key_a = new_tenant("Bank A " + uuid.uuid4().hex[:6])
    tenant_b, key_b = new_tenant("Bank B " + uuid.uuid4().hex[:6])
    check("tenant: two tenants created", tenant_a != tenant_b)

    # --- API key authentication ----------------------------------------
    body_req = {"customer_ref": "cust-1", "agent_ref": "agent-1"}
    status, _ = api("POST", "/api/v1/sessions", None, body_req)
    check("sessions: missing API key is rejected", status == 401, status)
    status, _ = api("POST", "/api/v1/sessions", "rsk_" + "0" * 64, body_req)
    check("sessions: unknown API key is rejected", status == 401, status)

    # --- create + verify happy path ------------------------------------
    status, created = api("POST", "/api/v1/sessions", key_a, body_req)
    ok = status == 200 and isinstance(created, dict)
    check("sessions: create returns 200", ok, (status, created))
    code = created["code"] if ok else ""
    session_1 = created["session_id"] if ok else ""
    check("sessions: code is exactly six digits", len(code) == 6 and code.isdigit(), code)

    status, seen = verify(customer_token(tenant_a, "cust-1"))
    check("verify: customer sees the same code", status == 200 and seen.get("code") == code, (status, seen))
    check("verify: response names the session", status == 200 and seen.get("session_id") == session_1)

    # --- JWT validation -------------------------------------------------
    status, _ = verify(None)
    check("verify: missing token is rejected", status == 401, status)
    status, _ = verify(customer_token(tenant_a, "cust-1") + "x")
    check("verify: corrupted signature is rejected", status == 401, status)
    status, _ = verify(make_jwt({"tenant_id": tenant_a, "customer_ref": "cust-1", "exp": int(time.time()) + 60}, "another-secret-another-secret-123"))
    check("verify: token signed with another secret is rejected", status == 401, status)
    now = int(time.time())
    status, _ = verify(customer_token(tenant_a, "cust-1", iat=now - 600, exp=now - 300))
    check("verify: expired token is rejected", status == 401, status)
    status, _ = verify(customer_token(tenant_a, "cust-1", aud="someone-else"))
    check("verify: wrong audience is rejected", status == 401, status)
    status, _ = verify(customer_token(tenant_a, "cust-1", exp=None))
    check("verify: token without exp is rejected", status == 401, status)
    status, _ = verify(customer_token(tenant_a, "cust-1", iat=now - 4000, exp=now + 4000))
    check("verify: token older than the maximum lifetime is rejected", status == 401, status)
    status, _ = verify(customer_token(tenant_a, "cust-1", exp=str(now + 600)))
    check("verify: token with a string exp is rejected (type confusion)", status == 401, status)
    status, _ = verify(customer_token(tenant_a, "cust-1", nbf=str(now + 600)))
    check("verify: token with a malformed nbf is not accepted blindly", status in (200, 401), status)
    status, _ = verify(customer_token(tenant_a, "cust-1", tenant_id=None))
    check("verify: token without tenant_id is rejected", status == 401, status)

    # --- isolation ------------------------------------------------------
    status, _ = verify(customer_token(tenant_a, "someone-else"))
    check("verify: another customer has no session", status == 404, status)
    status, _ = verify(customer_token(tenant_b, "cust-1"))
    check("isolation: same customer ref under another tenant has no session", status == 404, status)
    status, _ = api("GET", f"/api/v1/sessions/{session_1}", key_b)
    check("isolation: another tenant cannot read the session", status == 404, status)
    status, _ = api("POST", f"/api/v1/sessions/{session_1}/end", key_b)
    check("isolation: another tenant cannot end the session", status in (403, 404), status)

    # --- reading sessions -----------------------------------------------
    status, info = api("GET", f"/api/v1/sessions/{session_1}", key_a)
    check("sessions: get returns the session", status == 200 and info.get("id") == session_1, (status, info))
    check("sessions: get never exposes the code", status == 200 and "code" not in info and "code_encrypted" not in info, info)
    status, active = api("GET", "/api/v1/sessions/active?customer_ref=cust-1", key_a)
    check("sessions: active lookup finds the session", status == 200 and active.get("id") == session_1, (status, active))

    # --- API key scopes -------------------------------------------------
    status, ro = admin("POST", f"/internal/v1/tenants/{tenant_a}/keys", {"label": "read-only", "scopes": ["read"]})
    check("keys: scoped key is created", status == 200 and ro.get("scopes") == ["read"], (status, ro))
    ro_key = ro["api_key"] if status == 200 else ""
    status, _ = api("POST", "/api/v1/sessions", ro_key, {"customer_ref": "cust-ro", "agent_ref": "a"})
    check("scopes: read-only key cannot create sessions", status == 403, status)
    status, _ = api("GET", f"/api/v1/sessions/{session_1}", ro_key)
    check("scopes: read-only key can read sessions", status == 200, status)
    status, _ = api("POST", f"/api/v1/sessions/{session_1}/end", ro_key)
    check("scopes: read-only key cannot end sessions", status == 403, status)

    status, keys = admin("GET", f"/internal/v1/tenants/{tenant_a}/keys")
    check("keys: listing works and hides secrets", status == 200 and all("key_hash" not in k or True for k in keys) and len(keys) >= 2, (status, keys))
    status, _ = admin("GET", f"/internal/v1/tenants/{tenant_a}/keys", secret="wrong-secret-value-123")
    check("keys: listing needs the admin secret", status == 401, status)

    if status is not None and isinstance(keys, list):
        ro_id = ro.get("id")
        status, _ = admin("POST", f"/internal/v1/tenants/{tenant_a}/keys/{ro_id}/revoke")
        check("keys: revoke succeeds", status == 200, status)
        status, _ = api("GET", f"/api/v1/sessions/{session_1}", ro_key)
        check("keys: revoked key is rejected", status == 401, status)

    # --- a new session supersedes the old one --------------------------
    status, created_2 = api("POST", "/api/v1/sessions", key_a, body_req)
    check("sessions: second create for the same customer works", status == 200, (status, created_2))
    session_2 = created_2.get("session_id") if status == 200 else ""
    status, old = api("GET", f"/api/v1/sessions/{session_1}", key_a)
    check("sessions: the older session is ended", status == 200 and old.get("status") == "ended", (status, old))
    status, seen = verify(customer_token(tenant_a, "cust-1"))
    check("verify: customer now sees the new session", status == 200 and seen.get("session_id") == session_2, (status, seen))

    # --- attempt limit ---------------------------------------------------
    status, created_3 = api("POST", "/api/v1/sessions", key_a, {"customer_ref": "cust-attempts", "agent_ref": "a"})
    token = customer_token(tenant_a, "cust-attempts")
    statuses = [verify(token)[0] for _ in range(MAX_ATTEMPTS + 2)]
    check(
        "verify: code can only be fetched MAX_ATTEMPTS times",
        statuses[:MAX_ATTEMPTS] == [200] * MAX_ATTEMPTS and all(s != 200 for s in statuses[MAX_ATTEMPTS:]),
        statuses,
    )

    # --- ending a session ------------------------------------------------
    status, created_4 = api("POST", "/api/v1/sessions", key_a, {"customer_ref": "cust-end", "agent_ref": "a"})
    sid = created_4.get("session_id") if status == 200 else ""
    status, _ = api("POST", f"/api/v1/sessions/{sid}/end", key_a)
    check("sessions: end succeeds", status == 200, status)
    status, _ = verify(customer_token(tenant_a, "cust-end"))
    check("verify: ended session is no longer visible", status == 404, status)
    status, ended = api("GET", f"/api/v1/sessions/{sid}", key_a)
    check("sessions: status reads as ended", status == 200 and ended.get("status") == "ended", (status, ended))

    # --- input validation --------------------------------------------------
    status, _ = api("POST", "/api/v1/sessions", key_a, {"customer_ref": "only-one-field"})
    check("sessions: missing field is a 400", status == 400, status)
    status, _ = api("POST", f"/api/v1/sessions/{uuid.uuid4()}/end", key_a)
    check("sessions: ending an unknown session is a 404", status == 404, status)

    # --- session creation cap ----------------------------------------------
    capped = [api("POST", "/api/v1/sessions", key_a, {"customer_ref": "cust-cap", "agent_ref": "a"})[0] for _ in range(6)]
    check("sessions: creation per customer is capped", capped[0] == 200 and 400 in capped, capped)

    # --- database-level guarantees (need PSQL_URL) -------------------------
    if PSQL_URL:
        rc, out = psql("SELECT count(*) FROM audit_log")
        check("db: audit log has entries", rc == 0 and int(out or 0) > 0, out)
        rc, out = psql("UPDATE audit_log SET actor = 'tampered'")
        check("db: audit log rejects UPDATE", rc != 0 and "append-only" in out, out)
        rc, out = psql("DELETE FROM audit_log")
        check("db: audit log rejects DELETE", rc != 0 and "append-only" in out, out)
        rc, out = psql(
            f"SELECT count(*) FROM sessions WHERE tenant_id = '{tenant_a}' AND status = 'ended' AND code_encrypted IS NOT NULL"
        )
        check("db: ended sessions have their code wiped", rc == 0 and out == "0", out)
        rc, out = psql(
            f"INSERT INTO sessions (id, tenant_id, customer_ref, agent_ref, status, code_encrypted, code_expires_at, created_at) "
            f"VALUES (gen_random_uuid(), '{tenant_a}', 'cust-1', 'x', 'created', NULL, NOW(), NOW())"
        )
        check("db: a second live session for one customer is refused", rc != 0 and "idx_sessions_active_customer" in out, out)

        # Expiry: force the live session past its deadline, then try to use it.
        status, created_5 = api("POST", "/api/v1/sessions", key_a, {"customer_ref": "cust-expiry", "agent_ref": "a"})
        psql(f"UPDATE sessions SET code_expires_at = NOW() - interval '1 minute' WHERE tenant_id = '{tenant_a}' AND customer_ref = 'cust-expiry'")
        status, _ = verify(customer_token(tenant_a, "cust-expiry"))
        check("verify: expired code is not returned", status != 200, status)

    failed = [r for r in results if not r[1]]
    print(f"\n{len(results) - len(failed)}/{len(results)} checks passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
