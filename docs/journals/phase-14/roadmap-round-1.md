# Phase 14 round 1

Phase 14 brings the chan.app gateway (the account, sign-in, and
reverse-proxy surface) into this monorepo. Round 1 is the migration
itself: port the existing `chan-writer/chan-gateway` workspace in,
adapt it to the monorepo's hardened crates and conventions, and get it
building, tested, packaged, and documented. Status: done.

Round 2 (deployment to the VPS via the separate `chan-prod-setup`
repo, and whatever else comes next) is tracked in `roadmap-round-2.md`.

## Scope

Migrate / port / rewrite the existing chan-gateway components
(`profile`, `identity`, `drive-proxy`, `admin`, `gateway-common`) so
they live in and build from this monorepo, against the in-repo tunnel
crates, with the `chan-drive` -> `chan-workspace` terminology applied
end to end. The gateway is server-side only (linux amd64/arm64).

## Done

### Shape: isolated nested workspace

- The gateway lives at `gateway/` as its own Cargo workspace (own
  `Cargo.lock`, own `[workspace.dependencies]`), parallel to
  `desktop/`. It is NOT a member of the root workspace, so the
  monorepo's single-binary, no-runtime-deps core build stays free of
  Postgres / sqlx / oauth2. `cargo metadata` at the root lists zero
  gateway crates.
- Versioned in lockstep with the monorepo root (`0.18.0`); ships on the
  `vX.Y.Z` release line.

### Re-home onto the in-repo tunnel crates

- `workspace-proxy` (was `drive-proxy`) depends on
  `crates/chan-tunnel-{proto,client,server}` by path, retiring the old
  cross-repo `chan-core` checkout and version pin. The only code change
  needed to compile against the 0.18 tunnel crates was the
  `drive` -> `workspace` rename at the tunnel boundary, which also
  keeps the proxy wire-compatible with the monorepo `chan` client.

### drive -> workspace rename (suite-wide)

- Crate/binaries: `drive-proxy` -> `workspace-proxy`; the cookie is
  `workspace_gate`; the public host is `workspace.chan.app`
  (`*.workspace.chan.app` wildcard); routes are `/api/workspaces/*`;
  env vars are `WORKSPACE_*`; DB tables are `workspaces` /
  `workspace_grants`. Migrations were edited in place (pre-release, no
  data to preserve).

### Single-source domain config

- `gateway_common::domain::Domains` derives `id.<base>`,
  `workspace.<base>`, and `.workspace.<base>` from one `CHAN_DOMAIN`
  plus a shared `PUBLIC_SCHEME`. identity and workspace-proxy read the
  same two vars, so the hosts cannot drift (the workspace-gate JWT
  `aud` is the inbound host). The fine-grained vars remain as
  overrides; defaults are dev-shaped (`localtest.me` / `http`).

### CI + release

- New `gateway-ci.yml` runs the gateway gate (fmt + clippy + tests
  against a `postgres:16` service, plus the SPA check), scoped to
  `gateway/**`. `ci.yml` is path-guarded so the core gate and the
  gateway gate do not both run on a given change.
- `release.yml` builds the four gateway `.deb`s (amd64/arm64) on native
  runners and publishes them on the `v*` flow, with the tag asserted
  against the gateway version.
- sdme is local-dev only; CI uses the native GitHub Actions setup, the
  same split as the rest of chan.

### Packaging + docs reconcile

- The committed `*.env` templates, `configure.sh`, and the crate
  READMEs were brought back in line with the current env contract
  (required `IDENTITY_INTERNAL_TOKEN` / `WORKSPACE_GATE_SECRET`, a
  shared admin token, stale vars dropped). Fixed a pre-existing
  `configure.sh` bug (`install /dev/stdin` failing over an existing
  file).
- Added `docs/contributing/linux-and-macos.md` documenting the uniform
  sdme-based local Linux build/test flow for both core and gateway.

### Local validation

- Full gateway suite passes against a Postgres sdme container
  (`gateway/scripts/dev/sdme/chan-psql.sdme`): profile + identity +
  workspace-proxy integration tests plus unit tests.
- The prod packaging path was validated end to end in a systemd sdme
  container: build the four debs, `dpkg -i` (postinst creates the
  `chan-gateway` user + units), `configure.sh`, `systemctl enable
  --now`; all three services reach `active` and `/healthz` returns 200,
  with the shared `domain.env` loaded and the dashboard host derived.

## Follow-ups (round 2)

- Deploy to the VPS: adjust the separate `chan-prod-setup` repo (deb
  source -> monorepo release, drive -> workspace rename, adopt
  `CHAN_DOMAIN`/`PUBLIC_SCHEME`, wildcard TLS SAN, DB recreate). A gap
  analysis was done; it does not belong in this repo.
- Optional later: full `drive` -> `workspace` rename of the
  `chan-prod-setup` edge, and the gateway monorepo release that
  produces the `chan-gateway-*` debs.
