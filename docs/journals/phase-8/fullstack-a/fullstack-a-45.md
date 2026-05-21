# fullstack-a-45 — Terminal Settings migration to Hybrid Terminal back (Task B)

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: queued (sequenced AFTER fullstack-a-43 lands in HEAD)

## Goal

Migrate the Terminal section out of `SettingsPanel.svelte`
into the new `HybridTerminalConfig.svelte` mount point
introduced by `-a-43` (Task A).

## Background

Locked design:
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Hybrid back-side revisited". Task A
(`fullstack-a-43`) introduced the four
`Hybrid{Terminal,Editor,Graph,FileBrowser}Config.svelte`
components with empty body placeholders. Task B
populates Terminal.

Scope of the migration (settings that move):

* Scrollback buffer (MB) — from `-b-11`.
* Default TERM value — from `-b-11`.
* Any future font controls (parked).

Settings storage shape is unchanged. Only the
mounting point of the UI moves.

## Acceptance criteria

* Terminal section in `SettingsPanel.svelte` is
  removed; the same settings render inside
  `HybridTerminalConfig.svelte` instead.
* Warning copy added: "These settings apply to ALL
  terminals, not just this one." (Or similar; aligns
  with the round-2-plan Hybrid back-side scope note
  that per-type settings apply per-type, not per-tab.)
* Tests cover: settings persist across reload, the
  underlying `Preferences` shape is unchanged, the
  values bind to the new mount point correctly.
* Pre-push gate green: fmt + clippy + cargo test +
  svelte-check + npm build + vitest.

## How to start

1. Audit current Terminal section in
   `SettingsPanel.svelte` to inventory what moves.
2. Move the section into `HybridTerminalConfig.svelte`
   (the empty body placeholder introduced by `-a-43`).
3. Remove the corresponding section from
   `SettingsPanel.svelte`.
4. Add the "applies to all terminals" warning copy.
5. Wire tests + verify gate.

## Coordination

* SPA-only.
* Append "Commit readiness" + poke @@Architect when
  ready.

### Sequencing constraint — HARD prereq

Depends on
[`fullstack-a-43`](fullstack-a-43.md) landing in HEAD.
Task A introduces `HybridTerminalConfig.svelte` as an
empty body; Task B populates it.

## Numbering

Highest committed `-a-N` is `-a-41`; `-a-42` is About,
`-a-43` is Task A, `-a-44` is drag-to-rearrange; this
is `-a-45`. Task C (`-a-46`), Task E (`-a-47`), Task F
(`-a-48`) fan out alongside.

## 2026-05-21 — ready for review

Four-file change. SPA-only; no Rust touched.

### Architecture

Goal: move the Terminal section out of
`SettingsPanel.svelte` into the
`HybridTerminalConfig.svelte` stub introduced by
`-a-43` (Task A). Settings storage shape unchanged;
both surfaces still PATCH the same `GlobalConfig.
preferences.terminal` subtree.

Key design choice: **self-contained component with
merge-against-current-server save**. The new
component owns its own `editing` / `dirty` / autosave
lifecycle scoped to the terminal subtree. On save it
re-fetches the current `GlobalConfig` from the
server first (`api.config()`), then PATCHes a
payload that overlays only `preferences.terminal`
onto whatever the server currently holds. This
means an in-flight SettingsPanel save (theme /
editor / date) can NOT be clobbered by a parallel
Hybrid Terminal save, and vice versa. The dirty
comparator is also scoped: it compares only the
terminal subtree, so SettingsPanel-owned edits
elsewhere in the form don't trigger a Hybrid
Terminal autosave.

Alternative considered + rejected: extract a shared
`preferencesEdit.svelte.ts` module holding the
editing state for both surfaces. Cleaner, but
substantially bigger refactor — touches the entire
SettingsPanel save lifecycle, the autosave timing,
and the bind shape across half a dozen sections.
The merge-against-server pattern lands the same
guarantee with a much smaller blast radius.

### Files moved

`web/src/components/HybridTerminalConfig.svelte`:

* Imports: `clampScrollbackMb`,
  `SCROLLBACK_MB_DEFAULT/MIN/MAX` from
  `terminal/scrollback`; `drive` from
  `state/store.svelte`; `api` from `api/client`.
* TERM constants (`KNOWN_TERM_VALUES`,
  `DEFAULT_TERM`, `CUSTOM_TERM_SENTINEL`)
  carried over.
* Local `editing: Preferences | null` state with
  $effect-driven sync from `drive.info` when no
  local edit pending.
* `normalizeTerminal(p)` — scoped to the
  terminal subtree (the rest of
  `normalizePrefs` stays in SettingsPanel).
