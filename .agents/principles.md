# Project Principles

## Workspace is the boundary

All filesystem operations route through `chan_workspace::Workspace`, which sandboxes paths under the registered workspace root, refuses non-regular files (symlinks, FIFOs, sockets, devices), and performs atomic writes. Nothing in this repo should ever call `std::fs::*` on user content directly.

## Single binary, no runtime deps

No Node.js, no Python, no native daemons at runtime. The frontend embeds at build time. New dependencies must hold this line.

## Local-first by default, opt-in tunnel

The HTTP server binds `127.0.0.1` by default. Auth is a per-launch bearer token printed once on stderr and appended to the launch URL. No TLS at the local hop.

Tunnel mode (`chan serve --tunnel-token ...`, or `CHAN_TUNNEL_TOKEN` env var) replaces the local listener with a `chan-tunnel-client` dial to `workspace.chan.app/v1/tunnel`. The workspace is then published at `{user}.workspace.chan.app/{workspace}/*` over yamux substreams. The single-user, single-machine assumption still holds: one chan serve process owns the workspace's writes; the tunnel just relocates the inbound transport. The bearer-token gate is auto-disabled in tunnel mode (the gateway in front of workspace.chan.app is the trust boundary; default behavior 404s anonymous visitors, opt out with `--tunnel-public`). Wire protocol lives in `crates/chan-tunnel-proto`.

## App-level vs core

What lives in chan-workspace (filesystem, search, graph, watch, report) vs what lives in chan-server (HTTP, editor preferences, sessions, attachments, terminal, MCP bridge) is a hard line. Don't push library concerns into chan-workspace, and don't reimplement library primitives in chan-server. When in doubt, read `crates/chan-workspace/design.md`.

## MCP server only, no in-app agent

There is no in-app Agent overlay and no chan-server `/api/llm/*` / `/api/assistant/*` HTTP surface. External agents (claude, codex, gemini) connect through the in-process MCP server exposed over a Unix-domain socket by `mcp_bridge.rs`; the embedded terminal exports `CHAN_MCP_SERVER_JSON` and companion `CHAN_MCP_*` discovery variables. Chan does not write CLI-owned env namespaces; tools can translate the `CHAN_` descriptor into their own MCP config shape. Do not reintroduce in-app agent UI or chan-server-side chat APIs without an explicit decision from the maintainer.
