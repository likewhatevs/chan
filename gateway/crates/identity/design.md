# identity-service: design

## Problem

Public sign-in surface plus the only user-facing UI in the
chan-gateway suite. Owns:

- OAuth2 + PKCE sign-in against multiple providers.
- The session cookie. Host-only on `id.chan.app`; it never spans
  subdomains.
- Personal access tokens for the chan CLI / chan-tunnel.
- The dashboard: profile management plus the live-workspace list.
- Workspace-gate entry-token mint for the cross-origin handoff to
  `workspace.chan.app`.

Profile data (canonical user record, identities, audit) lives in
profile-service; identity must not race with itself or duplicate user
rows on concurrent first-time logins.

## Architecture

axum HTTP server with three layers of routing under `id.chan.app`:

1. `/auth/*`: pre-session OAuth flow. Sets a transient session key
   (`pending_oauth`) carrying CSRF state and the PKCE verifier; the
   callback consumes it and either upgrades the session to
   authenticated (`user_id`) or fails.
2. `/api/*`: session-gated JSON API for the embedded SPA. Covers
   `me`, profile management, PAT lifecycle, the workspace list, and the
   workspace-gate mint endpoint.
3. `/internal/v1/tokens/validate`: Bearer-gated endpoint called by
   chan-tunnel-server during handshake. Lives on its own sub-router
   so the session middleware doesn't try to load a cookie session for
   a non-cookie caller. A per-token-fingerprint throttle wraps it as
   defense in depth alongside the primary throttle in workspace-proxy.

Static SPA assets are baked in at build time via `rust_embed` and
served by `gateway_common::static_files::serve`. Anything not matched
by an explicit route falls through to the static handler; paths
without an extension serve `index.html` (SPA fallback).

The session layer (`SessionManagerLayer` from `tower_sessions`) sits
at the outermost edge and applies to every route. **Cookie scope is
host-only on `id.chan.app`.** No `Domain` attribute. workspace-proxy does
not share this cookie.

## Public surface

Full route table is in [`README.md`](README.md). Highlights:

### OAuth flow

`/auth/:provider` (GET):

1. Look up the provider config. Unknown provider returns 404.
2. Generate `(authorize_url, csrf_state, pkce_verifier)`.
3. Insert `PendingOauth { provider, state, verifier }` into the
   session under `pending_oauth`.
4. Redirect to `authorize_url`.

`/auth/:provider/callback` (GET):

1. Read `code` and `state` from query params; refuse on `?error=...`.
2. Remove `pending_oauth` from the session (consume on read).
3. Compare `state` with `pending.state` constant-time. State check
   runs before the non-constant-time provider compare so timing on
   the provider field cannot be used to oracle the session's
   expected provider.
4. Compare `provider` (URL path) with `pending.provider`.
5. Exchange the code at the provider with the PKCE verifier.
6. Fetch user info from the provider's REST endpoint.
7. `profile.upsert_by_identity` (one HTTP round trip, one Postgres
   transaction). Returns the user record.
8. If `user.is_blocked()`, write a `login_denied` audit row and
   return 403 (`Error::Forbidden`).
9. Resolve `profile.get_user_flags(user.id)`. If `oauth_login`
   resolves false, write a `login_denied` audit row (with note
   `oauth_login flag not granted`) and 303 to
   `/?denied=oauth_login`. The SPA's Login view reads the
   query param and renders a "sign-in is closed" panel. The
   gate runs *before* `cycle_id` so a denied callback never
   carries an authenticated session.
10. **Rotate the session id (`session.cycle_id()`)** at the privilege
    boundary, before storing `user_id`. Closes session fixation.
11. Insert `user_id`, write a `login` audit row, claim any pending
    workspace grants for this user's verified emails, 303 to the
    stashed post-login URL (or `/`).

### PAT lifecycle

PAT shape: `chan_pat_<32 random bytes, base64url, no pad>`.

- Random bytes from `rand::rngs::OsRng`.
- Hash: `SHA-256(token)` stored in `api_tokens.token_hash`. Plaintext
  leaves on the create response and is never persisted.
