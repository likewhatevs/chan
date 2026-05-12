# chan-tunnel-server: design

## Cross-crate context

chan-tunnel is split across three crates in `chan-writer/chan-core`:

- `chan-tunnel-proto`: pure wire types (`Hello`, `HelloAck`,
  `ProtocolVersion`), framing codec, drive-name and username
  validators, `H2Duplex`. See
  [`chan-tunnel-proto/design.md`](../chan-tunnel-proto/design.md)
  for byte-level details.
- `chan-tunnel-client`: dial side, embedded into `chan serve`.
- `chan-tunnel-server` (this crate): library form of the
  terminator. Consumed in-process by `chan-gateway/drive-proxy`,
  which supplies the `Validator`, mounts the listener and the
  public router, and runs behind nginx for TLS.

End-to-end shape: `chan serve` calls `chan_tunnel_client::run(cfg,
router)` which dials `{tunnel-host}/v1/tunnel`. nginx terminates
TLS at `drive.chan.app` and `grpc_pass`-es `/v1/tunnel` as h2c to
`serve_tunnel_listener`. Each accepted connection becomes a yamux
session managed by a per-tunnel driver task and indexed in the
shared `Registry`. The wildcard router (mounted at e.g.
`*.drive.chan.app`) parses `{user}` out of the host header, looks
up the `TunnelHandle` for `(user, drive)`, opens a fresh outbound
substream, and runs hyper h1 client over it to forward the request
(with WebSocket upgrade bridging).

This document covers terminator-side design. The wire format is in
chan-tunnel-proto's design.md.

## 1. Problem and scope

The terminator side of chan-tunnel needs to:

- Accept long-lived h2c POSTs from arbitrary `chan serve` clients.
- Authenticate the bearer token before committing to the body, so
  bad-token failures return 401 / 403 distinctly (not as a
  generic handshake error after a 200).
- Run the Hello / HelloAck round-trip and bind the registration to
  `(validated_user, requested_drive)`.
- Multiplex per-public-request substreams over the resulting yamux
  session.
- Expose live tunnels to a public-facing axum router so the gateway
  can route `drive.chan.app/{user}/{drive}/...` at the registered
  peer.
- Tolerate flap (a `chan serve` restart should reclaim its drive
  without waiting for a TCP timeout).

Out of scope:

- TLS termination. nginx does it; this crate runs h2c.
- Token issuance / identity. The `Validator` trait is the seam.
- Persistence. The registry is in-memory; a restart drops every
  tunnel and clients reconnect.
- Wire format (chan-tunnel-proto).

## 2. Architecture overview

```
                 nginx (drive.chan.app/v1/tunnel, TLS, grpc_pass)
                              |
                              v h2c
                 +---------------------------+
                 | serve_tunnel_listener     |
                 |  - TCP accept             |
                 |  - h2::server handshake   |
                 |  - 1st stream: POST       |
                 |    /v1/tunnel + Bearer    |
                 |  - validator.validate()   |
                 |    [BEFORE 200]           |
                 |  - 200, then              |
                 |    handshake_validated()  |
                 +-------------+-------------+
                               |
                               v
                       (Hello, Validated,
                        YamuxConnection)
                               |
                               v
                 +---------------------------+
                 | drive_tunnel (per-tunnel) |
                 |  - owns yamux conn        |
                 |  - serves OpenRequest     |
                 |    -> outbound substream  |
                 |  - shutdown on eviction   |
                 +-------------+-------------+
                               |
              +----------------+-----------------+
              |                                  |
              v                                  v
      +---------------+                  +---------------+
      |  Registry     | <----- get ----- |  public_router|
      | (user, drive) |     TunnelHandle |  on drive.    |
      |  -> handle    |                  |  chan.app     |
      +---------------+                  +-------+-------+
                                                 |
                                                 v
                              hyper h1 client over yamux::Stream
                                  (forward + upgrade bridging)
```

## 3. Components / responsibilities

