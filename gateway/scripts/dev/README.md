# Local dev stack

Bootstraps the four chan-gateway services (postgres + profile + identity + workspace-proxy) against a workspace `cargo run` build, so you can browse `id.localtest.me:17000` and exercise the dashboard, OAuth flow, and workspace-gate handoff against the real binaries.

`*.localtest.me` resolves to `127.0.0.1` for every subdomain via public DNS, which sidesteps the `/etc/hosts` surgery you would otherwise need to test the wildcard-subdomain shape locally.

Dev port layout (offset by `+10000` from the prod-shaped ports so the runner can coexist with an existing Lima/sdme deployment on the default ports):

| Service       | Port    | URL                                      |
|---------------|---------|------------------------------------------|
| profile       | `17001` | http://127.0.0.1:17001                   |
| identity      | `17000` | http://id.localtest.me:17000             |
| workspace-proxy   | `17002` | http://workspace.localtest.me:17002 (apex)   |
|               |         | http://*.workspace.localtest.me:17002 (wild) |
| workspace tunnel  | `17100` | http://workspace.localtest.me:17100 (h2c)    |

## One-time setup

1. Postgres reachable at `postgres://chan:chan@127.0.0.1/chan_gateway`. The dev runner uses the same database the integration tests expect (`chan_gateway`); if you do not have that DB yet, `createdb -U chan chan_gateway` against your local pg.

2. A GitHub OAuth dev app (Settings -> Developer settings -> OAuth Apps -> New OAuth App). Use: Homepage URL:       http://id.localtest.me:17000 Authorization callback: http://id.localtest.me:17000/auth/github/callback Copy the Client ID and a freshly-generated Client Secret.

3. Drop the GitHub creds into `scripts/dev/.env`:
   ```
   cp scripts/dev/env.example scripts/dev/.env
   $EDITOR scripts/dev/.env   # paste GITHUB_CLIENT_ID + GITHUB_CLIENT_SECRET
   ```

4. Generate secrets + ensure DB migrations are applied:
   ```
   scripts/dev/setup.sh
   ```
   This writes the four service env files into `scripts/dev/secrets/` and runs profile-service once to apply migrations. Idempotent; re-run is a no-op unless you pass `--force`.

## Run

```
scripts/dev/run.sh
```

Spawns profile, identity, and workspace-proxy in the foreground. Logs from all three multiplex to stdout, prefixed by service. Ctrl-C sends SIGINT to all three and waits for clean shutdown.

Then open:

* http://id.localtest.me:17000 -- dashboard. Sign in with GitHub; GitHub redirects to the callback at `id.localtest.me:17000`, identity-service mints a session cookie (host-only on `id.localtest.me`), and the SPA loads.

The Workspaces tab is empty until a `chan serve` instance registers a workspace (see below).

### First-time sign-in: enrol yourself

Both feature flags ship default-off, so the first OAuth sign-in will land on `/?denied=oauth_login` and the Workspaces tab stays hidden. Two-step bootstrap:

```sh
# 1. Try sign-in once. This creates your user row (the gate runs
#    after the row is upserted) and then 303s to the denied panel.
#    Pick up your email from the SPA panel or via:
PT=$(grep PROFILE_ADMIN_TOKEN secrets/profile.env | cut -d= -f2-)
CHAN_ADMIN_TOKEN="$PT" CHAN_ADMIN_PROFILE_URL=http://127.0.0.1:17001 \
  ../../target/debug/chan-gateway-admin user list

# 2. Grant the two seeded flags on your account, then sign in
#    again. Workspaces tab + share UI appear.
CHAN_ADMIN_TOKEN="$PT" CHAN_ADMIN_PROFILE_URL=http://127.0.0.1:17001 \
  ../../target/debug/chan-gateway-admin flag grant oauth_login <your-email>
CHAN_ADMIN_TOKEN="$PT" CHAN_ADMIN_PROFILE_URL=http://127.0.0.1:17001 \
  ../../target/debug/chan-gateway-admin flag grant share_workspaces <your-email>
```

Subsequent users repeat step 2. To open sign-in for everyone, flip the default with `chan-admin flag create oauth_login --default-on`.

## Registering a test workspace

In the `chan` repo (sibling of chan-gateway):

```
# Create a PAT under the Tokens tab on the dashboard, copy it.
export CHAN_TUNNEL_TOKEN=chan_pat_...

cd ../chan
cargo run -p chan -- serve <some-workspace-dir> \
  --tunnel-url=http://workspace.localtest.me:17100/v1/tunnel \
  --tunnel-workspace-name=blog
```

The `http://` scheme on the URL triggers chan-tunnel-client's h2c path (no TLS); workspace-proxy's tunnel listener is bound to `127.0.0.1:17100` and speaks h2c directly. Once connected, the dashboard's Workspaces tab lists the workspace; clicking Open redirects the browser through `/api/workspaces/open` to `http://<user>.workspace.localtest.me:17002/blog/`.

## Notes

* No TLS anywhere in this stack. `COOKIE_SECURE=false` is set in the identity env so the session cookie survives `http://`. Do not mirror this config into prod.
* The workspace-gate JWT redirect uses `WORKSPACE_PUBLIC_SCHEME=http` and `WORKSPACE_PUBLIC_PORT=:17002`, so the URL identity builds points at the dev port. Production sets both to their defaults (`https` and empty).
* Postgres state persists across runs. To wipe and start fresh: `dropdb chan_gateway && createdb chan_gateway && scripts/dev/setup.sh`.
* Stopping `run.sh` with Ctrl-C is clean. If a service hangs, kill the process group with `kill -INT -- -<pgid>` -- the dev runner publishes its pgid as `scripts/dev/.run.pid` for that case.