- Validate (`/internal/v1/tokens/validate`):
  - Per-token-fingerprint throttle (4 rps refill, 16 burst,
    4096-entry LRU map). Throttled requests return 401, identical on
    the wire to an unknown token.
  - `WHERE t.token_hash = $1 AND t.revoked_at IS NULL AND
     (t.expires_at IS NULL OR t.expires_at > now()) AND
     u.blocked_at IS NULL` joined to `users`. One statement does
    the lookup and bumps `last_used_at`.
  - Append `used` to `api_token_audit`.
- Revoke (`DELETE /api/tokens/:id`):
  - Mark the row revoked.
  - Best-effort: call workspace-proxy admin `kill_user_tunnels` for the
    user. Per-PAT eviction is not possible today
    (chan-tunnel-server does not track which token registered which
    substream); the conservative call is kill-all.

### Dashboard

`/api/me`:

1. Resolve `user_id` from the session.
2. `profile.get_user(uid)`. Flush session and 401 if the user is gone
   underneath the cookie.
3. Call workspace-proxy admin `GET /admin/v1/users/{username}/tunnels`
   for the live-workspace list. Empty for blocked users.
4. Return `{user, workspaces: [TunnelView]}`.

The SPA renders one card per workspace. Each card's "open" link points
at `/api/workspaces/open?u={user}&d={workspace}` (server-side, see below) so
the entry token is minted at click time, not at page-render time
(otherwise short-exp tokens go stale before the user clicks).

### Workspace-gate mint

`GET /api/workspaces/open?u={user}&d={workspace}`:

1. Resolve session; refuse if anonymous or blocked.
2. Resolve `u` to a user record via profile
   `GET /v1/users/by-username`. Unknown handle returns 404 (same
   shape as no-access and unknown-workspace).
3. Call profile `GET /v1/users/{owner_id}/workspaces/{d}/access?as=
   {session.user_id}`. Owner returns `owner`, an accepted grantee
   returns `viewer`/`editor`, anything else 404.
4. Verify the workspace is live on workspace-proxy for `u` (cheap defense-
   in-depth; workspace-proxy is the authority on registrations).
5. Mint a 30s `entry` JWT (HS256, `WORKSPACE_GATE_SECRET`) with
   `{sub: session.user_id, drv: d, aud: "{u}.workspace.chan.app", ...}`.
   `sub` is the *caller's* id, not the owner's, so the workspace_gate
   cookie minted on the next leg carries the right identity for
   upstream collab attribution.
6. 303 to `https://{u}.workspace.chan.app/{d}/?t=<jwt>`.

workspace-proxy verifies the JWT, mints its own 24h session-shape JWT,
sets it as a `Path=/<workspace>/` host-only cookie, and 303s to the clean
URL. The shared JWT type lives in `gateway_common::workspace_gate`.

### Share landing

`GET /s/:owner/:workspace` is the public entry for copied share links.
It is intentionally unauthenticated at the door so the owner can
mint a URL that works for any recipient.

1. Validate `owner` (username shape) and `workspace` (1-64 lowercase
   alnum + `[._-]`); malformed values 404.
2. No session: stash `/s/:owner/:workspace` under
   `post_login_redirect` and 303 to `/`. The SPA renders the OAuth
   picker; on callback, the stash is consumed and the user lands
   back here with a fresh session.
3. With a session: resolve owner -> profile access check -> mint
   entry JWT -> 303 to workspace-proxy. Same code path as
   `/api/workspaces/open` once auth is established.

The post-login redirect is validated to start with a single `/`
and to contain no `:` or `//` prefix, so a hostile stash cannot
point the callback at another origin.

### Per-workspace sharing grants (SPA surface)

The owner manages workspaces and grants from the dashboard. Routes
(all session-gated; the session user is implicitly the owner):

- `POST /api/workspaces` body `{workspace_name}` (idempotent create; the
  workspace persists with no grants and no live tunnel, so the
  dashboard can show it offline)
- `DELETE /api/workspaces/:workspace` (FK cascade drops every grant)
- `POST /api/workspaces/:workspace/grants` body `{grantee_email, role}`
  (idempotent create / role-promote; auto-upserts the parent workspace
  on the profile side)
- `GET  /api/workspaces/:workspace/grants`
- `DELETE /api/grants/:id`
- `GET  /api/workspaces/owned` (workspaces I own; from the `workspaces` table
  joined with grant counts)
- `GET  /api/workspaces/incoming` (workspaces shared with me)

All forward to profile-service over the service bearer. Validation
re-runs in profile; identity does only the cheap shape check before
the round trip.

### Feature flags

