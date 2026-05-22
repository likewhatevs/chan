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

## 2026-05-22 — slice -b-30-a implementation note (ready for commit clearance)

Picked up `-b-30` immediately post-dispatch. Sliced into
two: **slice a** (this commit) lands the cargo feature
gating + per-OS native-mono default + user-config-dir
filesystem-fallback on `serve_font`. **Slice b**
(follow-up) adds the Settings dropdown + download-on-
enable flow. Slice a is the architectural foundation;
slice b is the user-facing toggle.

### Slicing rationale

The task body specs 3 pieces (cargo gate, Settings
dropdown + download, per-OS default). Slice a covers
pieces 1 + 3 + the `serve_font` filesystem fallback
(piece 2's runtime path; the SPA dropdown UI itself
defers to slice b). Picking up slice a alone delivers
the "lean default" win — default `cargo build` no
longer bundles the font; users get OS native — without
locking in the slice-b UI choices (slice b needs an
empirical pass on Settings layout + on the download
URL source).

### Changes

* **`crates/chan-server/Cargo.toml`** — new feature
  `embed-font = []`. Default off; mirrors
  `systacean-6`'s `embed-model` precedent. No deps
  added; the feature only gates the existing
  rust-embed.
* **`crates/chan-server/src/static_assets.rs`** —
  * `FontAssets` struct now `#[cfg(feature = "embed-font")]`.
    Default builds compile without the rust-embed.
  * `serve_font` handler rewritten: try
    `bundled_font_bytes` (feature-gated; returns
    `None` in default builds) → fall back to
    `user_config_font_bytes` reading from
    `<user-config>/chan/fonts/<name>` → 404.
  * `bundled_font_bytes` + `user_config_font_bytes` +
    `chan_fonts_user_dir` + `font_response` helpers.
  * Path-traversal defense added: reject names
    containing `/`, `\`, or starting with `.`. New
    `serve_font_rejects_path_traversal_attempts` test
    pins the contract.
  * Existing `font_bundle_includes_source_code_pro_and_ofl_notice`
    + `serve_font_returns_bundled_bytes_with_immutable_cache`
    tests gated on `#[cfg(feature = "embed-font")]`
    — they still run on `cargo test --features
    embed-font`.
* **`web/src/components/TerminalTab.svelte`** —
  xterm.js fontFamily reordered: `SF Mono` →
  `SFMono-Regular` → `Cascadia Code` →
  `DejaVu Sans Mono` → `ui-monospace` → `Menlo` →
  `Consolas` → `Liberation Mono` → `Source Code Pro`
  → `monospace`. Per-OS native faces lead; Source
  Code Pro stays in the chain but only kicks in when
  none of the OS-native faces resolve (rare on any
  modern OS) OR when slice b's Settings UI overrides
  the order.
* **`web/src/components/TerminalTab.font.test.ts`** —
  the `-b-12` "fontFamily lists Source Code Pro
  before fallbacks" pin inverted to "fontFamily
  leads with per-OS native mono and trails with
  Source Code Pro". The font + OFL assertions stay
  (still load-bearing for `--features embed-font`
  builds).

### Cross-lane note

`systacean-6` precedent: chan-server's `embed-model`
feature was a Systacean lane addition. The
`embed-font` feature is structurally identical (one
Cargo.toml line + one rust-embed `#[cfg]` gate); per
@@Alex's "take -b-30" routing I added it directly to
unblock the slice. @@Systacean can review at their
discretion — flagged in the architect-side architect
inbox poke.

### Pre-push gate (local, macOS aarch64; -b-30-a scope)

| Surface                                                          | State                              |
|------------------------------------------------------------------|------------------------------------|
| `cargo test -p chan-server` (default)                            | 223 passing.                       |
| `cargo test -p chan-server --features embed-font`                | 225 passing (+2 gated tests).      |
| `cargo clippy -p chan-server -p chan-desktop --all-targets -- -D warnings` | Clean.                  |
| `cargo clippy -p chan-server --features embed-font --all-targets -- -D warnings` | Clean.            |
| `cargo build -p chan-server --no-default-features`               | Clean.                             |
| `web/` `npx svelte-check`                                        | 4032 / 0 / 0.                      |
| `web/` `npx vitest run` (TerminalTab*.test.ts)                   | 4 files / 15 tests pass.           |
| `web/` `npm run build`                                           | Clean.                             |

### Files to stage

```
crates/chan-server/Cargo.toml
crates/chan-server/src/static_assets.rs
web/src/components/TerminalTab.svelte
web/src/components/TerminalTab.font.test.ts
docs/journals/phase-8/fullstack-b/fullstack-b-30.md
```

### Suggested commit subject

```
chan-server + TerminalTab: embed-font cargo feature + per-OS native-mono default + user-config-dir font fallback (fullstack-b-30 slice a)
```

### Slice b plan (follow-up)

1. SPA Settings dropdown: "OS default (mono)" vs
   "Source Code Pro". Persists in `Preferences.terminal.font`
   (or similar).
2. On select "Source Code Pro" + font not bundled:
   POST `/api/fonts/source-code-pro/download` —
   chan-server fetches the woff2 + OFL.txt from a
   stable URL (Adobe's GitHub release tag),
   writes to `<user-config>/chan/fonts/`. Idempotent;
   skips fetch if file present + matches expected
   size.
3. On select "Source Code Pro" with font available
   (bundled OR downloaded): xterm.js fontFamily
   reorders to put SCP first. Persists across
   reload.
4. Resolver helper `resolve_font(name)` mirrors
   `systacean-6`'s `resolve_model` shape: embed →
   user-config → not-present-error.

Slice b lands the user-facing piece. Slice a's
serve_font fallback is the foundation; slice b just
hooks the SPA up to it.

### Runtime walkthrough

Standing chan-desktop runtime perm available;
otherwise routing to @@WebtestB. Empirical smoke:
default build → terminal renders in OS native mono;
`--features embed-font` build → bundled font still
available (test pin verifies the embed); user-config
fallback → drop a woff2 in `<config>/chan/fonts/` +
verify it's served (manual integration test).
