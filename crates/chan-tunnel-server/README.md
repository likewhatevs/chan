# chan-tunnel-server

Server-side library for chan-tunnel: terminates the long-lived h2c
POSTs from `chan serve` clients, runs Hello / HelloAck, registers
each `(user, drive)` in a shared `Registry`, and exposes a
public-facing axum `Router` that opens fresh yamux substreams to
forward requests (including WebSocket upgrades) into the registered
peer. Designed to be embedded into a host process (e.g. the gateway's
`drive-proxy`) which provides the `Validator` impl, mounts the
public router at `/{user}/{drive}/...`, and lets nginx terminate
TLS in front.

```toml
[dependencies]
chan-tunnel-server = "0.7"
```

## Public surface

```
Validator (trait)             implemented by host: token -> Validated
Validated                     user_id, username, scopes
ServerError                   uniffi-friendly error variants

handshake(socket, token,
          validator, pre_ack) free function: validate + Hello/HelloAck
                              over any tokio duplex
handshake_validated(...)      same, with already-validated identity

serve_tunnel_listener(
    listener, validator,
    registry, max_drives_per_user)
                              accept loop on a TCP listener; runs h2
                              server, validates, registers, drives

public_router(registry)       axum router for the public side
public_router_with(registry,  same, with PublicConfig knobs
                   cfg)
PublicConfig                  body cap, ...
DEFAULT_REQUEST_BODY_CAP      10 MiB

Registry                      live (user, drive) -> TunnelHandle
TunnelHandle                  open() -> yamux::Stream
DriveInfo / TunnelInfo        admin snapshots
OpenError                     Disconnected
```

## Build & test

From the workspace root:

```bash
cargo build -p chan-tunnel-server
cargo test  -p chan-tunnel-server
```

The full workspace gate (used by CI and the pre-push hook) is
`cargo fmt --check && cargo clippy --all-targets -- -D warnings &&
cargo test`.

## Design

See [`design.md`](design.md) for the listener / driver / registry
shape, the eviction policy, the public router's substream lifecycle
including upgrades, and the cross-crate context. The wire format
itself is in
[`chan-tunnel-proto/design.md`](../chan-tunnel-proto/design.md).

## License

Apache-2.0. See [`LICENSE`](../../LICENSE).