identity reads the per-user resolved flag map from profile
(`GET /v1/users/:id/flags`) at two points:

- OAuth callback (`oauth_login`): the allowlist gate described in
  the callback flow above. Fresh deploys ship `default_enabled =
  false`, so the operator must `chan-admin flag grant oauth_login
  <ident>` for the first user before they can sign in.
- `/api/me` (full map): the SPA gates UI affordances on the
  resolved values. Today that's `share_workspaces` (hides the Workspaces
  tab and the share panel inside `Workspaces.svelte` when off). The
  map is re-fetched on every `/api/me`, so a rollout takes effect
  on the next dashboard reload — no SPA logout / login dance.

Profile errors on either call degrade-soft: identity falls back to
an empty flag map, which is the safe default (every flag off = no
sign-in, no UI features). Tracing log captures the failure so the
operator can see why callers were getting denied.

### Claim sweep on OAuth callback

After `upsert_by_identity`, identity calls
`POST /v1/users/:id/grants/claim` with the user's primary email
plus the freshly-observed provider email (deduped). Pending grants
whose `grantee_email` matches any of those addresses are assigned
to `:id` and stamped `accepted_at = now()`. Best-effort: a failure
logs and continues so an unhealthy profile call does not block
sign-in. Previous providers' emails are not resent — they were
swept on their own callbacks.

### Account delete

`DELETE /api/profile`:

1. Look up the user (need the username for the next step).
2. `profile.delete_user(uid)`. FK cascades clean up identities and
   `api_tokens`.
3. Best-effort: call workspace-proxy `kill_user_tunnels(username)` so
   live yamux registrations die at the same time the DB row goes.
4. Flush the session.

## Key decisions

### Pluggable providers

Each provider lives at `src/providers/<name>.rs` behind a small
`Provider` trait (authorize_url, exchange, fetch user info). Adding
a new provider is one new file plus wiring in `Config::from_env`.

Not wired:

- **Microsoft**: tenant admins can mint accounts whose verified email
  is unverifiable from the SaaS side. Email-based linking (used by
  `upsert_by_identity`) would let those accounts attach to existing
  users.
- **Apple**: high setup friction (signing key + team id + key id + JWT
  rotation) for the projected user share.

### Email-based identity linking lives in profile

Handled by profile-service's `upsert_by_identity`. identity passes
the email along; profile decides whether the provider context
warrants linking. Server-side decision blocks two identity callers
from racing on the link.

### Username rules

`valid_username` (in `http.rs`):

- 3-32 chars total
- first and last char in `[a-z0-9]`
- inner chars in `[a-z0-9-]`

Plus:

- `RESERVED_USERNAMES` blocks anything that could collide with a
  top-level path under `chan.app/`. Sorted alphabetically; checked
  with `binary_search`.
- `rustrict` filter blocks profanity / leet-speak heuristically.
  False positives surface as 400; users can unblock specific handles
  via the `RUSTRICT_ALLOWLIST` env var (comma-separated,
  case-insensitive).

### Session contract

- Cookie name `id_session`. **Host-only on `id.chan.app`.** No
  `Domain` attribute.
- `HttpOnly`, `SameSite=Lax`, 30-day inactivity expiry.
- `Secure` follows the `COOKIE_SECURE` env var.
- workspace-proxy does **not** read this cookie. Cross-service auth flows
  through the workspace-gate JWT, not through cookie sharing.

### Session id rotates on login

`session.cycle_id()` runs immediately before storing `user_id` on a
successful OAuth callback. Prevents an attacker-planted session
cookie from being carried into the authenticated state.

### Constant-time everywhere

- OAuth `state` compared with `subtle::ConstantTimeEq`.
- Internal validate bearer compared the same way.
- PAT validate compares hashes (the upstream lookup is a parameterised
  SQL query, not a string compare).

### Workspace-gate mint, not session sharing

`WORKSPACE_GATE_SECRET` is the only credential identity uses to talk
auth to workspace-proxy. It is distinct from `PROFILE_AUTH_TOKEN`,
`IDENTITY_INTERNAL_TOKEN` and `WORKSPACE_ADMIN_TOKEN`. identity uses it
only to mint `typ: entry` tokens; workspace-proxy uses it to verify
those and to mint its own `typ: session` cookies.

### IDENTITY_INTERNAL_TOKEN is required and distinct

