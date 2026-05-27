# chan-tunnel-proto

Wire types and control frames for chan-tunnel: the length-prefixed
JSON `Hello` / `HelloAck` pair, the workspace-name validator, and an
`H2Duplex` adapter that turns an h2 `(SendStream, RecvStream)` pair
into one `tokio::io` duplex. Pure data plus a minimal codec; no I/O,
no async, no tokio runtime. Both `chan-tunnel-client` and
`chan-tunnel-server` depend on it; bumping the on-the-wire shape is
done here, not in the I/O crates.

```toml
[dependencies]
chan-tunnel-proto = "0.11"
```

## Public surface

```
control::    Hello, HelloAck, ProtocolVersion
workspace_name:: is_valid_workspace_name, is_valid_username,
             sanitize_workspace_name,
             MAX_WORKSPACE_NAME_LEN, MAX_USERNAME_LEN
frame::      encode_frame, decode_frame, FrameError
io::         read_frame, write_frame, IoFrameError
h2_duplex::  H2Duplex
const TUNNEL_PATH: &str            = "/v1/tunnel"
const MAX_CONTROL_FRAME_BYTES: usize = 64 * 1024
```

## Build & test

From the workspace root:

```bash
cargo build -p chan-tunnel-proto
cargo test  -p chan-tunnel-proto
```

The full workspace gate (used by CI and the pre-push hook) is
`cargo fmt --check && cargo clippy --all-targets -- -D warnings &&
cargo test`.

## Design

See [`design.md`](design.md) for the wire format, framing, workspace-
name rules, the `MAX_CONTROL_FRAME_BYTES` rationale, the proto-stays-
pure invariant, and the cross-crate context.

## License

Apache-2.0. See [`LICENSE`](../../LICENSE).
