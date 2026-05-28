# gateway-common

Shared internal library for the chan-gateway crates. Houses the
typed profile-service HTTP client and the SPA-fallback static
asset handler used by both `identity-service` and `drive-proxy`.

Not published; not user-facing. Has no binary.

## Role in the system

Two consumers (identity-service and drive-proxy) need the same
plumbing for talking to profile-service and serving an embedded
Svelte SPA. This crate is the home for that shared code so the
duplicate copies in each consumer can be reduced to a few-line
re-export.

## Build

```bash
cargo build -p gateway-common
cargo test  -p gateway-common
```

## Modules

- `profile_client`: typed reqwest wrapper over profile-service
  HTTP API. Owns its own `ProfileError` enum so this crate has
  no axum / IntoResponse dependency.
- `static_files`: generic SPA-fallback handler over
  `rust_embed::RustEmbed`. Each consumer keeps its own
  `#[derive(Embed)]` (rust_embed resolves the `#[folder]` path
  relative to the deriving crate) and calls
  `static_files::serve::<Assets>(uri, banner)`.

## Design rationale

See [`design.md`](design.md).
