#!/usr/bin/env python3
"""Stub identity-service for the devserver-proxy tunnel e2e.

The proxy validates every tunnel dial's PAT against
  POST {IDENTITY_URL}/internal/v1/tokens/validate
  Authorization: Bearer <IDENTITY_INTERNAL_TOKEN>
  body {"token": "<chan_pat_*>"}
and expects 200 {user_id, username, devserver_id, scopes}. This stub
answers every well-formed validate with a fixed tunnel identity. It also
exposes the desktop entry endpoint used by this rig: two explicit bearer
tokens mint precomputed owner/editor entry JWTs, and every requested
owner/full-id/path is validated before the exact response is returned.
The proxy and devserver remain the real binaries while postgres-backed
identity and profile are the only omitted services.

Env (all optional, sane defaults):
  STUB_BIND        default 0.0.0.0:7000
  STUB_USER_ID     default 11111111-1111-4111-8111-111111111111
  STUB_USERNAME    default alice
  STUB_DEVSERVER_ID default 64-hex
  STUB_SCOPES      default "tunnel" (comma-separated)
  STUB_PROXY_ORIGIN exact public proxy origin for desktop entry responses
  STUB_DESKTOP_OWNER_PAT / STUB_DESKTOP_EDITOR_PAT accepted desktop bearers
  STUB_OWNER_ENTRY_TOKEN / STUB_EDITOR_ENTRY_TOKEN precomputed entry JWTs
Requests are logged to stderr; stdout stays clean.
"""
import json, os, sys
from datetime import datetime, timedelta, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

USER_ID = os.environ.get("STUB_USER_ID", "11111111-1111-4111-8111-111111111111")
USERNAME = os.environ.get("STUB_USERNAME", "alice")
DEVSERVER_ID = os.environ.get(
    "STUB_DEVSERVER_ID",
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
)
SCOPES = [s for s in os.environ.get("STUB_SCOPES", "tunnel").split(",") if s]
PROXY_ORIGIN = os.environ.get("STUB_PROXY_ORIGIN", "")
OWNER_PAT = os.environ.get("STUB_DESKTOP_OWNER_PAT", "")
EDITOR_PAT = os.environ.get("STUB_DESKTOP_EDITOR_PAT", "")
OWNER_ENTRY_TOKEN = os.environ.get("STUB_OWNER_ENTRY_TOKEN", "")
EDITOR_ENTRY_TOKEN = os.environ.get("STUB_EDITOR_ENTRY_TOKEN", "")


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
        if self.path == "/desktop/v1/devserver/entry":
            bearer = self.headers.get("Authorization", "")
            if bearer == f"Bearer {OWNER_PAT}" and OWNER_PAT:
                token, role = OWNER_ENTRY_TOKEN, "owner"
            elif bearer == f"Bearer {EDITOR_PAT}" and EDITOR_PAT:
                token, role = EDITOR_ENTRY_TOKEN, "editor"
            else:
                return self._json(401, {"error": "unauthorized"})
            try:
                body = json.loads(raw or b"{}")
            except Exception:
                return self._json(400, {"error": "bad json"})
            owner = body.get("owner")
            devserver_id = body.get("devserver_id")
            path = body.get("path") or "/"
            if owner != USERNAME or devserver_id != DEVSERVER_ID:
                return self._json(404, {"error": "not found", "reason": "access_denied"})
            if not isinstance(path, str) or not path.startswith("/") or "://" in path:
                return self._json(400, {"error": "invalid entry path"})
            if not PROXY_ORIGIN or not token:
                return self._json(503, {"error": "entry fixture is not configured"})
            sep = "&" if "?" in path else "?"
            expires = datetime.now(timezone.utc) + timedelta(minutes=5)
            sys.stderr.write(
                f"[stub-identity] desktop entry owner={owner} "
                f"devserver_id={devserver_id[:12]}... role={role} path={path}\n"
            )
            return self._json(
                200,
                {
                    "username": USERNAME,
                    "devserver_id": DEVSERVER_ID,
                    "proxy_origin": PROXY_ORIGIN,
                    "entry_url": f"{PROXY_ORIGIN}{path}{sep}t={token}",
                    "expires_at": expires.isoformat().replace("+00:00", "Z"),
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
