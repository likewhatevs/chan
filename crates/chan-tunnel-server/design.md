# chan-tunnel-server: design

## Cross-crate context

chan-tunnel is split across three crates under `crates/` in this repository:

- `chan-tunnel-proto`: pure wire types (`Hello`, `HelloAck`, `ProtocolVersion`, `error_code`), framing codec, workspace-name and username validators, `H2Duplex`. See [`chan-tunnel-proto/design.md`](../chan-tunnel-proto/design.md) for byte-level details.
- `chan-tunnel-client`: dial side, embedded into `chan serve`.
- `chan-tunnel-server` (this crate): library form of the terminator. Two embedders: the gateway's `workspace-proxy` (`gateway/crates/workspace-proxy`), which supplies an identity-service `Validator` and runs behind nginx for TLS; and chan-desktop (`desktop/src-tauri`), which binds a loopback-only listener fed by `ssh -R` forwards.

End-to-end shape (gateway deployment): `chan serve` calls `chan_tunnel_client::run(cfg, router)` which dials `{tunnel-host}/v1/tunnel`. nginx terminates TLS and `grpc_pass`-es `/v1/tunnel` as h2c to `serve_tunnel_listener`. Each accepted connection becomes a yamux session managed by a per-tunnel driver task and indexed in the shared `Registry`. The public side looks up the `TunnelHandle` for `(user, workspace)`, opens a fresh outbound substream, and runs hyper h1 client over it to forward the request (with WebSocket upgrade bridging). This crate ships a turn-key path-routed `public_router` (`/{user}/{workspace}/...`); workspace-proxy instead mounts its own proxy layer that parses `{user}` out of the wildcard host (`{user}.workspace.chan.app`) and calls `TunnelHandle::open` directly.

This document covers terminator-side design. The wire format is in chan-tunnel-proto's design.md.

## 1. Problem and scope

The terminator side of chan-tunnel needs to:

- Accept long-lived h2c POSTs from arbitrary `chan serve` clients.
- Authenticate the bearer token before committing to the body, so bad-token failures return 401 / 403 distinctly (not as a generic handshake error after a 200).
- Run the Hello / HelloAck round-trip and bind the registration to `(validated_user, requested_workspace)`, emitting structured `HelloAck::Refused` frames for policy failures.
- Multiplex per-public-request substreams over the resulting yamux session.
- Expose live tunnels to a public-facing axum router so the host can route public requests at the registered peer.
- Tolerate flap (a `chan serve` restart should reclaim its workspace without waiting for a TCP timeout).

Out of scope:

- TLS termination. The gateway's nginx does it; the desktop relies on loopback + SSH. This crate runs h2c.
- Token issuance / identity. The `Validator` trait is the seam.
- Persistence. The registry is in-memory; a restart drops every tunnel and clients reconnect.
- Wire format (chan-tunnel-proto).

## 2. Architecture overview

```
                 fronting layer (nginx grpc_pass / ssh -R)
                              |
                              v h2c
                 +---------------------------+
                 | serve_tunnel_listener     |
                 |  - TCP accept (permit)    |
                 |  - h2::server handshake   |
                 |  - 1st stream: POST       |
                 |    /v1/tunnel + Bearer    |
                 |  - validator.validate()   |
                 |    [BEFORE 200]           |
                 |  - 200, then              |
                 |    handshake_validated()  |
                 |  - register_with_cap()    |
                 +-------------+-------------+
                               |
                               v
                       (Hello, Validated,
                        YamuxConnection)
                               |
                               v
                 +-----------------------------+
                 | workspace_tunnel (per-tunnel)|
                 |  - owns yamux conn          |
                 |  - serves OpenRequest       |
                 |    -> outbound substream    |
                 |  - shutdown on eviction     |
                 +-------------+---------------+
                               |
              +----------------+-----------------+
              |                                  |
              v                                  v
      +------------------+              +---------------+
      |  Registry        | <-- get ---- |  public_router|
      | user -> workspace|  TunnelHandle|  (or the      |
      |  -> handle       |              |  host's own   |
      +------------------+              |  proxy layer) |
                                        +-------+-------+
                                                |
                                                v
                             hyper h1 client over yamux::Stream
                                 (forward + upgrade bridging)
```

## 3. Components / responsibilities