* Derived view: `scrollbackMb`, `currentTerm`,
  `isKnownTerm`, `termSelectValue`.
* Setters: `setScrollbackMb`, `setTermSelection`,
  `setCustomTerm`.
* Dirty / save lifecycle: `terminalDirty()`,
  `scheduleSave()`, `save()`,
  `terminalSnapshot()`. SAVE_STATUS surfaced in
  the header band so the user sees autosave
  progress.
* Markup: warning copy + Scrollback field +
  Default TERM field with the custom-TERM
  follow-up; ids re-namespaced to
  `hybrid-terminal-*` so the legacy
  `terminal-*` ids don't collide.
* CSS: `.terminal-field`, `.terminal-label`,
  `.terminal-control`, `.scrollback-control`,
  `.terminal-unit`, `.hint.warning` (new),
  `.save-status`, `.config-header /
  -title / -body`.

`web/src/components/HybridTerminalConfig.test.ts`
(new): 8 pinned-source assertions covering
warning copy, scrollback wiring, range bounds,
TERM dropdown shape, custom-TERM rendering,
save merge-against-server pattern,
normalizeTerminal backfills, dirty scope.

### Files trimmed

`web/src/components/SettingsPanel.svelte`:

* Removed: `clampScrollbackMb` /
  `SCROLLBACK_MB_*` imports, TERM constants,
  scrollbackMb / currentTerm / isKnownTerm /
  termSelectValue derived view, setScrollbackMb
  / setTermSelection / setCustomTerm setters,
  Terminal section markup (88 lines), Terminal
  CSS scope (`.terminal-section`,
  `.terminal-field`, `.terminal-label`,
  `.terminal-control`, `.scrollback-control`,
  `.terminal-unit`).
* `normalizePrefs(p)` stripped of the terminal
  subtree branch; doc comment updated to
  point at `-a-45`.
* GlobalConfig round-trip path unchanged:
  SettingsPanel still PATCHes the full payload;
  the terminal subtree just rides through as
  read-only from its perspective.

`web/src/components/SettingsPanel.terminal.test.ts`
(was 7 pins, now 5): repurposed as a regression
guard that the Terminal section is GONE from
SettingsPanel (header / control ids / TERM
constants / scrollback imports / normalizePrefs
terminal branch). Wiring assertions migrated to
`HybridTerminalConfig.test.ts`.

### Tests

* `HybridTerminalConfig.test.ts`: 8 new pins.
* `SettingsPanel.terminal.test.ts`: 5 negative
  pins (regression guard against re-introducing
  the moved surface).

### Gate

* vitest **606 / 606** (+6 net from -a-44's 600
  baseline; 8 new pins in HybridTerminalConfig
  + 5 negative pins in SettingsPanel.terminal
  - 7 old pins = +6).
* svelte-check 0 errors / 0 warnings across
  3987 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions flagged

* **Parallel-surface save races**: the
  merge-against-current-server pattern handles
  the canonical case (SettingsPanel autosave +
  HybridTerminalConfig autosave both pending
  within the 500 ms debounce window). Worst
  case: the SECOND save's `api.config()` fetch
  loses to a third-party update that lands
  between the first save's PATCH and the
  second save's fetch. Atomic on the server
  side; last-writer-wins remains the contract.
  Flag if a stricter contract is wanted.
* **`hybrid-terminal-*` id namespacing**:
  changed from `terminal-*` so a user with
  both surfaces open at once doesn't see
  duplicate ids. Optional; could revert if
  the namespace collision risk is theoretical
  (SettingsPanel surface is now empty of
  terminal controls).
* **save-status indicator copied into the new
  component**: HybridTerminalConfig has its own
  "saving… / saved / save failed" pill in the
  header. SettingsPanel keeps its own. Two
  parallel indicators when both surfaces are
  open; arguably this is correct (each
  reports its own debounce).

### Suggested commit subject

```
Migrate Terminal Settings to Hybrid Terminal back-side (fullstack-a-45)
```

Single commit. The four files are tightly
coupled around the same move; intermediate
states would not compile (SettingsPanel
imports `setScrollbackMb` etc. that no longer
exist locally).

### Files for `git add` (per-path discipline)

* `web/src/components/HybridTerminalConfig.svelte`
* `web/src/components/HybridTerminalConfig.test.ts`
* `web/src/components/SettingsPanel.svelte`
* `web/src/components/SettingsPanel.terminal.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-44.md`
  (audit-trail correction append, per @@Architect's
  routing of the `a8e991a` incident — bundled here
  per your "your call" note)
* `docs/journals/phase-8/fullstack-a/fullstack-a-45.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`
  (commit-readiness poke)

Push held — multi-agent tree commit discipline.
Standing by for clearance.