| File           | Owns                                            |
|----------------|-------------------------------------------------|
| `lib.rs`       | `Validator`, `Validated`, `ServerError`,        |
|                | `handshake`, `handshake_validated`,             |
|                | `tunnel_yamux_config`, `HELLO_READ_TIMEOUT`     |
| `tunnel.rs`    | `serve_tunnel_listener`, `handle_tunnel_conn`,  |
|                | `extract_bearer`                                |
| `driver.rs`    | `drive_tunnel`: per-tunnel task that owns the   |
|                | yamux connection                                |
| `registry.rs`  | `Registry`, `TunnelHandle`, `DriveInfo`,        |
|                | `TunnelInfo`, `OpenError`, eviction policy      |
| `public.rs`    | `public_router`, `public_router_with`,          |
|                | `PublicConfig`, request rewriting, upgrade      |
|                | bridging, idle watchdog                         |

### Listener flow (`tunnel.rs`)

`serve_tunnel_listener(listener, validator, registry,
max_drives_per_user)`:

1. `TcpListener::accept`. Try to acquire one permit from a
   per-listener `Semaphore::new(MAX_INFLIGHT_HANDSHAKES)` (1024).
   If the semaphore is empty, the TCP socket is dropped and the
   loop continues; this bounds memory against floods of half-open
   peers that have not yet hit a per-stage timeout. Otherwise
   spawn `handle_tunnel_conn` carrying the owned permit.
2. `h2::server::handshake(tcp)` under `H2_HANDSHAKE_TIMEOUT` (10s).
3. First `conn.accept()` under `FIRST_STREAM_TIMEOUT` (10s).
4. Reject `(method != POST) || (path != TUNNEL_PATH)` with 404.
5. Parse `Authorization: Bearer ...` (case-insensitive scheme,
   trimmed token); reject missing / empty with 401.
6. Spawn an h2 frame driver task BEFORE awaiting the validator:
   the validator may be a network round-trip and h2 only progresses
   while polled. The task rejects any subsequent stream on the
   same connection with 409 (clients must only ever open one) and
   `abrupt_shutdown(ENHANCE_YOUR_CALM)` after
   `MAX_DRAINER_REJECTIONS` (16) rejections.
7. Call `validator.validate(token).await` under `VALIDATE_TIMEOUT`
   (10s, independent of any timeout the `Validator` impl enforces
   internally). On timeout, reply 504. On error: 401
   (`InvalidToken`), 502 (`Identity`), or 500. Bare 401 / 403
   responses arrive at the client as distinct `ClientError`
   variants; collapsing them into a generic 200-then-close hid
   auth failures behind transport failures.
8. Verify the validated token's `scopes` contains `"tunnel"`; 403
   otherwise.
9. Send 200 (response headers, body open). Wrap `(SendStream,
   recv_body)` in `H2Duplex`.
10. `handshake_validated(duplex, validated, pre_ack)`:
   - Defense-in-depth username check (`is_valid_username`).
   - `read_frame::<Hello>` with `HELLO_READ_TIMEOUT` (15s) bound.
   - Reject non-V1 protocol; reject invalid drive name.
   - Run `pre_ack(&hello, &validated)` for post-validate policy
     (e.g. per-user drive cap).
   - Build and write `HelloAck { prefix: "/{user}/{drive}", ... }`.
   - Wrap the duplex in yamux server mode with
     `tunnel_yamux_config()` (max 256 concurrent substreams).
11. `registry.register(user, drive, public, peer_addr)` returns
    a `TunnelHandle`, the open-request `mpsc::Receiver`, and the
    eviction `oneshot::Receiver`. The in-flight semaphore permit
    is dropped here: the per-tunnel driver runs without holding
    one so a long-lived tunnel does not consume an accept slot.
12. `drive_tunnel(...)` runs until close or eviction. On exit,
    `registry.deregister_if_owner(&handle)`.

### Driver loop (`driver.rs`)