| File           | Owns                                            |
|----------------|-------------------------------------------------|
| `lib.rs`       | `Validator`, `Validated`, `ServerError`,        |
|                | `TUNNEL_SCOPE`, `TUNNEL_PUBLIC_SCOPE`,          |
|                | `handshake`, `handshake_validated`, refusal     |
|                | mapping, yamux config, handshake timeouts       |
| `tunnel.rs`    | `serve_tunnel_listener`, `handle_tunnel_conn`,  |
|                | `extract_bearer`, public-scope check            |
| `driver.rs`    | `workspace_tunnel`: per-tunnel task that owns   |
|                | the yamux connection                            |
| `registry.rs`  | `Registry`, `TunnelHandle`, `WorkspaceInfo`,    |
|                | `TunnelInfo`, `OpenError`, eviction policy,     |
|                | atomic per-user cap                             |
| `public.rs`    | `public_router`, `public_router_with`,          |
|                | `PublicConfig`, request rewriting, upgrade      |
|                | bridging, idle watchdog                         |

### Listener flow (`tunnel.rs`)

`serve_tunnel_listener(listener, validator, registry, max_workspaces_per_user)`:

1. `TcpListener::accept`. Try to acquire one permit from a per-listener `Semaphore::new(MAX_INFLIGHT_HANDSHAKES)` (1024). If the semaphore is empty, the TCP socket is dropped and the loop continues; this bounds memory against floods of half-open peers that have not yet hit a per-stage timeout. Otherwise spawn `handle_tunnel_conn` carrying the owned permit.
2. `h2::server::handshake(tcp)` under `H2_HANDSHAKE_TIMEOUT` (10s).
3. First `conn.accept()` under `FIRST_STREAM_TIMEOUT` (10s).
4. Reject `(method != POST) || (path != TUNNEL_PATH)` with 404.
5. Parse `Authorization: Bearer ...` (case-insensitive scheme, SP/HTAB separator, trimmed token); reject missing / empty with
   401.
6. Spawn an h2 frame driver task BEFORE awaiting the validator: the validator may be a network round-trip and h2 only progresses while polled. The task rejects any subsequent stream on the same connection with 409 (clients must only ever open one) and `abrupt_shutdown(ENHANCE_YOUR_CALM)` after `MAX_DRAINER_REJECTIONS` (16) rejections.
7. Call `validator.validate(token).await` under `VALIDATE_TIMEOUT` (10s, independent of any timeout the `Validator` impl enforces internally). On timeout, reply 504. On error: 401 (`InvalidToken`), 502 (`Identity`), or 500. Bare 401 / 403 responses arrive at the client as distinct errors; the validator runs before the 200 precisely so auth failures are not collapsed into generic transport failures.
8. Verify the validated token's `scopes` contains `"tunnel"`; 403 otherwise.
9. Send 200 (response headers, body open). Wrap `(SendStream, recv_body)` in `H2Duplex`.
10. `handshake_validated(duplex, validated, pre_ack)`:
   - Defense-in-depth username check (`is_valid_username`).
   - `read_frame::<Hello>` with `HELLO_READ_TIMEOUT` (15s) bound.
   - Reject non-V1 protocol and invalid workspace names. Each rejection writes a `HelloAck::Refused { code, message }` frame (best-effort) before returning so the client receives a structured error instead of a transport disconnect.
   - Run `pre_ack(&hello, &validated)` for post-validate policy. The listener's closure enforces the public scope (`Hello.public = true` without `TUNNEL_PUBLIC_SCOPE` fails with `MissingPublicScope`) and a best-effort per-user workspace-count check. On failure, the `ServerError` is mapped to a stable refusal code (`chan_tunnel_proto::error_code`) and a `HelloAck::Refused` is written before returning.
   - On success, write `HelloAck::Ok(HelloAckOk { prefix: "/{workspace}", user, workspace, .. })` and wrap the duplex in yamux server mode with a 256-substream cap.
11. `registry.register_with_cap(...)` returns a `TunnelHandle`, the open-request `mpsc::Receiver`, and the eviction `oneshot::Receiver`. This is the authoritative cap check: the `pre_ack` count was best-effort, and two parallel dials could both pass it; `register_with_cap` does count + insert under one lock acquisition. A loser here has already received HelloAck; dropping the yamux connection on the early return surfaces as a transport disconnect. The in-flight semaphore permit is dropped after registration so a long-lived tunnel does not consume an accept slot.
12. `workspace_tunnel(...)` runs until close or eviction. On exit, `registry.deregister_if_owner(&handle)`.

### Driver loop (`driver.rs`)

