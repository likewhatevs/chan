# chan-tunnel-client: design

## Cross-crate context

chan-tunnel is split across three crates under `crates/` in this repository:

- `chan-tunnel-proto`: pure wire types (`Hello`, `HelloAck`, `ProtocolVersion`, the `error_code` refusal constants), the framing codec, the workspace-name and username validators, and `H2Duplex`. See [`chan-tunnel-proto/design.md`](../chan-tunnel-proto/design.md) for byte-level details.
- `chan-tunnel-client` (this crate): dials the terminator, runs the Hello round-trip, multiplexes yamux substreams onto an axum router. Embedded into `chan-server` (`crates/chan-server`) and driven by `chan devserver`.
- `chan-tunnel-server`: terminator library, consumed by the gateway's `devserver-proxy`. Owns `Validator`, `Registry`, and the substream-forwarding seam.

End-to-end shape: `chan devserver` calls `chan_tunnel_client::run(cfg, router)`. `run` POSTs to `{tunnel-host}/v1/tunnel` over h2/TLS, exchanges Hello / HelloAck through the resulting bidirectional stream, then yamux-multiplexes per-request substreams. The terminator accepts the connection in `serve_tunnel_listener`, registers the devserver in its `Registry`, and opens fresh substreams to forward public requests.

This document is the dial and handshake reference. The wire format itself lives in chan-tunnel-proto's design.md.

## 1. Problem and scope

A user running `chan devserver` on a box wants their library reachable on a public URL without opening a port or configuring DNS. The constraint is "dial out only, HTTPS only." The shape that fits is one long-lived `POST /v1/tunnel` carrying yamux frames after a short handshake.

This crate owns:

