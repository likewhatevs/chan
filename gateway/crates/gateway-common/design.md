# gateway-common: design

## Problem

identity-service, workspace-proxy and profile-service need the same plumbing in several places:

- a `ProfileClient` calling profile-service over HTTP;
- a `WorkspaceAdminClient` calling workspace-proxy admin (used by identity on revoke / delete / dashboard reads and by profile on admin block);
- the JWT shape used by the devserver-gate handoff between identity (mint) and workspace-proxy (verify + mint sessions);
- the public-hostname derivation both public services must agree on;
- username validation rules that profile, identity and workspace-proxy all enforce;
- the token-bucket primitive both validate throttles wrap.

Per-crate copies would risk drift and make cross-cutting choices (timeouts, error mapping, MIME guessing, signing claims, throttle limits) live in multiple places.

## Architecture

Library crate with eight modules and no axum / IntoResponse coupling in the data-layer types:

- `domain` (`src/domain.rs`)
  - `Domains`: public hostnames (`base`, `id_host`, `devserver_apex`, `devserver_wildcard_suffix`) derived from one base domain via `from_base` / `from_env` (`CHAN_DOMAIN`, default `localtest.me`). identity and workspace-proxy derive the same hosts from the same env, so the devserver-gate `aud` cannot drift. std-only.
- `profile_client` (`src/profile_client.rs`)
  - `ProfileClient`: reqwest-backed client with a 10-second per-request timeout. Bearer token lives inside; callers do not deal with auth. Idempotent GETs on the dashboard / OAuth-callback read path retry once after 100 ms on connect error, timeout, or 5xx (`send_idempotent`); writes never retry.
  - Serde types matching profile-service's wire shapes: `User`, `Identity`, `UpsertResponse`, `FeatureFlag*`, `FlagMap`, `Workspace`, `WorkspaceGrant`, `WorkspaceAccess`, `OwnedWorkspaceSummary`, `IncomingShare`. `User` is the superset of every field profile returns; consumers ignore the fields they do not need.
  - `ProfileError`: thiserror enum with `NotFound`, `BadRequest(String)`, `Conflict(String)`, `Upstream(String)`, `Reqwest(reqwest::Error)`.
- `shutdown` (`src/shutdown.rs`)
  - `shutdown_signal()`: completes on the first of SIGTERM (Unix) or Ctrl-C; every service binary gates its graceful shutdown on it.
- `static_files` (`src/static_files.rs`)
  - `serve<R: RustEmbed>(uri, banner) -> Response`: SPA fallback handler. Tries the requested path, falls back to `index.html` for paths without an extension, serves the banner (as 503, so a missing bundle surfaces to monitoring) if no `index.html` is embedded, else 404.
