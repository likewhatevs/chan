# Bob Task 2: Privacy And Abuse Surface

## Owner

Bob, syseng.

## Goal

Audit what user data, credentials, host metadata, and request headers can cross the tunnel or appear in logs. Confirm malicious public clients cannot spoof sensitive routing metadata downstream.

## Code To Inspect

- `crates/chan-tunnel-server/src/public.rs`
- `crates/chan-tunnel-server/src/tunnel.rs`
- `crates/chan-tunnel-server/src/lib.rs`
- `crates/chan-tunnel-client/src/dial.rs`

## Questions

- Can bearer tokens or proxy credentials appear in `tracing` logs or returned error strings?
- Are `Authorization`, `Cookie`, `Proxy-*`, `Forwarded`, `X-Real-IP`, and `X-Forwarded-*` treated deliberately for public requests?
- Does `Host` allowlist behavior match the deployment shape, including bare host and suffix cases?
- Does the CONNECT proxy path ever echo proxy credentials into errors?
- Are public/private drive semantics impossible to bypass with `Hello.public` or stale registrations?
- Do response errors leak local filesystem paths or internal chan-serve details to public visitors?

## Expected Output

- Findings added to `task/journal.md`.
- Tests for any header/log privacy gaps.
- Recommendation on default `PublicConfig` for production drive-proxy.

## Verification

- `cargo test -p chan-tunnel-server public`
- `cargo test -p chan-tunnel-client`
