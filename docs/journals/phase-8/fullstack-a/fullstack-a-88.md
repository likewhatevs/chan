# fullstack-a-88 — First-boot: remove "open FB tab" rule; always boot with docked FB on the left

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Replace the first-boot UX from "open a File Browser
tab when chan opens a drive for the first time" with
"always boot with the docked File Browser on the
LEFT-hand side."

## Reference

@@Alex 2026-05-22: "we had previously created this
rule that when chan boots a drive for the first
time, we open a file browser.. we no longer need
that, and we will always do the first boot with the
docked file browser on the left hand side."

## Scope

### 1. Remove first-boot FB-tab spawning

Audit + remove whatever logic spawns a File Browser
TAB on first drive boot. Likely lives in
`store.svelte.ts` or `App.svelte`'s drive-load /
SerTab-restore path. Look for FB-tab spawn calls
that are gated on "no prior layout state" / "first
launch" / "empty SerTab".

### 2. Default docked FB to LEFT on first-boot

On first-boot (when there's no prior
`browser_side_panes` preference in
`~/.chan/preferences.toml`):

* Set `browser_side_panes.left = true` (docked FB
  visible on left).
* Set `browser_side_panes.right = false` (right
  stays empty).
* Persist this as the user's preference on first
  write so subsequent boots respect their toggle.

### 3. Preserve existing user preferences

* If the user has already configured
  `browser_side_panes` (left/right docked or both
  hidden), the boot respects their setting.
* Only the FIRST-BOOT (empty preferences) path
  changes.

## Acceptance

1. **First-boot opens with docked FB on left**: a
   brand-new drive (no `~/.chan/preferences.toml`
   OR no `browser_side_panes` in it) opens with
   the FB docked on the left, NO separate FB tab
   spawned.
2. **No FB-tab spawn on first-boot**: previously
   the first-boot spawned an FB as a tab; that
   spawn path is REMOVED.
3. **Existing user preferences respected**: if
   user has `browser_side_panes.right = true`
   from prior session, boot keeps it that way.
4. **No regression** on drive switch / reopen
   flows beyond this first-boot defaulting.

### Tests

Vitest pin on:
* First-boot (empty preferences) → docked FB on left.
* First-boot does NOT spawn an FB tab.
* Existing preferences preserved.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA. SPA-only.
* Atomic-audit-commit.

## Authorization

Yes for `web/src/state/store.svelte.ts` +
`web/src/App.svelte` (or wherever the first-boot
logic lives) + preferences default + tests + task
tail + outbound.

## Numbering

This is `-a-88`.

## Out of scope

* Re-styling the docked FB.
* Changing the FB tab behavior (open via menu /
  Cmd+O still works).
* Re-doing the carousel for empty panes.

## 2026-05-22 — ready for review

Four-file change. Cross-stack (chan-server +
SPA).

### What landed

`crates/chan-server/src/preferences.rs`:
* `BrowserSidePanes::default()` flipped from
  derived `Default` (both false) to a manual
  `Default` impl returning `{left: true,
  right: false}`. A fresh `preferences.toml`
  now ships with the docked-left FB on
  first-boot.
* +2 new Rust unit pins:
  * `browser_side_panes_default_is_left_docked` —
    direct default check.
  * `editor_prefs_default_carries_left_docked_fb`
    — cross-check via `EditorPrefs::default()`.

`web/src/App.svelte`:
* Removed the boot-time
  `if (!hasAnyTab) openBrowser()` rule + the
  `openBrowser` import.
* Replacement comment cross-references
  `-a-88` + the chan-server-side mirror
  default.

`web/src/state/store.svelte.ts`:
* `browserSidePanes` initial state flipped
  from `{left: false, right: false}` to
  `{left: true, right: false}`. Matches the
  chan-server default so the brief
  pre-preferences-load window doesn't flip
  the dock visually.

`web/src/state/firstBootDockedFb.test.ts`
(new): 5 raw-source pins:
* App.svelte no longer calls
  `openBrowser()` in the empty-layout
  branch.
* App.svelte no longer imports
  `openBrowser`.
* App.svelte references `fullstack-a-88` +
  the "docked FB on left" rationale.
* SPA default is `{left: true, right: false}`.
* Rationale comment cites the chan-server
  mirror.

### Acceptance

1. **First-boot opens with docked FB on left**
   ✓ — `BrowserSidePanes::default()` ships
   `left: true`; chan-server returns this on
   fresh prefs; SPA applies via
   `applySidePanesPreferences`. @@WebtestA
   walk for empirical.
2. **No FB-tab spawn on first-boot** ✓ —
   `if (!hasAnyTab) openBrowser()` removed.
3. **Existing user preferences respected**
   ✓ — chan-server reads disk first; user's
   persisted `browser_side_panes` overrides
   the default. The SPA load path
   (`applySidePanesPreferences`) blindly
   takes whatever the server returns.
4. **No regression on drive switch / reopen**
   ✓ — flow unchanged beyond the first-boot
   default.

### Gate

* `cargo test -p chan-server --lib`: **220
  passed** (+2 net from prior 218).
* vitest **938 / 938** (+9 net from `-a-87`'s
  929; combined first-boot pins + cross-stack
  related pins).
* svelte-check 0 errors / 0 warnings across
  4028 files.
* npm build clean.

### Decisions

* **Default fix in chan-server** — single
  source of truth on the wire. The SPA
  default flip is a belt-and-suspenders
  catch for the brief pre-load window.
* **Removed `openBrowser` import** from
  App.svelte — no other call site needed it
  in that file (the spawnBrowserFromContext
  function uses `openBrowserInActivePane`
  directly).
* **Kept the empty-pane carousel** — the
  task body's framing left the carousel
  intact. With the docked FB on the left,
  the main pane can stay empty (logo +
  shortcut hints) while the dock provides
  the launch surface.
* **No serde-default-via-flatten trick** —
  the new `Default` impl is the cleanest
  shape; chan-server's `EditorPrefs`
  already uses `#[serde(default)]` to read
  it via `BrowserSidePanes::default()`.

### Suggested commit subject

```
First-boot: docked FB on left by default, remove FB-tab spawn (fullstack-a-88)
```

Single commit. chan-server default + App.svelte
removal + SPA default flip + tests.

### Files for `git add` (per-path discipline)

* `crates/chan-server/src/preferences.rs`
* `web/src/App.svelte`
* `web/src/state/store.svelte.ts`
* `web/src/state/firstBootDockedFb.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-88.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.