- `token_bucket` (`src/token_bucket.rs`)
  - `TokenBucket`: per-fingerprint token bucket with a bounded map (LRU-style eviction at `map_cap`; new fingerprints start at one token, not a full burst, so rotating fingerprints can't bank capacity). `fingerprint` is a SipHash-64 of the candidate token.
  - `DEFAULT_REFILL_PER_SEC` / `DEFAULT_CAPACITY` / `DEFAULT_MAP_CAP`: the shared limits (4 rps, 16 burst, 4096 entries) both validate throttles use, single-sourced so the defense-in-depth twins cannot drift.
- `validators` (`src/validators.rs`)
  - `valid_username`: 3-32 chars, `[a-z0-9-]`, no boundary hyphens.
  - `MAX_USERNAME_EDITS`: the lifetime rename cap (4).
- `workspace_admin_client` (`src/workspace_admin_client.rs`)
  - `WorkspaceAdminClient`: reqwest-backed client for the apex admin tree. 5-second timeout. Exposes `kill_user_tunnels(username) -> usize` and `list_user_tunnels(username) -> Vec<TunnelView>`. Bearer token lives inside.
  - `WorkspaceAdminError`: thiserror enum with `Upstream(String)` and `Reqwest(reqwest::Error)`.
- `devserver_gate` (`src/devserver_gate.rs`)
  - `Claims`: serde struct matching the entry / session JWT envelope (`iss`, `sub`, `drv`, `aud`, `typ`, `iat`, `exp`).
  - `TokenType::{Entry, Session}` and `encode_*` / `decode` helpers wrapping `jsonwebtoken` with HS256 hard-required. No `alg: none` path exists.
  - `DevserverGateError`: thiserror enum covering decode/signature failures, expiry, and aud / drv / typ mismatch.

## Public surface

`profile_client::ProfileClient`:

| Method                       | Purpose                                  |
|------------------------------|------------------------------------------|
| `get_user`                   | by uuid                                  |
| `find_user_by_username`      | case-insensitive handle lookup           |
| `find_user_by_identity`      | by (provider, subject)                   |
| `create_user`                | new user                                 |
| `update_avatar`              | best-effort avatar refresh               |
| `link_identity`              | attach OAuth identity to existing user   |
| `upsert_by_identity`         | atomic find-or-create-or-link            |
| `update_username`            | rename (consumes a slot)                 |
| `write_auth_audit`           | append login / logout / block event      |
| `delete_user`                | hard delete                              |
| `get_user_flags`             | resolved feature-flag map for one user   |
| `admin_list_flags`           | every flag + override count              |
| `admin_upsert_flag`          | idempotent flag create / update          |
| `admin_delete_flag`          | drop flag (cascades overrides)           |
| `admin_list_flag_overrides`  | per-user overrides on a flag             |
| `admin_upsert_flag_override` | set a per-user override                  |
| `admin_delete_flag_override` | clear a per-user override                |
| `create_workspace`           | idempotent workspace create              |
| `list_workspaces`            | owner's workspaces                       |
| `delete_workspace`           | drop workspace (cascades grants)         |
| `create_workspace_grant`     | create-or-promote a share grant          |
| `list_workspace_grants`      | grants on one workspace                  |
| `delete_workspace_grant`     | owner-scoped grant revoke                |
| `workspace_access`           | per-request access gate (`role` or 404)  |
| `list_owned_workspaces`      | workspaces the user shares + counts      |
| `list_incoming_shares`       | workspaces shared with the user          |
| `claim_grants`               | claim pending grants by verified emails  |

`workspace_admin_client::WorkspaceAdminClient`:

| Method                       | Purpose                              |
|------------------------------|--------------------------------------|
| `kill_user_tunnels(user)`    | bulk evict; returns count            |
| `list_user_tunnels(user)`    | snapshot for the dashboard           |

`devserver_gate`:

| Item                                              | Purpose                       |
|---------------------------------------------------|-------------------------------|
| `Claims` (serde)                                  | entry + session JWT envelope  |
| `encode_entry(secret, sub, drv, aud)`             | identity mints (30s exp)      |
| `encode_session(secret, sub, drv, aud)`           | workspace-proxy mints (24h)   |
| `decode(secret, token, typ, aud, drv)`            | verify; returns `Claims`      |

`static_files::serve` is a single async function with one type parameter for the embedded asset set. `token_bucket::TokenBucket`, `validators::valid_username`, `domain::Domains`, and `shutdown_signal` are plain types / functions described above.

## Key decisions

### No axum coupling in the data-layer types

`ProfileError`, `WorkspaceAdminError`, `DevserverGateError` and `Claims` are plain thiserror / serde types. Each consumer maps the error onto its local request-handler error via a `From` impl. Keeps gateway-common free of HTTP-framing decisions and lets each consumer decide whether a given variant is a distinct status or folds into another.

### `User` is the superset

`User` carries every field profile-service returns: `username_edits`, `avatar_url`, `blocked_at`, `block_reason`, `display_name`, `email`. Consumers ignore the fields they do not need. Splitting into per-consumer sub-structs would force parallel maintenance for negligible benefit.

### Shared devserver_gate

Both identity and workspace-proxy depend on the same JWT envelope and the same HS256 verification config (hard-required alg, no fallback). One module here is the canonical place for both; the secret is shared between the two services via env var (`WORKSPACE_GATE_SECRET`).

### `static_files::serve` is generic over `RustEmbed`

`rust_embed` resolves `#[folder = "web/dist/"]` relative to the crate that has the derive. Each consumer owns its own `Assets` struct; the shared crate cannot derive once and share the embedded bytes. The function takes the `R: RustEmbed` type parameter and calls `R::get(path)`; the consumer site is two lines of declaration plus one call.

Only identity-service ships an SPA, so it is the module's only consumer; the module stays generic in case a future service grows a UI.

### Banners stay per-consumer

Each consumer's "frontend not built" banner names the right crate and its right `npm run build` directory. Parameterising the banner template would obscure that; each consumer ships its own `&'static [u8]` constant.

## Invariants

- `gateway_common` does not pull axum or any HTTP-routing framework into its data-layer surface. axum is a dependency only because `static_files::serve` returns `axum::response::Response`; nothing in `profile_client`, `workspace_admin_client` or `devserver_gate` knows axum exists.
- Bearer tokens passed to `ProfileClient::new` or `WorkspaceAdminClient::new` live inside the client; the `Debug` impl deliberately elides the token.
- `devserver_gate` enforces `alg: HS256` on every decode. No "alg: none" is ever accepted; no asymmetric algorithm is enabled.
- HTTP calls run through one reqwest client per type with a fixed timeout (10s for profile, 5s for workspace-admin). New methods reuse the existing builder.

## Error model

`ProfileError`:

| Variant       | Construction                               |
|---------------|--------------------------------------------|
| `NotFound`    | upstream returned 404                      |
| `BadRequest`  | upstream returned 400 (body in payload)    |
| `Conflict`    | upstream returned 409 (body in payload)    |
| `Upstream`    | any other non-success status               |
| `Reqwest`     | `From<reqwest::Error>` for transport       |

`WorkspaceAdminError`:

| Variant       | Construction                               |
|---------------|--------------------------------------------|
| `Upstream`    | non-success status with body               |
| `Reqwest`     | `From<reqwest::Error>`                     |

`DevserverGateError`:

| Variant          | Construction                                     |
|------------------|--------------------------------------------------|
| `Decode`         | malformed token, unsupported alg, or HMAC verify failed |
| `Expired`        | `exp` in the past                                |
| `WrongAudience`  | `aud` claim does not match                       |
| `WrongWorkspace` | `drv` claim does not match                       |
| `WrongType`      | `typ` did not match expected value               |

Consumers map these into their local axum errors.

## What's wired

- `reqwest` with `rustls-tls` and `json` features (both clients)
- `chrono` for `DateTime<Utc>` in the user / audit types
- `serde` + `serde_json` for the wire shapes
- `thiserror` for the error enums
- `jsonwebtoken` for `devserver_gate`
- `axum` (response types only), `mime_guess`, `rust-embed` for `static_files::serve`
- std-only `domain`, `validators`, `token_bucket` (SipHash via `DefaultHasher`); `tokio::signal` for `shutdown`

## What is not wired

- Caching of profile responses (every call hits the upstream)
- Connection pooling beyond reqwest defaults
- Retries beyond the single idempotent-GET retry in `send_idempotent` (callers decide whether a write failure is fatal or fire-and-forget)
- Asymmetric JWT (HS256 is the only algorithm enabled)
- An axum middleware that rewrites client errors into responses directly (consumers do the mapping in their own `IntoResponse`)
