# Chan Gateway

The gateway is the self-hostable server side of chan's tunnel. It lets a
fleet of `chan serve` instances dial out and be reached from anywhere in the
browser, behind sign-in, without opening inbound ports on each machine. It
lives in this repo under `gateway/` and is yours to run; the maintainer's own
deployment at `id.chan.app` and `workspace.chan.app` is experimental, ships
with sign-in off by default, and is not a hosted product.

## How chan uses it

Instead of binding a local port, `chan serve` can publish a workspace over an
outbound tunnel to a gateway, which reverse-proxies it back to you:

```sh
export CHAN_TUNNEL_TOKEN=chan_pat_...     # a token your gateway issued
chan serve ~/notes --tunnel-url https://workspace.example.com/v1/tunnel
```

`chan` dials the gateway, registers the workspace over an authenticated
tunnel, and serves every request through the same router the local listener
uses. The workspace is then reachable at
`{user}.workspace.example.com/{workspace}/*`, gated by the gateway.

## Services

The gateway is a separate Cargo workspace under `gateway/`. Its public and
internal services are:

```
service          surface                role
---------------  ---------------------  -------------------------------------
identity         id.chan.app            OAuth sign-in (GitHub, Google,
                                        GitLab), sessions, personal access
                                        tokens, the account SPA
workspace-proxy  workspace.chan.app     tunnel registration (POST /v1/tunnel)
                 + *.workspace.chan.app  + reverse proxy into each chan serve
profile          internal               users, identities, tokens over
                                        Postgres; called by the others
admin            CLI                     operator tooling against profile +
                                        workspace-proxy admin routes
```

Personal access tokens (`chan_pat_...`) issued by identity are what
`CHAN_TUNNEL_TOKEN` carries. Access to a published workspace is gated by a
short-lived entry token minted by identity and a path-scoped session cookie
minted by the workspace proxy, so one workspace cannot read another's
session.

## Self-deploy

The gateway is Postgres-backed and targets Linux (amd64 / arm64). Standing up
your own deployment means running the three services, a Postgres database, and
configuring at least one OAuth provider; macOS contributors can run Postgres
inside a Lima VM via `sdme`. Adding an OAuth provider is one new file plus
config wiring.

The full route tables, environment variables, trust boundaries, and dev setup
are documented in the gateway README:
[github.com/fiorix/chan/tree/main/gateway](https://github.com/fiorix/chan/tree/main/gateway).

## Verification status

The `chan serve --tunnel-url` command shape is verified against the documented
flags. A full end-to-end run against a live gateway (tunnel registration ->
browser access behind OAuth) requires a deployed gateway and is not exercised
by the local audit; it is left for a deployment-time check.
