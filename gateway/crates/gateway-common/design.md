# gateway-common: design

## Problem

identity-service, drive-proxy and profile-service each had (or would
have had) their own copies of:

- a `ProfileClient` calling profile-service over HTTP, with
  near-identical method signatures and a duplicate `User` struct;
- a `DriveAdminClient` calling drive-proxy admin (used by identity on
  revoke / delete and by profile on admin block);
- a `static_files` handler embedding a Svelte SPA via `rust_embed`,
  identical except for the embed folder and a "frontend not built"
  banner string;
- the shared JWT shape used by the drive-gate handoff between
  identity (mint) and drive-proxy (verify + mint sessions).

Keeping the duplicates risked drift and made cross-cutting choices
(timeouts, error mapping, MIME guessing, signing claims) live in
multiple places.

## Architecture

Library crate with four modules and no axum / IntoResponse coupling
in the data-layer types:

- `profile_client` (`src/profile_client.rs`)
  - `ProfileClient`: reqwest-backed client with a 10-second
    per-request timeout. Bearer token lives inside; callers do not
    deal with auth.
  - `User`, `Identity`, `UpsertResponse`: serde types matching
    profile-service's wire shape. `User` is the superset of every
    field profile returns; consumers ignore the fields they do not
    need.
  - `ProfileError`: thiserror enum with `NotFound`,
    `BadRequest(String)`, `Conflict(String)`, `Upstream(String)`,
    `Reqwest(reqwest::Error)`.
- `drive_admin_client` (`src/drive_admin_client.rs`)
  - `DriveAdminClient`: reqwest-backed client for the apex
    `https://drive.chan.app` admin tree. 5-second timeout. Today
    exposes `kill_user_tunnels(username) -> usize` and the helpers
    needed by identity-side dashboard reads. Bearer token lives
    inside.
  - `DriveAdminError`: thiserror enum with `Upstream(String)` and
    `Reqwest(reqwest::Error)`.
- `drive_gate` (`src/drive_gate.rs`)
  - `Claims`: serde struct matching the entry / session JWT envelope
    (`iss`, `sub`, `drv`, `aud`, `typ`, `iat`, `exp`).
  - `TokenType::{Entry, Session}` and `encode_*` / `decode_*` helpers
    wrapping `jsonwebtoken` with HS256 hard-required. No `alg: none`
    path exists.
  - `DriveGateError`: thiserror enum covering expiry, signature,
    aud / drv / typ mismatch, and decode failures.
- `static_files` (`src/static_files.rs`)
  - `serve<R: RustEmbed>(uri, banner) -> Response`: SPA fallback
    handler. Tries the requested path, falls back to `index.html` for
    paths without an extension, falls back to the banner if no
    `index.html` is embedded (fresh checkout), else 404.

## Public surface

`profile_client::ProfileClient`:

| Method                  | Purpose                                  |
|-------------------------|------------------------------------------|
| `get_user`              | by uuid                                  |
| `find_user_by_identity` | by (provider, subject)                   |
| `create_user`           | new user                                 |
| `update_avatar`         | best-effort avatar refresh               |
| `link_identity`         | attach OAuth identity to existing user   |
| `upsert_by_identity`    | atomic find-or-create-or-link            |
| `update_username`       | rename (consumes a slot)                 |
| `write_auth_audit`      | append login / logout / block event      |
| `delete_user`           | hard delete                              |

`drive_admin_client::DriveAdminClient`:

| Method                       | Purpose                              |
|------------------------------|--------------------------------------|
| `kill_user_tunnels(user)`    | bulk evict; returns count            |
| `list_user_tunnels(user)`    | snapshot for the dashboard           |

`drive_gate`:

| Item                            | Purpose                            |
|---------------------------------|------------------------------------|
| `Claims` (serde)                | entry + session JWT envelope       |
| `encode_entry(secret, claims)`  | identity mints                     |
| `encode_session(secret, claims)`| drive-proxy mints                  |
| `decode(secret, token, aud)`    | both verify; returns `Claims`      |

`static_files::serve` is a single async function with one type
parameter for the embedded asset set.

## Key decisions