One task per registered tunnel. Owns the yamux `Connection`.
Three concerns merged into a single `poll_fn`:

- Shutdown takes priority. The `oneshot::Receiver` resolves either
  on explicit `()` send or sender drop (the registry drops it on
  eviction). Either signal exits the loop and `poll_close`s yamux.
- Drain pending `OpenRequest`s from the public router and call
  `poll_new_outbound`; reply with the new substream over the
  oneshot in the request.
- Poll for inbound substreams. The protocol does not use them;
  any inbound substream is logged and dropped (yamux RSTs it on
  the next poll).

`poll_fn` rather than `select!` because two of the three branches
need `&mut conn` and `select!` over multiple `poll_fn`s holding
that borrow conflicts.

### Registry (`registry.rs`)

- `HashMap<(Arc<str>, Arc<str>), Entry>` under `parking_lot::Mutex`.
- `Entry { handle: TunnelHandle, _shutdown_tx: oneshot::Sender<()> }`.
  Dropping the entry drops the sender, which wakes the per-tunnel
  driver's receiver, which closes yamux.
- Collision: last-writer-wins. `register` evicts any prior entry
  for the same key, logs the prior age, and returns the new
  handle. This matches "chan-serve restart reclaims its drive."
- `TunnelHandle::open()` sends an `OpenRequest`
  (`oneshot::Sender<Result<yamux::Stream, OpenError>>`) over the
  per-tunnel mpsc and awaits the reply. Returns
  `OpenError::Disconnected` if either channel is gone.
- `deregister_if_owner` removes the entry only if it still points
  at the same handle, so a driver shutting down after eviction
  can't accidentally remove its successor.
- Admin views: `list_drives_for(user)`, `list_all()` for the
  drive-proxy dashboard and `tunnel ps`-style admin tooling.

### Public router (`public.rs`)

`public_router(registry)` builds an `axum::Router` with three
routes (`/{user}/{drive}`, `/{user}/{drive}/`,
`/{user}/{drive}/*rest`) mounted on `any` method. All three call
`proxy(...)`:

1. `registry.get(user, drive)` returns a `TunnelHandle`, else 502
   ("tunnel not connected").
2. `handle.open().await` returns a `yamux::Stream`, else 502
   ("tunnel disconnected").
3. `hyper::client::conn::http1::handshake(io)` over the substream;
   spawn the conn driver with `with_upgrades()`.
4. Pre-extract `OnUpgrade` from the public request *before*
   forwarding so it isn't lost when the body is moved.
5. `build_forwarded`: rewrite path (drop `/{user}/{drive}` prefix),
   strip URI scheme/authority (h1 over a substream doesn't use
   them), append `X-Forwarded-For` (chained, not clobbered), set
   `X-Forwarded-Proto` (honour upstream value, default `https`),
   set `X-Forwarded-Host` (from original `Host`).
6. `sender.send_request(forwarded).await` yields the response.
7. If status is `101 SWITCHING_PROTOCOLS`: pre-extract the tunnel-
   side `OnUpgrade`, spawn a task that awaits both upgrade
   futures, wraps each in an `Activity` adapter (bumps an atomic
   on every byte that moved), and runs `copy_bidirectional` with a
   watchdog that tears down the bridge after
   `UPGRADE_IDLE_TIMEOUT` (5 minutes) of inactivity.

The `Activity` watchdog uses `Instant::elapsed()` (monotonic) to
avoid wall-clock jumps (NTP slew, suspend/resume) registering as
activity.

### Why h1 over yamux, not h2

The substream is already a multiplexed channel; running h2 inside
would be mux-on-mux. h1 maps cleanly: one substream is one
request. WebSocket upgrades work with `with_upgrades()`. Body
streaming works through the yamux flow-control window.

### Why h2c (not TLS) on the listener

nginx is the TLS terminator at `drive.chan.app` and forwards h2c
via `grpc_pass` on the `/v1/tunnel` path only. Running rustls again
here would duplicate trust config and complicate cert rotation. For
local dev or other deployments the host can put any TLS layer in
front; the listener itself is h2c-only.

