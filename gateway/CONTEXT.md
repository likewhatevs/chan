# Gateway

The account, sign-in, and reverse-proxy surface for chan.app, a separate nested
Cargo workspace. This glossary fixes the domain language; decisions and their
rationale live in `docs/adr/`.

## The devserver model

**devserver**:
The single, gateway-exposed `chan devserver` process a user runs; it hosts a
library and holds one tunnel registration. One per user is reachable through the
gateway at a time.
_Avoid_: remote, instance, node

**library**:
The set of workspaces a devserver hosts (the `~/.chan` workspace registry on that
machine). The devserver is the process; the library is its contents.
_Avoid_: collection, registry

**workspace**:
A single project directory; the tenant unit inside a library. It is not a
permission or sharing unit.
_Avoid_: project, folder, drive

**tenant**:
A workspace as routed and served inside the devserver, mounted at a route slug.
_Avoid_: site, app

## Gate and identity

**devserver-proxy**:
The gateway reverse-proxy service at `devserver.chan.app` (apex) and
`*.devserver.chan.app` (wildcard). Renamed from workspace-proxy.
_Avoid_: workspace-proxy, tenant-proxy

**devserver token**:
The owner PAT (`chan_pat_*`) that authorizes a devserver to register over the
tunnel. One token identifies one devserver; the backend resolves the devserver
from the token's hash. The PAT stays opaque.
_Avoid_: tunnel-name, api-key

**devserver_gate**:
The host-only JWT-cookie access gate on the wildcard host. Gates per devserver:
the `drv` claim is the devserver, the cookie is scoped `Path=/`, and there is one
access check. Renamed from workspace_gate.
_Avoid_: workspace_gate, auth-cookie

**devserver grant**:
A profile record that a caller may access an owner's devserver, meaning its whole
library. The sharing unit. Replaces the per-workspace grant.
_Avoid_: workspace-grant, share, ACL

**entry token**:
The short-lived `?t=` JWT that identity mints after a devserver access check, which
devserver-proxy exchanges for the `devserver_gate` session cookie.
_Avoid_: handoff-token
