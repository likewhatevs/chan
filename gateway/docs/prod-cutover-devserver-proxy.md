# Prod cutover: workspace-proxy -> devserver-proxy

Task doc for the agent operating the prod server's `chan-prod-setup` clone. Apply
the changes below to that local clone, then deploy. This is the infra half of the
gateway "devserver-proxy migration"; the code half (the renamed crate + the
per-devserver gate) lands separately in the `chan` source the build pulls from.

The authoritative design (for a human, not needed to execute this) lives in the
`chan` repo at `dev/v0.42.0-gateway/design.md` + `gateway/docs/adr/0001-devserver-is-the-sharing-unit.md`.

## What is changing and why

The gateway is moving from one-tunnel-per-workspace to one-tunnel-per-devserver.
The public service renames `workspace-proxy` -> `devserver-proxy` and the public
host family moves `workspace.chan.app` -> `devserver.chan.app`. This is a pure
INFRA rename for `chan-prod-setup`: container/service names, nginx server_names
and upstreams, the DNS records, the wildcard cert SAN, and the env filename. The
gate-model change (per-devserver auth, drop of the public path) is entirely inside
the `.deb` code; nothing in this repo encodes it.

Pre-release, no users, fresh state: this is a HARD cut. Bring the new surface up,
verify, then retire the old. No back-compat, no dual-run beyond the verify window.

## Prerequisite (do this first)

The `.deb` packages are built from the `chan` source at `$(CHAN_SRC)` (defaults to
`../chan`) by `bin/build-debs.sh`. That source MUST already contain the renamed
`devserver-proxy` crate (gateway Track B landed: branch `gateway-devserver-proxy`
or merged). Confirm before building:

```
ls $(CHAN_SRC)/gateway/crates/devserver-proxy/Cargo.toml   # must exist
grep -R "name = \"workspace-proxy\"" $(CHAN_SRC)/gateway    # must be empty
```

If the source still has `workspace-proxy`, STOP: the code migration has not landed
and this cutover would build the old binary under a new name.

## Invariants -- do NOT change these

- Ports: `:7002` (axum HTTP, admin + healthz + tenant content) and `:7100` (raw
  h2c tunnel). nginx grpc_pass of `/v1/tunnel` -> `:7100` is unchanged.
- The endpoint path `/v1/tunnel` does NOT rename.
- The multi-SAN cert still lives at `/etc/letsencrypt/live/id.chan.app/`; all
  vhosts keep referencing `id.chan.app/fullchain.pem`. You are adding the
  `devserver` SANs to that cert, not making a new live dir.
- Shared-secret / code env VARIABLE NAMES stay: `WORKSPACE_GATE_SECRET`,
  `WORKSPACE_ADMIN_TOKEN`, `WORKSPACE_ADMIN_URL`, `CHAN_ADMIN_WORKSPACE_URL`,
  `IDENTITY_INTERNAL_TOKEN`, `PROFILE_*`. The gateway crates still read these
  names (renaming a shared secret var is a coordinated two-sided change for no
  gain). Only their URL VALUES change (the container hostname they point at).
- `PROFILE_*`, `chan-psql`, `chan-profile`, `chan-id`, `chan-nginx` are untouched.

## The rename map

```
workspace-proxy                  ->  devserver-proxy        (crate / service base)
chan-workspace-proxy             ->  chan-devserver-proxy   (sdme service + container)
chan-gateway-workspace-proxy     ->  chan-gateway-devserver-proxy  (deb + systemd unit)
workspace-proxy.env              ->  devserver-proxy.env    (secrets filename)
workspace.chan.app               ->  devserver.chan.app     (apex host)
*.workspace.chan.app             ->  *.devserver.chan.app   (wildcard host)
HOST_WORKSPACE                   ->  HOST_DEVSERVER         (prod-setup-local var)
```