## 4. Public API surface

```rust
// Validator seam
#[async_trait]
pub trait Validator: Send + Sync + 'static {
    async fn validate(&self, token: &str)
        -> Result<Validated, ServerError>;
}

pub struct Validated {
    pub user_id: uuid::Uuid,
    pub username: String,
    pub scopes: Vec<String>,
}

pub enum ServerError {
    InvalidToken,
    MissingScope,
    MissingPublicScope,
    Identity(String),
    Io(std::io::Error),
    Handshake(String),
    TooManyDrives { user: String, max: usize },
}

pub const TUNNEL_SCOPE: &str = "tunnel";
pub const TUNNEL_PUBLIC_SCOPE: &str = "tunnel.public";

// Handshake free functions
pub async fn handshake<S, V, F>(
    socket: S, token: &str, validator: &V, pre_ack: F,
) -> Result<(Hello, Validated, YamuxConnection<Compat<S>>), ServerError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    V: Validator + ?Sized,
    F: FnOnce(&Hello, &Validated) -> Result<(), ServerError>;

pub async fn handshake_validated<S, F>(
    socket: S, validated: Validated, pre_ack: F,
) -> Result<(Hello, Validated, YamuxConnection<Compat<S>>), ServerError>;

// Listener
pub async fn serve_tunnel_listener(
    listener: TcpListener,
    validator: Arc<dyn Validator>,
    registry: Arc<Registry>,
    max_drives_per_user: usize,
) -> std::io::Result<()>;

// Public router
pub fn public_router(registry: Arc<Registry>) -> axum::Router;
pub fn public_router_with(
    registry: Arc<Registry>, cfg: PublicConfig,
) -> axum::Router;

pub struct PublicConfig {
    pub request_body_cap: usize,
    pub trust_forwarded_for: bool,
    pub allowed_host_suffixes: Vec<String>,
    pub upstream_request_timeout: Duration,
}
pub const DEFAULT_REQUEST_BODY_CAP: usize = 10 * 1024 * 1024;
pub const DEFAULT_UPSTREAM_REQUEST_TIMEOUT: Duration =
    Duration::from_secs(30);

// Registry
pub struct Registry { /* ... */ }
impl Registry {
    pub fn new() -> Arc<Self>;
    pub fn get(&self, user: &str, drive: &str) -> Option<TunnelHandle>;
    pub fn list_drives_for(&self, user: &str) -> Vec<DriveInfo>;
    pub fn list_all(&self) -> Vec<TunnelInfo>;
    pub fn evict(&self, user: &str, drive: &str) -> bool;
}

#[derive(Clone)]
pub struct TunnelHandle {
    pub user: Arc<str>,
    pub drive: Arc<str>,
    pub public: bool,
    pub peer_addr: Option<SocketAddr>,
    pub connected_at: DateTime<Utc>,
    /* + open_tx: mpsc::Sender<OpenRequest> */
}
impl TunnelHandle {
    pub async fn open(&self) -> Result<yamux::Stream, OpenError>;
}

pub enum OpenError { Disconnected }

pub struct DriveInfo {
    pub drive: Arc<str>,
    pub public: bool,
    pub peer_addr: Option<SocketAddr>,
    pub connected_at: DateTime<Utc>,
}

pub struct TunnelInfo {
    pub user: Arc<str>,
    pub drive: Arc<str>,
    pub public: bool,
    pub peer_addr: Option<SocketAddr>,
    pub connected_at: DateTime<Utc>,
}
```

## 5. Wire format / framing

The wire format is owned by chan-tunnel-proto. See
[`chan-tunnel-proto/design.md`](../chan-tunnel-proto/design.md)
sections 2 and 5 for the byte layout, the JSON envelope rationale,
the 64 KiB cap, and `H2Duplex`.

Server-specific notes:

