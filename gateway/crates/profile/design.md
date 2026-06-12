# profile-service: design

## Problem

Other gateway services need a single, authoritative store for user
identity. Two writers must not race when a user signs in for the
first time on two providers concurrently; an admin block must revoke
every live PAT in one operation and tear down the user's live
yamux registrations on workspace-proxy; renames must be capped so the
public `chan.app/{username}` namespace doesn't churn.

## Architecture

Small axum service in front of Postgres. Schema:

- `users (id, email, display_name, username, username_edits,
  created_at, updated_at, blocked_at, block_reason, avatar_url)`
- `identities (id, user_id, provider, provider_subject, email,
  created_at)` with `UNIQUE (provider, provider_subject)`
- `api_tokens (id, user_id, label, token_hash, expires_at,
  created_at, revoked_at, last_used_at, scopes)`
- `api_token_audit (id, ts, token_id, action, ip, user_agent)`
- `auth_audit (id, ts, user_id, action, ip, user_agent, note)`
- `workspaces (id, owner_user_id, workspace_name, created_at)` with
  `UNIQUE (owner_user_id, workspace_name)`. First-class entity for an
  owner's workspace: lets the dashboard list a workspace that has no grants
  and no live tunnel yet, and acts as the FK target for grants.
- `workspace_grants (id, owner_user_id, workspace_name, grantee_email,
  grantee_user_id, role, created_at, accepted_at)` with
  `UNIQUE (owner_user_id, workspace_name, lower(grantee_email))` and an
  FK on `(owner_user_id, workspace_name)` -> `workspaces` (cascade delete).
- `feature_flags (key PK, description, default_enabled, created_at,
  updated_at)`: registry of named flags.
- `feature_flag_overrides (flag_key, user_id, enabled, set_at,
  PRIMARY KEY (flag_key, user_id))`: per-user explicit
  enable/disable rows. The effective value for `(flag, user)` is
  the override row when present, else `default_enabled`.

Migrations live in the workspace root `migrations/` directory and
run on startup.

The router splits into three sub-routers:

- `/v1/users/*` and `/v1/auth-audit`: gated by `auth` middleware.
  Either `PROFILE_AUTH_TOKEN` or `PROFILE_ADMIN_TOKEN` admits.
- `/v1/admin/*`: gated by `admin_auth` middleware. Only
  `PROFILE_ADMIN_TOKEN` admits.
- `/healthz`: no auth.

All bearer comparisons run through `subtle::ConstantTimeEq`
(`bearer_eq` in `http.rs`). Both checks always run on the service
API so a wrong token cannot oracle which leg matched first.

profile-service holds a `WorkspaceAdminClient` (from
`gateway_common::workspace_admin_client`) when `WORKSPACE_ADMIN_URL` and
`WORKSPACE_ADMIN_TOKEN` are set. The block flow fires
`kill_user_tunnels` server-side at the same moment `blocked_at` is
written, so live workspace-proxy registrations die without an extra hop
from the operator CLI.

## Public surface

JSON in, JSON out. Status codes:

- 200 OK: successful read or update
- 201 Created: successful create
- 204 No Content: successful delete or audit write
- 400 Bad Request: malformed input (missing email, bad uuid)
- 401 Unauthorized: missing or wrong bearer
- 404 Not Found: user / token id absent
- 409 Conflict: unique violation (`23505`); used for username taken
  and rename-cap reached

Full HTTP route list is in [`README.md`](README.md).

## Key decisions

### Two-tier auth, single-token-friendly

Routes split into "service" and "admin" tiers, each gated by a
distinct env var. The service-tier middleware also accepts the admin
token, so a single-token deployment (one secret in vault, both env
vars set to it) works without code changes. Deployments that want
independent rotation set the env vars to different values; the gate
logic does not care.

`bearer_eq` runs both checks unconditionally to avoid leaking
which-token-matched timing.

### Atomic upsert by identity

`POST /v1/users/upsert-by-identity` is one transaction:

1. Look up `(provider, provider_subject)` in `identities`.
2. If found, update `users.avatar_url` if it changed and return the
   user with `user_created=false, identity_created=false`.
3. If not, look up `users` by email (case-insensitive). If found,
   insert a new `identities` row pointing at the existing user.
4. If still nothing, insert the user (with placeholder username) and
   the identity row in the same transaction.