One task per registered tunnel. Owns the yamux `Connection`. Three concerns merged into a single `poll_fn`:

- Shutdown takes priority. The `oneshot::Receiver` resolves either on explicit `()` send or sender drop (the registry drops it on eviction). Either signal exits the loop and `poll_close`s yamux.
- Drain pending `OpenRequest`s from the public side into a local queue and call `poll_new_outbound`; reply with the new substream over the oneshot in the request.
- Poll for inbound substreams. The protocol does not use them; any inbound substream is logged and dropped (yamux RSTs it on the next poll).

On exit the driver replies `OpenError::Disconnected` to any open requests still queued, then deregisters itself if it still owns the registry slot.

`poll_fn` rather than `select!` because two of the three branches need `&mut conn` and `select!` over multiple `poll_fn`s holding that borrow conflicts.

### Registry (`registry.rs`)

- Two-level map `user -> workspace -> Entry` (keys `Arc<str>`) under `parking_lot::Mutex`. The split lets `get(&str, &str)` resolve via `Borrow<str>` without allocating, and makes per-user enumeration a direct inner-map walk. Empty user buckets are removed.
- `Entry { handle: TunnelHandle, _shutdown_tx: oneshot::Sender<()> }`. Dropping the entry drops the sender, which wakes the per-tunnel driver's receiver, which closes yamux.
- Collision: last-writer-wins. `register_with_cap` evicts any prior entry for the same key, logs the prior registration's age (flap visibility), and returns the new handle. This matches "chan-serve restart reclaims its workspace."
- Per-user cap: `register_with_cap` refuses (`RegisterCapped`) when the user already holds `max_workspaces_per_user` distinct workspaces and this key is not among them; `0` disables the check. Count and insert happen under the same lock, so parallel dials cannot race past the cap.
- `TunnelHandle::open()` sends an open request (`oneshot::Sender<Result<yamux::Stream, OpenError>>`) over the per-tunnel mpsc and awaits the reply; `OpenError::Disconnected` if either channel is gone.
- `deregister_if_owner` removes the entry only if it still points at the same handle (mpsc channel identity), so a driver shutting down after eviction can't accidentally remove its successor.
- Admin views: `list_workspaces_for(user)` and `list_all()`, both sorted, carrying the `public` bit, peer address, and connect time for dashboard / `ps`-style tooling. `evict(user, workspace)` forces a tunnel offline.

### Public router (`public.rs`)

`public_router(registry)` builds an `axum::Router` with three routes (`/{user}/{workspace}`, `/{user}/{workspace}/`, `/{user}/{workspace}/*rest`) mounted on `any` method. All three call `proxy(...)`:

1. Optional host gate: with a non-empty `allowed_host_suffixes`, a `Host` header that doesn't end with one of the suffixes gets 421 Misdirected Request.
2. `registry.get(user, workspace)` returns a `TunnelHandle`, else 502 ("tunnel not connected").
3. A single deadline (`upstream_request_timeout`) covers the next three awaits: `handle.open()` (502 on `Disconnected`, 504 on timeout), the h1 handshake over the substream, and `send_request` up to response headers.
4. `hyper::client::conn::http1::handshake(io)` over the substream; spawn the conn driver with `with_upgrades()`.
5. Pre-extract `OnUpgrade` from the public request *before* forwarding so it isn't lost when the body is moved.
6. `build_forwarded`: rewrite path (drop `/{user}/{workspace}` prefix, keep the query), strip URI scheme/authority (h1 over a substream doesn't use them), sanitise headers (section 6), set `X-Forwarded-For` / `X-Forwarded-Proto` / `X-Forwarded-Host`.
7. `sender.send_request(forwarded).await` yields the response.
8. If status is `101 SWITCHING_PROTOCOLS`: pre-extract the tunnel-side `OnUpgrade`, spawn a task that awaits both upgrade futures, wraps each half in an `Activity` adapter (stamps a shared atomic on every byte that moves), and runs `copy_bidirectional` raced against an idle watchdog.
9. Otherwise: strip `Content-Length`, wrap the response body in `http_body_util::Limited` at `response_body_cap`, and stream it back.

The `Activity` stamps use `Instant`-derived milliseconds (monotonic) so wall-clock jumps (NTP slew, suspend/resume) cannot register as activity. The watchdog samples the counter every `UPGRADE_IDLE_TIMEOUT / 4` (floored at 15s; 75s with the 5-minute constant) and tears the bridge down when a full sample window passes with no bytes in either direction.

### Why h1 over yamux, not h2

The substream is already a multiplexed channel; running h2 inside would be mux-on-mux. h1 maps cleanly: one substream is one request. WebSocket upgrades work with `with_upgrades()`. Body streaming works through the yamux flow-control window.

### Why h2c (not TLS) on the listener

The deployment in front owns transport security: nginx terminates TLS at the gateway and forwards h2c via `grpc_pass` on the `/v1/tunnel` path; chan-desktop binds 127.0.0.1 and lets `ssh -R` provide confidentiality. Running rustls here would duplicate trust config and complicate cert rotation. The listener itself is h2c-only; any host can put its own TLS layer in front.

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
    TooManyWorkspaces { user: String, max: usize },
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
    max_workspaces_per_user: usize, // 0 disables the cap
) -> std::io::Result<()>;

