# fullstack-b-20: v0.11.2 hotfix — externalBin per-target + unused `app` rename

Owner: @@FullStackB
Date: 2026-05-21

## Goal

Two trivial chan-desktop fixes surfaced by `ci-8` dry-run #2
(`chan-v0.11.99-dryrun.2` run `26207525095`). Both block
the v0.11.2 signed tag-cut. Single commit covering both.

## Background

* **`ci-8` dry-run #2 result** (per @@CI's 2026-05-21 poke):
  workflow executed for real (16m22s; ci-4 `^2` fix worked,
  ci-7 + ci-9 verify steps all green) but both jobs failed
  on real build-side regressions. Detail at
  [`../alex/event-ci-architect.md`](../alex/event-ci-architect.md)
  2026-05-21 "ci-8 dry-run #2" section.

### Bug #1 — macOS Tauri externalBin per-target mismatch

`desktop/src-tauri/tauri.conf.json`'s
`bundle.externalBin = ["binaries/chan"]` expands per-target
to `binaries/chan-<target-triple>`. Tauri-bundler on
`macos-latest` (aarch64) tries BOTH `chan-aarch64-apple-darwin`
AND `chan-x86_64-apple-darwin` — universal2 expectation.
`desktop/Makefile`'s `chan-bin` recipe stages ONLY the host
triple, so the x86_64 file doesn't exist + bundle step
errors:

```
Error failed to bundle project Failed to copy external binaries:
  resource path `binaries/chan-x86_64-apple-darwin` doesn't exist
make: *** [app-notarized] Error 1
```

### Bug #2 — Ubuntu Rust unused-variable in main.rs

Ubuntu job hit `-D warnings` against unused inner `app` in
`desktop/src-tauri/src/main.rs:910`:

```rust
app.run(move |app, event| {  // <-- outer app shadowed; inner unused
    ...
});
```

Likely a regression from one of `-b-17` (`9f68b11` Tab
Reload + Inspector) or `-b-19` (`59f5688` zoom chords)
adding to the `app.run` closure body without using the
`app` parameter inside. Trivial fix.

## Authorization

**Authorization: yes**, covers:

* `desktop/src-tauri/tauri.conf.json` (bundle.externalBin
  shape change).
* `desktop/src-tauri/src/main.rs:910` (1-char `app` →
  `_app` rename).

@@FullStackB may proceed without further @@Alex confirmation.

## The fixes

### Bug #1 — Option (a): scope to host triple only

Drop universal2 expectation for v0.11.2. Aarch64-only DMG
ships. Full universal2 work (Makefile + lipo + CI matrix
x86_64 entry) deferred to a post-v0.11.2 ci-N follow-up.

Option for the JSON change (implementer picks the cleanest
shape Tauri supports):

* **(i)** Drop `bundle.externalBin` entirely; rely on the
  Makefile staging the binary into `Contents/MacOS/` post-
  bundle. Cleanest if Tauri's per-target expansion can't be
  selectively disabled.
* **(ii)** Move `bundle.externalBin` to a per-target
  override (Tauri 2 supports per-target overrides via
  `bundle.macOS.externalBin` or similar). Set only the
  host triple value.
* **(iii)** Set `bundle.externalBin = ["binaries/chan-<host-triple>"]`
  with explicit triple to bypass the auto-expansion.

Recommend (ii) if Tauri 2's per-target override system
supports `externalBin`; otherwise (i) or (iii). Tauri docs
authoritative.

Document the temporary nature in `desktop/CLAUDE.md`'s
"Bundled chan sidecar" section: "aarch64-only DMG for
v0.11.2; universal2 (lipo) work cuts as a post-v0.11.2
ci-N task once the Makefile + CI matrix support both
arches".

### Bug #2 — `app` → `_app` at line 910

```diff
-    app.run(move |app, event| {
+    app.run(move |_app, event| {
```

1-char change. Verify locally with
`cargo build -p chan-desktop --release` + (importantly)
`RUSTFLAGS=-D warnings cargo build -p chan-desktop --release`
to match the Ubuntu job's strict mode.

## Acceptance criteria

* `cargo tauri build` on macos-latest (aarch64) succeeds
  without the missing-x86_64-binary error.
* `RUSTFLAGS=-D warnings cargo build -p chan-desktop` on
  Ubuntu succeeds (no unused-variable warning).
* `desktop/CLAUDE.md` updated to note the aarch64-only
  shape for v0.11.2 + the deferred universal2 follow-up.
* Pre-push gate: clean (chan-desktop cargo build + fmt +
  clippy `-D warnings` + the no-default-features build).

## How to start

1. Inspect Tauri 2's per-target override docs for
   `bundle.externalBin` (the shape question). Pick (i) /
   (ii) / (iii) based on what Tauri supports.
2. Apply both fixes (bug #1 JSON change + bug #2 1-char
   rename).
3. Local pre-push gate, including the strict-warnings
   run.
4. Append commit-readiness.

## Coordination

* **Blocks v0.11.2 tag-cut**: ci-8 dry-run #3 fires
  after this commits → if green, @@WebtestB second-Mac
  verify → @@Alex "cut it" → @@Systacean cuts
  `chan-v0.11.2`.
* **Composes with `-b-15`'s bundled-chan-binary work**:
  the universal2 follow-up (post-v0.11.2 ci-N) extends
  the same `bundle.externalBin` plumbing.

## Suggested commit subject

```
chan-desktop: aarch64-only DMG for v0.11.2 + unused-app rename (fullstack-b-20)
```

## Open questions

(populated as you investigate)
