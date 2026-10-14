#!/usr/bin/env python3
"""Rate-limit tests. Run against a server started with low limits:

  RATE_LIMIT_REQUESTS=10 RATE_LIMIT_WINDOW_SECONDS=60
  UNAUTH_RATE_LIMIT_REQUESTS=5 UNAUTH_RATE_LIMIT_WINDOW_SECONDS=60
  TRUSTED_PROXY_COUNT=0

Environment: RINGSEAL_URL (public base URL), RINGSEAL_INTERNAL, ADMIN_SECRET.
Standard library only.
"""
import json
import os
import sys
import urllib.error
import urllib.request

PUBLIC = os.environ.get("RINGSEAL_URL", "http://127.0.0.1:8081")
INTERNAL = os.environ.get("RINGSEAL_INTERNAL", "http://127.0.0.1:9091")
ADMIN_SECRET = os.environ["ADMIN_SECRET"]

results = []


def request(method, url, headers=None, body=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    for k, v in (headers or {}).items():
        req.add_header(k, v)
    if data is not None:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            return resp.status, dict(resp.headers), resp.read()
    except urllib.error.HTTPError as err:
        return err.code, dict(err.headers), err.read()


def check(name, condition, detail=""):
    results.append(bool(condition))
    print(("PASS " if condition else "FAIL ") + name + ("" if condition else f"  -> {detail}"))


def main():
    # The internal listener is not rate limited; use it to get an API key.
    status, _, raw = request(
        "POST", INTERNAL + "/internal/v1/tenants",
        {"X-Admin-Secret": ADMIN_SECRET}, {"name": "rate-limit-bank"},
    )
    assert status == 200, (status, raw)
    api_key = json.loads(raw)["api_key"]

    # Health checks are exempt, however many there are.
    codes = [request("GET", PUBLIC + "/api/v1/health")[0] for _ in range(40)]
    check("health: exempt from rate limiting", codes == [200] * 40, codes)

    # Anonymous requests: the stricter limit applies (5 per window).
    anon = [request("GET", PUBLIC + "/api/v1/verify")[0] for _ in range(7)]
    check("anonymous: first requests reach the handler (401)", anon[:5] == [401] * 5, anon)
    check("anonymous: further requests are limited (429)", anon[5:] == [429, 429], anon)
    status, headers, raw = request("GET", PUBLIC + "/api/v1/verify")
    lowered = {k.lower(): v for k, v in headers.items()}
    check("limited response carries Retry-After", status == 429 and lowered.get("retry-after") == "60", (status, headers))
    check("limited response is the JSON error format", status == 429 and "error" in json.loads(raw), raw)

    # A forged X-Forwarded-For must not give a fresh allowance.
    spoofed = [
        request("GET", PUBLIC + "/api/v1/verify", {"X-Forwarded-For": f"10.9.8.{i}"})[0]
        for i in range(3)
    ]
    check("spoofing X-Forwarded-For does not evade the limit", spoofed == [429] * 3, spoofed)

    # The per-IP limit also covers authenticated traffic (10 per window overall).
    authed = [
        request("GET", PUBLIC + "/api/v1/sessions/active?customer_ref=x", {"X-API-Key": api_key})[0]
        for _ in range(3)
    ]
    check("authenticated requests share the per-IP limit", all(code == 429 for code in authed), authed)

    print(f"\n{sum(results)}/{len(results)} checks passed")
    return 0 if all(results) else 1


if __name__ == "__main__":
    sys.exit(main())