// Public router
pub fn public_router(registry: Arc<Registry>) -> axum::Router;
pub fn public_router_with(
    registry: Arc<Registry>, cfg: PublicConfig,
) -> axum::Router;

pub struct PublicConfig {
    pub request_body_cap: usize,         // default 10 MiB
    pub trust_forwarded_for: bool,       // default false
    pub allowed_host_suffixes: Vec<String>, // default empty (off)
    pub upstream_request_timeout: Duration, // default 30s
    pub response_body_cap: usize,        // default 100 MiB
    pub rate_limit_per_second: u64,      // default 0 (off)
    pub rate_limit_burst: u32,           // default 32
}
pub const DEFAULT_REQUEST_BODY_CAP: usize = 10 * 1024 * 1024;

// Registry
pub struct Registry { /* ... */ }
impl Registry {
    pub fn new() -> Arc<Self>;
    pub fn get(&self, user: &str, workspace: &str)
        -> Option<TunnelHandle>;
    pub fn list_workspaces_for(&self, user: &str) -> Vec<WorkspaceInfo>;
    pub fn list_all(&self) -> Vec<TunnelInfo>;
    pub fn evict(&self, user: &str, workspace: &str) -> bool;
}

#[derive(Clone)]
pub struct TunnelHandle {
    pub user: Arc<str>,
    pub workspace: Arc<str>,
    pub public: bool,
    pub peer_addr: Option<SocketAddr>,
    pub connected_at: DateTime<Utc>,
    /* + open_tx: mpsc::Sender<OpenRequest> */
}
impl TunnelHandle {
    pub async fn open(&self) -> Result<yamux::Stream, OpenError>;
}

pub enum OpenError { Disconnected }

