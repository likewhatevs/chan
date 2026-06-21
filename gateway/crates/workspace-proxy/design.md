# workspace-proxy: design

## Problem

`chan serve` instances register over chan-tunnel and live until the peer disconnects. The gateway needs a service that:

1. Accepts the tunnel registration handshake.
2. Reverse-proxies HTTP and WebSocket traffic into the registered workspace.
3. Gates workspaces behind a token minted by identity-service. workspace-proxy holds no session cookie of its own beyond a short-lived, per-workspace gate cookie scoped to one path.
4. Supports admin operations (snapshot, evict, per-user list and bulk evict).

The workspace list, sign-in surface and every piece of user-facing UI live in identity-service. workspace-proxy has no SPA and no public `/api/*` of its own.

## Architecture

Two public hostnames pointed at the same process:

- `devserver.chan.app` (apex): admin + tunnel + healthz only.
  - `POST /v1/tunnel` -- raw h2c, handled by `chan-tunnel-server` on a separate internal listener (`TUNNEL_BIND_ADDR`). nginx `grpc_pass`es this path; everything else on the apex hits the axum HTTP listener.
  - `/admin/v1/*` -- bearer-gated admin tree.
  - `/healthz` -- liveness.
  - Anything else -- 404.

- `*.devserver.chan.app` (wildcard): tenant content only.
  - `/` -- 302 to `https://id.chan.app/workspaces`.
  - `/{workspace}` -- 308 to `/{workspace}/` (trailing slash canonical).
  - `/{workspace}/?t=<jwt>` -- entry: validate the entry token, set the `devserver_gate` cookie, 303 to the clean URL.
  - `/{workspace}/...` -- proxy to the registered tunnel for `(host_user, workspace)`, requires a valid `devserver_gate` cookie. Anything else -- 404.

A single axum router serves both apex and wildcard via a Host-keyed dispatch. The wildcard host's `{user}` is parsed out of the request's `Host` header; the prefix before `.devserver.chan.app` is the username.

The tunnel listener is unchanged: `chan-tunnel-server` runs raw h2 on `TUNNEL_BIND_ADDR`, with the validator chain `CapturingValidator -> ThrottlingValidator -> IdentityValidator`. On a successful handshake the registry caches `(username -> user_id)`.

The `Registry` (`registry.rs`) is the in-process map from `(username, workspace)` to the live `TunnelHandle` plus the username cache. The admin tree reads from the same registry that the proxy handler reads.

## Workspace gate

workspace-proxy reads no `tower_sessions` cookie. Authentication for the proxy path uses a JWT minted by identity-service, signed with `WORKSPACE_GATE_SECRET` (HMAC-SHA256). The secret is shared between identity (mints both shapes) and workspace-proxy (verifies, mints the session shape).

Two tokens are involved:

- **Entry token**: 30s exp, carried in `?t=` on the first hit to a workspace. Issued by identity at `GET /api/workspaces/open?u=...&d=...` after the dashboard verified the user owns the workspace. Claims: `{iss: "id.chan.app", sub: user_id, drv: <slug>, aud: "<host>", typ: "entry", iat, exp}`.

- **Session cookie**: 24h hard exp, written as `Set-Cookie: devserver_gate=<jwt>; HttpOnly; Secure; SameSite=Lax; Path=/<workspace>/`. Minted by workspace-proxy on entry-token validation. Same claim envelope, `typ: "session"`. Stateless: no server-side store.

Cookie `Path` scopes the credential to one workspace. JS in `alice.devserver.chan.app/blog/...` cannot read or send the cookie for `alice.devserver.chan.app/journal/...` (path does not match). Cross-user attacks are blocked by browser origin separation: `alice.devserver.chan.app` and `bob.devserver.chan.app` are distinct origins.

The shared JWT type and signing helpers live in `gateway_common::devserver_gate`.

## Public surface

Full route table is in [`README.md`](README.md). Critical paths:

### Tunnel registration (apex only)

