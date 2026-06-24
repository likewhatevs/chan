# devserver tunnel e2e (cross-container, sdme)

An end-to-end test that drives a real HTTP request through a real
`devserver-proxy-service` and a real `chan devserver --tunnel-url`, running in
two separate sdme containers, over the gateway tunnel, into a mounted
workspace. A `200` carrying the workspace SPA is the proof that the
`--tunnel-url` path works against a real proxy. The production chan-gateway
run (`--tunnel-url` against `devserver.chan.app`) is a separate follow-up.

## What it proves

```
host  curl ─▶ devserver-proxy :7002  (public surface, Host-routed)
                  │  gate: devserver_gate session cookie (HS256, aud+drv)
                  ▼
              tunnel :7100  (h2c, devserver-proxy is the tunnel SERVER)
                  ▲
                  │  chan devserver dialed in and registered (PAT validated)
              chan devserver  ─▶  workspace `notes` mounted at /notes-<hash8>
```

The request `GET /notes-<hash8>/` with `Host: alice.devserver.localtest.me`
returns `200` with the workspace SPA shell — identifiable by the injected
`<meta name="chan-prefix" content="/notes-<hash8>">`. That response only exists
if the request reached the devserver's mounted tenant THROUGH the proxy and the
tunnel. A `504`/`404` would mean it did not.

The two binaries under test — `devserver-proxy-service` and `chan` (with the
tunnel-client/-proto crates) — are real release builds. Two pieces are shims,
because auth is not what this test exercises:

- **stub identity** (`stub-identity.py`): the proxy validates each tunnel dial's
  PAT against identity-service's `/internal/v1/tokens/validate`. The stub
  answers that one endpoint with a fixed `{user_id, username, devserver_id,
  scopes:["tunnel"]}`, so the proxy stays real without standing up
  postgres + profile + identity. It runs on the proxy container's loopback.
- **gate cookie** (`mint-gate-token.py`): the proxy gates every tenant request
  with a `devserver_gate` HS256 cookie (`gateway-common::devserver_gate`). We
  hold `WORKSPACE_GATE_SECRET`, so the harness mints a `session` token directly
  (`sub`=user_id, `drv`=devserver_id, `aud`=Host) — `Gate::Pass`, no redirect.

## Topology: why one zone, two containers

The charter asked for two SEPARATE sdme zones. On this host that is not
reachable with the round's privileges, and the e2e documents why
(`zone-isolation-probe.sh`). Measured sdme 0.9.0 network behaviour here:

| path                              | result  |
|-----------------------------------|---------|
| host → container                  | OK      |
| container → host (TCP)            | BLOCKED (ICMP only — host INPUT firewall) |
| container → container, same zone  | OK      |
| container → container, cross zone | BLOCKED |
| `-p` published port, cross zone   | BLOCKED |

Each zone bridge (`vz-<zone>`) reuses `169.254.0.0/16`, and the host drops
container-initiated TCP to itself and forwards nothing between zone bridges. So
a container can only initiate TCP to a **same-zone** peer. The tunnel is
`chan devserver → devserver-proxy` (client → server), so the two must share a
zone. Bridging two isolated zones would need host `iptables`/forwarding changes
(root); this round's sudo is `sdme`-only (`NOPASSWD /usr/local/bin/sdme`).

The containers are still fully separate (own netns, fs, process tree); the
tunnel genuinely crosses between two containers. Only the L2 zone is shared.
Running the proxy and devserver in two ACTUAL zones is a host-networking
follow-up (open the firewall / add inter-zone forwarding as root), tracked for
@@Alex alongside the production `--tunnel-url` e2e.

## Run

```sh
# 1. Build the two release binaries inside the toolchain container (one-time,
#    ~10 min cold; reuses the cargo cache after). No host rust toolchain needed.
gateway/scripts/dev/sdme/devserver-tunnel-e2e/build-bins.sh

# 2. Run the e2e (builds the chan-e2e-run runtime rootfs on first run).
gateway/scripts/dev/sdme/devserver-tunnel-e2e/run.sh

# tear down
gateway/scripts/dev/sdme/devserver-tunnel-e2e/run.sh --clean

# the network finding, standalone
gateway/scripts/dev/sdme/devserver-tunnel-e2e/zone-isolation-probe.sh
```

`run.sh` leaves the containers up on PASS for inspection
(`sudo sdme join gw-e2e-proxy`, `sudo sdme logs gw-e2e-ds`).

## Files

| file                      | role                                                   |
|---------------------------|--------------------------------------------------------|
| `build-bins.sh`           | build `devserver-proxy-service` + `chan` in a container |
| `chan-e2e-run.sdme`       | runtime rootfs (ubuntu + iproute2 + curl + python3)     |
| `run.sh`                  | stand up both containers, register, drive the request   |
| `stub-identity.py`        | minimal identity `/internal/v1/tokens/validate` stub    |
| `mint-gate-token.py`      | mint a `devserver_gate` HS256 session cookie            |
| `zone-isolation-probe.sh` | demonstrate same-zone OK / cross-zone BLOCKED           |

## Config the harness sets

| name                     | value                                   |
|--------------------------|-----------------------------------------|
| `APEX_HOST`              | `devserver.localtest.me`                |
| `WILDCARD_SUFFIX`        | `.devserver.localtest.me`               |
| `FORWARDED_PROTO`        | `http` (no TLS anywhere; tunnel is h2c) |
| proxy public / tunnel    | `0.0.0.0:7002` / `0.0.0.0:7100`         |
| `IDENTITY_URL`           | `http://127.0.0.1:7799` (loopback stub) |
| `CHAN_DEVSERVER_LISTEN`  | `1` (bind the mgmt API so the host can read the mounted prefix) |
| tenant                   | user `alice`, workspace `notes`         |