Container hostnames in URL VALUES move with the service rename, e.g.
`http://chan-workspace-proxy:7002` -> `http://chan-devserver-proxy:7002`,
`grpc://chan-workspace-proxy:7100` -> `grpc://chan-devserver-proxy:7100`.

## File-by-file changes

Apply the rename map. Use `git mv` for the file renames so history follows. After
editing, grep the whole repo for residuals (see Verify). Your clone's line numbers
may differ; match on content, not line numbers.

1. `services/chan-workspace-proxy.sdme` -> `git mv` to `services/chan-devserver-proxy.sdme`.
   Inside: the `ls /tmp/dist/chan-gateway-devserver-proxy_*.deb` glob; the
   systemd drop-in dir + `systemctl enable chan-gateway-devserver-proxy`; the
   `EnvironmentFile=/run/chan-secrets/devserver-proxy.env`; the header comments
   (`workspace.chan.app` -> `devserver.chan.app`).

2. nginx vhosts -- `git mv` + edit `server_name` and upstreams:
   - `etc/nginx/conf.d/workspace.chan.app.conf` -> `devserver.chan.app.conf`
     (`server_name workspace.chan.app;` -> `devserver.chan.app;`).
   - `etc/nginx/conf.d/wildcard.workspace.chan.app.conf` ->
     `wildcard.devserver.chan.app.conf` (`server_name *.workspace.chan.app;` ->
     `*.devserver.chan.app;`).
   - `etc/nginx/tls-vhosts/workspace.chan.app.tls.conf` ->
     `devserver.chan.app.tls.conf`: `server_name`; `grpc_pass
     grpc://chan-workspace-proxy:7100` -> `chan-devserver-proxy:7100`; both
     `set $upstream "chan-workspace-proxy:7002"` -> `chan-devserver-proxy:7002`.
   - `etc/nginx/tls-vhosts/wildcard.workspace.chan.app.tls.conf` ->
     `wildcard.devserver.chan.app.tls.conf`: `server_name`; the
     `set $upstream "chan-workspace-proxy:7002"` upstream.
   - `etc/nginx/nginx.conf`: the two comments mentioning `*.workspace.chan.app`
     and `chan-workspace-proxy`.

   NOTE: `bin/setup-tls.sh` installs the `tls-vhosts/*.tls.conf` files into
   `/etc/nginx/conf.d/` by name; if it greps or lists them by the old filename,
   update that too (see #4).

3. `.env.example`: `HOST_WORKSPACE=workspace.chan.app` -> `HOST_DEVSERVER=devserver.chan.app`;
   the `CHAN_DOMAIN` derivation comment (`workspace.<CHAN_DOMAIN>` /
   `*.workspace.<CHAN_DOMAIN>` -> `devserver.*`). If a real `.env` exists on the
   host, make the same edit there.

4. `bin/setup-tls.sh`: `HOST_WORKSPACE` default -> `HOST_DEVSERVER=devserver.chan.app`;
   every `workspace.chan.app` / `*.workspace.chan.app` in the SAN list and
   comments -> `devserver`. The wildcard `*.devserver.chan.app` SAN still goes
   through DNS-01 (certbot-dns-cloudflare), HTTP-01 cannot issue a wildcard.

5. `bin/build-debs.sh`: `cargo build ... -p workspace-proxy` -> `-p devserver-proxy`;
   the `for c in profile identity workspace-proxy admin` loop -> `devserver-proxy`.
   (Deb output becomes `chan-gateway-devserver-proxy_*.deb`, matching #1's glob.)

6. `Makefile`, `bin/status.sh`, `bin/deploy.sh`: the `SERVICES` lists
   `chan-workspace-proxy` -> `chan-devserver-proxy`; in `deploy.sh` the
   per-service `case` arm and the `*.workspace.chan.app` SAN comment.

7. `bin/secrets-init.sh`: the `write_env ".../workspace-proxy.env"` target ->
   `devserver-proxy.env`; the URL VALUES `http://chan-workspace-proxy:7002` ->
   `http://chan-devserver-proxy:7002` (both `WORKSPACE_ADMIN_URL` and
   `CHAN_ADMIN_WORKSPACE_URL`); comments. KEEP the var NAMES per Invariants.

8. `etc/postgresql/postgresql.conf`: the comment listing
   `chan-{profile,id,workspace-proxy}` (cosmetic).

9. `README.md`: the service table row, the DNS table (`workspace.chan.app` +
   `*.workspace.chan.app` -> `devserver.*`), the tunnel-URL and tenant-content
   lines, and the prose. (Docs; cosmetic but keep it truthful.)

## DNS (Cloudflare, before the cert step)

Add, pointing at the same host IP as the existing `workspace` records:

```
devserver.chan.app      A    <host IPv4>   (AAAA if you run v6)
*.devserver.chan.app    A    <host IPv4>
```

Leave the old `workspace` records in place until the verify passes; remove them at
the end (Retire).

## Cert

Re-issue the multi-SAN cert so it covers the devserver hosts (the wildcard via
DNS-01). The CF API token (Zone:DNS:Edit on chan.app) is already in the secrets;
`bin/setup-tls.sh` uses it. After the rename in #4:

```
make build-debs           # rebuild with the devserver-proxy crate
sudo bin/setup-tls.sh      # adds devserver.chan.app + *.devserver.chan.app SANs
```

Confirm the live cert now lists the devserver SANs:
`openssl x509 -in /etc/letsencrypt/live/id.chan.app/fullchain.pem -noout -text | grep -i dns`.

## Cutover sequence

1. Apply all file changes (above); `git diff` and review.
2. `make build-debs` (builds `chan-gateway-devserver-proxy_*.deb` from the renamed
   crate). Fails fast if the prerequisite was skipped.
3. Add the DNS records.
4. `sudo bin/setup-tls.sh` (wildcard `*.devserver` SAN via DNS-01).
5. `sudo bin/secrets-init.sh` (writes `devserver-proxy.env`, updates the URL
   values; var names unchanged).
6. `sudo bin/deploy.sh` (rebuilds + restarts the `chan-devserver-proxy` container
   and `chan-nginx`; `nginx -t` runs as part of it -- if not, run `sudo nginx -t`
   before reload).
7. Verify (below).
8. Retire: once verify passes, remove the old `workspace` DNS records, drop the
   `workspace` SANs on the next cert renewal, and delete any leftover
   `chan-workspace-proxy` container/unit/env on the host
   (`sdme rm chan-workspace-proxy`; `rm /run/chan-secrets/workspace-proxy.env`).

## Verify

- `bin/status.sh` shows `chan-devserver-proxy` healthy; no `chan-workspace-proxy`.
- `sudo nginx -t` passes; `https://devserver.chan.app/healthz` returns 200.
- Cert lists `devserver.chan.app` + `*.devserver.chan.app` (openssl grep above).
- A `chan devserver --tunnel-url https://devserver.chan.app/v1/tunnel
  --tunnel-token <pat>` registers (the apex grpc_pass to `:7100` works); the
  registry shows the `(user, devserver)` entry via `bin/chan-gateway-admin`.
- Through a browser: `{user}.devserver.chan.app/{workspace}/` serves tenant content
  after the id sign-in handoff; `/api/devserver/*` returns 404 over the wildcard
  (management is local-only by design); a WebSocket tenant terminal stays open.
- Residual sweep (must be empty except CHANGELOG/history prose):
  `grep -rn --exclude-dir=.git -iE 'workspace-proxy|workspace\.chan\.app' .`

## Rollback

Pre-release, fresh state, so rollback is config-only: `git revert` the cutover
commit (or check out the prior tag), `make build-debs`, `sudo bin/deploy.sh`, and
re-point DNS to `workspace.chan.app` if you had already removed it. No data
migration is involved in either direction.