The single transaction is what closes the orphan window when two
browser tabs race a first-time login. Concurrent calls can still
collide on the unique indexes; a caller that retries on `23505` hits
step 1 or step 2 on the retry and converges.

### Deterministic placeholder usernames

New users get `u<12 hex chars from the row id>` as a placeholder
handle. identity-service renames on first sign-in (the SPA prompts
for one). The `u`-prefix shape lets future admin queries identify
never-renamed accounts trivially. Real users cannot collide because
the unique index plus the rename CAS prevent it.

### Rename cap of 4

`update_username` runs a single CTE that performs the CAS update
and selects the "rename to current value" no-op row in one
statement:

```
WITH current AS (
    SELECT id, lower(username) AS handle, username_edits
    FROM users WHERE id = $1
),
renamed AS (
    UPDATE users
       SET username = $2, username_edits = username_edits + 1, ...
     WHERE id = $1
       AND id IN (SELECT id FROM current
                  WHERE username_edits < 4 AND handle <> $2)
    RETURNING ...
)
SELECT * FROM renamed
UNION ALL
SELECT ... FROM users
WHERE id = $1 AND lower(username) = $2
  AND NOT EXISTS (SELECT 1 FROM renamed)
```

When the CTE returns no rows the handler runs one follow-up SELECT
to distinguish "user not found" (404) from "rename cap reached"
(409). Collapsing the original two-statement diagnosis into the CTE
closes the TOCTOU window where a concurrent rename could change
state between the CAS UPDATE and the diagnostic SELECT. The unique
index on `lower(username)` still raises `23505` on the rare name
collision, which surfaces as 409 with the database's error
message.

### Block fans out to workspace-proxy server-side

`POST /v1/admin/users/:id/block`:

1. Set `users.blocked_at = now()` and `block_reason` in one
   transaction with the next two steps.
2. Update `api_tokens` to set `revoked_at = now()` for every live
   PAT belonging to the user.
3. Append an `auth_audit` row with action `blocked`.
4. Best-effort: if a `WorkspaceAdminClient` is configured, call
   workspace-proxy `/admin/v1/users/{username}/tunnels/kill` to evict
   live yamux substreams. Failures are logged at warn; the next
   handshake from a peer with a stale PAT fails on the DB join
   anyway, so the gap closes either way.

workspace-proxy is the authority on live registrations; profile is the
authority on `blocked_at`. The block flow keeps both views consistent
within the same operation.

### Email rewrite is admin-only

`PATCH /v1/users/:id` (the service-tier route) accepts only
`display_name` and `avatar_url`. Email is the identity-linking key
in `upsert_by_identity` branch (b): a service-bearer holder that
could rewrite email could pivot account ownership to any account
whose verified OAuth email matched the new value. Email mutation
therefore lives behind the admin bearer on
`POST /v1/admin/users/:id/email`, runs in a single transaction with
an `auth_audit` row of action `email_changed` (note carries the
old + new addresses), and surfaces unique-constraint conflicts as
409.

### Workspaces are first-class

A workspace is a row in `workspaces` keyed on `(owner_user_id, workspace_name)`.
The dashboard creates one before either grants land or `chan serve`
registers a live tunnel, so the offline state is always
representable. Live tunnels (held in workspace-proxy's in-memory
Registry) reference the same `(owner, workspace_name)` pair; the FK
from `workspace_grants` -> `workspaces` makes workspace deletion atomic
(cascading every grant on it).

`POST /v1/users/:owner/workspaces` is idempotent: 201 on insert, 200
when the name already existed. `POST .../grants` upserts the
parent `workspaces` row in the same transaction (so a caller that
skips the explicit workspace-create still produces a valid graph and
the FK never fires).

### Per-workspace sharing grants

A user can share one of their workspaces (the path segment served at
`{owner}.workspace.chan.app/{workspace}/`) with another user by email.
Grants live in `workspace_grants` keyed on
`(owner_user_id, workspace_name, lower(grantee_email))`:

- The owner pre-seeds grants from id.chan.app's SPA *before* (or
  alongside) running `chan serve --tunnel-workspace-name=<name>`. The grant
  row exists independently of any live tunnel.
- `grantee_user_id` is `NULL` until a sign-in is observed with a
  verified email matching `grantee_email`. Two resolution paths:
  (a) at grant-create time, if `users` already has a row for the
  email; (b) at OAuth-callback time, via `POST
  /v1/users/:id/grants/claim` which identity-service calls with the
  union of the user's verified emails.
