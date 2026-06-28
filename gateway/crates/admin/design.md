# chan-gateway-admin: design

## Problem

Operators need to manage users, tokens, and live tunnels without SSH plus direct database access. A CLI is faster for routine tasks (block a user, check an audit log, kill a tunnel) than a web UI; it composes with shell loops, jq pipelines, and CI.

## Architecture

Single Rust binary that talks HTTP to profile-service and devserver-proxy. No database access; the CLI only consumes the existing admin HTTP routes.

The `tokio` runtime is `current_thread` (commands are sequential and short-lived; a multi-threaded runtime is unnecessary overhead). `clap` derives the command tree.

Two HTTP clients live inside the binary:

- `AdminClient`: profile-service (`CHAN_ADMIN_PROFILE_URL`). Resolves `<ident>` and calls the admin tree.
- `WorkspaceClient`: devserver-proxy apex (`CHAN_ADMIN_WORKSPACE_URL`, typically `https://devserver.chan.app`). Talks to `/admin/v1/*` and decodes the SSE snapshot stream for `tunnel watch`.

Both clients use plain `reqwest` with a shared bearer configuration; the CLI sets a 15-second per-call timeout (no global timeout on the watch stream so it can idle between snapshots).

## Public surface

Full command list is in [`README.md`](README.md). Notable behaviours:

### `<ident>` resolution

`AdminClient::resolve_user` tries, in order:

1. Parse `ident` as a uuid; if it parses, fetch by id.
2. If it contains `@`, treat as an email substring; require exactly one match.
3. Else treat as an exact username and require exactly one match.

Ambiguous matches (more than one) error with `ambiguous`; no match errors with the `NotFound` exit code.

### `user block`

The block flow lives server-side: profile-service fans out to devserver-proxy in the same operation. The CLI still calls both for robustness against split deployments where the CLI's devserver-proxy URL may differ from the profile container's view of devserver-proxy, but the second call is idempotent (`killed: 0` is fine when profile already swept the registrations).

Order:

1. `POST /v1/admin/users/:id/block` on profile-service. This sets `blocked_at`, revokes every live PAT, writes an `auth_audit` row and, server-side, fires devserver-proxy `kill_user_tunnels` for the user. If profile fails the CLI stops here.
2. `POST /admin/v1/users/:user/tunnels/kill` on devserver-proxy. Belt-and-braces. A failure here surfaces as a warning on stderr but does not change the profile-side outcome.

The ordering ensures a partial failure leaves the user in a "blocked but maybe a tunnel still alive" state rather than the inverse, which is the safer direction.

### `flag`

Manage feature flags and per-user overrides via profile-service's admin tree. `flag list` and `flag overrides <key>` render a table / `--json`; `flag create` is idempotent (re-issuing for the same key bumps `default_enabled` and description); `flag grant <key> <ident> [--enabled|--disabled]` upserts the per-user override, and `flag revoke` clears it. `<ident>` resolution is the same uuid / email / username pipeline as the user subcommand. Default for `flag grant` is `--enabled`; `--disabled` lets an operator record a deny override against a default-on flag.

### `tunnel watch`

devserver-proxy's `/admin/v1/tunnels/watch` is an SSE stream. `watch_loop` consumes the stream, parses `event: snapshot` blocks, and re-renders. TTY mode clears the screen between renders (`\x1b[2J\x1b[H`) so the output behaves like `watch -n1`. `--json` emits one JSON line per event for `jq` piping.

### Output

Default rendering uses `comfy_table` with the `NOTHING` preset (no Unicode lines), targeting 80 columns. Columns are chosen per command (e.g. `USER`, `WORKSPACE`, `PUBLIC`, `PEER`, `UPTIME`, `CONNECTED` for `tunnel ps`). UUIDs are truncated to 8 chars in table mode.

`--json` emits prettified JSON via `serde_json::to_string_pretty` because operators copy-paste output into tickets; the small overhead is fine for CLI workloads.

## Key decisions

### Single bearer

`CHAN_ADMIN_TOKEN` is the only credential knob: one secret, sent to both services (single-token deployments set `PROFILE_ADMIN_TOKEN` and `WORKSPACE_ADMIN_TOKEN` to the same value). Deployments that rotate the two service tokens independently pass `--token` per invocation with the value matching the service that invocation talks to.

### Exit codes are part of the contract

0 / 1 / 2 / 3 are documented in the README and used by shell wrappers (CI, smoke tests, ops scripts). Adding a new exit code is a public-API change; rotating the existing meanings is not allowed.

### --json everywhere

Every read command supports `--json` so the CLI can be piped into jq. Adding a new subcommand without `--json` would be a regression in operability; reviewers should reject such PRs.

### No interactive features

No menus, no TUI. All commands are non-interactive except for `user delete` which prompts `[y/N]` (skippable with `--yes`). The CLI is meant to compose with `xargs` and `parallel`.

### Minimal local URL encoding

Path segments contain only `[a-z0-9_.-]` (validated upstream), so the CLI ships a tiny inline `urlencoding::encode_path` rather than pulling in a real urlencoding crate. The full RFC 3986 table is overkill for a value that already passed username / workspace-name validation.

## Invariants

- The CLI is read-mostly. State changes go through documented HTTP routes; there are no direct database writes.
- A blocked user always has every PAT revoked; the profile block flow handles this server-side, and also fires devserver-proxy eviction in the same transaction.
- `user delete` cascades through profile-service's FK chain; the CLI does not orchestrate the deletion across multiple endpoints.
- `tunnel kill` is idempotent: a second kill of the same registration returns 404, which the CLI surfaces as exit 3.
- Output is deterministic on TTY-vs-`--json` choice. No mixed output on a single command.

## Error model

Errors surface as `anyhow::Error` chains; the top-level dispatch calls `eprintln!("error: {e:#}")` and exits with the code from `exit_code_for`. `ClientError` is the typed boundary between HTTP responses and the CLI:

| `ClientError`     | Exit | Notes                                  |
|-------------------|------|----------------------------------------|
| `NotFound`        | 3    | upstream returned 404                  |
| `BadInput(s)`     | 2    | upstream returned 400 (with body)      |
| `Upstream{...}`   | 1    | any other non-2xx status               |

Network failures (`reqwest::Error`) reach `exit_code_for` as plain `anyhow::Error` instances, which exit 1.

## What's wired

The crate list mirrors `Cargo.toml`; only the non-obvious choices need explaining. `reqwest` carries the `stream` feature so `tunnel watch` can consume the SSE byte stream incrementally (`tokio-stream` drives the byte chunks). The `tokio` runtime is `current_thread` because the commands are sequential and short-lived, so a multi-threaded runtime is wasted overhead.

## What is not wired

- Shell completion (`clap` has `--generate` for it; not generated yet)
- A config file (`~/.chan-gateway-admin/config.toml`)
- Batch operations (`--input batch.jsonl`)
- Inline editor for `user update --email` (the CLI takes a flag, no `$EDITOR` round trip)
