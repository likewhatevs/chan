# devserver-control

Singleton, database-free control plane for the devserver-proxy fleet. Owns the dynamic proxy directory, the aggregate tunnel view, fleet admission, and kill-command routing. Every devserver-proxy node holds one authenticated h2c control session to it on a dedicated listener; identity, profile, and the admin CLI read one coherent fleet view from its `/admin/v1/*` tree. No SPA, no database, no tenant traffic: the controller carries metadata and commands only.

## Role in the system

devserver-proxy keeps its registrations in a process-local registry, so with more than one proxy node nothing in the fleet can answer "who holds this tunnel" or "how many devservers does this user run." devserver-control is that answer. Each proxy publishes a full registry snapshot plus deltas over its control session, asks the controller for an admission decision before acknowledging any tunnel registration, and executes the controller's kill commands. Consumers point `DEVSERVER_ADMIN_URL` at this service and always see either one coherent fleet view or an explicit upstream failure, never a partial process snapshot. The split and its failure semantics are decided in [ADR-0002](../../docs/adr/0002-control-plane-owns-proxy-fleet-state.md).

## Build

```bash
cargo build -p devserver-control
```

The binary is `devserver-control-service`. No frontend: devserver-control ships no SPA.

## Dev run

```bash
export BIND_ADDR=127.0.0.1:7003
export PROXY_BIND_ADDR=127.0.0.1:7101
export DEVSERVER_ADMIN_TOKEN=dev-admin-token
export DEVSERVER_PROXY_TOKEN=dev-proxy-token
export DEVSERVER_PROXY_BASE_URL_TEMPLATE='http://{proxy_id}.devserver.localtest.me:7002'
cargo run -p devserver-control
```

For the full local stack (with identity + profile + Postgres), prefer `packaging/gateway/scripts/dev/setup.sh` + `packaging/gateway/scripts/dev/run.sh`. Two listeners come up:

- `BIND_ADDR` (7003): admin HTTP. `/healthz` and `/readyz` are unauthenticated; `/admin/v1/*` is Bearer-gated by `DEVSERVER_ADMIN_TOKEN`. Loopback in dev; reached only by peer services and the operator CLI.
- `PROXY_BIND_ADDR` (7101): h2c. Each devserver-proxy node dials `POST /v1/proxies/connect` here with `DEVSERVER_PROXY_TOKEN` and holds the stream for the life of its control session.

The template must expand each proxy's `DEVSERVER_PROXY_ID` to exactly that node's `DEVSERVER_PROXY_BASE_URL`; a mismatch closes the control session at the handshake. With the dev values above, a proxy runs with `DEVSERVER_PROXY_ID=p1` and `DEVSERVER_PROXY_BASE_URL=http://p1.devserver.localtest.me:7002`.

## Env vars

Required:

| Name                                | Notes                             |
|-------------------------------------|-----------------------------------|
| `DEVSERVER_ADMIN_TOKEN`             | bearer for `/admin/v1/*`          |
| `DEVSERVER_PROXY_TOKEN`             | bearer proxies present on connect |
| `DEVSERVER_PROXY_BASE_URL_TEMPLATE` | one `{proxy_id}` origin template  |

Optional:

| Name                      | Default          | Purpose                      |
|---------------------------|------------------|------------------------------|
| `BIND_ADDR`               | `127.0.0.1:7003` | admin listener               |
| `PROXY_BIND_ADDR`         | `127.0.0.1:7101` | proxy control listener (h2c) |
| `MAX_DEVSERVERS_PER_USER` | `100` (`0` off)  | fleet-wide per-user cap      |
| `RUST_LOG`                | `info`           | tracing filter               |

The two tokens are independent secrets and must stay distinct: one gates operator reads and kills, the other gates fleet membership.

## Routes

Admin listener (`BIND_ADDR`); all `/admin/v1/*` routes Bearer-gated by `DEVSERVER_ADMIN_TOKEN`:

| Method | Path                                         | Purpose             |
|--------|----------------------------------------------|---------------------|
| GET    | `/healthz`                                   | liveness; no auth   |
| GET    | `/readyz`                                    | 503 while warming   |
| GET    | `/admin/v1/tunnels`                          | aggregate tunnels   |
| GET    | `/admin/v1/tunnels/watch`                    | SSE snapshot stream |
| POST   | `/admin/v1/tunnels/{user}/{devserver_id}/kill` | exact kill; 204   |
| GET    | `/admin/v1/users/{user}/tunnels`             | one user's rows     |
| POST   | `/admin/v1/users/{user}/tunnels/kill`        | user-wide kill      |
| GET    | `/admin/v1/proxies`                          | proxy directory     |
| GET    | `/admin/v1/proxies/watch`                    | SSE proxy stream    |

Proxy listener (`PROXY_BIND_ADDR`):

| Method | Path                  | Purpose                            |
|--------|-----------------------|------------------------------------|
| POST   | `/v1/proxies/connect` | raw h2c control stream per proxy   |

Aggregate reads return 503 until the controller is ready (at least one complete proxy snapshot plus reconciliation; see [`design.md`](design.md)). Kills route by registration UUID to the owning proxy only; a user-wide kill that only partly confirms answers 502 with the confirmed count and is safe to retry. See [`design.md`](design.md) for the session lifecycle, admission rules, reconciliation, and kill semantics.

## Design rationale

See [`design.md`](design.md).