`POST /v1/tunnel` on `devserver.chan.app:443`. nginx routes this exact path to the h2c tunnel listener (`grpc_pass`, `TUNNEL_BIND_ADDR`, default `:7100`); everything else on the apex `proxy_pass`es to the axum listener on `:7002`. The h2c handler in `chan-tunnel-server` validates the Bearer PAT via identity-service `/internal/v1/tokens/validate`, then registers the workspace in the shared registry.

### Reverse proxy (wildcard host)

Auth gate for `*.devserver.chan.app/<workspace>/...`, in order:

1. Registration `(host_user, workspace)` not found in the registry -> 404.
2. Request carries `?t=<jwt>` -> verify signature + exp + aud + drv match. On success: mint a session JWT, write `devserver_gate` cookie scoped to `Path=/<workspace>/`, 303 to `/<workspace>/` (clean URL).
3. Request carries `devserver_gate` cookie -> verify signature + exp + claim match against `(host_user, workspace)`. Pass through.
4. Anything else (no cookie, expired cookie, bad signature, wrong user) -> 404.

The gate always runs: every tunnel is authenticated, there is no un-gated pass-through.

The 404 path checks `Accept: text/html`; browsers get the styled "workspace not found" page, everything else gets the JSON `{"error":"not found"}` shape. Owners returning after the 24h cookie expires bounce through `id.chan.app/workspaces`; a bookmark to a workspace URL is not a session.

### Hop-by-hop hygiene

`HOP_BY_HOP_NAMES` lists the RFC 7230 6.1 hop-by-hop headers: `Connection`, `Keep-Alive`, `Proxy-Authenticate`, `Proxy-Authorization`, `TE`, `Trailer`, `Transfer-Encoding`, `Upgrade`. In addition, `connection_listed_headers` parses the inbound `Connection` value and strips every header it names (also RFC 7230 6.1). Applied on both legs.

Inbound `Host`, `Cookie`, and `Authorization` are dropped. `X-Forwarded-For` is recomputed as `<existing chain>, <peer ip>`. `X-Forwarded-Proto` is set from `FORWARDED_PROTO` (default `https`, configured to match the terminator that fronts this listener). `X-Forwarded-Host` is set from the inbound `Host` header workspace-proxy itself routed on. Inbound `X-Forwarded-{Host,Proto}` are NOT trusted: they are client-controllable and an upstream that builds absolute URLs from XFH/XFProto would otherwise be steerable from outside.

The `dispatch` handler likewise reads the raw `Host` header directly rather than going through axum's `Host` extractor, which consults `Forwarded` and `X-Forwarded-Host` before `Host` and would let a hostile client route into a different tenant's wildcard surface by spoofing those headers.

### HTTP upstream

`hyper::client::conn::http1::handshake` over a yamux substream wrapped in `tokio_util::compat::FuturesAsyncReadCompatExt`. `with_upgrades()` keeps the substream alive past 101 so WebSocket can ride the same path. For pure HTTP the connection future ends when the response body finishes.

Request bodies are wrapped in `http_body_util::Limited` at `MAX_REQUEST_BYTES` (default 100 MiB). Response bodies are wrapped at `MAX_RESPONSE_BYTES` (default 100 MiB). Either `0` disables the cap. A total per-request deadline (`REQUEST_TIMEOUT_SECS`, default 60s) covers both the `send_request` future AND the response body stream: the response body is wrapped in `DeadlineBody`, which holds a `tokio::time::Sleep` anchored at the deadline and errors out the stream if the upstream slow-drips past it. `DeadlineBody` also owns the upstream conn task's `AbortHandle` and aborts on drop so a client that bails mid-response does not strand the yamux substream. On the headers-side miss the proxy returns 504. WebSocket requests bypass the total-timeout and run under per-half idle timeouts instead (see below).

### WebSocket upstream

`tokio_tungstenite::client_async` runs the WS handshake directly on the yamux substream. Two halves run inside a `tokio::select!`. Each half has a 300s idle timeout: if neither side sends a frame within the window, the half drops, the other half falls out of scope, and the substream closes. Without this, an idle peer could pin a yamux window indefinitely.

### Admin tree

Routes:

