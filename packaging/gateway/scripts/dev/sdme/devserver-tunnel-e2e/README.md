# devserver tunnel e2e (cross-container, sdme)

An end-to-end test that drives authenticated desktop entry URLs through a real `devserver-proxy-service` and a real `chan devserver --tunnel-url`, running in two separate sdme containers, over the gateway tunnel, into a mounted workspace. A `200` carrying the workspace SPA is the data-path proof. The same sessions exercise both native-trust mutation routes: an owner reaches the desktop-bridge guard, while an editor is rejected by `require_local_mutation`. The production chan-gateway run (`--tunnel-url` against `devserver.chan.app`) is a separate follow-up.

## What it proves

```
stub identity ─▶ exact desktop entry response (owner + editor)
                         │
host  curl ─▶ devserver-proxy :7002  (public surface, Host-routed)
                  │  gate: devserver_gate session cookie (HS256, aud+drv)
                  ▼
              tunnel :7100  (h2c, devserver-proxy is the tunnel SERVER)
                  ▲
                  │  chan devserver dialed in and registered (PAT validated)
              chan devserver  ─▶  workspace `notes` mounted at /notes-<hash8>
```

The request `GET /notes-<hash8>/` at the exact `{owner}--{disc}` origin returns `200` with the workspace SPA shell, identifiable by the injected `<meta name="chan-prefix" content="/notes-<hash8>">`. The entry response is bearer-gated and pins `username`, the full 64-character devserver id, `proxy_origin`, and same-origin `entry_url`; the proxy exchanges its entry JWT for session + CSRF cookies. That response only exists if the request reached the mounted tenant through the proxy and tunnel.

With those authenticated sessions, the harness sends both `PUT` and `DELETE` to `/api/library/devservers/{id}/native-trust`. The owner gets the expected `409` no-desktop result, proving the guarded route was reached on this headless devserver. The editor gets the exact `403` from `require_local_mutation`, proving a shared tunnel caller cannot cross the mutation gate.

The two binaries under test -- `devserver-proxy-service` and `chan` (with the tunnel-client/-proto crates) -- are real release builds. Two pieces are shims because the rig does not stand up postgres-backed identity/profile:

- **stub identity** (`stub-identity.py`): the proxy validates each tunnel dial's PAT against `/internal/v1/tokens/validate`. The stub returns the fixed tunnel identity and exposes bearer-gated owner/editor `/desktop/v1/devserver/entry` responses. It runs on the proxy container's loopback.
- **entry JWT mint** (`mint-gate-token.py`): the stub's entry responses carry HS256 tokens matching `gateway-common::devserver_gate`, including role and caller identity. The real proxy verifies them, issues its own session + CSRF cookies, and signs the per-request gateway assertion consumed by the real devserver.

## Topology: why one zone, two containers

The charter asked for two SEPARATE sdme zones. On this host that is not reachable with the round's privileges, and the e2e documents why (`zone-isolation-probe.sh`). Measured sdme 0.9.0 network behaviour here:

| path                              | result                                   |
|-----------------------------------|------------------------------------------|
| host → container                  | OK                                       |
| container → host (TCP)            | BLOCKED (ICMP only; host INPUT firewall) |
| container → container, same zone  | OK                                       |
| container → container, cross zone | BLOCKED                                  |
| `-p` published port, cross zone   | BLOCKED                                  |

Each zone bridge (`vz-<zone>`) reuses `169.254.0.0/16`, and the host drops container-initiated TCP to itself and forwards nothing between zone bridges. So a container can only initiate TCP to a **same-zone** peer. The tunnel is `chan devserver → devserver-proxy` (client → server), so the two must share a zone. Bridging two isolated zones would need host `iptables`/forwarding changes (root); this round's sudo is `sdme`-only (`NOPASSWD /usr/local/bin/sdme`).

The containers are still fully separate (own netns, fs, process tree); the tunnel genuinely crosses between two containers. Only the L2 zone is shared. Running the proxy and devserver in two ACTUAL zones is a host-networking follow-up (open the firewall / add inter-zone forwarding as root), tracked for Alex alongside the production `--tunnel-url` e2e.

## Run

```sh
# 1. Build the two release binaries inside the toolchain container (one-time,
#    ~10 min cold; reuses the cargo cache after). No host rust toolchain needed.
packaging/gateway/scripts/dev/sdme/devserver-tunnel-e2e/build-bins.sh

# 2. Run the e2e (builds the chan-e2e-run runtime rootfs on first run).
packaging/gateway/scripts/dev/sdme/devserver-tunnel-e2e/run.sh

# tear down
packaging/gateway/scripts/dev/sdme/devserver-tunnel-e2e/run.sh --clean

# the network finding, standalone
packaging/gateway/scripts/dev/sdme/devserver-tunnel-e2e/zone-isolation-probe.sh
```

`run.sh` leaves the containers up on PASS for inspection (`sudo sdme join gw-e2e-proxy`, `sudo sdme logs gw-e2e-ds`).

## Files

| file                      | role                                             |
|---------------------------|--------------------------------------------------|
| `build-bins.sh`           | container-build of the two release binaries      |
| `chan-e2e-run.sdme`       | runtime rootfs (ubuntu, iproute2, curl, python3) |
| `run.sh`                  | stand up containers, register, drive the request |
| `stub-identity.py`        | tunnel validation + authenticated entry stub     |
| `mint-gate-token.py`      | mint role/identity-bearing entry or session JWTs |
| `zone-isolation-probe.sh` | demonstrate same-zone OK / cross-zone BLOCKED    |

## Config the harness sets

| name                    | value                                              |
|-------------------------|----------------------------------------------------|
| `APEX_HOST`             | `devserver.localtest.me`                           |
| `WILDCARD_SUFFIX`       | `.devserver.localtest.me`                          |
| `FORWARDED_PROTO`       | `http` (no TLS anywhere; tunnel is h2c)            |
| proxy public / tunnel   | `0.0.0.0:7002` / `0.0.0.0:7100`                    |
| `IDENTITY_URL`          | `http://127.0.0.1:7799` (loopback stub)            |
| `CHAN_DEVSERVER_LISTEN` | `1` (bind mgmt API; host reads the mounted prefix) |
| tenant                  | user `alice`, workspace `notes`                    |
| desktop entry origins   | `alice--<id-prefix>.devserver.localtest.me:7002`   |
