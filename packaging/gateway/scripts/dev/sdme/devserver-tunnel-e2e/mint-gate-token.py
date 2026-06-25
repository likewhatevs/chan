#!/usr/bin/env python3
"""Mint a devserver-gate session JWT (HS256), matching
gateway-common/src/devserver_gate.rs::encode_session.

Claims envelope: {iss, sub, drv, aud, typ, iat, exp}. The proxy's
resolve_gate() decodes with TokenType::Session and checks aud==Host and
drv==devserver_id. We control WORKSPACE_GATE_SECRET, so a self-minted
session cookie yields Gate::Pass with no identity round trip.

Usage:
  mint_gate_token.py --secret S --sub UUID --drv DEVSERVER_ID --aud HOST [--ttl 86400]
Prints the compact JWT to stdout.
"""
import argparse, base64, hashlib, hmac, json, time, sys


def b64u(b: bytes) -> str:
    return base64.urlsafe_b64encode(b).rstrip(b"=").decode("ascii")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--secret", required=True)
    ap.add_argument("--sub", required=True, help="user_id UUID")
    ap.add_argument("--drv", required=True, help="devserver_id (== registry key)")
    ap.add_argument("--aud", required=True, help="wildcard host, e.g. alice.devserver.localtest.me")
    ap.add_argument("--typ", default="session", choices=["session", "entry"])
    ap.add_argument("--ttl", type=int, default=86400)
    a = ap.parse_args()

    now = int(time.time())
    header = {"typ": "JWT", "alg": "HS256"}
    iss = "devserver.chan.app" if a.typ == "session" else "id.chan.app"
    claims = {
        "iss": iss,
        "sub": a.sub,
        "drv": a.drv,
        "aud": a.aud,
        "typ": a.typ,
        "iat": now,
        "exp": now + a.ttl,
    }
    signing_input = (
        b64u(json.dumps(header, separators=(",", ":")).encode())
        + "."
        + b64u(json.dumps(claims, separators=(",", ":")).encode())
    ).encode("ascii")
    sig = hmac.new(a.secret.encode(), signing_input, hashlib.sha256).digest()
    print(signing_input.decode("ascii") + "." + b64u(sig))
    return 0


if __name__ == "__main__":
    sys.exit(main())
