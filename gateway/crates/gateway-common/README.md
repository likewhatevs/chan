# gateway-common

Shared internal library for the chan-gateway crates: typed HTTP
clients, the workspace-gate JWT, hostname derivation, validators,
and the token-bucket throttle primitive.

Not published; not user-facing. Has no binary.

## Role in the system

The gateway services need the same plumbing in several places —
calling profile-service and workspace-proxy admin, agreeing on
public hostnames, validating usernames, signing / verifying the
workspace-gate JWT, throttling token validates. This crate is the
single home for that shared code so per-crate copies cannot drift.

## Build

```bash
cargo build -p gateway-common
cargo test  -p gateway-common
```

## Modules

- `domain`: public hostnames (`id.<base>`, `workspace.<base>`,
  `.workspace.<base>`) derived from `CHAN_DOMAIN`; identity and
  workspace-proxy use it so the workspace-gate `aud` cannot drift.
- `profile_client`: typed reqwest wrapper over profile-service's
  HTTP API. Owns its own `ProfileError` enum so this crate has
  no axum / IntoResponse dependency.
- `shutdown`: graceful-shutdown future (SIGTERM or Ctrl-C) used by
  every service binary.
- `static_files`: generic SPA-fallback handler over
  `rust_embed::RustEmbed`. Each consumer keeps its own
  `#[derive(Embed)]` (rust_embed resolves the `#[folder]` path
  relative to the deriving crate) and calls
  `static_files::serve::<Assets>(uri, banner)`.
- `token_bucket`: per-fingerprint token bucket with a bounded map,
  plus the shared default limits for the two validate throttles.
- `validators`: username shape validation + the lifetime rename cap.
- `workspace_admin_client`: typed reqwest wrapper over
  workspace-proxy's `/admin/v1/*` tree.
- `workspace_gate`: the entry / session JWT envelope and HS256
  encode / decode helpers for the workspace-gate handoff.

## Design rationale

See [`design.md`](design.md).
