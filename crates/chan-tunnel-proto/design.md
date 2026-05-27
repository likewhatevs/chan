# chan-tunnel-proto: design

## Cross-crate context

chan-tunnel is split into three crates that live together in
`chan-writer/chan-core`:

- `chan-tunnel-proto` (this crate): pure wire types and a sync
  codec. Hello / HelloAck, the workspace-name and username validators,
  the `H2Duplex` adapter, and the `TUNNEL_PATH` /
  `MAX_CONTROL_FRAME_BYTES` constants. No I/O, no async runtime.
- `chan-tunnel-client`: dials a tunnel terminator over h2/TLS,
  runs the Hello round-trip, hands the post-handshake duplex to
  yamux, and serves inbound substreams with a user-supplied axum
  router. Embedded into `chan serve`.
- `chan-tunnel-server`: terminates tunnel connections from clients,
  exposes a `Validator` trait the consumer implements, registers
  live tunnels in a shared `Registry` keyed by `(user, workspace)`,
  and offers a `public_router` that opens fresh yamux substreams
  to forward public-facing HTTP requests.

End-to-end shape: `chan serve` (running on the user's laptop)
embeds chan-tunnel-client and dials the public terminator at
`POST {tunnel-host}/v1/tunnel`. The terminator is chan-tunnel-server
hosted inside `chan-gateway/workspace-proxy`. After Hello / HelloAck the
single h2 stream becomes a yamux session; the public router opens
one substream per public request and runs hyper h1 over it.

This document is the canonical reference for the wire format and
framing. The client and server design.md files reference back here
for any byte-level detail.

## 1. Problem and scope

A chan user wants their local workspace reachable on a public URL
(`drive.chan.app/{user}/{workspace}/...`) without opening a port,
configuring DNS, or running a TURN/STUN stack. The constraint is
"works through corporate NAT and HTTP-only egress." The shape that
fits both is one long-lived HTTPS request.

This crate owns:

- Control frames (`Hello`, `HelloAck`) and their JSON serde shape.
- Length-prefixed framing (`[u32 BE len][json bytes]`) used only
  for the two control messages.
- Workspace-name and username validators applied identically by client
  and server (defense-in-depth gate against URL-unsafe identifiers).
- `H2Duplex`: an `AsyncRead + AsyncWrite + Unpin` over an h2
  `(SendStream<Bytes>, RecvStream)` pair, used by both ends to feed
  the post-handshake byte stream into yamux.
- `TUNNEL_PATH` and `MAX_CONTROL_FRAME_BYTES` constants.

Out of scope here, owned by the I/O crates:

- TLS, h2 client/server setup, request routing.
- yamux multiplexing, substream lifecycle.
- Token validation, registry of live tunnels.
- HTTP-level rewriting, X-Forwarded-* injection, upgrade bridging.

## 2. Architecture overview

```
client                                           server
------                                           ------
                  POST /v1/tunnel
                  Authorization: Bearer <token>
                ---------------------------------->
                  HTTP/2 stream opens

                  [u32 len][json Hello]
                ---------------------------------->
                                                  validate token,
                                                  run pre_ack hook
                  [u32 len][json HelloAck]
                <----------------------------------
                  yamux frames (both directions)
                <=================================>
                  per-public-request substreams
                  carry h1 (req+headers+body / resp)
```

Framing for the two control messages is identical in both
directions:

```
+--------+----------------------------+
|  u32   | json bytes                 |
|  BE    | length given by the prefix |
+--------+----------------------------+
0        4                            4 + len
```

After `HelloAck` is fully read on the client and fully written on
the server, the byte stream belongs to yamux. The codec in this
crate is not used again on that connection.

## 3. Components / responsibilities

| File              | Owns                                       |
|-------------------|--------------------------------------------|
| `control.rs`      | `Hello`, `HelloAck`, `ProtocolVersion`     |
| `frame.rs`        | sync codec (`encode_frame`, `decode_frame`)|
| `io.rs`           | tokio helpers (`read_frame`, `write_frame`)|
| `workspace_name.rs`   | workspace + username validators, sanitizer     |
| `h2_duplex.rs`    | `H2Duplex` adapter over h2 streams         |
| `lib.rs`          | re-exports + `TUNNEL_PATH`, byte cap       |

The split between `frame.rs` (sync, `BytesMut`-based) and `io.rs`
(async, tokio `AsyncRead/Write`) is deliberate: the sync codec is
self-contained and reusable from any I/O loop. The async helpers
exist because both real callers run on tokio; a third caller on a
different runtime would consume `frame.rs` directly.

## 4. Public API surface

```rust
// Constants
pub const TUNNEL_PATH: &str = "/v1/tunnel";
pub const MAX_CONTROL_FRAME_BYTES: usize = 64 * 1024;

// Control frames
pub struct ProtocolVersion(pub u16);
impl ProtocolVersion { pub const V1: ProtocolVersion = ProtocolVersion(1); }

pub struct Hello {
    pub protocol: ProtocolVersion,
    pub client_version: String,
    pub workspace: String,
    pub public: bool, // additive; #[serde(default)]
}

pub struct HelloAck {
    pub protocol: ProtocolVersion,
    pub prefix: String,
    pub user: String,
    pub workspace: String,
}

// Sync codec
pub fn encode_frame<T: Serialize>(value: &T, out: &mut BytesMut)
    -> Result<(), FrameError>;
pub fn decode_frame<T: DeserializeOwned>(buf: &mut BytesMut)
    -> Result<T, FrameError>;
pub enum FrameError { TooLarge(u32), Incomplete { need: usize }, Json(_) }

// Async helpers (tokio)
pub async fn read_frame<R, T>(r: &mut R) -> Result<T, IoFrameError>
    where R: AsyncRead + Unpin, T: DeserializeOwned;
pub async fn write_frame<W, T>(w: &mut W, value: &T)
    -> Result<(), IoFrameError>
    where W: AsyncWrite + Unpin, T: Serialize;
pub enum IoFrameError { Frame(FrameError), Io(std::io::Error) }

// Workspace + username rules
pub const MAX_WORKSPACE_NAME_LEN: usize = 32;
pub const MAX_USERNAME_LEN: usize = 64;
pub fn is_valid_workspace_name(s: &str) -> bool;
pub fn is_valid_username(s: &str) -> bool;
pub fn sanitize_workspace_name(input: &str) -> Option<String>;

// h2 duplex adapter
pub struct H2Duplex { /* ... */ }
impl H2Duplex {
    pub fn new(send: SendStream<Bytes>, recv: RecvStream) -> Self;
}
// implements AsyncRead + AsyncWrite + Unpin
```

All public types are owned (`String`, `PathBuf`, plain enums); no
borrowed lifetimes. Errors flatten to primitives on `Display` so
client and server can surface them through their own umbrella enums
without re-exporting `h2::Error` or `serde_json::Error`.

## 5. Wire format / framing

### Why a length-prefixed JSON envelope, not HTTP headers

The handshake fields could in theory ride on the request line or
custom headers (`X-Chan-Workspace: notes`, etc.). They don't, for three
reasons:

1. The terminator runs behind nginx with `grpc_pass`. nginx will
   strip or rewrite arbitrary request headers on h2-to-h2 forwarding
   depending on configuration; the body is opaque to it. Putting the
   contract in the body is invariant under proxy churn.
2. The reverse direction (`HelloAck`) needs to carry structured data
   back to the client (`prefix`, `user`, `workspace`). HTTP responses
   could use headers, but then the schemas are asymmetric and adding a
   field on the response side is a header-name fight rather than a
   serde additive change.
3. JSON in the body is symmetric, evolvable (`#[serde(default)]`),
   and trivially testable without standing up h2.

### Why JSON

The control frame is exchanged once per tunnel lifetime; encode
cost is irrelevant. JSON is debuggable on the wire, additive-friendly
via serde, and removes a transitive dep on a binary codec the rest
of the workspace doesn't already use. If the frame ever grows hot
(e.g. multi-hello multiplex), revisit; today it costs maybe 200 B.

### Length prefix

`u32` big-endian. The decoder reads the prefix first so it can size
the read buffer before allocating. Without a prefix the decoder
would have to scan for end-of-JSON, which is fragile (escaped
quotes, embedded objects).

`u32` instead of varint to keep the cap check trivial: any value
above `MAX_CONTROL_FRAME_BYTES` is rejected immediately, before any
body bytes hit memory.

### MAX_CONTROL_FRAME_BYTES

64 KiB. Real frames are well under 1 KiB. The cap exists because a
malicious or buggy peer could send `0xFFFFFFFF` followed by no
data; without a cap, the receiver would either OOM trying to
allocate a 4 GiB buffer or hang reading a non-existent body. 64 KiB
is small enough to fit any plausible additive growth (richer
metadata, optional preferences) and small enough that even the
worst-case allocation is harmless on every target.

`encode_frame` checks the cap before writing; `decode_frame` checks
it before allocating. Both surfaces refuse frames over the cap with
`FrameError::TooLarge(len)`.

### Hello.public

Additive field, `#[serde(default)]`. Older clients omit it; the
server treats them as `public = false`. Newer servers reading older
client frames also see `false`. The default mirrors the historical
behaviour (private workspaces only) so the silent default is safe.

### HelloAck.prefix

Server-assigned public path prefix, e.g. `/alice/notes`. Always
starts with `/`, never ends with one. The client uses it to wire the
inner router so the operator does not pass `--prefix` manually.

### ProtocolVersion negotiation

Carried inside `Hello`. The path (`/v1/tunnel`) does not bump on
version changes; that path is a stable mount point. Bumping
`ProtocolVersion` is reserved for incompatible changes; additive
ones use serde defaults. The server may accept multiple versions in
the future by branching on `hello.protocol`; today only `V1` is
defined and both sides reject anything else.

### H2Duplex

`h2` exposes a request/response as a `SendStream<Bytes>` plus a
`RecvStream`. yamux wants a single `AsyncRead + AsyncWrite +
Unpin`. `H2Duplex` is the glue:

- `poll_read` pulls a `Bytes` chunk from `RecvStream::poll_data`,
  copies into the caller's buffer, and calls `release_capacity` on
  the chunk's length so the peer's flow-control window keeps moving.
  Best-effort on `release_capacity`: a stream the peer already reset
  errors here and we ignore it; the next read will surface it.
- `poll_write` reserves capacity (`reserve_capacity` +
  `poll_capacity`), then `send_data(chunk, false)` for the smaller
  of `capacity()` and the caller's buffer.
- `poll_shutdown` issues `send_data(Bytes::new(), true)` once to
  half-close the write side; subsequent calls are no-ops.

Symmetric: server side's `RecvStream` is the request body and
`SendStream` is the response body; client side is the reverse. The
adapter doesn't care which.

## 6. Trust boundaries / validation

This crate is the validator surface for two values that flow into a
public URL: workspace name (from the client's `Hello`) and username
(from the server's `Validated`).

### Workspace name (`is_valid_workspace_name`)

Rules: 1..=32 ASCII bytes; characters `[a-z0-9-]`; first and last
character alphanumeric (no leading/trailing hyphen). Both sides
call it. The client refuses to send an invalid name; the server
refuses to accept one. The duplication is intentional: it catches
buggy clients without trusting them, and catches a future server
that introduces a new path scheme that would let an old client
still register a name the new server can't safely route.

`sanitize_workspace_name` is a best-effort transform from a free-form
string (often the workspace directory's basename) into a valid name:
lowercase ASCII, collapse non-alnum runs to single `-`, trim,
truncate. Returns `None` when the result would be empty so the
caller can prompt the user instead of inventing a name.

### Username (`is_valid_username`)

Slightly looser than the workspace validator because real identity
services emit mixed-case names with underscores: ASCII alphanumerics,
`-`, `_`; first character alphanumeric (no leading punctuation);
1..=64. Applied by chan-tunnel-server after the validator returns,
to keep `Validated::username` from carrying `..` / `alice/bob` /
whitespace into the public path.

### Frame-size cap

64 KiB, enforced in both `encode_frame` and `decode_frame`. See
section 5.

## 7. Error model

Two enums, both flat:

```rust
pub enum FrameError {
    TooLarge(u32),
    Incomplete { need: usize },
    Json(serde_json::Error),
}

pub enum IoFrameError {
    Frame(FrameError),
    Io(std::io::Error),
}
```

`FrameError::Incomplete` is recoverable: the caller leaves the
`BytesMut` untouched, reads at least `need` more bytes, and tries
again. Every other variant is terminal for the handshake; the
caller closes the stream.

The async helpers return `IoFrameError`; the sync codec returns
`FrameError`. Client and server both convert into their own umbrella
enums via `From`, flattening through `Display` so `h2::Error` and
`serde_json::Error` never appear in their public surface.

## 8. Consumers

- `chan-tunnel-client` (this workspace): runtime dep. Imports
  `Hello`, `HelloAck`, `ProtocolVersion`, `read_frame`,
  `write_frame`, `is_valid_workspace_name`, `MAX_WORKSPACE_NAME_LEN`, and
  `H2Duplex`.
- `chan-tunnel-server` (this workspace): runtime dep. Imports the
  same control types plus `is_valid_username`.

Transitively:

- `chan-writer/chan/chan-server`: depends on chan-tunnel-client and
  re-exports `is_valid_workspace_name`, `sanitize_workspace_name`, and
  `MAX_WORKSPACE_NAME_LEN` for the `chan serve` CLI to validate the
  user-typed workspace name before dialing.
- `chan-writer/chan-gateway/workspace-proxy`: depends on chan-tunnel-
  server at runtime; chan-tunnel-client is a dev-dependency only,
  used in `tests/api.rs` to workspace a fake `chan serve` against a
  real `serve_tunnel_listener` for end-to-end tests.

## 9. Open questions / future extensions

- Multi-workspace over a single tunnel. Today one h2 stream registers
  one `(user, workspace)`. A `Hello { workspaces: Vec<...> }` shape would
  amortise TLS/h2 setup for users running several `chan serve`
  instances. Requires a registry rework on the server to attribute
  inbound substreams to the right workspace.
- Negotiated frame cap. Both sides currently hard-code 64 KiB; a
  larger cap negotiated inside `Hello` would let future versions
  carry richer initial metadata without a protocol bump.
- Binary codec for `H2Duplex` flow-control hints. Today the adapter
  releases `RecvStream` capacity per chunk; a richer scheme could
  pace based on yamux backpressure. Not load-bearing at present
  traffic volumes.
