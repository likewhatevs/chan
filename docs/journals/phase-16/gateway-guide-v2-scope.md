# gateway-guide-v2 scope (port change + all-sdme-container topology)

Scope-first for @@Lead -> @@Host. Two COUPLED asks landed: (1) change the
gateway default ports 7000->17000 (dodge macOS AirPlay), (2) rework the
guide so EVERY service runs as its own sdme container (like prod), nothing
on host. I read `../chan-prod-setup/` to ground this (it is checked out
locally; `chan-psql.sdme` is a sanitized dev copy of its prod version).
NOT writing or editing code yet, per "scope-first, do not guess."

## What I grounded from chan-prod-setup (the prod pattern)

- 5 containers: chan-psql, chan-profile, chan-id, chan-workspace-proxy,
  chan-nginx (TLS terminator + reverse proxy + certbot, the only one with
  `-p 80:443`).
- Per-service `.sdme`: `FROM ubuntu` -> `COPY dist` -> `apt install` the
  service `.deb` -> systemd drop-in points EnvironmentFile at bind-mounted
  `/run/chan-secrets/{domain,identity}.env` -> `systemctl enable`.
- Networking: all on a PRIVATE `chan-svc` zone (`--network-zone chan-svc`
  in bin/deploy.sh); siblings resolve by HOSTNAME (chan-id "talks to
  chan-profile:7001"), NOT 127.0.0.1. Not host networking.
- Build: `gateway-build.sdme` (already in gateway/scripts/dev/sdme/) is a
  Rust+cargo-deb build container that produces the .debs the service
  containers install.

## BLOCKER to resolve FIRST: does the topology make the port change moot?

The 7000->17000 change was to dodge macOS AirPlay (:7000/:7001) when
running gateway binaries BARE on the macOS host. But the topology decision
("all services in sdme containers, nothing on host") means no gateway
service binds on the macOS host at all - inside a Linux container on the
`chan-svc` zone, :7000 never touches the macOS host, so AirPlay cannot
collide.

So the port change is only needed if the LOCAL service containers use
HOST networking (the way dev chan-psql exposes Postgres on macOS
localhost:5432). If they mirror prod's private zone, the clash is gone and
the port change is moot.

QUESTION 1 (@@Host): with the all-container topology, do you still want the
7000->17000 default-port code change? Options:
  (a) DROP it - containers on a private zone never hit AirPlay; keep the
      code defaults at 7000 (matches prod env + chan-prod-setup, zero churn
      across ~15 gateway files + the prod repo).
  (b) KEEP it - you still sometimes run a service bare on the host, or want
      the out-of-box default airplay-safe regardless of topology. (Then I
      also need: do the packaging/.env + scripts/configure.sh + your
      chan-prod-setup nginx upstreams move to 17000 too, or stay 7000?)
My read: (a) is consistent with the container decision and avoids touching
the prod config surface. Your call.

## Guide-rework decisions I need (to write the container steps accurately)

QUESTION 2 - container source: create sanitized DEV copies of each service
`.sdme` (chan-id/chan-profile/chan-workspace-proxy/chan-nginx, mirroring how
chan-psql.sdme is a sanitized dev copy) under gateway/scripts/dev/sdme/?
Or, per "mirror LITTLE", show the PATTERN with ONE example + point at
chan-prod-setup for the rest? (I lean: pattern + one worked example +
reference, not 4 new full dev rootfs files.)

QUESTION 3 - local networking + browser reach: mirror prod's private
`chan-svc` zone (services resolve by hostname; only the nginx container
publishes ports), and on macOS-in-Lima expose the nginx container's :443 to
the macOS browser via Lima host-networking / a port-forward? Or keep
host-networking for the service containers locally (simpler, but then the
AirPlay/port question in Q1 comes back)?

QUESTION 4 - build/run flow: confirm the local flow is
gateway-build.sdme -> build the 4 .debs -> each service container COPYs
dist + apt-installs its .deb + systemd, then `sdme create --network-zone`
+ `sdme start`. (This is the prod flow via dev-sanitized .sdme files.)

## Plan once answered (no writing until then)

- If Q1=(a): no gateway code change; the guide-v2 rework alone (dev-setup.md
  topology rewrite, supersedes dfbf3c57's host-binary sections) + the
  per-service container steps per Q2/Q3/Q4.
- If Q1=(b): land the port code change first (config.rs defaults + cross-
  service URL defaults + the agreed doc/.env surface; surface is ~15 files,
  bigger than the original line-ref list - full map ready), then guide-v2.
- VERIFY: `make gateway-build` is linux-only; @@Lead's gate.sh excludes it.
  I'll verify via lima+sdme (gateway-build.sdme) and/or CI gateway-ci.yml,
  and call out that macOS-native can't gate it. Config-default unit tests
  (no Postgres) can run on macOS; integration tests need lima Postgres.

dfbf3c57 (host-binaries guide) stays committed; its topology sections get
rewritten in guide-v2. Holding for @@Host's Q1-Q4.
