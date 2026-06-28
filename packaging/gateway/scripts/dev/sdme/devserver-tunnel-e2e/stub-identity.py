#!/usr/bin/env python3
"""Stub identity-service for the devserver-proxy tunnel e2e.

The proxy validates every tunnel dial's PAT against
  POST {IDENTITY_URL}/internal/v1/tokens/validate
  Authorization: Bearer <IDENTITY_INTERNAL_TOKEN>
  body {"token": "<chan_pat_*>"}
and expects 200 {user_id, username, devserver_id, scopes}. This stub
answers every well-formed validate with a FIXED identity, so the proxy
stays the real binary while we skip standing up postgres+profile+identity.
The PAT itself is not checked (any token validates) — auth is out of
scope for the tunnel-routing e2e; the gate cookie is minted separately
with the same DEVSERVER_GATE_SECRET.

Env (all optional, sane defaults):
  STUB_BIND        default 0.0.0.0:7000
  STUB_USER_ID     default 11111111-1111-4111-8111-111111111111
  STUB_USERNAME    default alice
  STUB_DEVSERVER_ID default 64-hex
  STUB_SCOPES      default "tunnel" (comma-separated)
Requests are logged to stderr; stdout stays clean.
"""
import json, os, sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

USER_ID = os.environ.get("STUB_USER_ID", "11111111-1111-4111-8111-111111111111")
USERNAME = os.environ.get("STUB_USERNAME", "alice")
DEVSERVER_ID = os.environ.get(
    "STUB_DEVSERVER_ID",
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
)
SCOPES = [s for s in os.environ.get("STUB_SCOPES", "tunnel").split(",") if s]


class H(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        sys.stderr.write("[stub-identity] " + (fmt % args) + "\n")

    def _json(self, code, obj):
        body = json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_POST(self):
        n = int(self.headers.get("Content-Length", 0))
        raw = self.rfile.read(n) if n else b""
        if self.path == "/internal/v1/tokens/validate":
            try:
                tok = json.loads(raw or b"{}").get("token", "")
            except Exception:
                tok = ""
            sys.stderr.write(
                f"[stub-identity] validate token={tok[:12]}... -> user={USERNAME} "
                f"devserver_id={DEVSERVER_ID[:12]}... scopes={SCOPES}\n"
            )
            return self._json(
                200,
                {
                    "user_id": USER_ID,
                    "username": USERNAME,
                    "devserver_id": DEVSERVER_ID,
                    "scopes": SCOPES,
                },
            )
        return self._json(404, {"error": "not found"})

    def do_GET(self):
        if self.path == "/healthz":
            return self._json(200, {"ok": True})
        return self._json(404, {"error": "not found"})


def main():
    bind = os.environ.get("STUB_BIND", "0.0.0.0:7000")
    host, port = bind.rsplit(":", 1)
    srv = ThreadingHTTPServer((host, int(port)), H)
    sys.stderr.write(
        f"[stub-identity] listening on {bind}; user={USERNAME} "
        f"user_id={USER_ID} devserver_id={DEVSERVER_ID}\n"
    )
    srv.serve_forever()


if __name__ == "__main__":
    main()