### No axum coupling in the data-layer types

`ProfileError`, `DriveAdminError`, `DriveGateError` and `Claims` are
plain thiserror / serde types. Each consumer maps the error onto its
local request-handler error via a `From` impl. Keeps gateway-common
free of HTTP-framing decisions and lets each consumer decide whether
a given variant is a distinct status or folds into another.

### `User` is the superset

`User` carries every field profile-service returns:
`username_edits`, `avatar_url`, `blocked_at`, `block_reason`,
`display_name`, `email`. Consumers ignore the fields they do not need.
Splitting into per-consumer sub-structs would force parallel
maintenance for negligible benefit.

### Shared drive_gate

Both identity and drive-proxy depend on the same JWT envelope and the
same HS256 verification config (hard-required alg, no fallback). One
module here is the canonical place for both; the secret is shared
between the two services via env var (`DRIVE_GATE_SECRET`).

### `static_files::serve` is generic over `RustEmbed`

`rust_embed` resolves `#[folder = "web/dist/"]` relative to the crate
that has the derive. Each consumer owns its own `Assets` struct; the
shared crate cannot derive once and share the embedded bytes. The
function takes the `R: RustEmbed` type parameter and calls
`R::get(path)`; the consumer site is two lines of declaration plus
one call.

After the wildcard-subdomain refactor drive-proxy no longer has an
SPA, so only identity-service uses this module today; it stays here
in case a future service grows a UI.

### Banners stay per-consumer

Each consumer's "frontend not built" banner names the right crate and
its right `npm run build` directory. Parameterising the banner
template would obscure that; each consumer ships its own
`&'static [u8]` constant.

## Invariants

- `gateway_common` does not pull axum or any HTTP-routing framework
  into its data-layer surface. axum is a dependency only because
  `static_files::serve` returns `axum::response::Response`; nothing
  in `profile_client`, `drive_admin_client` or `drive_gate` knows
  axum exists.
- Bearer tokens passed to `ProfileClient::new` or
  `DriveAdminClient::new` live inside the client; the `Debug` impl
  deliberately elides the token.
- `drive_gate` enforces `alg: HS256` on every decode. No "alg: none"
  is ever accepted; no asymmetric algorithm is enabled.
- HTTP calls run through one reqwest client per type with a fixed
  timeout (10s for profile, 5s for drive-admin). New methods reuse
  the existing builder.

## Error model

`ProfileError`:

| Variant       | Construction                               |
|---------------|--------------------------------------------|
| `NotFound`    | upstream returned 404                      |
| `BadRequest`  | upstream returned 400 (body in payload)    |
| `Conflict`    | upstream returned 409 (body in payload)    |
| `Upstream`    | any other non-success status               |
| `Reqwest`     | `From<reqwest::Error>` for transport       |

`DriveAdminError`:

| Variant       | Construction                               |
|---------------|--------------------------------------------|
| `Upstream`    | non-success status with body               |
| `Reqwest`     | `From<reqwest::Error>`                     |

`DriveGateError`:

| Variant            | Construction                          |
|--------------------|---------------------------------------|
| `Expired`          | `exp` in the past                     |
| `InvalidSignature` | HMAC verify failed                    |
| `WrongAudience`    | `aud` claim does not match            |
| `WrongType`        | `typ` did not match expected value    |
| `Decode`           | malformed token / unsupported alg     |

Consumers map these into their local axum errors.

## What's wired

- `reqwest` with `rustls-tls` and `json` features (both clients)
- `chrono` for `DateTime<Utc>` in the user / audit types
- `serde` + `serde_json` for the wire shapes
- `thiserror` for the error enums
- `jsonwebtoken` for `drive_gate`
- `axum` (response types only), `mime_guess`, `rust-embed` for
  `static_files::serve`

## What is not wired

- Caching of profile responses (every call hits the upstream)
- Connection pooling beyond reqwest defaults
- Retries with exponential backoff (callers decide whether a failure
  is fatal or fire-and-forget)
- Asymmetric JWT (HS256 is the only algorithm enabled)
- An axum middleware that rewrites client errors into responses
  directly (consumers do the mapping in their own `IntoResponse`)
