# devserver-proxy

Public-facing service at devserver.chan.app (apex) and `*.devserver.chan.app` (wildcard). Reverse-proxies HTTP / WebSocket traffic into a user's running `chan devserver` instances. Embeds `chan-tunnel-server` to terminate registrations from those instances on a separate h2c listener; every registration is admitted by devserver-control before the client sees `HelloAck::Ok`. No SPA, no database; it is a stateless proxy.

## Role in the system

devserver-proxy is the surface where a devserver is served in the browser. It does NOT read identity's `id_session` cookie. Entry is gated by the devserver-gate handoff: identity mints a short-lived entry JWT and 303s the browser to `{user}.devserver.<domain>/{workspace}/?t=<jwt>` (or the devserver root for owner launcher opens); devserver-proxy verifies it (signature + `aud` = inbound host + `drv`), mints its own host-only `Path=/` `devserver_gate` cookie, and forwards authenticated traffic to the right `chan devserver` peer through a yamux substream owned by an active tunnel. The `aud`-equals-inbound-host check is what enforces user-to-user isolation.

## Build

```bash
cargo build -p devserver-proxy
```

No frontend: devserver-proxy ships no SPA.

## Dev run

```bash
export BIND_ADDR=127.0.0.1:7002
export TUNNEL_BIND_ADDR=127.0.0.1:7100
export IDENTITY_URL=http://127.0.0.1:7000
export IDENTITY_INTERNAL_TOKEN=dev-internal-token
export DEVSERVER_GATE_SECRET=dev-devserver-gate-secret
export DASHBOARD_URL=http://127.0.0.1:7000/workspaces
export DEVSERVER_CONTROL_URL=http://127.0.0.1:7200
export DEVSERVER_PROXY_TOKEN=dev-proxy-token
export DEVSERVER_PROXY_ID=p1
export DEVSERVER_PROXY_BASE_URL=http://devserver.localtest.me:7002
export DEVSERVER_TUNNEL_ORIGIN=http://devserver.localtest.me:7100
cargo run -p devserver-proxy
```

For the full local stack (with identity + profile + Postgres), prefer `packaging/gateway/scripts/dev/setup.sh` + `packaging/gateway/scripts/dev/run.sh`. Two listeners come up:

- `BIND_ADDR` (7002): public HTTP. devserver.chan.app sits behind nginx + TLS in production; loopback in dev.
- `TUNNEL_BIND_ADDR` (7100): h2c. nginx `grpc_pass`es `/v1/tunnel` on the apex here; `chan devserver` instances dial it for the handshake.

## Env vars

Required:

| Name                       | Notes                                              |
|----------------------------|----------------------------------------------------|
| `IDENTITY_INTERNAL_TOKEN`  | bearer presented on identity's validate            |
| `DEVSERVER_GATE_SECRET`    | HS256 secret; equals identity's                    |
| `DASHBOARD_URL`            | sign-in redirect target for the bare wildcard root |
| `DEVSERVER_CONTROL_URL`    | h2c origin of the devserver-control proxy listener |
| `DEVSERVER_PROXY_TOKEN`    | bearer presented only to the controller            |
| `DEVSERVER_PROXY_ID`       | provisioned node id; one lowercase DNS label       |
| `DEVSERVER_PROXY_BASE_URL` | exact public origin of this node's wildcard host   |
| `DEVSERVER_TUNNEL_ORIGIN`  | public origin of the tunnel listener; gives the apex host |

Optional:

| Name                       | Default                  | Purpose              |
|----------------------------|--------------------------|----------------------|
| `BIND_ADDR`                | `127.0.0.1:7002`         | public listener      |
| `TUNNEL_BIND_ADDR`         | `127.0.0.1:7100`         | tunnel listener (h2c)|
| `IDENTITY_URL`             | `http://127.0.0.1:7000`  | base for validate    |
| `MAX_RESPONSE_BYTES`       | `100 MiB` (`0` disables) | response body cap    |
| `MAX_REQUEST_BYTES`        | `100 MiB` (`0` disables) | request body cap     |
| `REQUEST_TIMEOUT_SECS`     | `60` (`0` disables)      | end-to-end timeout   |
| `FORWARDED_PROTO`          | `https`                  | `X-Forwarded-Proto`  |

## Routes

- Apex (`devserver.<domain>`): `POST /v1/tunnel` (raw h2c, on the tunnel listener), `/healthz`, and `/readyz`. `/readyz` is 200 only once the controller session reaches `FleetReady`; until then new tunnel admissions are refused with the `control_unavailable` code. Per-user devserver capacity is a fleet-wide decision made by the controller at admission and surfaces as `too_many_workspaces`. The aggregate `/admin/v1/*` tree lives on devserver-control, not on the proxy.
- Wildcard (`{user}--{disc}.devserver.<domain>` addressing one devserver by the first 12 hex chars of its id, or bare `{user}.devserver.<domain>` resolved via the gate credential): the per-devserver reverse proxy. The gate runs on the resolved devserver (`?t=` entry token / `devserver_gate` cookie); on pass the full `/{workspace}/...` path is forwarded into the tunnel (segment-preserving) and the devserver routes the tenant. `/api/devserver/*` (the local-only management API) is 404'd here.

See [`design.md`](design.md) for the authoritative route list, the auth-gate order, and the reverse-proxy hygiene rules.

## Design rationale

See [`design.md`](design.md).
