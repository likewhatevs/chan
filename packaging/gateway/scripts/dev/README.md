# Local dev stack

Bootstraps the chan-gateway services (postgres + profile + identity + devserver-control + one to three devserver-proxy nodes) against a workspace `cargo run` build, so you can browse `id.localtest.me:17000` and exercise the dashboard, OAuth flow, and workspace-gate handoff against the real binaries.

`*.localtest.me` resolves to `127.0.0.1` for every subdomain via public DNS, which sidesteps the `/etc/hosts` surgery you would otherwise need to test the wildcard-subdomain shape locally.

Dev port layout (offset by `+10000` from the prod-shaped ports so the runner can coexist with an existing Lima/sdme deployment on the default ports):

| Service          | Port    | URL                                            |
|------------------|---------|------------------------------------------------|
| profile          | `17001` | http://127.0.0.1:17001                         |
| identity         | `17000` | http://id.localtest.me:17000                   |
| devserver-control | `17003` | http://127.0.0.1:17003 (aggregate admin)      |
|                  | `17101` | h2c proxy control listener                     |
| devserver-proxy.pN | `17002` | http://pN.devserver.localtest.me:17002 (node) |
|                  | `17100` | 127.0.0.N:17100 (h2c tunnel)                   |

Every proxy node binds the same two ports on its own loopback alias (p1 on `127.0.0.1`, p2 on `127.0.0.2`, p3 on `127.0.0.3`) because the controller's origin template pins one shared port for the whole fleet. A `chan devserver` client dials one node's tunnel listener directly, e.g. `http://127.0.0.2:17100/v1/tunnel` for p2.

## One-time setup

1. Postgres reachable at `postgres://chan:chan@127.0.0.1/chan_gateway`. The dev runner uses the same database the integration tests expect (`chan_gateway`); if you do not have that DB yet, `createdb -U chan chan_gateway` against your local pg.

2. A GitHub OAuth dev app (Settings -> Developer settings -> OAuth Apps -> New OAuth App). Use: Homepage URL:       http://id.localtest.me:17000 Authorization callback: http://id.localtest.me:17000/auth/github/callback Copy the Client ID and a freshly-generated Client Secret.

3. Drop the GitHub creds into `packaging/gateway/scripts/dev/.env`:
   ```
   cp packaging/gateway/scripts/dev/env.example packaging/gateway/scripts/dev/.env
   $EDITOR packaging/gateway/scripts/dev/.env   # paste GITHUB_CLIENT_ID + GITHUB_CLIENT_SECRET
   ```

4. Generate secrets + ensure DB migrations are applied:
   ```
   packaging/gateway/scripts/dev/setup.sh
   ```
   This writes the service env files into `packaging/gateway/scripts/dev/secrets/` and runs profile-service once to apply migrations. Idempotent; re-run is a no-op unless you pass `--force`.

## Run

```
packaging/gateway/scripts/dev/run.sh
```

Spawns profile, identity, devserver-control, and one devserver-proxy in the foreground. Set `CHAN_DEV_PROXIES=3` to boot the full three-node fleet (p1-p3 on their own loopback aliases). Logs from all services multiplex to stdout, prefixed by service. Ctrl-C sends SIGINT to all of them and waits for clean shutdown. The controller holds a 30s convergence window on boot, so a fresh stack takes about half a minute before proxies report ready and admit tunnels.

Then open:

* http://id.localtest.me:17000 -- dashboard. Sign in with GitHub; GitHub redirects to the callback at `id.localtest.me:17000`, identity-service mints a session cookie (host-only on `id.localtest.me`), and the SPA loads.

The Workspaces tab is empty until a `chan devserver` instance registers a workspace (see below).

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
  --tunnel-url=http://devserver.localtest.me:17100/v1/tunnel \
  --tunnel-workspace-name=blog
```

The `http://` scheme on the URL triggers chan-tunnel-client's h2c path (no TLS); devserver-proxy p1's tunnel listener is bound to `127.0.0.1:17100` and speaks h2c directly. Once connected, the dashboard's Workspaces tab lists the workspace; clicking Open redirects the browser through `/api/workspaces/open` to the owning node's tenant host, `http://<user>--<disc>.p1.devserver.localtest.me:17002/blog/`.

## Notes

* No TLS anywhere in this stack. `COOKIE_SECURE=false` is set in the identity env so the session cookie survives `http://`. Do not mirror this config into prod.
* The workspace-gate JWT redirect uses `DEVSERVER_PUBLIC_SCHEME=http` and `DEVSERVER_PUBLIC_PORT=:17002`, so the URL identity builds points at the dev port. Production sets both to their defaults (`https` and empty).
* Postgres state persists across runs. To wipe and start fresh: `dropdb chan_gateway && createdb chan_gateway && packaging/gateway/scripts/dev/setup.sh`.
* Stopping `run.sh` with Ctrl-C is clean. If a service hangs, kill the process group with `kill -INT -- -<pgid>` -- the dev runner publishes its pgid as `packaging/gateway/scripts/dev/.run.pid` for that case.
