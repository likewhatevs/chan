# devserver-proxy cutover runbook (Alex-owned ops)

Distilled from `design.md` §7 + §8 for the human-run cutover. Lead owns code + the staging smoke; Alex owns DNS/cert/nginx + the `chan-prod-setup` repo. This is a HARD cut (no overlap vhost): there are no live single-workspace users and the cutover is fresh-state (`rm -rf ~/.chan`), so the old `*.workspace.<domain>` surface is retired in the same change, not kept live (design §7.2).

## Order is load-bearing: GATEWAY first, CLIENT after
A chan client that defaults `--tunnel-url` to `https://devserver.chan.app/...` is broken until that host resolves, serves, and holds the matching `WORKSPACE_GATE_SECRET`. So: deploy the gateway + DNS, pass the staging smoke, THEN ship the chan release with the moved flags (design §8).

## Step 1 — DNS (add; do not remove old yet until step 5 nginx swap)
    devserver.<domain>        A/AAAA -> gateway host   (apex: admin + tunnel)
    *.devserver.<domain>      A/AAAA -> gateway host   (per-user devserver host)

## Step 2 — Wildcard cert (dns-01; http-01 cannot issue wildcards)
Issue `*.devserver.<domain>` via certbot dns-01 with the same provider plugin already used for `*.workspace.<domain>` (`gateway/docs/dev-setup.md:117,162`).

## Step 3 — chan-prod-setup repo (the separate repo, dev-setup.md:104)
Apply the matching rename in the prod copies Lead can't touch:
- service unit + env FILENAME: chan-gateway-workspace-proxy -> chan-gateway-devserver-proxy (`.service`, `EnvironmentFile`, `Description`, `Documentation`). The renamed crate ships these under `gateway/crates/devserver-proxy/packaging/` after Proxy's rename — mirror them into chan-prod-setup.
- Keep the SHARED-SECRET env VAR names generic (WORKSPACE_GATE_SECRET, WORKSPACE_ADMIN_TOKEN, IDENTITY_INTERNAL_TOKEN do NOT rename — design §2.1).
- `CHAN_DOMAIN` is unchanged; the derivation now yields `devserver.*` from it.

## Step 4 — Build + stage the renamed deb
Lead/CI builds `chan-gateway-devserver-proxy` (the renamed crate; bin `devserver-proxy-service`, package `devserver-proxy`) into the deb set. Confirm `make gateway-release-crates` lists the new name (the release matrix is single-sourced from the Makefile crate list, MEMORY release_gate_covers_every_workspace).

RELEASE-PIPELINE DRIFT (must fix before the next release; Proxy task-Lead-Proxy-4): `web-marketing/scripts/{generate-release-metadata,collect-release-assets, verify-release-assets}.mjs` hardcode `chan-gateway-workspace-proxy` -> they will break the install page + asset verification on the next release. Being single-sourced from the Makefile crate list (or name-updated). Verify this is landed before tagging the phase-2 client release.

## Step 5 — nginx server_name + grpc_pass (the swap; retire old in same change)
Edit `chan-prod-setup/etc/nginx/`. Route map (design §2.4, §7.2):
    id.<domain>                 -> unchanged
    <domain> apex               -> admin + healthz (unchanged)
    <domain>/v1/tunnel  (grpc)  -> :7100 h2c grpc_pass (PATH unchanged)
    *.workspace.<domain>        -> DELETE
    *.devserver.<domain>        -> devserver-proxy:7002   (NEW)
`/v1/tunnel` the path does NOT rename (generic registration endpoint).

## Step 6 — Deploy the renamed unit; stop + remove the old
Deploy `chan-gateway-devserver-proxy.service` + its env file; `systemctl stop` + remove `chan-gateway-workspace-proxy.service`. CHAN_DOMAIN unchanged.

