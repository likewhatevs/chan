# chan-tunnel-server

Server-side of the chan-tunnel implementation. A library that
terminates tunnels dialed by `chan serve` and exposes the
drive-side substreams to a public-facing router.

## What it does

1. Exposes an `axum::Router` mounted at `TUNNEL_PATH`. nginx
   (`grpc_pass`) forwards h2c traffic from the public tunnel host
   (e.g. `tunnel.chan.app`) into this router.
2. On a new tunnel connection: reads `Hello`, calls the consumer's
   `Validator` to check the token / scope / drive identity, writes
   `HelloAck`.
3. Hands the duplex to yamux and inserts the new tunnel into the
   shared `Registry`, keyed by `(user, drive)`.
4. On a public-facing request, the gateway looks up the live
   tunnel in `Registry` and opens a yamux substream to forward the
   request.

## Public surface

```
Validator                       trait the consumer implements to
                                authenticate Hello frames
Validated                       opaque token returned by Validator
ServerError                     uniffi-friendly error variants

handshake(...)                  free function: read Hello, validate,
                                write HelloAck (used by wire tests)
serve_tunnel_listener(...)      accept loop bound to a tokio listener
public_router(...)              axum router for the public-facing
                                side, wired against Registry
Registry                        live (user, drive) -> tunnel map
TunnelHandle                    handle to a registered tunnel
OpenError                       failure mode for opening a substream
```

## Notes

- The library is async-first; the proto-shaped `handshake` is
  available for tests against an in-memory duplex.
- Errors flatten to primitives (no `h2::Error` or `yamux::Error`
  in public types) so consumers can map them through uniffi later.