- The 200 response is sent BEFORE the framed `Hello` is read but
  AFTER the validator runs. This split is the reason
  `handshake_validated` exists alongside `handshake`: the listener
  needs to fail with 401 / 403 prior to committing to the body.
- `HELLO_READ_TIMEOUT = 15s` bounds slow-loris-style peers that
  connect, get the 200, and never frame a `Hello`. 15s is plenty
  for trans-pacific; tighter would risk false positives on slow
  mobile uplinks.
- `tunnel_yamux_config()` overrides yamux's upstream default of
  8192 max concurrent streams down to 256. Per-tunnel cap; a
  visitor opening many slow requests is bounded.
- `pre_ack(&hello, &validated)` runs after the Hello is read and
  validated and before the `HelloAck` is written. The listener
  uses it to enforce `max_drives_per_user`: if registering this
  drive would exceed the cap and the user doesn't already have it
  registered, return `ServerError::TooManyDrives`. Reconnect of an
  existing drive always passes (the eviction step removes the old
  entry first).

## 6. Trust boundaries / validation

- **Token authentication**: the consumer's `Validator` impl is
  the only authority. This crate calls it; on success it gets a
  `Validated { user_id, username, scopes }`. Order is fixed:
  validator runs *before* the 200 response so 401 / 403 propagate
  to the client distinctly. After 200, the handshake cannot
  surface auth errors.
- **Tunnel scope**: the validator returns scopes; the listener
  refuses tokens missing `TUNNEL_SCOPE` (`"tunnel"`) with 403.
- **Public scope**: `Hello.public = true` is a privilege-escalation
  request (the public router skips the OAuth gate for that drive),
  so it is gated on a second scope `TUNNEL_PUBLIC_SCOPE`
  (`"tunnel.public"`). Tokens that hold only the base scope can
  still register a drive but must run it private; if they request
  `public = true` the handshake fails with `MissingPublicScope`
  *before* HelloAck is written, so the client cannot grant the bit
  to itself at runtime. A token carrying both scopes retains
  per-drive choice: `chan serve --public` (true) or default
  (false) both work on the same token, so one user can host both
  a public docs drive and a private notes drive.
- **Username validation** (`is_valid_username`): defense-in-depth.
  The username flows into the public path `/{user}/{drive}`; if
  the upstream identity service ever emits `..`, slashes, or
  whitespace, the public router would mis-route. The handshake
  refuses any username that wouldn't be URL-safe.
- **Drive name validation** (`is_valid_drive_name`): every Hello's
  `drive` field is checked; clients pre-check too but we don't
  trust them.
- **Method / path gate**: 404 for anything other than `POST
  /v1/tunnel`. The drainer task continues to reject additional
  streams on the same connection with 409.
- **Bearer parsing**: scheme name is case-insensitive (RFC 6750);
  empty / whitespace-only tokens are rejected.
- **Body cap on the public side**: `DEFAULT_REQUEST_BODY_CAP` is
  10 MiB via `tower_http::limit::RequestBodyLimitLayer`. Operators
  override via `PublicConfig::request_body_cap`. Without a cap a
  public client could stream gigabytes through to chan-serve
  (paid for in tunnel egress and chan-serve memory).
- **Upgrade idle timeout**: hijacked WebSockets are torn down
  after `UPGRADE_IDLE_TIMEOUT` (5 min) of no bytes either way.
  Keeps a public client that 101'd and went silent from pinning
  the substream forever.
- **Listener back-pressure cap**: at most
  `MAX_INFLIGHT_HANDSHAKES` (1024) connections may sit in the
  authenticate-and-handshake stages simultaneously. Above that the
  TCP socket is closed immediately so a flood of half-open peers
  cannot exhaust memory.
- **Public-side host allowlist**: when
  `PublicConfig::allowed_host_suffixes` is non-empty, the public
  router replies 421 Misdirected Request to any request whose
  `Host` header does not end with one of the listed suffixes.
  Empty (default) trusts the fronting proxy's host routing. Used
  as a defence-in-depth wall if the public listener is ever
  exposed directly.