## Step 7 — Staging smoke BEFORE any client ships (Lead-run, design §7.3)
Against the sdme/mkcert staging stack, under a throwaway HOME (fresh state):
1. `chan devserver` from this branch dials staging devserver-proxy with `--tunnel-token <staging-pat> --tunnel-url http://devserver.<staging-domain>/v1/tunnel` (no --tunnel-name/--tunnel-public).
2. Assert HelloAck::Ok + one (user, devserver) entry in proxy admin `GET /admin/v1/tunnels`.
3. Mount two workspaces (blog, journal); grant the DEVSERVER to a 2nd user via the profile per-devserver grant route.
4. OWNER opens `/blog/` then `/journal/` on `{user}.devserver.<staging>` — both pass on ONE `devserver_gate` cookie scoped `Path=/`.
5. GRANTEE opens both (pass); a NON-grantee 3rd user opens `/blog/` -> 404.
6. `/api/devserver/*` over the public wildcard -> 404 for everyone.
7. WebSocket: open a tenant terminal through the wildcard; the WS bridge survives.
Only after 1-7 pass does the chan client release ship.

## Step 7b — Enable the sign-in + dashboard feature flags (CRITICAL)
Two `feature_flags` defaults are OFF and must be enabled in prod (surfaced by the 3b smoke; live per-request resolve, no restart):
- **`oauth_login` (CRITICAL)** — default OFF DENIES all sign-in (identity http.rs:366 -> `/?denied=oauth_login`). Without it NO ONE can sign in. Enable it.
- **`share_workspaces`** — default OFF hides the dashboard Devservers tab. Enable it for the per-devserver dashboard to show. (The `share_workspaces` -> `share_devservers` rename is a deferred follow-up; the name is a misnomer now but functionally gates sharing.)

## Step 7c — Prod GitHub OAuth app (distinct from the dev app)
The dev OAuth app (creds in `packaging/gateway/scripts/dev/.env`) is STAGING-SMOKE ONLY; never ship it. Prod identity needs its own GitHub OAuth app:
- Authorization callback URL = `https://id.chan.app/auth/github/callback` (identity host UNCHANGED by this migration — only `workspace.* -> devserver.*` changed; `redirect_uri` is BASE_URL-derived, so prod `BASE_URL=https://id.chan.app` yields this callback automatically; cutover-safe, proven by Proxy).
- Set its `GITHUB_CLIENT_ID` / `GITHUB_CLIENT_SECRET` in the PROD identity service via the prod secret mechanism (GitHub Actions Secrets / `chan-prod-setup`), NEVER in chat/journals/commits (secrets boundary).
- If prod ALREADY has a working OAuth app for `id.chan.app`, it keeps working unchanged (the callback host didn't move); only create a new one if prod has none yet.

## CONFIRMED nginx fix — long-lived tunnel 60s flap (apply at cutover)
Proxy's live 3b smoke surfaced a ~60s tunnel drop+reconnect; root cause CONFIRMED on the live stack (hypothesis-tested, not the h2-split theory): nginx **`client_body_timeout` (default 60s)** times out the long-lived `/v1/tunnel` POST request-body read on an idle uplink. The grpc_pass/tunnel path is unchanged by this migration -> PRE-EXISTING, present in prod. FIX (verified: 0 exits in 85s, was 1/60s): on the apex `/v1/tunnel` server block set **`client_body_timeout 1d;` + `client_header_timeout 1d;`** (alongside the existing `grpc_read_timeout`/`grpc_send_timeout 1d`). One line each; apply to the prod nginx at cutover (step 5). Captured in the dev gen.sh. Detail: task-Proxy-Profile-6.

## Step 8 — Ship the chan release (phase 2)
First chan release AFTER the gateway is live: moved flags, `--tunnel-url` default = `https://devserver.chan.app/v1/tunnel`. Ride-along: native-Windows chan + Windows install.sh + Windows devserver-daemon flag + the A2 connect-URL dropdown example.
