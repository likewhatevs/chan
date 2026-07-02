# Gateway

The account, sign-in, and reverse-proxy surface for chan.app, a separate nested
Cargo workspace. This glossary fixes the domain language; decisions and their
rationale live in `docs/adr/`.

## Topology

```mermaid
flowchart TB
    subgraph browser["Browser"]
        IDSPA["identity SPA — id.chan.app"]
        LAUNCH["web-launcher SPA<br/>(served through the proxy at the devserver root)"]
    end

    subgraph gw["chan gateway (nested Cargo workspace)"]
        ID["identity-service · id.chan.app<br/>OAuth · sessions · PATs · /s/{owner} open · token validate"]
        PROXY["devserver-proxy<br/>devserver.chan.app apex: admin + tunnel + healthz<br/>*.devserver.chan.app wildcard: launcher root + tenants + devserver_gate"]
        PROFILE["profile-service<br/>internal HTTP over Postgres · users · identities · devserver grants"]
        ADMIN["admin-service"]
        COMMON["gateway-common<br/>domain · devserver_gate · profile_client"]
        PG[("Postgres")]
    end

    subgraph box["User's machine"]
        DS["chan devserver · library = ~/.chan workspaces<br/>serves the launcher at / · tenants under /{workspace}/ · /api/library/*"]
    end

    IDSPA -->|OAuth · manage devservers · Open| ID
    ID -->|mint entry token (drv, aud)| IDSPA
    ID <-->|users · grants · access| PROFILE
    PROFILE --- PG
    PROXY -->|validate PAT · /internal/v1/tokens/validate| ID
    DS ==>|tunnel register with PAT · devserver.chan.app/v1/tunnel| PROXY
    PROXY ==>|gated tenant + root traffic over the tunnel| DS
    LAUNCH -->|/api/library/* via the proxy| PROXY
    ID --> COMMON
    PROXY --> COMMON
    PROFILE --> COMMON
```

`admin-service` is the operator console; `gateway-common` holds the shared
domain config, the `devserver_gate` JWT type, and the profile/workspace-admin
clients. devserver-proxy renders no UI of its own — it forwards the launcher
that the devserver serves at its root.

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