- `GET    /admin/v1/tunnels`                       list every live tunnel
- `POST   /admin/v1/tunnels/:user/:workspace/kill`     evict one tunnel
- `GET    /admin/v1/users/:user/tunnels`           per-user snapshot
- `POST   /admin/v1/users/:user/tunnels/kill`      bulk evict for a user
- `GET    /admin/v1/tunnels/watch`                 SSE snapshot stream

All bearer-gated by `WORKSPACE_ADMIN_TOKEN` (constant-time compare via `subtle`). Lives on the apex hostname so tenant content cannot reach it via fetch. There is deliberately no per-IP rate limit on this tree: behind nginx every request arrives from one upstream IP, so a per-IP bucket degenerates into a single global one that an attacker could use to lock out the operator CLI; nginx is the rate-limit layer for this surface.

## Key decisions

### One process, two listeners, one registry

The h2c tunnel listener and the axum HTTP listener share the in-process `Registry`. A registration on the tunnel listener is visible to the proxy handler on the very next request with no out-of-band sync. If horizontal scale becomes necessary the registry moves into a shared store (Redis, Postgres LISTEN/NOTIFY); both listeners become independent again.

### No cookie session for the proxy path

workspace-proxy reads nothing from `tower_sessions`. The browser never sends an `.chan.app`-scoped cookie to `*.devserver.chan.app` because no such cookie exists; id.chan.app's cookie is host-only on id.

This is load-bearing for cross-tenant isolation:

- Malicious tenant content at `evil.devserver.chan.app` can run JS, but the only cookies it can access are its own host-only ones. The browser will not auto-attach an id.chan.app cookie to a fetch on `evil.devserver.chan.app`.
- Same-host attacks across a single user's workspaces are blocked by the `Path=/<workspace>/` scope on the `devserver_gate` cookie.
- Cross-user attacks are blocked by browser origin separation; each user has their own subdomain.

### JWT, HS256, two-token

Entry tokens have 30s exp so a leak (referer, browser history, ops log) closes in under a minute. Session cookies have 24h exp so day-to-day navigation is one click from the dashboard. Both signed with `WORKSPACE_GATE_SECRET` (HS256, no "alg: none" path; the validator hard-requires HS256). The crate is `jsonwebtoken`.

There is no sliding session-cookie expiry and no server-side revocation (revoked-jti set); rotation of `WORKSPACE_GATE_SECRET` is the only immediate invalidation knob.

### Username cache populated on handshake

The tunnel validator returns `(user_id, username)`. `CapturingValidator` records that pair in the registry on every successful handshake. The proxy gate does not compare `owner_id` against the token's `sub` (that comparison would lock grantees out of shared workspaces); the cache exists as metadata for admin tooling and as a defense-in-depth signal for future enforcement that needs to correlate the live tunnel with a specific account. A cache miss reads as "unknown registration" -> 404 because tunnel presence is what the registry tracks.

### Auth gate trust model

The auth assertion on the wildcard path is the entry JWT, not "sub matches owner". identity-service calls `profile.workspace_access(owner, workspace, caller)` before minting any entry token, so a valid signature plus the right `aud` (= the inbound host, which is `{owner}.devserver.chan.app`) plus the right `drv` (= the requested workspace) proves the caller was authorized at mint time. identity owns the access-control policy; workspace-proxy verifies the signed assertion. The session cookie minted on entry-token validation carries the entry's `sub` unchanged so the upstream attribution chain knows whether the request belongs to the owner or a grantee.

Tenant isolation is enforced by `aud`. A token minted for one subdomain (`alice.devserver.chan.app`) cannot be replayed on another (`bob.devserver.chan.app`) because `decode` rejects on `aud` mismatch. Workspace isolation is enforced by `drv`. There is no separate "this user is the owner" check, and intentionally so: requiring it would prevent accepted grantees from accessing the workspaces they have been granted.

### Tunnel handshake throttles by token fingerprint

`ThrottlingValidator` keeps an in-process map of fingerprint -> token bucket (SipHash of the candidate token, 4 rps refill, 16-burst capacity, 4096-entry cap with LRU eviction). Guesses at a specific PAT are bounded regardless of attacker source-IP diversity. A twin of this throttle lives in identity-service's `/internal/v1/tokens/validate` handler as defense in depth: if the internal bearer leaks and someone hits identity directly, the identity-side throttle catches it. Either throttle alone is enough to make a guess loop glacial.

