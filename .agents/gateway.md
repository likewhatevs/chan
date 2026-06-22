# Gateway agent guide

Contribution guidelines for agents and contributors working on the `gateway/` workspace. Source files live under `gateway/`; this file documents them from the shared `.agents/` home.

## What this workspace is

The `gateway/` Cargo workspace runs the account, sign-in and reverse-proxy surface for chan.app. Five crates:

- `profile`: internal HTTP API in front of Postgres. Owns users, linked OAuth identities, audit logs, and the `api_tokens` table. Called only by sibling gateway services; not public.
- `identity`: public service at `id.chan.app`. Runs OAuth2 sign-in (GitHub, Google, GitLab), holds the cookie session (host-only on `id.chan.app`), serves the only Svelte SPA in the suite (profile, PAT lifecycle, workspace list). Mints workspace-gate entry tokens used to hand the user off to `workspace.chan.app`. Owns no domain data; calls `profile` over HTTP and `workspace-proxy` admin for the live-workspace list.
- `workspace-proxy`: public service at `workspace.chan.app` (apex) and `*.workspace.chan.app` (wildcard). Apex carries the tunnel registration endpoint (`POST /v1/tunnel` via raw h2c), the admin tree, and `/healthz`. Wildcard carries the per-user tenant content surface: `{user}.workspace.chan.app/{workspace}/*` reverse-proxies into the registered `chan serve` peer, gated by a host-only, path-scoped `workspace_gate` JWT cookie minted by workspace-proxy on first entry. No SPA in this crate.
- `admin`: operator CLI. Talks to `profile` admin routes and `workspace-proxy` admin routes over Bearer auth.
- `gateway-common`: shared internal library. Holds the typed `profile-service` HTTP client, the `workspace-proxy` admin client used by identity (on revoke / delete) and profile (on admin block), the shared workspace-gate JWT envelope (HS256), and the SPA-fallback static-asset handler used by identity. No binary.

Each public-facing crate ships two docs: `README.md` is the consumer-facing entry (pitch, install, build, route table, env vars) and `design.md` is the canonical design reference (problem, architecture, public surface, key decisions, invariants, error model). Update `design.md` in the same commit as any change that affects HTTP routes, the on-the-wire shape of a public response, the session contract, or the inter-service trust model.

## Build & Test

