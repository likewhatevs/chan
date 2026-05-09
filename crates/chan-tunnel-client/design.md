# chan-tunnel-client: design

## Cross-crate context

chan-tunnel is split across three crates in `chan-writer/chan-core`:

- `chan-tunnel-proto`: pure wire types (`Hello`, `HelloAck`,
  `ProtocolVersion`), the framing codec, the drive-name and
  username validators, and `H2Duplex`. See
  [`chan-tunnel-proto/design.md`](../chan-tunnel-proto/design.md)
  for byte-level details.
- `chan-tunnel-client` (this crate): dials the terminator, runs
  the Hello round-trip, multiplexes yamux substreams onto an axum
  router. Embedded into `chan serve`.
- `chan-tunnel-server`: terminator library, consumed by
  `chan-gateway/drive-proxy`. Owns `Validator`, `Registry`, and
  the public-facing router.

End-to-end shape: `chan serve` calls `chan_tunnel_client::run(cfg,
router)`. `run` POSTs to `{tunnel-host}/v1/tunnel` over h2/TLS,
exchanges Hello / HelloAck through the resulting bidirectional
stream, then yamux-multiplexes per-request substreams. drive-proxy
on the public side accepts the connection in
`serve_tunnel_listener`, registers the drive in its `Registry`, and
opens fresh substreams to forward public requests.

This document is the dial and handshake reference. The wire format
itself lives in chan-tunnel-proto's design.md.

## 1. Problem and scope

A user running `chan serve` on a laptop wants their drive at
`drive.chan.app/{user}/{drive}` without opening a port or
configuring DNS. The constraint is "dial out only, HTTPS only."
The shape that fits is one long-lived `POST /v1/tunnel` carrying
yamux frames after a short handshake.

This crate owns:

- The TLS + h2 dial path (rustls with native roots, ALPN h2).
- The h2c branch for local dev / in-cluster stacks (loopback warning
  for non-loopback http:// hosts).
- The Hello / HelloAck round-trip over the resulting duplex.
- yamux client mode over the post-handshake byte stream.
- Per-substream HTTP/1.1 service via hyper, fed by a user-supplied
  `axum::Router`.
- A reconnect loop with exponential backoff and per-attempt timeout.
- A `tokio::sync::mpsc` event channel for "connected / disconnected
  / dial failed" UI hooks.

Out of scope:

- The wire format (chan-tunnel-proto).
- Server-side validation, registry, or public routing
  (chan-tunnel-server).
- Token acquisition (the user fetches it; this crate consumes it).

## 2. Architecture overview

```
+-----------------------+                +--------------------+
|  ClientConfig         |                |   axum::Router     |
|  (url, token, drive,  |                |  (provided by the  |
|   public, backoff,    |                |   embedder, e.g.   |
|   timeout, events)    |                |   chan serve's     |
+-----------------------+                |   inner app)       |
            |                            +--------------------+
            v                                    ^
       run(cfg, router)                          |
            |                                    |
            v                                    |
   +------------------+   tcp + tls + h2  +-----+--------+
   | dial_with_tls    |------------------>| serve_       |
   |  - resolve url   |    H2Duplex       |  substreams  |
   |  - TLS (h2 ALPN) |    (proto crate)  |  (yamux loop)|
   |  - h2 client     |                   +-----+--------+
   |  - Authorization |                         |
   |  - read 200/4xx  |                         | per inbound
   |  - handshake()   |                         | yamux::Stream
   +------------------+                         v
            |                       hyper h1 server (per stream)
            v                            with_upgrades()
   (Registration,                            |
    yamux Client)                            v
                                       axum router oneshot
```

Connection lifecycle:

1. `run` builds the rustls config once (caches native roots).
2. Loop: `dial_with_tls` opens TCP, runs TLS, runs h2, sends
   `POST /v1/tunnel` with `Authorization: Bearer <token>`, awaits
   the 200 response, wraps `(SendStream, RecvStream)` in
   `H2Duplex`, runs `handshake()`.
3. On success: emits `Connected(Registration)`, resets backoff,
   calls `serve_substreams` which polls the yamux connection until
   it ends.
4. On failure or disconnect: emits `DialFailed { error, retry_in }`
   or `Disconnected { retry_in }`, sleeps `backoff` (capped at
   `max_backoff`), doubles backoff, loops.

## 3. Components / responsibilities

| File         | Owns                                              |
|--------------|---------------------------------------------------|
| `lib.rs`     | `ClientConfig`, `Registration`, `TunnelEvent`,    |
|              | `ClientError`, `handshake`, `serve_substreams`,   |
|              | `run`                                             |
| `dial.rs`    | TLS + h2 connect, request POST, status mapping,   |
|              | `H2Duplex` construction, `build_tls_config`       |

`handshake` and `serve_substreams` are free functions over a
generic `S: AsyncRead + AsyncWrite + Unpin + Send + 'static` so
the wire test (in chan-gateway/drive-proxy/tests/api.rs) can pass
a `tokio::io::duplex` half and exercise the Hello round-trip
without standing up TLS. The same generic lets `dial` pass an
`H2Duplex` produced from a real h2 stream.

### Per-substream serving

For each inbound yamux substream (`yconn.poll_next_inbound`):

1. Wrap the futures-io stream into tokio via `compat()`, then into
   hyper's IO via `TokioIo::new`.
2. Run `hyper::server::conn::http1::Builder::serve_connection(io,
   service).with_upgrades()`. The `with_upgrades()` is required so
   WebSocket 101 responses keep the substream alive.
3. The service is a `tower::service_fn` that converts hyper's
   `Request<Incoming>` into `Request<axum::body::Body>` and runs
   the user's router via `tower::ServiceExt::oneshot`.

Each substream is one logical HTTP request from the public side.
Stacking h2 here would be mux-on-mux; h1 over yamux is the right
shape.

### Reconnect loop and backoff

Exponential, doubled per attempt, capped at `max_backoff`. Reset
to `initial_backoff` after a successful registration (not after
just the TCP connect, so a server that 200s and then immediately
closes still backs off). `dial_timeout` wraps the entire single
attempt; without it, an unreachable host hangs each attempt for
the OS TCP timeout (minutes), defeating the backoff. Default 30s
covers trans-pacific with margin.

### TunnelEvent channel

`ClientConfig::events` is an `Option<mpsc::Sender<TunnelEvent>>`.
Backpressure: `run` uses `try_send`, so a slow consumer drops
events rather than blocking the dial loop. The events are tee
material for logs and a UI; missing one isn't load-bearing.

## 4. Public API surface

```rust
pub struct ClientConfig {
    pub tunnel_url: Url,
    pub token: String,
    pub drive: String,
    pub client_version: String,
    pub public: bool,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub dial_timeout: Duration,
    pub events: Option<mpsc::Sender<TunnelEvent>>,
}
impl Default for ClientConfig { /* tunnel.chan.app/v1/tunnel */ }

pub struct Registration {
    pub prefix: String,
    pub user: String,
    pub drive: String,
}

pub enum TunnelEvent {
    Connected(Registration),
    Disconnected { retry_in: Duration },
    DialFailed { error: String, retry_in: Duration },
}

pub enum ClientError {
    InvalidUrl(String),
    Tls(String),
    Io(std::io::Error),
    Handshake(String),
    TransportClosed,
}

pub async fn dial(cfg: &ClientConfig)
    -> Result<(Registration, YamuxConnection<Compat<H2Duplex>>),
              ClientError>;

pub async fn dial_with_tls(
    cfg: &ClientConfig,
    tls: Option<&Arc<RustlsClientConfig>>,
) -> Result<(Registration, YamuxConnection<Compat<H2Duplex>>),
            ClientError>;

pub fn build_tls_config() -> Result<RustlsClientConfig, ClientError>;

pub async fn handshake<S>(cfg: &ClientConfig, socket: S)
    -> Result<(Registration, YamuxConnection<Compat<S>>), ClientError>
    where S: AsyncRead + AsyncWrite + Unpin + Send + 'static;

pub async fn serve_substreams<S>(
    conn: YamuxConnection<S>,
    router: axum::Router,
) -> Result<(), ClientError>
    where S: futures::AsyncRead + futures::AsyncWrite
           + Unpin + Send + 'static;

pub async fn run(cfg: ClientConfig, router: axum::Router)
    -> Result<(), ClientError>;
```

`run` is the long-lived future. Dropping it cancels everything
(yamux, the h2 driver task, the in-flight dial). It returns only on
configuration errors that retrying cannot recover from (empty token,
invalid drive name, unsupported URL scheme).

## 5. Wire format / framing

The wire format is owned by chan-tunnel-proto. See
[`chan-tunnel-proto/design.md`](../chan-tunnel-proto/design.md)
sections 2 and 5 for the byte layout, the JSON envelope rationale,
the 64 KiB cap, and `H2Duplex`.

Client-specific notes:

- Request: `POST {tunnel_url}` with `Authorization: Bearer <token>`,
  empty body. The "body" is the bidirectional h2 stream the
  handshake then runs over.
- Response codes that `dial` recognises:
  - `200 OK`: handshake proceeds.
  - `401 UNAUTHORIZED`: bad token. Mapped to
    `ClientError::Handshake("unauthorized (bad token)")`.
  - `403 FORBIDDEN`: token missing tunnel scope. Mapped to
    `ClientError::Handshake("forbidden (token missing tunnel scope)")`.
  - anything else: `ClientError::Handshake("unexpected status ...")`.
- After the response headers arrive, `H2Duplex::new(send, recv)`
  becomes the duplex; `handshake` writes the `Hello` and reads the
  `HelloAck`.
- yamux client mode (`Mode::Client`) over the duplex. Substreams
  are inbound; the client never opens outbound ones.

## 6. Trust boundaries / validation

- **Server certificate**: rustls with `rustls-native-certs` for the
  trust store, ALPN forced to `h2`. `build_tls_config` is built
  once and reused across reconnects (the macOS keychain walk is
  expensive).
- **URL scheme gate**: only `https://` and `http://` are accepted.
  `http://` against a non-loopback host logs a warning (bearer
  token in cleartext); we don't refuse outright because legitimate
  cases exist (private VPN, Tailscale, in-cluster service).
- **Drive name** (`is_valid_drive_name` from chan-tunnel-proto):
  checked before sending `Hello`. The server checks again, but
  catching it locally avoids a needless round-trip and surfaces a
  config error to the user.
- **Token**: empty token is rejected by `run` before the first
  dial. The token itself is opaque to this crate; the server's
  `Validator` decides whether it's valid.

## 7. Error model

Single umbrella enum `ClientError` with five primitive variants
(see section 4). `From` impls flatten `chan_tunnel_proto::FrameError`
and `IoFrameError` through `Display` so the public surface stays
free of `h2::Error`, `serde_json::Error`, and `rustls::Error`.

`run` itself returns `Result<(), ClientError>` and only errors on
non-recoverable misconfiguration; transient failures (TLS, h2,
401, network) loop with backoff and are surfaced through
`TunnelEvent::DialFailed` instead.

## 8. Consumers

- `chan-writer/chan/chan-server`: runtime dep. The `chan serve`
  CLI calls `chan_tunnel_client::run(cfg, router)` to expose its
  inner axum app on a public URL. Consumes `ClientConfig`,
  `TunnelEvent`, and `Registration` to wire the prefix and to
  surface "connected / retrying" status to the operator.
- `chan-writer/chan-gateway/drive-proxy`: dev-dependency only. The
  end-to-end test in `drive-proxy/tests/api.rs` uses
  `handshake` + `serve_substreams` to drive a fake `chan serve`
  against a real `chan_tunnel_server::serve_tunnel_listener`,
  validating the full wire path without needing a separate process.

## 9. Open questions / future extensions

- Per-leg dial timeouts. Today `dial_timeout` is a single global
  cap on TCP + TLS + h2 + Hello. Splitting into TCP / TLS / Hello
  legs would give better diagnostics (which step stalled) but
  multiplies the config surface; punted until operators ask for it.
- TLS session resumption. The current `build_tls_config` does not
  configure a session store; every reconnect re-runs the full TLS
  handshake. For a host that flaps frequently, a small in-process
  resumption store would shave a round trip.
- HTTP/2 keep-alive (PING). h2 has its own keepalive but rustls
  does not push it through; long idle periods can stall behind
  intermediaries that drop quiet TCP. Today we rely on traffic;
  add explicit pings if idle-NAT becomes an issue.
- uniffi shim. The errors are flat and types are owned, but no
  uniffi binding exists yet; the path is "wrap `run` in a
  callback-based handle" once Swift / Kotlin shells land.