- The TLS + h2 dial path (rustls with native roots, ALPN h2).
- The h2c branch for local dev / in-cluster stacks (loopback warning for non-loopback http:// hosts).
- An optional outbound HTTP proxy leg: HTTP/1.1 CONNECT (with Basic auth from the proxy URL's userinfo) before TLS/h2.
- The Hello / HelloAck round-trip over the resulting duplex, including structured server refusals.
- yamux client mode over the post-handshake byte stream.
- Per-substream HTTP/1.1 service via hyper, fed by a user-supplied `axum::Router`.
- A reconnect loop with jittered exponential backoff and a per-attempt timeout.
- A `tokio::sync::mpsc` event channel for "connected / disconnected / dial failed" UI hooks.

Out of scope:

- The wire format (chan-tunnel-proto).
- Server-side validation, registry, or public routing (chan-tunnel-server).
- Token acquisition (the user fetches it; this crate consumes it).

## 2. Architecture overview

```
+--------------------------+             +--------------------+
|  ClientConfig            |             |   axum::Router     |
|  (url, token, workspace, |             |  (provided by the  |
|   backoff, timeout,      |             |   embedder, e.g.   |
|   events, proxy,         |             |   chan devserver's |
|   substream cap)         |             |   inner app)       |
+--------------------------+             +--------------------+
            |                                    ^
            v                                    |
       run(cfg, router)                          |
            |                                    |
            v                                    |
   +------------------+   tcp + tls + h2  +-----+--------+
   | dial_with_tls    |------------------>| serve_       |
   |  - normalize url |    H2Duplex       |  substreams  |
   |  - CONNECT proxy |    (proto crate)  |  (yamux loop)|
   |    (optional)    |                   +-----+--------+
   |  - TLS (h2 ALPN) |                         |
   |  - h2 client     |                         | per inbound
   |  - Authorization |                         | yamux::Stream
   |  - read 200/4xx  |                         v
   |  - handshake()   |            hyper h1 server (per stream)
   +------------------+                 with_upgrades()
            |                                    |
            v                                    v
   (Registration,                      axum router oneshot
    yamux Client)
```

Connection lifecycle:

1. `run` validates the config (token present, workspace name valid, scheme https/http) and builds the rustls config once (caches native roots).
2. Loop: `dial_with_tls` opens TCP (optionally via an HTTP CONNECT proxy), runs TLS for https:// URLs, runs h2, sends `POST /v1/tunnel` with `Authorization: Bearer <token>`, awaits the 200 response, wraps `(SendStream, RecvStream)` in `H2Duplex`, runs `handshake()`.
3. On success: emits `Connected(Registration)`, resets backoff, calls `serve_substreams_with_limit` which polls the yamux connection until it ends, then emits `Disconnected`.
4. On failure: emits `DialFailed { error, retry_in }`.
5. Sleep a jittered backoff (+/- 20%), double the base (capped at `max_backoff`), loop.

## 3. Components / responsibilities

| File         | Owns                                              |
|--------------|---------------------------------------------------|
| `lib.rs`     | `ClientConfig`, `Registration`, `TunnelEvent`,    |
|              | `ClientError`, `handshake`, `serve_substreams`,   |
|              | `serve_substreams_with_limit`, `run`, jitter      |
| `dial.rs`    | TCP / CONNECT-proxy leg, TLS + h2 connect, URL    |
|              | normalization, request POST, status mapping,      |
|              | `H2Duplex` construction, `build_tls_config`       |

`handshake` and `serve_substreams` are free functions over a generic `S: AsyncRead + AsyncWrite + Unpin + Send + 'static` so wire tests (e.g. `gateway/crates/devserver-proxy/tests/api.rs`) can pass a duplex built from a raw h2 stream and exercise the Hello round-trip without standing up TLS. The same generic lets `dial` pass an `H2Duplex` produced from a real h2 stream.

### Per-substream serving

For each inbound yamux substream (`poll_next_inbound`):

1. Wrap the futures-io stream into tokio via `compat()`, then into hyper's IO via `TokioIo::new`.
2. Run `hyper::server::conn::http1::Builder::serve_connection(io, service).with_upgrades()`. The `with_upgrades()` is required so WebSocket 101 responses keep the substream alive.
3. The service is a `tower::service_fn` that converts hyper's `Request<Incoming>` into `Request<axum::body::Body>` and runs the user's router via `tower::ServiceExt::oneshot`.

Each substream is one logical HTTP request from the public side. Stacking h2 here would be mux-on-mux; h1 over yamux is the right shape.

`serve_substreams` caps concurrent handler tasks at `DEFAULT_MAX_CONCURRENT_SUBSTREAMS` (128) via a semaphore. `run` uses `ClientConfig::max_concurrent_substreams` (clamped to >= 1), and direct callers can use `serve_substreams_with_limit`. When the cap is full, the client stops polling new inbound yamux substreams until a handler exits, so floods backpressure at the mux instead of spawning unbounded h1 tasks.

### Reconnect loop and backoff

Exponential, doubled per attempt, capped at `max_backoff`. Reset to `initial_backoff` after a successful registration (not after just the TCP connect, so a server that 200s and then immediately closes still backs off). Each sleep is jittered by +/- 20% so a fleet of clients disconnected by the same server restart does not synchronise into a thundering herd; the jitter source is the low bits of the system clock, which avoids a `rand` dependency.

`dial_timeout` wraps the entire single attempt; without it, an unreachable host hangs each attempt for the OS TCP timeout (minutes), defeating the backoff. Default 30s covers trans-pacific with margin.

### Outbound HTTP proxy (CONNECT)

When `ClientConfig::proxy` is set, `open_tcp` connects to the proxy and issues an HTTP/1.1 `CONNECT host:port`, with `Proxy-Authorization: Basic ...` derived from the proxy URL's userinfo when present. The response headers are read byte-by-byte up to the `\r\n\r\n` terminator (hard-capped at 16 KiB) so no bytes belonging to the tunnelled upstream — the TLS ClientHello or h2 preface — are over-read by a buffered reader. Only 2xx CONNECT responses proceed. Only `http://` proxies are supported (plain CONNECT); HTTPS-to-proxy and SOCKS are out of scope. `HTTP_PROXY` / `NO_PROXY` env vars are not honoured automatically so embedders get a deterministic surface.

### TunnelEvent channel

`ClientConfig::events` is an `Option<mpsc::Sender<TunnelEvent>>`. Backpressure: `run` uses `try_send`, so a slow consumer drops events rather than blocking the dial loop. The events are tee material for logs and a UI; missing one isn't load-bearing.

## 4. Public API surface

```rust
pub const DEFAULT_MAX_CONCURRENT_SUBSTREAMS: usize = 128;

pub struct ClientConfig {
    pub tunnel_url: Url,
    pub token: String,
    pub workspace: String,
    pub client_version: String,
    pub public: bool,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub dial_timeout: Duration,
    pub events: Option<mpsc::Sender<TunnelEvent>>,
    pub proxy: Option<Url>,
    pub max_concurrent_substreams: usize,
}
impl Default for ClientConfig { /* devserver.chan.app/v1/tunnel */ }

pub struct Registration {
    pub prefix: String,
    pub user: String,
    pub workspace: String,
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
    RemoteRefusal { code: String, message: String },
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

pub async fn serve_substreams_with_limit<S>(
    conn: YamuxConnection<S>,
    router: axum::Router,
    max_concurrent_substreams: usize,
) -> Result<(), ClientError>
    where S: futures::AsyncRead + futures::AsyncWrite
           + Unpin + Send + 'static;

pub async fn run(cfg: ClientConfig, router: axum::Router)
    -> Result<(), ClientError>;
```

`run` is the long-lived future. Dropping it cancels everything (yamux, the h2 driver task, the in-flight dial). It returns only on configuration errors that retrying cannot recover from (empty token, invalid workspace name, unsupported URL scheme, no native CA roots available).

## 5. Wire format / framing

The wire format is owned by chan-tunnel-proto. See [`chan-tunnel-proto/design.md`](../chan-tunnel-proto/design.md) sections 2 and 5 for the byte layout, the JSON envelope rationale, the 64 KiB cap, and `H2Duplex`.

Client-specific notes:

- URL normalization: when the configured URL has no path (or just `/`), the dial path substitutes `chan_tunnel_proto::TUNNEL_PATH`, so callers can pass a bare `http://host:port` base and the wire constant stays single-sourced. A non-trivial path is preserved verbatim, so a typo like `/v2/tunnel` still surfaces as a visible 404 instead of being silently corrected.
- Request: `POST {tunnel_url}` with `Authorization: Bearer <token>`, empty body. The "body" is the bidirectional h2 stream the handshake then runs over.
- Response codes that `dial` recognises:
  - `200 OK`: handshake proceeds.
  - `401 UNAUTHORIZED`: bad token. Mapped to `ClientError::Handshake("unauthorized (bad token)")`.
  - `403 FORBIDDEN`: token missing tunnel scope. Mapped to `ClientError::Handshake("forbidden (token missing tunnel scope)")`.
  - anything else: `ClientError::Handshake("unexpected status ...")`.
- After the response headers arrive, `H2Duplex::new(send, recv)` becomes the duplex; `handshake` writes the `Hello` and reads the `HelloAck`. A `HelloAck::Refused` becomes `ClientError::RemoteRefusal { code, message }`; the codes are the `chan_tunnel_proto::error_code` strings, so callers can match known refusals and fall back to the message for unknown codes. A non-V1 ack protocol is a `Handshake` error.
- yamux client mode (`Mode::Client`) over the duplex. Substreams are inbound; the client never opens outbound ones.

## 6. Trust boundaries / validation

- **Server certificate**: rustls with `rustls-native-certs` for the trust store, ALPN forced to `h2`. `run` builds the TLS config once and reuses it across reconnects (the macOS keychain walk is expensive); `dial_with_tls` lets other callers do the same.
- **URL scheme gate**: only `https://` and `http://` are accepted. `http://` against a non-loopback host logs a warning (bearer token in cleartext); we don't refuse outright because legitimate cases exist (private VPN, Tailscale, in-cluster service, a loopback listener behind `ssh -R`).
- **Workspace name** (`is_valid_workspace_name` from chan-tunnel-proto): checked before sending `Hello`. The server checks again, but catching it locally avoids a round-trip and surfaces a config error to the user.
- **Token**: empty token is rejected by `run` before the first dial. The token itself is opaque to this crate; the server's `Validator` decides whether it's valid.
- **Proxy credentials**: taken from the proxy URL's userinfo and sent only as the Basic CONNECT header. CONNECT failure messages carry the numeric status but never echo proxy-supplied response text, so a hostile proxy cannot reflect credentials into logs.

## 7. Error model

Single umbrella enum `ClientError` with six variants (see section 4). `From` impls flatten `chan_tunnel_proto::FrameError` and `IoFrameError` through `Display` so the public surface stays free of `h2::Error`, `serde_json::Error`, and `rustls::Error`. `RemoteRefusal` is the one structured variant: it preserves the server's stable refusal code for UI matching.

`run` itself returns `Result<(), ClientError>` and only errors on non-recoverable misconfiguration; transient failures (TLS, h2, 401, network) loop with backoff and are surfaced through `TunnelEvent::DialFailed` instead.

## 8. Consumers

- `crates/chan-server`: runtime dep. `chan devserver --tunnel-token` calls `chan_tunnel_client::run(cfg, router)` to expose its inner axum app through the tunnel; consumes `TunnelEvent` and `Registration` to wire the prefix and surface "connected / retrying" status to the operator.
- `crates/chan-tunnel-server`: dev-dep. The e2e test (`tests/listener_e2e.rs`) uses `dial` to drive a real client against `serve_tunnel_listener` over localhost h2c.
- `gateway/crates/devserver-proxy`: dev-dep. `tests/api.rs` uses `handshake` + `serve_substreams` over a hand-built h2 stream to register a fake `chan devserver` against the proxy's real listener.

## 9. Open questions / future extensions

- Per-leg dial timeouts. Today `dial_timeout` is a single global cap on proxy CONNECT + TCP + TLS + h2 + Hello. Splitting into legs would give better diagnostics (which step stalled) but multiplies the config surface; punted until operators ask.
- TLS session resumption. `build_tls_config` does not configure a session store; every reconnect re-runs the full TLS handshake. For a host that flaps frequently, a small in-process resumption store would shave a round trip.
- HTTP/2 keep-alive (PING). Long idle periods can stall behind intermediaries that drop quiet TCP; today we rely on traffic. Add explicit pings if idle-NAT becomes an issue.