The bearer workspace-proxy presents on `/internal/v1/tokens/validate` is
`IDENTITY_INTERNAL_TOKEN`. It is required; there is no fallback to
`PROFILE_AUTH_TOKEN`. Rotating one bearer never rotates another.

### PAT validate runs its own throttle

Mirror of workspace-proxy's `ThrottlingValidator`. Throttled requests
return 401, identical on the wire to an unknown token, so the
throttle is not observable from the outside. The workspace-proxy
throttle catches the typical case; this one catches a leaked
internal bearer being used to brute-force PATs directly.

### Domain config is single-source

The public hostnames are derived from one base domain (`CHAN_DOMAIN`,
e.g. `chan.app`) plus `PUBLIC_SCHEME`, via
`gateway_common::domain::Domains`. identity-service and workspace-proxy
read the same two vars and derive the same `id.<base>` /
`workspace.<base>` / `.workspace.<base>` hosts, so they cannot drift.
This matters because the workspace-gate JWT `aud` is the inbound host:
if the two services disagreed on the domain, the handoff would fail or
isolation assumptions would shift.

identity derives `BASE_URL` (its OAuth-callback origin) and
`workspace_wildcard_suffix` from `CHAN_DOMAIN`; the fine-grained vars
(`BASE_URL`, `WORKSPACE_WILDCARD_SUFFIX`, `WORKSPACE_PUBLIC_SCHEME`)
remain as explicit overrides for non-default layouts (e.g. a dev port).
Defaults are dev-shaped (`localtest.me` / `http`); production sets
`CHAN_DOMAIN` + `PUBLIC_SCHEME` once in the shared
`/etc/chan-gateway/domain.env`. The domain is still coupled to DNS, the
wildcard TLS cert, and nginx `server_name`, so it is deploy-time
config, not a runtime knob.

## Invariants

- A signed-in session always carries `user_id: Uuid` under `KEY_USER`.
- `pending_oauth` is removed on the first read in the callback. A
  cold-reloaded callback (missing pending) returns 400, not a fresh
  flow.
- Blocked accounts cannot start a session: the login flow writes
  `login_denied` and returns 403.
- Accounts whose `oauth_login` flag resolves to false cannot start
  a session either: the login flow writes `login_denied` and 303s
  to `/?denied=oauth_login` so the SPA can explain why.
- PATs hash to `SHA-256(token)`; plaintext is never persisted.
- Session id rotates on every successful sign-in.
- The id.chan.app cookie has no `Domain` attribute; it never spans
  subdomains.
- Bearer comparisons run at constant time.

## Error model

`identity::Error` (`src/error.rs`):

| Variant       | HTTP | Notes                                  |
|---------------|------|----------------------------------------|
| Unauthorized  | 401  | session missing or invalid             |
| Forbidden     | 403  | account blocked                        |
| BadRequest    | 400  | input or OAuth-flow failure            |
| NotFound      | 404  | unknown provider, missing user / token |
| Conflict      | 409  | username taken, rename cap reached     |
| Upstream      | 502  | profile / workspace-proxy unhappy          |
| Anyhow        | 500  | startup or unexpected                  |
| Reqwest       | 502  | network failure to a sibling service   |

`From<gateway_common::profile_client::ProfileError>` and
`From<gateway_common::workspace_admin_client::WorkspaceAdminError>` plug
sibling-service errors into the local enum so request handlers can
`?` straight through.

## What's wired

- axum 0.7 + `tower_sessions` + Postgres session store (host-only
  cookie scope)
- `oauth2` crate with `rustls-tls` for PKCE + token exchange
- `reqwest` for profile-service, workspace-proxy admin, and the OAuth
  providers' REST APIs
- `gateway-common` for the profile-service client, the workspace-admin
  client, the shared workspace_gate JWT type, and the SPA static-asset
  handler
- `jsonwebtoken` for HS256 entry-token mint
- `subtle`, `rustrict`, `rand::rngs::OsRng`, `sha2::Sha256`
- Svelte SPA embedded at build time

## What is not wired

- WebAuthn / passkeys
- Magic-link sign-in
- Device flow for headless clients
- Per-PAT scopes (PATs authenticate a user; workspace-gate enforces workspace
  ownership separately at the URL layer)
- Refresh of the workspace-gate session cookie on the workspace.chan.app side
  (24h hard exp; users re-enter via the dashboard)