- `role` is one of `viewer`, `editor`. Re-adding the same email on
  the same `(owner, workspace)` is idempotent: the SQL is
  `INSERT ... ON CONFLICT DO UPDATE SET role = EXCLUDED.role`, with
  `grantee_user_id` and `accepted_at` preserved via `COALESCE` so a
  role change does not re-pend an already-claimed grant.

Access decisions: identity-service calls
`GET /v1/users/:owner/workspaces/:workspace/access?as=<caller_user_id>`
before minting a workspace-gate entry JWT. The response is
`{role: "owner"|"editor"|"viewer"}` on access, 404 otherwise. The
404 shape is shared with "unknown workspace": neither the access
endpoint nor the share landing page leaks which workspaces an owner is
sharing.

Workspace-name normalization: handler lowercases + trims and rejects
anything outside `[a-z0-9._-]{1,64}` so the stored value is always
the canonical path segment workspace-proxy serves. Email uniqueness is
case-insensitive via a functional `lower(grantee_email)` index;
display preserves the as-typed casing.

Listings: `GET /v1/users/:id/grants/owned` returns
`(workspace_name, grant_count)` per workspace the user has configured shares
on; `GET /v1/users/:id/grants/incoming` returns workspaces shared *with*
the user (claimed grants only). FK cascades on `users(id)` drop
grants when either the owner or the grantee is deleted.

### Feature flags

Two-tier table layout (`feature_flags` + `feature_flag_overrides`)
behind admin endpoints. Resolution is `COALESCE(override.enabled,
flag.default_enabled, false)` so unknown flags are closed by
default. identity-service reads the resolved map for a user via
`GET /v1/users/:id/flags` (service tier) to gate OAuth sign-in
(`oauth_login`) and to surface UI affordances on the SPA
(`share_workspaces`).

The seeded flags ship `default_enabled = false`, so a fresh deploy
refuses every sign-in until an operator grants `oauth_login` on at
least one user. Override-or-default keeps the rollout knob simple:
flip the default once the feature is ready for everyone; revoke
the per-user override for a deny rule. Audit-style history is the
`set_at` column on each override; full audit is deferred.

### All SQL is parameterized

Column lists are constants `format!`'d into queries; user input
always rides through `.bind()` at `$N`. Substring search on email in
the admin list endpoint uses `position($1 in lower(email)) > 0` with
the substring as a bound parameter.

## Invariants

- `users.email` is `NOT NULL`, indexed by `lower(email)`.
- `identities` has `UNIQUE (provider, provider_subject)`.
- `users.username_edits` only increases; never reset.
- `users.blocked_at` is `NULL` or a timestamp; `NULL` means active.
- `api_token_audit.action` is one of `created`,
  `created_via_desktop`, `used`, `revoked`.
- Block always: revokes every active PAT, fires the workspace-proxy
  eviction (if configured), appends one `auth_audit` row.
- Bearer comparisons run at constant time.
- `workspace_grants.role` is one of `viewer`, `editor` (CHECK
  constraint). `accepted_at` is `NULL` iff `grantee_user_id` is
  `NULL`; both flip together at claim time.

## Error model

`profile::Error` (`src/error.rs`):

| Variant       | HTTP | Notes                              |
|---------------|------|------------------------------------|
| Unauthorized  | 401  | bearer missing or wrong            |
| NotFound      | 404  | user / token id missing            |
| BadRequest    | 400  | input validation                   |
| Conflict      | 409  | unique violation, rename cap       |
| Db (sqlx)     | 500  | logged at error level              |
| Anyhow        | 500  | startup / unexpected               |

Database errors are logged with `tracing::error!(error = ?e, ...)`;
clients see a generic `internal error` message.

## What's wired

- axum 0.7 HTTP server
- sqlx with `runtime-tokio` + `tls-rustls` + Postgres
- `subtle` for constant-time bearer comparison
- `tower-http` tracing layer
- `gateway_common::workspace_admin_client::WorkspaceAdminClient` (best-effort
  workspace-proxy eviction on admin block)
- migrations checked on startup

## What is not wired

- mTLS (auth is Bearer only)
- Soft deletes (delete cascades via FK)
- Rate limiting on the service API (mitigated at the network layer;
  admin tree is bearer-gated by a separate token)
