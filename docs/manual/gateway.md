# Chan Gateway

The gateway is the self-hostable server side of chan's tunnel. It lets a fleet of `chan devserver` instances dial out and be reached from anywhere in the browser, behind sign-in, without opening inbound ports on each machine. It lives in this repo under `gateway/` and is yours to run; the maintainer's own deployment at `id.chan.app` and `devserver.chan.app` is experimental, ships with sign-in off by default, and is not a hosted product.

![Where Chan Gateway sits: each chan devserver dials out over an authenticated tunnel to a self-hosted gateway — identity, devserver-proxy, profile on Postgres, and an admin CLI — while a browser signs in at identity and reaches each workspace through the devserver-proxy's reverse proxy.](images/gateway-architecture.svg#w=736)

## How chan uses it

Instead of binding a local port, `chan devserver` can publish your whole library over a single outbound tunnel to a gateway, which reverse-proxies it back to you:

```sh
export CHAN_TUNNEL_TOKEN=chan_pat_...     # a token your gateway issued
chan devserver --tunnel-url https://devserver.example.com/v1/tunnel
```

`chan devserver` dials the gateway, registers over an authenticated tunnel, and serves every request through the same router the local listener uses. Each mounted workspace is then reachable at `{user}.devserver.example.com/{workspace}/*`, gated by the gateway.

## Services

The gateway is a separate Cargo workspace under `gateway/`. Its public and internal services are:

```
service          surface                role
---------------  ---------------------  -------------------------------------
identity         id.chan.app            OAuth sign-in (GitHub, Google,
                                        GitLab), sessions, personal access
                                        tokens, the account SPA
devserver-proxy  devserver.chan.app     tunnel registration (POST /v1/tunnel)
                 + *.devserver.chan.app  + reverse proxy into each chan devserver
profile          internal               users, identities, workspaces, flags
                                        over Postgres; called by identity
                                        and the admin CLI
admin            CLI                     operator tooling against profile +
                                        devserver-proxy admin routes
```

Personal access tokens (`chan_pat_...`) issued by identity are what `CHAN_TUNNEL_TOKEN` carries. Access to a published workspace is gated by a short-lived entry token minted by identity and a path-scoped session cookie minted by the workspace proxy, so one workspace cannot read another's session.

## Self-deploy

The gateway is Postgres-backed and targets Linux (amd64 / arm64). Standing up your own deployment means running the three services, a Postgres database, and configuring at least one OAuth provider; macOS contributors can run Postgres inside a Lima VM via `sdme`. Adding an OAuth provider is one new file plus config wiring.

The full route tables, environment variables, trust boundaries, and dev setup are documented in the gateway README: [github.com/fiorix/chan/tree/main/gateway](https://github.com/fiorix/chan/tree/main/gateway).
