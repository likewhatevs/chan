# Tunnel Basics

Local use is the default. Tunnel mode is opt-in.

## Local server

Without tunnel flags, `chan serve` binds to `127.0.0.1` and gates requests with a per-launch bearer token.

## Tunnel mode

With a tunnel token, the server dials the Chan tunnel service and publishes the workspace under `workspace.chan.app`. The local bearer-token gate is disabled in tunnel mode because the gateway in front of the public route becomes the trust boundary.

```sh
export CHAN_TUNNEL_TOKEN=chan_pat_...     # a token your gateway issued
chan serve ~/notes --tunnel-url https://workspace.example.com/v1/tunnel
```

`chan` dials the gateway's `/v1/tunnel`, runs a handshake that names the workspace, and serves every inbound request through the same router the local listener uses. The flag form `--tunnel-token <TOKEN>` works too but exposes the token in `ps`; prefer the env var. `--tunnel-workspace-name <name>` publishes under a different name (lowercase `[a-z0-9-]`, 1-32 chars).

## Public tunnel

Anonymous public access is not the default: the gateway returns a 404 to anyone but the workspace owner (or a user the owner granted access). `--tunnel-public` opts out of that gate so anyone with the URL can reach the workspace.

## Self-hosting

The gateway is part of chan and ships in this repo under `gateway/`. See [Chan Gateway](gateway.md) and `gateway/README.md` to stand up your own; `--tunnel-url` selects which gateway your server publishes through.