- **Upstream request timeout**: the public router caps the time
  spent on the h1 handshake plus the wait for response headers
  against the registered chan-serve via
  `PublicConfig::upstream_request_timeout` (default 30s); a 504
  Gateway Timeout is returned on miss. Body streaming after
  headers is intentionally uncapped so long downloads / uploads
  are not artificially limited.
- **Forwarded-header sanitisation**: the public router strips
  `Forwarded`, `X-Forwarded-Proto`, `X-Forwarded-Host`, `X-Real-IP`,
  `Proxy-Authorization`, and `Proxy-Authenticate` from incoming
  requests before re-injecting its own `X-Forwarded-*`. The public
  side does not get to dictate any of these to chan-serve.
  `X-Forwarded-For` is the one knob: when
  `PublicConfig::trust_forwarded_for` is `false` (default), the
  incoming value is discarded and downstream sees only the
  ConnectInfo IP. Operators behind a proxy that already overwrites
  XFF with the real client address (e.g. nginx
  `proxy_set_header X-Forwarded-For $remote_addr`) can set the
  flag to `true` so chan-serve sees the real source IP appended
  with the proxy hop. Trusting the value when the upstream uses
  `$proxy_add_x_forwarded_for` lets a public client spoof its
  source IP, so the default is the safe one.

## 7. Error model

Single umbrella enum `ServerError` with six primitive variants
(see section 4). Conversions from `chan_tunnel_proto::FrameError`
and `IoFrameError` flatten through `Display`, so the public
surface stays free of `h2::Error`, `serde_json::Error`, and
`yamux::Error`.

`OpenError::Disconnected` is the single failure mode of
`TunnelHandle::open()`: either the request channel is gone (the
driver has already exited) or the reply channel was dropped (the
driver couldn't allocate the substream because yamux is closing).
Public-side callers map both into 502.

## 8. Consumers

- `chan-writer/chan-gateway/drive-proxy`: runtime dep. Wires this
  crate end-to-end:
  - `serve_tunnel_listener` on the h2c listener that nginx
    `grpc_pass`-es into.
  - An `IdentityValidator` that calls the gateway's identity
    service to validate bearer tokens.
  - A wrapping `registry::Registry` (drive-proxy's own struct)
    over `chan_tunnel_server::Registry`, used by the proxy and
    admin handlers to look up tunnels and render the dashboard.
  - `public_router`-style proxying inside `proxy.rs` (drive-proxy
    has its own forward-proxy layer that calls `TunnelHandle::open`
    directly rather than mounting `public_router`; the latter is
    a turn-key alternative for hosts that don't need custom
    middleware).
- `chan-writer/chan-gateway/drive-proxy` (dev only): pulls
  `chan-tunnel-client` as a dev-dependency so its end-to-end
  test can drive a fake `chan serve` against a real listener.

## 9. Open questions / future extensions

- Persistent registry. Today a drive-proxy restart drops every
  tunnel and clients reconnect. A small on-disk index would let
  the public router serve `tunnel offline since X` errors with
  context instead of a bare 502 during a restart.
- Per-tunnel quotas. `max_drives_per_user` caps drive count; it
  doesn't cap concurrent in-flight requests, total bandwidth, or
  request rate. `tower-http`'s `RateLimitLayer` would slot in on
  the public router but needs a key strategy (per-tunnel? per-
  visitor?).
- Multi-drive per tunnel. See chan-tunnel-proto's design.md
  section 9; would change the registry shape from
  `(user, drive) -> handle` to `(user, drive) -> (handle,
  multiplex_id)`.
- Health probe on the substream. The driver currently learns
  about a dead peer when yamux's keepalive fires or an `open`
  fails. An explicit application-level ping over a control
  substream would give the public router faster failover.
- uniffi shim. Same status as the client crate: errors and types
  are FFI-shaped, no bindings yet.