// Admin snapshots. TunnelInfo = WorkspaceInfo + user.
pub struct WorkspaceInfo {
    pub workspace: Arc<str>,
    pub public: bool,
    pub peer_addr: Option<SocketAddr>,
    pub connected_at: DateTime<Utc>,
}
pub struct TunnelInfo { /* user + the WorkspaceInfo fields */ }
```

Registration itself (`register_with_cap`) is crate-private: the only way a tunnel enters the registry is through the listener / handshake path, so external code cannot mint handles that bypass validation.

## 5. Wire format / framing

The wire format is owned by chan-tunnel-proto. See [`chan-tunnel-proto/design.md`](../chan-tunnel-proto/design.md) sections 2 and 5 for the byte layout, the JSON envelope rationale, the 64 KiB cap, and `H2Duplex`.

Server-specific notes:

- The 200 response is sent BEFORE the framed `Hello` is read but AFTER the validator runs. This split is the reason `handshake_validated` exists alongside `handshake`: the listener needs to fail with 401 / 403 prior to committing to the body.
- Failures after the 200 (bad protocol, bad workspace name, `pre_ack` policy) are reported in-band as `HelloAck::Refused` with a stable code, written best-effort before the stream is dropped. `refusal_for` maps `MissingPublicScope` and `TooManyWorkspaces` to their dedicated codes; anything else surfaces as `internal` with the error's `Display` as message.
- `HELLO_READ_TIMEOUT = 15s` bounds slow-loris-style peers that connect, get the 200, and never frame a `Hello`. 15s is plenty for trans-pacific; tighter would risk false positives on slow mobile uplinks.
- The yamux config overrides the upstream default of 8192 max concurrent streams down to 256. Per-tunnel cap; a visitor opening many slow requests is bounded.
- `HelloAckOk.prefix` is `/{workspace}`. The username travels in the wildcard host on the public side, not in the path prefix the client embeds.

## 6. Trust boundaries / validation

- **Token authentication**: the consumer's `Validator` impl is the only authority. This crate calls it; on success it gets a `Validated { user_id, username, scopes }`. Order is fixed: validator runs *before* the 200 response so 401 / 403 propagate to the client distinctly. After 200, policy failures are reported via `HelloAck::Refused` instead. The validator contract (documented on the trait) forbids implementations from logging or echoing the token: the listener logs `ServerError` values, so anything echoed lands in operator journals.
- **Tunnel scope**: the validator returns scopes; the listener refuses tokens missing `TUNNEL_SCOPE` (`"tunnel"`) with 403.
- **Public scope**: `Hello.public = true` is a privilege-escalation request (the host's auth gate skips its sign-in check for that workspace), so it is gated on a second scope `TUNNEL_PUBLIC_SCOPE` (`"tunnel.public"`). Tokens holding only the base scope can still register a workspace but must run it private; requesting `public = true` fails with `MissingPublicScope` *before* HelloAck is written, so the client cannot grant the bit to itself at runtime. A token carrying both scopes retains per-workspace choice, so one user can host a public docs workspace and a private notes workspace on the same token.
- **Username validation** (`is_valid_username`): defense-in-depth. The username flows into public routing; if the upstream identity service ever emits `..`, slashes, or whitespace, the public side would mis-route. The handshake refuses any username that wouldn't be URL-safe.
- **Workspace name validation** (`is_valid_workspace_name`): every Hello's `workspace` field is checked; clients pre-check too but we don't trust them.
- **Per-user workspace cap**: `max_workspaces_per_user` bounds how many distinct workspaces one token can keep registered. Checked best-effort in `pre_ack` (clean refusal on the wire) and authoritatively under the registry lock at insert.
- **Method / path gate**: 404 for anything other than `POST /v1/tunnel`. The drainer task rejects additional streams on the same connection with 409 and abrupt-shutdowns the connection (ENHANCE_YOUR_CALM) after 16 rejections.
- **Bearer parsing**: scheme name is case-insensitive (RFC 6750); the scheme/token separator is one or more SP / HTAB (RFC 7230 BWS); empty / whitespace-only tokens are rejected.
- **Listener back-pressure cap**: at most `MAX_INFLIGHT_HANDSHAKES` (1024) connections may sit in the authenticate-and-handshake stages simultaneously. Above that the TCP socket is closed immediately so a flood of half-open peers cannot exhaust memory. Per-stage timeouts (h2 handshake 10s, first stream 10s, validate 10s, Hello read 15s) bound each slot.
- **Request body cap on the public side**: `PublicConfig::request_body_cap` (default 10 MiB) via `tower_http::limit::RequestBodyLimitLayer`. Without a cap a public client could stream gigabytes through to chan-serve (paid for in tunnel egress and chan-serve memory).
- **Response body cap**: `PublicConfig::response_body_cap` (default 100 MiB) wraps the upstream body in `http_body_util::Limited`. Past the cap the body stream errors mid-flight; the public client sees a truncated read. The `Content-Length` header is stripped before wrapping, so a truncated body cannot disagree with a declared length (hyper refuses to serialise that mismatch); the response goes out chunked. Counterpart to the request cap: a compromised chan-serve cannot burn unbounded egress on a single request.
- **Upstream request timeout**: `PublicConfig::upstream_request_timeout` (default 30s) is a shared deadline across opening the substream, the h1 handshake, and waiting for response headers; 504 Gateway Timeout on miss. Body streaming after headers is intentionally uncapped so long downloads / uploads are not artificially limited.
- **Upgrade idle watchdog**: hijacked WebSockets are torn down when no bytes move in either direction for a full watchdog tick (`UPGRADE_IDLE_TIMEOUT / 4`, floored at 15s). Keeps a public client that 101'd and went silent from pinning the substream forever.
- **Public-side host allowlist**: when `PublicConfig::allowed_host_suffixes` is non-empty, the public router replies 421 Misdirected Request to any request whose `Host` header (port stripped, case-insensitive) does not end with one of the listed suffixes. Empty (default) trusts the fronting proxy's host routing. Defence-in-depth for a public listener that is ever exposed directly.
- **Per-visitor rate limit**: optional, off by default. `PublicConfig::rate_limit_per_second` (`0` disables) plus `rate_limit_burst` wire a `tower_governor` layer keyed on `PeerIpKeyExtractor` (raw `ConnectInfo`, NOT X-Forwarded-For — consistent with the header-trust model below). Above the burst, requests return 429. When the public listener sits behind nginx and the visible peer is always the proxy, the limiter keys on a single tenant; rate-limiting then belongs upstream (`limit_req_zone $binary_remote_addr`).
- **Forwarded-header sanitisation** (`build_forwarded`): the public router strips `Forwarded`, `X-Forwarded-Proto`, `X-Forwarded-Host`, `X-Real-IP`, `Proxy-Authorization`, and `Proxy-Authenticate` from incoming requests before re-injecting its own values; the public side does not get to dictate any of these to chan-serve. It also strips `Authorization`, `Cookie`, and `Set-Cookie`: public visitors must not be able to inject bearer tokens or cookie state into the local chan-serve process (public-side authentication is the fronting host's job). `X-Forwarded-Proto` is set to `https` (production assumption: the fronting layer terminates TLS); `X-Forwarded-Host` comes from the original `Host` header. `X-Forwarded-For` is the one knob: with `trust_forwarded_for = false` (default) the incoming value is discarded and downstream sees only the ConnectInfo IP; with `true` the ConnectInfo IP is appended to the incoming chain. Trusting it is only safe when the immediate upstream *overwrites* XFF (nginx `proxy_set_header X-Forwarded-For $remote_addr`); otherwise a public client can spoof its source IP, so the default is the safe one.

## 7. Error model

Single umbrella enum `ServerError` with seven variants (see section 4). Conversions from `chan_tunnel_proto::FrameError` and `IoFrameError` flatten through `Display`, so the public surface stays free of `h2::Error`, `serde_json::Error`, and `yamux::Error`. On the wire, pre-ack policy errors additionally map to stable `HelloAck::Refused` codes via `refusal_for`.

`OpenError::Disconnected` is the single failure mode of `TunnelHandle::open()`: either the request channel is gone (the driver has already exited) or the reply channel was dropped (the driver couldn't allocate the substream because yamux is closing). Public-side callers map both into 502.

## 8. Consumers

- `gateway/crates/workspace-proxy` (separate Cargo workspace): runtime dep. Wires this crate end-to-end:
  - `serve_tunnel_listener` on the h2c listener that nginx `grpc_pass`-es into.
  - A `Validator` backed by the gateway's identity service (wrapped with throttling), which also caches `username -> user_id` for the proxy's auth gate.
  - A thin facade (`workspace_proxy::registry::Registry`) over `chan_tunnel_server::Registry`, used by the proxy, dashboard, and admin handlers.
  - Its own reverse-proxy layer (`proxy.rs`) for `{user}.workspace.chan.app/{workspace}/...`: wildcard-host routing, an entry-JWT / cookie auth gate, hyper h1 and tungstenite WebSocket over substreams from `TunnelHandle::open` — rather than mounting `public_router`, which remains the turn-key alternative for hosts that don't need custom middleware.
  - chan-tunnel-client as a dev-dep so `tests/api.rs` can register a fake `chan serve` against a real listener.
- `desktop/src-tauri` (chan-desktop): runtime dep. Embeds a loopback-only tunnel listener (user-initiated, fed by `ssh -R` from a remote `chan serve`) with a local `Validator` whose token doubles as the tenant label, plus per-tenant 127.0.0.1 listeners that wrap `public_router` behind a path-prepending layer. A supervisor polls `Registry::list_all()` for fresh registrations.
- This crate's own e2e tests (`tests/listener_e2e.rs`, `tests/public_e2e.rs`) drive a real chan-tunnel-client against `serve_tunnel_listener` and `public_router_with` over localhost, covering the auth gates, refusal codes, response-body cap, substream concurrency, and the rate limiter.

## 9. Open questions / future extensions

- Persistent registry. Today a host restart drops every tunnel and clients reconnect. A small on-disk index would let the public side serve "tunnel offline since X" errors with context instead of a bare 502 during a restart.
- Per-tunnel quotas. `max_workspaces_per_user` caps workspace count; nothing caps a single tunnel's concurrent in-flight requests (beyond the 256-substream yamux cap), total bandwidth, or request rate.
- Multi-workspace per tunnel. See chan-tunnel-proto's design.md section 9; would change the registry shape so one yamux session can serve several workspaces.
- Health probe on the substream. The driver currently learns about a dead peer when yamux errors or an `open` fails. An explicit application-level ping over a control substream would give the public side faster failover.
