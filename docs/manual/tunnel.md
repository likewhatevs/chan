# Tunnel Basics

Local use is the default. Tunnel mode is opt-in.

TODO: include mermaid block diagram for the components, then sequence diagram for the connection workflow

## Local server

Without tunnel flags, `chan open` binds to `127.0.0.1` and gates requests with a per-launch bearer token.

## Tunnel mode

Publishing over the tunnel is a property of `chan devserver`, the headless multi-workspace host, not of a single `chan open`. With a tunnel token, the devserver dials the Chan tunnel service and publishes its whole library under `{user}.devserver.chan.app`, with each mounted workspace reachable at `{user}.devserver.chan.app/{workspace}/`. The local bearer-token gate gives way to the gateway in front of the public route, which becomes the trust boundary.

```sh
export CHAN_TUNNEL_TOKEN=chan_pat_...     # a token your gateway issued
chan devserver --tunnel-url https://devserver.example.com/v1/tunnel
```

`chan devserver` dials the gateway's `/v1/tunnel`, runs a handshake, and serves every inbound request through the same router the local listener uses. The registration is keyed on the devserver identity the gateway resolves from the token, so there is no workspace name to pass; one registration carries every workspace mounted on the devserver. The flag form `--tunnel-token <TOKEN>` works too but exposes the token in `ps`; prefer the env var. The default `--tunnel-url` is `https://devserver.chan.app/v1/tunnel`.

Tunnel mode is foreground-only: combined with `--systemd` / `--launchd` it is refused, since a supervised backend would have to persist the token. The management API (`/api/devserver/*`) stays local-only — the gateway does not expose it on the public route — so manage the devserver over its direct (`host:port` / `ssh -L`) connection.

## Authentication

There is no anonymous-readable path. The gateway gates every request on the workspace owner, or a user the owner granted access; an ungranted caller gets a 404. A grant covers the whole devserver, so the gateway issues a host-scoped session cookie for `{user}.devserver.chan.app`; the `{workspace}` path segment is tenant routing only and never gates.

## Self-hosting

The gateway is part of chan and ships in this repo under `gateway/`. See [Chan Gateway](gateway.md) and `gateway/README.md` to stand up your own; `--tunnel-url` selects which gateway your devserver publishes through.