```bash
cargo build
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

The Rust toolchain is pinned in `rust-toolchain.toml`. The pre-push hook (`./scripts/install-hooks` to install) runs the same gate as CI; a passing local push will not fail in the cloud.

Database setup for tests:

```bash
createdb chan_gateway        # dev database
createdb chan_gateway_test   # test database used by integration tests
export DATABASE_URL=postgres://localhost/chan_gateway
```

Only identity-service ships a SPA. `crates/identity/web` and the shared `web-common` package live in one npm workspace at the repo root:

```bash
npm install                   # one install for the whole workspace
npm run build --workspaces    # build the identity SPA bundle
```

Per-app dev:

```bash
npm run dev -w crates/identity/web    # vite dev server for id.chan.app
```

A fresh checkout without `web/dist/` still builds; identity's SPA endpoint returns a "frontend not built" banner that points at the right command. workspace-proxy has no SPA.

## Writing Rules

- **No em dashes** in comments or documentation. Use commas, semicolons, parentheses, or separate sentences.
- **Tables**: pure ASCII, target 80 columns, left-aligned, no Unicode box-drawing.
- **Factual**: no marketing language ("just", "easy", "blazing"). Verify every claim against the implementation; flag drift.
- **Comments**: explain WHY, not WHAT. The code shows what; the comment explains the reasoning, the trade-off, or the constraint.

## Workspace Principles

These rules cut across every crate. Per-crate specifics live in each crate's `design.md`.

### Constant-time secret comparisons

Every bearer token, OAuth state value, JWT signature compare, and CSRF-shaped check uses `subtle::ConstantTimeEq` (or an equivalent timing-safe operation). Plain `==` on a secret is never acceptable, even when the rest of the request gates require an authenticated session. The known leak (length inequality short-circuits) is acknowledged in a comment next to each compare.

### HTTP error mapping

Each request-handler crate (`profile`, `identity`, `workspace-proxy`) defines a `thiserror::Error` enum with an `IntoResponse` impl that maps every variant to a precise HTTP status code. Public-facing messages are short and intentionally generic (`unauthorized`, `internal error`, `upstream unreachable`); detailed context goes into the `tracing` log on the server side. `anyhow::Error` is acceptable in `main.rs` and in startup paths; request handlers return explicit thiserror variants.

`gateway_common::profile_client::ProfileError`, `gateway_common::workspace_admin_client::WorkspaceAdminError` and `gateway_common::workspace_gate::WorkspaceGateError` are the cross-service client errors. Each consumer maps them onto its local error via a `From` impl so request handlers can `?` straight through.

### Session contract

identity-service owns the only session cookie in the suite: `id_session`, host-only on `id.chan.app` (no `Domain` attribute), `HttpOnly`, `SameSite=Lax`, 30-day inactivity expiry. `Secure` follows `COOKIE_SECURE`. workspace-proxy does not read this cookie.

workspace-proxy's only cookie is `workspace_gate`: host-only on `{user}.workspace.chan.app`, `Path=/{workspace}/`, 24h hard exp, HS256 JWT signed with `WORKSPACE_GATE_SECRET`. Minted by workspace-proxy after verifying an entry JWT issued by identity. Not shared with id.

This split is the load-bearing piece of the cross-tenant isolation: no `.chan.app`-scoped cookie exists, so a browser does not auto-attach an id session to a fetch on `evil.workspace.chan.app`. Cookie sharing across the two services is replaced by an explicit workspace-gate handoff (entry token in URL `?t=`, session cookie set by workspace-proxy on validation).

### Reverse-proxy trust boundary

`workspace-proxy` strips hop-by-hop headers (RFC 7230 6.1) on both the request and response legs, **including every header named by the inbound `Connection` value** (also required by 6.1). Drops the inbound `Host`, `Cookie`, and `Authorization` headers. Recomputes `X-Forwarded-For` as `<existing chain>, <peer ip>`, `X-Forwarded-Proto` from `FORWARDED_PROTO` (configured to match the terminator that fronts this listener; default `https`), and `X-Forwarded-Host` from the inbound `Host` header workspace-proxy itself routed on. Inbound `X-Forwarded-Host` / `X-Forwarded-Proto` from clients are NOT trusted; nginx may not scrub them and the gateway must not assume it does. Upstream is reached over a yamux substream owned by an authenticated tunnel; there is no SSRF risk because the upstream URL is never user-supplied.

Request bodies are bounded by `MAX_REQUEST_BYTES` (default 100 MiB). Response bodies are bounded by `MAX_RESPONSE_BYTES` (default 100 MiB). Setting either to `0` disables the cap. HTTP requests are bounded end-to-end by `REQUEST_TIMEOUT_SECS` (default 60s), including the response body stream (a slow-drip upstream is cut at the deadline via `DeadlineBody`); the same wrapper aborts the upstream conn task on client drop so a bailed request does not strand the yamux substream. WebSockets bypass the total-timeout and use a 300s per-half idle timeout instead.

### Database pools

Every service caps its Postgres pool at 4 connections. Postgres non-superuser slots are a shared resource; running `profile` plus `identity` plus `workspace-proxy` on a single dev Postgres alongside running tests can otherwise run the slot count out. The cap is documented in each service's pool-build site.

### Atomic upserts in profile-service

The user / identity / email triangle has a known concurrent first-time-login race (two providers, same email, same user, in the same second). `profile-service` resolves it in a single transaction (`POST /v1/users/upsert-by-identity`); identity-service calls only that endpoint. New code that reaches across users and identities should use the same atomic shape rather than reimplement a multi-step dance.

### Service-to-service bearers

Three distinct bearers, all `openssl rand -hex 32`:

- `PROFILE_AUTH_TOKEN`: identity-service -> profile-service service API. profile-service also accepts `PROFILE_ADMIN_TOKEN` here so a single-token deployment works; the middleware runs both checks unconditionally to avoid which-token-matched timing leaks.
- `IDENTITY_INTERNAL_TOKEN`: workspace-proxy -> identity-service `/internal/v1/tokens/validate`. Required; no fallback to `PROFILE_AUTH_TOKEN`. Rotating one does not rotate the other.
- `WORKSPACE_ADMIN_TOKEN`: identity-service and profile-service -> workspace-proxy admin tree. profile uses it on admin block; identity uses it on revoke, delete, and dashboard reads.

Plus one symmetric secret:

- `WORKSPACE_GATE_SECRET`: HS256 signing key shared by identity (mints entry JWTs) and workspace-proxy (verifies entry, mints session JWTs).

## Contributor Patterns

Per-crate rules that come up often when editing this code. For the full design rationale, read the crate's `design.md`.

### profile

- **Two-tier auth.** Routes use `PROFILE_AUTH_TOKEN` for the service API (`/v1/users/*`, `/v1/auth-audit`) and `PROFILE_ADMIN_TOKEN` for the admin tree (`/v1/admin/*`). Single-token deployments may set them to the same value; the middleware accepts either where both apply.
- **Placeholder usernames are deterministic.** New rows seed `username = 'u' || substr(replace(uuid::text, '-', ''), 1, 12)`. identity-service renames on first sign-in; the hard cap of 4 lifetime renames is enforced in `update_username` via a CAS update. Don't invent an alternate seeding scheme.
- **All SQL is parameterized.** Constants like `USER_COLS` are `format!`'d into queries; user input always goes through `.bind()` and `$N`.
- **Block fans out server-side.** `POST /v1/admin/users/:id/block` also calls workspace-proxy `kill_user_tunnels` (best-effort) when a `WorkspaceAdminClient` is configured. Operators do not need a second hop.

### identity

- **OAuth providers are pluggable.** Each lives at `src/providers/<name>.rs` behind a small `Provider` trait. Registering a new provider requires one file plus wiring in `Config::from_env`. Microsoft and Apple are intentionally not wired (see design.md for why).
- **PAT shape: `chan_pat_<32 random bytes, base64url, no pad>`.** Generated with `OsRng`; the database stores only the SHA-256(token), so a table dump leaks no live secrets. Plaintext appears once on the create response.
- **Reserved usernames live in `RESERVED_USERNAMES`.** Anything that could collide with a top-level path under chan.app/ goes in the alphabetically-sorted list. The list is checked with `binary_search`; keep it sorted. `rustrict` false positives can be unblocked via `RUSTRICT_ALLOWLIST` env var.
- **OAuth callback validates state before provider.** Plain `pending.provider != provider` runs only after a constant-time state compare so timing on the provider check can't be used to oracle the session's expected provider.
- **Session id rotates on login.** `session.cycle_id()` runs at the privilege boundary, before storing `user_id`. Closes session fixation.
- **Token revoke evicts tunnels.** `DELETE /api/tokens/:id` fires workspace-proxy `kill_user_tunnels` best-effort after the DB update.
- **Workspace-gate mint is its own route.** `GET /api/workspaces/open?u= &d=` mints a 30s entry JWT and 303s to `https://{u}.workspace.chan.app/{d}/?t=<jwt>`. The dashboard links to this route, not to workspace-proxy directly, so the token is minted at click time.

### workspace-proxy

- **Apex vs wildcard.** `workspace.chan.app` (apex): tunnel + admin + healthz only. `*.workspace.chan.app` (wildcard): tenant content only. A single axum router dispatches on the `Host` header. The h2c tunnel endpoint runs on a separate internal listener; nginx `grpc_pass`es `/v1/tunnel` on the apex to it.
- **No cookie session for the proxy path.** workspace-proxy reads nothing from `tower_sessions`. The proxy gate uses the `workspace_gate` JWT cookie minted on entry-token validation. Path-scoped to `/{workspace}/` so JS in one workspace cannot read or send the cookie for another workspace on the same user's subdomain.
- **Auth gate order on `/{workspace}/*`** (in `proxy::handle`): registration missing -> 404; `entry.public` -> pass through; `?t=<entry-jwt>` -> verify signature + aud + drv, mint session cookie carrying the entry's `sub`, 303 to the clean URL; valid `workspace_gate` cookie (signature + aud + drv) -> pass through; anything else -> 404. Same 404 shape for "unknown workspace" and "no token" so unauthenticated probes cannot enumerate registrations. The gate does not compare `sub` against the registry-cached owner_id: identity-service is the access-control authority (it calls `profile.workspace_access` before minting), and a sub-equals-owner check locks every accepted grantee out of its shared workspaces. The `aud` claim (= the inbound host) is what enforces tenant isolation.
- **Hop-by-hop stripping is complete.** `HOP_BY_HOP_NAMES` lists the static names; `connection_listed_headers` parses the inbound `Connection` value and strips every name it lists. Both applied on every leg.
- **Two listeners, one Registry.** The h2c tunnel listener and the axum HTTP listener share the in-process `Registry`. A registration on the tunnel listener is visible to the proxy handler on the very next request.
- **JWT alg hard-required.** `gateway_common::workspace_gate::decode` rejects anything other than HS256. No "alg: none" path exists in this codebase.

### admin

- **Three exit codes.** 0 success; 1 upstream/network error; 2 user input error (bad uuid, missing arg); 3 not found. Exit codes are part of the contract for shell wrappers.
- **`--json` everywhere.** TTY default is a `comfy_table` plain-text table; `--json` emits the same data as JSON for jq piping. Adding a new subcommand without `--json` is a regression.

### gateway-common

- **No axum / IntoResponse coupling in data-layer types.** `ProfileError`, `WorkspaceAdminError`, `WorkspaceGateError` and `Claims` are plain thiserror / serde. Each consumer maps via `From` for its local error.
- **`User` is the superset.** The struct carries every field profile-service can return; consumers ignore the fields they don't need. Don't fork the struct per consumer.
- **`workspace_gate` is the single source of JWT shape.** Both identity (mint) and workspace-proxy (verify + mint sessions) call through this module. The HS256 alg is hard-required on every decode.

## Documentation

- **Workspace overview**: [`gateway/README.md`](../gateway/README.md)
- **Crate design references** (canonical; `README.md` next to each is the consumer-facing entry):
  - [`gateway/crates/profile/design.md`](../gateway/crates/profile/design.md): schema, two-tier auth, atomic upsert, block fan-out.
  - [`gateway/crates/identity/design.md`](../gateway/crates/identity/design.md): OAuth providers, PAT lifecycle, session contract, workspace-gate mint, dashboard.
  - [`gateway/crates/devserver-proxy/design.md`](../gateway/crates/devserver-proxy/design.md): apex / wildcard split, workspace-gate verify, registry model, reverse-proxy hygiene.
  - [`gateway/crates/admin/design.md`](../gateway/crates/admin/design.md): command surface, output contract, exit codes.
  - [`gateway/crates/gateway-common/design.md`](../gateway/crates/gateway-common/design.md): why a shared crate, what belongs and what does not.
- **Issue tracker**: GitHub repo `fiorix/chan`.
