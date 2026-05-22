# fullstack-b-30 — Font shipping spec: Source Code Pro behind cargo feature + Settings dropdown + per-OS native-mono default (broader -b-29 follow-up)

Owner: @@FullStackB (primary; cross-lane to @@Systacean for cargo feature)
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Complete the broader font-shipping spec
@@WebtestA flagged as still-pending post-`-b-29`.
`-b-29`'s WebGL renderer addon solved the
TUI-alignment user pain; this task lands the
opt-in font architecture.

## Reference

[`../phase-8-bugs.md`](../phase-8-bugs.md) §"Source
Code Pro font architecture" (line ~245):

> Default build ships NO font; per-OS native mono
> is the default. Source Code Pro is opt-in via
> Settings; downloaded on demand to user-config
> dir; cargo feature flag keeps the embedded-
> shipping path for power users / offline installs.

## Scope (3 pieces)

### 1. cargo feature flag

* `crates/chan-server/Cargo.toml`: new feature
  `embed-font` (default off).
* `rust-embed` font bundle gated behind the
  feature. Mirrors `embed-model` from
  `systacean-6`.
* Default `cargo build` no longer ships the
  woff2.

### 2. Settings dropdown + download-on-enable

* SPA Settings: dropdown for terminal font —
  options: "OS default (mono)" + "Source Code Pro".
* On select "Source Code Pro": if feature flag
  off → download font to user-config dir +
  activate. If feature flag on (embedded build)
  → just activate.

### 3. Per-OS native-mono default fontFamily

* macOS: SF Mono / Menlo.
* Windows: Cascadia / Consolas.
* Linux: DejaVu Sans Mono / monospace.

xterm.js `fontFamily` config + the CSS rule for
chan's terminal class both honor the default.

## Cross-lane scope

* @@FullStackB primary (chan-desktop bundle +
  Settings UI).
* @@Systacean cargo feature flag (mirrors
  `systacean-6` embed-model precedent).
* Optional cross-lane to @@FullStackA if Settings
  surface lives in shared SPA territory.

## Acceptance

1. Default `cargo build` produces a binary that
   does NOT embed Source Code Pro.
2. `cargo build --features embed-font` keeps the
   embed path.
3. Settings dropdown surfaces "OS default" +
   "Source Code Pro" options.
4. Selecting Source Code Pro on a non-embedded
   build downloads the font.
5. Per-OS native mono is the unselected default.

### Tests

* Rust-side: cargo feature toggle test (compile
  shape).
* SPA-side: Settings dropdown + download-on-enable
  flow.

### Gate

`cargo / npm` gates green; `--features embed-font`
build path verified.

## Authorization

Yes for `crates/chan-server/Cargo.toml` +
`crates/chan-server/resources/fonts/` (gating) +
SPA Settings + chan-desktop bundle config + tests
+ task tail + outbound.

## Numbering

This is `-b-30`.