### Path strip is one segment

`{user}` lives in the host, not the path. The wildcard router strips exactly one segment (`/<workspace>`) before forwarding to the upstream, which expects no prefix (chan serve in tunnel mode refuses `--prefix`).

### Admin tree on the apex

Admin routes intentionally live on `devserver.chan.app`, not on the wildcard. Tenant content has no way to call them: the wildcard router never proxies `/admin/v1/*` upstream, and the apex never serves tenant content.

### Domain config is single-source

The apex, wildcard, and the dashboard redirect's id host all derive from one base domain (`CHAN_DOMAIN`, e.g. `chan.app`) plus `PUBLIC_SCHEME`, via `gateway_common::domain::Domains`. identity-service derives the same hosts from the same two vars, so the two cannot drift (the devserver-gate JWT `aud` is the inbound host and must match). `APEX_HOST`, `WILDCARD_SUFFIX`, and `DASHBOARD_URL` are explicit overrides; the wildcard follows the apex unless set, and `DASHBOARD_URL` is needed when the id host runs on a non-default port (the derived form carries none). Defaults are dev-shaped (`localtest.me` / `http`); production sets `CHAN_DOMAIN` + `PUBLIC_SCHEME` once in the shared `/etc/chan-gateway/domain.env`, loaded by both systemd units.

## Invariants

- Every registered tunnel has a known `owner_id`.
- Tunnel registrations are ephemeral; they vanish when the peer disconnects or via admin evict.
- The proxy path reads no `tower_sessions` cookie. The only cookie it reads or writes is the host-only, path-scoped `devserver_gate`.
- Bearer comparisons run at constant time.
- Hop-by-hop headers are stripped on both legs of every request, including every header named by the inbound `Connection` value.
- Reverse-proxy paths forward to the tunnel unchanged modulo the single `/<workspace>` segment strip.
- Request and response bodies are bounded by the configured caps.
- HTTP requests are bounded end-to-end by `REQUEST_TIMEOUT_SECS`.
- WebSocket halves are bounded by a 300s idle timeout each.

## Error model

`workspace_proxy::Error` (`src/error.rs`):

| Variant       | HTTP | Notes                                     |
|---------------|------|-------------------------------------------|
| Unauthorized  | 401  | not used on the proxy path; the 404       |
|               |      | path is preferred so existence does not   |
|               |      | leak                                      |
| NotFound      | 404  | unknown workspace, invalid or missing gate    |
|               |      | token, wrong user                         |
| BadRequest    | 400  | input or proxy precondition failure       |
| Upstream      | 502  | tunnel disconnected, h1 handshake failed  |
| Anyhow        | 500  | startup or unexpected                     |
| Reqwest       | 502  | identity-service unreachable              |

workspace-proxy carries no `Conflict` variant: nothing on this surface PATCHes a unique-constrained row.

## What's wired

- axum 0.7 (HTTP) and `chan-tunnel-server` (h2c) as a library
- `jsonwebtoken` for HS256 verify and mint (session shape only)
- `hyper` h1 client over yamux substreams for the HTTP proxy path
- `tokio_tungstenite::client_async` for the WebSocket proxy path
- `http_body_util::Limited` for request and response byte caps
- `tokio::time::timeout_at` + the `DeadlineBody` wrapper for the end-to-end HTTP request timeout
- `gateway-common` for the shared devserver_gate JWT type, the token-bucket primitive, the username validator, and the domain derivation

## What is not wired

- A SPA: no `web/` bundle, no static_files in this crate
- Sessions: no `tower_sessions` integration anywhere
- Per-tunnel labels (the workspace slug is the default; `Hello` carries no separate label)
- Per-PAT scopes (tunnel scope is implicit on validated PATs)
- Multi-instance horizontal scale (one process, in-process registry)
- Server-side session revocation (24h cookie exp is the only knob; rotating `WORKSPACE_GATE_SECRET` is the nuclear option)
- Sliding session-cookie expiry
