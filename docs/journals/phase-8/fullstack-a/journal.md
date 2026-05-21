# @@FullStackA's phase-8 journal

Author: @@FullStackA
Date: 2026-05-19

Frontend + backend lane A. Same profile as @@FullStackB; operates
in parallel to clear the bug queue and (Round 2) feature queue.

Append-only. New entries go at the bottom under a dated heading.

## 2026-05-19 — Round 1 sweep, fullstack-a-1 through -8

Cleared the initial Round-1 queue in one session. All eight
tasks closed; -1, -2, -3 committed locally with architect
clearance; -4 through -8 awaiting commit clearance.

| Task            | Topic                                        | Status         |
|-----------------|----------------------------------------------|----------------|
| fullstack-a-1   | FB tab title = parent-dir + trailing slash   | committed      |
| fullstack-a-2   | Status-bar clicks + watcher dot yellow       | committed      |
| fullstack-a-3   | Hybrid label + drop flash + 1/2/3 immediate  | committed      |
| fullstack-a-4   | Rich prompt cluster (focus, overlay, spawn)  | review pending |
| fullstack-a-5   | Editor cluster (img scroll, empty pane, repop) | review pending |
| fullstack-a-6   | Cmd+K F focuses search input                 | review pending |
| fullstack-a-7   | Hybrid NAV: Cmd+K → Cmd+.                    | review pending |
| fullstack-a-8   | Restore wobble on Hybrid + right-click menus | review pending |

Highlights:

* `SpawnDialog` lifted to App root via a new
  `state/spawnDialog.svelte.ts` singleton (-4). Fixes the
  "backdrop without dialog" visibility regression by moving the
  dialog out of every ancestor stacking context that clipped
  its `position: fixed` (rich-prompt's z-index: 20, pane's
  overflow-hidden, Hybrid NAV's filter on unfocused panes).

* New `BrowserLabelCtx = { driveName?; selectedIsDir? }` plumbed
  from `Pane.svelte` through `tabLabel`/`tabLabelInPane` (-1).
  Tree-derived `is_dir` lookup keeps the FB tab title honest
  even when the selected file just got deleted — the lookup
  returns undefined and the title falls through to the drive
  display name.

* `BubbleOverlay.visibleEvents` now filters surveys whose `id`
  matches a sibling `survey-reply` event (-5). Picked option (b)
  from the task's three fix options since the chan-server reply
  endpoint already writes a pair-by-id record; pure SPA-side
  fix, no cross-stack coordination.

* Audited eight right-click / overlay surfaces for the
  easeOutBack wobble (-8). Four were missing it after the
  phase-7 `fullstack-80` / `-82` rework. Added a 260ms
  cubic-bezier(0.34, 1.56, 0.64, 1) open animation to each,
  scoped to local keyframes + `prefers-reduced-motion` cancel.

Gate green throughout: vitest 452/456, `npm run check` clean,
`npm run build` clean. Pre-push Rust gate not touched (no Rust
changes in any of -1 through -8).

Awaiting architect clearance on -4 through -8 before
committing. No push gate cleared yet for any commit — pushes
wait for Round-1 close per the architect's standing rule.

## 2026-05-20 — round-2 rich-prompt mini-wave kickoff

Fresh recycled session. Picked up the rich-prompt mini-wave
dispatch (`fullstack-a-28` / `-29` / `-30`). Cross-lane peer
is `fullstack-b-13` (Shell/Agent submit-mode toggle on the
PTY-write side). @@Alex's session-bootstrap warning about
"fullstack-a-13 or fullstack-b-28" — both numbers off by
one: a-13 was committed long ago (`887d19c`), b-28 doesn't
exist. Closest live peer is b-13. Mapped the seam before
editing: the "poke<Enter>" emitter is server-side
(`terminal_sessions.rs:502`), not SPA-side — b-13's
territory, my -a-28 doesn't touch it. SerTab additions are
non-overlapping (`dbi` vs `rpsm`).

`fullstack-a-28` complete: filter generalization (comment +
test pin against the already-general predicate) + explicit
dismiss button + Loading-flicker root-cause fix (the per-
poll `loading=true→false` swap was hiding the post-reply
filter outcome, not the predicate itself). Gate green
(vitest 512/512, check 0/0, build clean). Awaiting commit
clearance.

Moving on to `fullstack-a-29` (terminal-host margin
recompute on rich-prompt collapse transition).

### -29 complete

ResizeObserver on the prompt root drives a new non-
persisted `measuredHeightPx` field; `TerminalTab`'s
margin reactor prefers it over `heightPx`. Auto-adapts
to collapse + drag-resize + viewport clamps with one
source of truth. Gate green (vitest 512/512).

### -30 complete

Extended the same observer to also track width
(`measuredWidthPx`). New per-prompt `pageWidthRatio` on
TerminalRichPromptState + SerTab `rppw` field. Inline
`--chan-page-max-width` override on `.rich-prompt`
breaks the pane's cascade so narrowing in one tile no
longer affects sibling tiles. Slider mirrors the
editor's tab-menu slider verbatim, lives in the
existing `.ctx` right-click menu. Default behaviour
shifts to "no cap" per prompt — chat-style composers
feel less cramped under tiling; users who want narrow
opt back in via the new slider. Gate green (vitest
514/514).

Mini-wave complete on my lane: -28 / -29 / -30 all
ready for review. Three standalone commits queued in
the working tree (no commits landed yet — push protocol
unchanged). Standing down for the session unless
redirected.

## 2026-05-20 — fresh-session resume: -33 ready for review

Bootstrapped a fresh @@FullStackA session per the
recycle. Previous incarnation landed `-28` / `-29` /
`-30` / `-31` then flagged context-fill on the
remaining queue. Picking it up from -33 first per the
architect's hard-pair order (-33 before -32).

### -33 complete

Two-file change: `GraphPanel.svelte` +
`revealBrowserActions.test.ts`.

* New `scopeAncestors` derived returns a list of
  `{label, scopeId, current}` crumbs for path-based
  scopes (`drive` / `dir:` / `file:`); tag /
  git_repo / global return empty so the band hides.
  Drive root is always the head so the user can hop
  back from any depth.
* New `rescopeFromHere(scopeId)` helper mutates the
  current tab in place — distinct from
  `scopeFsGraphFromHere` (in `store.svelte.ts`)
  which spawns a new tab and is still used by
  `FileBrowserSurface`. Depth resets to 1;
  selection clears; no-op on the current crumb.
* Breadcrumb band renders at the top of the
  `<Inspector>` body, above the existing `{#if}`
  chain. `<button class="crumb">` for each hop
  except the current one (rendered as `<span
  class="crumb current">`); slash separator;
  wraps on narrow inspector widths.
* Four `onSetAsScope` props dropped from
  GraphPanel: `DriveInfoBody`, fs-mode
  `InspectorBody`, semantic-mode `InspectorBody`.
  Component-level prop in `InspectorBody` / child
  bodies stays for `FileBrowserSurface` (the FB
  sidepane still spawns graphs from the button).
* `scopeFsGraphFromHere` import removed from
  GraphPanel — no longer consumed here. Still
  exported for the FB consumer.

### Tests

`revealBrowserActions.test.ts`: dropped the old
"GraphPanel passes a re-scope callback to
DriveInfoBody" pin (regex pinned the now-gone
block); added four pins covering negative
`onSetAsScope` on DriveInfoBody + InspectorBody
and positive `scopeAncestors` / `scope-crumbs` /
button-bound `rescopeFromHere` / drive-root head.

### Gate green

vitest **525 / 525** (+3 net from the previous-
session 522 baseline; one dropped pin, four new
pins). svelte-check 0/0 across 3976 files. npm
build clean. No Rust touched.

Impl note + design rationale at the tail of
[fullstack-a-33.md](fullstack-a-33.md). Picking up
`fullstack-a-32` next.

### -32 complete

Landed as one cohesive bundle across the chord layer +
cheatsheets + native bridge + three menu surfaces.

* `shortcuts.ts` — three new chord descriptors
  (`app.files.toggle`, `app.terminal.richPrompt` chord
  update, `app.graph.toggle`); registry comment refreshed.
* `store.svelte.ts` — new `openGraphWithContext(ctx)` for
  the live-layout sibling of `paneModeOpenGraph`.
* `App.svelte` — four context-aware spawn helpers
  (`spawn{Terminal,Browser,RichPrompt,Graph}FromContext`);
  top-level handlers for `Cmd+Alt+O/P` + `Cmd+Shift+M`;
  `Cmd+Alt+T` rewired through helper; `Alt+Space` legacy
  alias retained; `chan:command` bridge routes through
  helpers; Hybrid NAV `1/2/3/4` cases dropped; `o/O` +
  `v/V` mnemonic cases added; `t/T` + `p/P` retained.
* `Pane.svelte` — single `spawnActions` list = source of
  truth. Pane hamburger menu prepends the four entries +
  separator above Enter Hybrid NAV. Empty-pane right-
  click menu uses the same list + Search + Settings.
* `EmptyPaneCarousel.svelte` — slide 1 gains a 4-up
  spawn-row above the ASCII table; dispatches `chan:command`
  so it routes through the helpers.
* `PaneModeHelp.svelte` — Spawn group switched to letter
  mnemonics only (`t/o/p/v`); numeric caps gone.
* `crates/chan/src/main.rs` — `SERVE_LONG_ABOUT`
  regenerated; Hybrid NAV section updated.
* `desktop/src-tauri/src/serve.rs` — `KEY_BRIDGE_JS` gains
  native bindings for Cmd+O / Cmd+P / Cmd+Shift+M; legacy
  tests updated.

### Context-aware spawn flow

`resolveSpawnContext()` (shipped `fullstack-43`) returns
`{ dir, file? }` based on the focused tab kind. Each new
chord handler resolves fresh at keypress and threads
through the matching spawn API:

* `Cmd+T` → `openTerminalInActivePane({ cwd: ctx.dir })`
* `Cmd+O` → `revealAndSelect` + `openBrowser()` (primes
  `browserSelection` before new tab mounts)
* `Cmd+P` → `showOrSpawnRichPromptInFocusedPane()`
* `Cmd+Shift+M` → `openGraphWithContext(ctx)` → scoped
  spawn that lands on `-a-33`'s default from-here render

### Surface unification

Same four items + ordering across carousel slide 1,
pane hamburger, and empty-pane right-click. Click +
chord route to identical destinations via the same
helpers.

### Gate green

* vitest **530 / 530** (+5 net from `-33`'s 525).
* svelte-check 0 errors / 0 warnings across 3976 files.
* npm build clean.
* `cargo fmt --check` clean.
* `cargo clippy -p chan -- -D warnings` clean.
* `cargo test -p chan` 58 / 58.
* `cargo test --no-default-features key_bridge` (desktop)
  2 / 2.

Full impl note + chord-set table + composition notes at
the tail of [fullstack-a-32.md](fullstack-a-32.md).

Queue: -34 (Wysiwyg paste escape, independent), -35 (file
rename UX, needs chan-drive op). Picking up -34 next
since it's the smaller of the two.

### -34 complete

Root cause was turndown's default text-node escape (NOT
chan-side). HTML-paste handler (`paste_html.ts`) runs
ahead of CM6's plain-text paste, so any paste with HTML
on the clipboard — most pastes from Xcode / VS Code /
browser source-view — goes through turndown and gets
`\*` / `\_` / `\[` etc. baked into the markdown output.
Default `escape` lives at `TurndownService.prototype.escape`
and is callable per-instance as `td.escape(string)`. One-
line fix: override with identity on the instance. Picked
the simple shape per the task spec (drop the escape pass
entirely); source mode (`-a-26`) is the escape hatch for
users who want literal pasted text.

`htmlToMarkdown` exported for the test pin (8 new tests
in `paste_html.test.ts` covering asterisk emphasis,
strong, underscore, link, backtick code, heading hash,
list dash, plus a still-converts-rich-HTML guard).

Gate green: vitest 538/538 (+8), svelte-check 0/0, build
clean. Two-file change.

### -35 complete

chan-drive `Drive::rename_with_link_rewrite` + chan-server
`POST /api/move` + SPA `performMove` already provided the
full atomic-rename + link-rewrite + watcher-suppression
+ tab-rekey chain. Only the UX wrapper was missing.

* `fileOps.renameInPlace(path, next, isDir)` added in
  `store.svelte.ts` as the inline-rename entry point —
  same `performMove` machinery; just bypasses the
  uiPathPrompt modal.
* `FileEditorTab.svelte` rewires `doRename` to flip a
  per-tab `renameActive` flag (focus + select-all on
  the band's input via queueMicrotask) instead of
  popping the modal. New `commitRename` / `cancelRename`
  / `onRenameKeydown`. Header band markup `{#if
  renameActive}` sits above the editor toolbar block,
  outside the `--chan-page-max-width` cap.
* `.rename-band` CSS: full-width header band; input
  takes flex: 1 with monospaced font matching the rest
  of the file-path chrome.
* New `fileRenameBand.test.ts` — 6 raw-source pins for
  the wiring shape (doRename flips state vs modal;
  commit/cancel/keydown wiring; band sits above editor
  toolbars; full-width band + flex-1 input;
  `fileOps.renameInPlace` exists + uses performMove).

Gate green: vitest 544/544 (+6), svelte-check 0/0 across
3977 files, build clean. No Rust changes — pre-existing
infrastructure landed the heavy lifting.

### Round-1 detour sub-wave complete

-32 / -33 / -34 / -35 all ready for review on top of the
in-flight mini-wave (-28 / -29 / -30 / -31 already in
HEAD). Six commits ready for the patch-release commit-
grouping cut once architect clears each. Standing down
unless redirected.

## 2026-05-21 — Round-2 Wave-2 Task A: -a-43 ready for review

Fresh @@FullStackA session bootstrapped through the live
rich-prompt-watcher pre-flight test. Once the user
signalled work could start, picked up the dispatched
`-a-43` (Hybrid back-side architecture refactor — Task A)
from `event-architect-fullstack-a.md` 2026-05-21 tail.

### Scope

Foundational refactor for the round-2-plan §"Hybrid
back-side revisited" wave. Back of a Hybrid pane stops
being a tab collection (the `fullstack-48` shape) and
becomes a per-surface configuration view scoped to the
active front-tab type. Per the task body, this lands the
ARCHITECTURE; Tasks B / C / D / F populate the four
config component bodies; Task E collapses the front/back
theme split; Task G (cut as `-a-42`) builds out the
remaining About section once the Settings overlay is
trimmed.

### What landed

* `HybridSide` slimmed to `{ theme? }` — tab collection
  removed from the type.
* `flipHybrid()` no longer swaps tabs; only toggles
  `showingBack` + (vestigially) swaps the per-side
  theme override. `pane.tabs` is now invariantly the
  front-side tabs.
* `Pane.svelte` hides the tab strip when
  `pane.showingBack`; mounts a `HybridXConfig`
  component matching `active?.kind` in the `.editor-wrap`
  body when flipped. Pane-mode preview still operates on
  front content; terminal each-block (kept mounted for
  scrollback) gains the new `!pane.showingBack` gate on
  active+focused props so xterm doesn't steal focus
  through the config view.
* Four new stub components in `web/src/components/`:
  `HybridTerminalConfig.svelte`,
  `HybridEditorConfig.svelte`,
  `HybridGraphConfig.svelte`,
  `HybridFileBrowserConfig.svelte`. Title band only; each
  documents the populating task in its script header.
* `.back-attention` chrome + CSS + `backHasAttention`
  derived all dropped. Under the new model the back has
  no "unread/activity" surface to flag; the chrome
  stayed lean.
* Serialization: `bt` (back tabs) no longer emitted;
  legacy `bt` payloads tolerated on deserialize (the
  tab contents are discarded). `hb` + `sb` round-trip
  unchanged.

### Tests

vitest 588/588 across the rewritten + new pins:
* `tabs.test.ts`: 4 flip-suite pins rewritten to match
  the new "front tabs never swap" invariant. Split-from-
  back pin updated. Serialize/restore pin updated to
  forbid `bt` emission.
* `Pane.test.ts`: 2 obsolete `.back-attention` pins
  dropped; 4 new pins added under a new
  `describe("Pane back-side configuration view")` block.
* `paneTerminalMount.test.ts`: pin regex tightened to
  include the new `!pane.showingBack` gate.

### Gate

vitest 588/588, svelte-check 0/0 across 3983 files,
npm build clean, `cargo fmt --check` clean,
`cargo clippy -p chan --all-targets -- -D warnings`
clean. No Rust touched.

### Subtle deviations flagged in the impl note

* Theme swap preserved (task body's "stays for now"
  reading). Task E collapses.
* Back-existence round-trip: pre-`-a-43` serializer had
  the equivalent edge-case loss; new shape is
  structurally identical. Documented in the round-trip
  test.
* Empty-pane back state: rendered as a generic Hybrid
  placeholder asking the user to open a front tab.

Full impl note + suggested commit subject at the tail of
[fullstack-a-43.md](fullstack-a-43.md). Five files
changed, four new files. Outbound poke fired to
@@Architect with Commit readiness; standing by for
review + commit routing.

## 2026-05-21 — Round-2 Wave-2 -a-44: ready for review

Recycled session bootstrap (post tear-down) ran clean.
PRE-RECYCLE HANDOVER queue picked up cleanly: `-a-43`
committed (`b36ca96`) by the previous incarnation;
`-a-44` (Hybrid drag-to-rearrange + transaction-mode
NAV) is the next item, prereq in HEAD.

### -a-44 complete

Four-file change (SPA + state only; no Rust touched):
* `web/src/state/tabs.svelte.ts` — `paneMode` shape
  gains `transactionMode` / `grabPaneId` /
  `hoverPaneId`. New `enterPaneModeTransaction`,
  `paneModeSetGrab`, `paneModeSetHover`,
  `paneModeSwapWith` exports.
* `web/src/state/tabs.test.ts` — 8 new pins under
  a `Hybrid NAV transaction mode (fullstack-a-44)`
  describe block covering Entry A / Entry B / swap
  semantics / state-gating / commit-clear /
  cancel-clear.
* `web/src/components/Pane.svelte` — dead-zone
  div between last `.tab` and `.actions`, with
  manual mousedown + 5-px threshold mousemove
  tracking (Entry A) + dblclick (Entry B). Pane
  root augmented with `transaction-active` /
  `transaction-grab` / `transaction-drop-target`
  class flags + mouseenter/leave/up handlers that
  drive grab + hover state. CSS for grab cursor +
  dashed outline on grab pane + inset overlay on
  drop target. `position: relative` added to
  `.pane` so the `::after` drop-target overlay
  anchors correctly.
* `web/src/components/Pane.test.ts` — 4 new pins
  under a `Pane Hybrid NAV transaction mode
  (fullstack-a-44)` describe block covering
  dead-zone presence, dblclick → Entry B,
  class-flip dynamics through grab/hover state,
  and a raw-source guard that the dead zone uses
  manual mousedown (not HTML5 dragstart).

### Design rationale

* **Manual mousedown + threshold, not HTML5
  drag**: tabs in the strip own `draggable="true"`
  for inter-pane tab DnD. Routing the dead zone
  through the same pipeline would collide.
  Manual state-machine keeps the two flows clean.
* **`paneModeSwap` refactored on top of
  `paneModeSwapWith`**: directional swap now
  reduces to "resolve neighbour, swap by id".
  The new function is the underlying primitive;
  the keyboard NAV's WASD-swap reads the same.
* **Transaction state cleared by
  commit/cancel**: the existing keyboard NAV
  Enter/Esc paths already clear the new fields
  via my updates to `commitPaneMode` /
  `cancelPaneMode`. No App.svelte chord
  additions needed.
* **Chain semantics**: each drop clears grab +
  hover but keeps `transactionMode` on, matching
  the task's "Drag continues until
  commit/dismiss" rule.
* **Drop-target visuals distinct from focus**:
  `.transaction-drop-target` uses an inset
  overlay (not the `.pane.focused` solid ring),
  so the user can tell drop-here apart from
  keyboard-active.

### Gate

* vitest **600 / 600** (+12 net: 8 in
  tabs.test.ts, 4 in Pane.test.ts).
* svelte-check 0 errors / 0 warnings across
  3987 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions flagged in the task tail

* Cmd+. mid-transaction left as a no-op (task
  default was "yes exit" but existing keyboard
  NAV doesn't exit on Cmd+. either; consistency
  argument cuts both ways). Esc dismisses
  cleanly.
* Click-without-drag → no-op release (matches
  task default).
* Every pane can be the drop target (not just
  Hybrid-marked panes). Matches @@Alex's
  "rearrange any pane" intent.

Impl note + suggested commit subject at the tail
of [fullstack-a-44.md](fullstack-a-44.md).
Outbound poke fired to @@Architect with Commit
readiness; standing by for review + commit
routing. Queue waiting: `-a-45..-48` (Hybrid
back-side Tasks B/C/E/F) → `-a-49..52` (graph
overhaul first sub-wave) → `-a-42` (About).

## 2026-05-21 — -a-44 incident routed; -a-45 ready for review

`-a-44` cleared with all 3 deviations accepted. On
commit I caught a cross-agent commit-hygiene
incident at pre-commit audit: @@WebtestB's session
used a broad `git add` and swept my work-in-progress
files into their `a8e991a docs: webtest-b-3 …`
commit. Net: `-a-44` work is in HEAD verbatim under
the wrong commit subject. Flagged to @@Architect
via [`../alex/event-fullstack-a-architect.md`](../alex/event-fullstack-a-architect.md)
incident poke (`e9315df`); architect routed
(b) audit-trail correction + (c) architect-side
grep-anchor commit (`3baaa6d`). Audit-trail
correction appended to
[`fullstack-a-44.md`](fullstack-a-44.md) tail.

### -a-45 complete (Hybrid back-side Task B)

Four-file change. SPA-only.

* `web/src/components/HybridTerminalConfig.svelte`
  — populated from a stub to the full Terminal
  settings surface. Self-contained component
  with its own editing / dirty / autosave
  lifecycle scoped to the
  `preferences.terminal` subtree.
* `web/src/components/HybridTerminalConfig.test.ts`
  (new) — 8 pins covering warning copy,
  scrollback wiring, TERM dropdown shape,
  custom-TERM rendering, save merge-against-
  server pattern, normalize backfills, dirty
  scope.
* `web/src/components/SettingsPanel.svelte` —
  Terminal section markup (88 lines), TERM
  constants, scrollback imports, derived view
  helpers, setters, and CSS scope all removed.
  `normalizePrefs` trimmed to non-terminal
  branches. GlobalConfig round-trip path
  unchanged.
* `web/src/components/SettingsPanel.terminal.test.ts`
  — repurposed from wiring pins to a regression
  guard that the Terminal section is GONE
  (header / control ids / TERM constants /
  scrollback imports / normalize terminal
  branch).

### Save-race contract

The architecturally interesting bit: rather than
extract a shared `preferencesEdit.svelte.ts`
module (cleaner but big refactor; touches the
entire SettingsPanel save lifecycle), I kept
each surface self-contained and used a
**merge-against-current-server** save shape.
HybridTerminalConfig re-fetches the current
`GlobalConfig` on save before PATCHing, so an
in-flight SettingsPanel save (theme / editor /
date) can't be clobbered, and vice versa. The
dirty comparator is also scoped to the
terminal subtree so SettingsPanel-owned edits
elsewhere don't trigger spurious Hybrid
Terminal autosaves.

### Gate

* vitest **606 / 606** (+6 net: +8
  HybridTerminalConfig + 5 negative pins on
  SettingsPanel - 7 old wiring pins).
* svelte-check 0 errors / 0 warnings across
  3987 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions flagged

* `hybrid-terminal-*` id namespacing (changed
  from `terminal-*` to avoid duplicate-id risk
  when both surfaces are mounted at once).
* Two parallel save-status indicators (one
  per surface) — arguably correct since each
  reports its own debounce. Flag if a single
  indicator was wanted.
* merge-against-current-server pattern is
  last-writer-wins; atomic on the server side;
  flag if a stricter contract was wanted.

Impl note + suggested commit subject at the tail
of [fullstack-a-45.md](fullstack-a-45.md).
Outbound poke fired to @@Architect with Commit
readiness; standing by. Queue waiting: `-a-46`
(Task C — Editor Settings migration), `-a-47`
(Task E — front/back theme collapse), `-a-48`
(Task F — Search / Indexing / Reports settings
migration + chan-reports toggle restore), then
`-a-49..52` (graph overhaul) + `-a-42` (About).

## 2026-05-21 — -a-45 committed; -a-46 ready for review

`-a-45` cleared with all 3 deviations accepted +
the (b) audit-trail bundle confirmed. Committed
cleanly as `1f80d09 Migrate Terminal Settings to
Hybrid Terminal back-side (fullstack-a-45)`. Pre-
commit `git diff --staged --stat` matched the
cleared path list exactly (8 files); post-commit
`git show --stat HEAD` confirmed no stowaways
(the dirty-worktree work from other lanes —
chan-drive / ci-12 / systacean-15/17/18 /
webtest-a / webtest-b channels — all stayed
unstaged). The pre-commit audit lesson from the
`-a-44` incident applied cleanly this beat.

### -a-46 complete (Hybrid back-side Task C)

Three-file change. SPA-only.

* `web/src/components/HybridEditorConfig.svelte`
  populated from a stub with 5 sections (Editor
  theme, Appearance, Layout, Date pills, On save).
  Self-contained editing / dirty / autosave
  lifecycle scoped to the editor-related slice.
  Two side-effects carried over from
  SettingsPanel (live-apply data-editor-theme,
  sync editorToolsPrefs).
* `web/src/components/HybridEditorConfig.test.ts`
  (new) — 11 wiring pins + 4 negative pins
  against SettingsPanel (regression guard).
* `web/src/components/SettingsPanel.svelte` —
  5 sections (~140 lines), the two side-effects,
  3 imports, the normalize-editor branches, and
  the related CSS scope all removed. `.theme-opt`
  stays for the semantic-search toggle.

### Save / dirty discipline

Same merge-against-current-server save pattern
as `-a-45`: fetch latest GlobalConfig on save,
overlay only the editor-related fields, PATCH.
The dirty comparator is explicit-field-by-field
(editor_theme, theme, line_spacing,
date_format, strip_trailing_whitespace_on_save)
rather than whole-object so SettingsPanel-owned
edits elsewhere don't fire spurious editor
PATCHes.

### Gate

* vitest **621 / 621** (+15 net from -a-45's
  606; 11 wiring + 4 negative pins).
* svelte-check 0 errors / 0 warnings across
  3988 files. CSS sweep dropped the now-unused
  `.section-row`, `.theme-row`, `select`
  combined-rule, and `.theme-opt
  input[type="radio"]` reset from
  SettingsPanel.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions flagged

* Appearance included in the migration per the
  task body's "Theme (per-Hybrid, surviving the
  per-Hybrid override from `-b-5`)". Alternative
  read possible (Appearance stays in
  SettingsPanel as a global UI setting); flag
  if wrong.
* `.strip-toggle` renamed from
  `.semantic-toggle` in the new component (the
  legacy name didn't fit the On save toggle).
* Defensive name attribute namespacing
  (`hybrid-editor-*` / `hybrid-appearance` /
  `hybrid-line-spacing`).

Impl note + suggested commit subject at the tail
of [fullstack-a-46.md](fullstack-a-46.md).
Outbound poke fired to @@Architect with Commit
readiness; standing by. Queue waiting: `-a-47`
(Task E — front/back theme collapse), `-a-48`
(Task F — Search/Indexing/Reports), then
`-a-49..52` (graph overhaul) + `-a-42` (About).

## 2026-05-21 — -a-46 committed; -a-47 ready for review

`-a-46` cleared + committed as `5166223 Migrate
Editor Settings to Hybrid Editor back-side
(fullstack-a-46)` with all 3 deviations
accepted. Pre/post audits matched the cleared
path list (7 files; small deviation: bundled
`fullstack-a-45.md`'s dangling "committed as"
trailing append to avoid leaving it dangling
across sessions).

### -a-47 complete (Hybrid back-side Task E)

Two-file change. State-only (chan-server
unaffected — Preferences shape is unchanged).
No Rust touched.

* `web/src/state/tabs.svelte.ts` — `HybridSide`
  collapsed to `{}` empty marker; `flipHybrid`
  drops the theme-swap dance; `inverseTheme`
  deleted; serialization stops emitting `hb`
  and starts emitting a new `bm` (back-
  materialised marker); deserialization
  accepts legacy `hb` / `bt` as Hybrid signals
  but drops their payload contents.
* `web/src/state/tabs.test.ts` — 3 existing
  Hybrid-flip tests rewritten to the new
  contract (no theme swap on flip; serialize
  emits `bm` + omits `hb`); 1 new test for the
  legacy `hb` migration shape (`ht` wins,
  `hb` ignored, back marker materialised).

### Design decision

The migration spec says "pick the front-side
value as the canonical one." For users who only
ever changed theme on the front, no change. For
users who set different themes on each side,
front wins; back-side preference is lost.
Acceptable per the task body's explicit call-
out.

### Gate

* vitest **622 / 622** (+1 net).
* svelte-check 0 errors / 0 warnings across
  3989 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

Impl note + suggested commit subject at the tail
of [fullstack-a-47.md](fullstack-a-47.md).
Outbound poke fired to @@Architect with Commit
readiness; standing by. Queue waiting: `-a-48`
(Task F — Search/Indexing/Reports + chan-reports
toggle restore), then `-a-49..52` (graph
overhaul) + `-a-42` (About).

## 2026-05-21 — -a-47 committed; -a-48 (option B) ready for review

`-a-47` cleared + committed as `dd586fc Drop
front/back independent theme; single per-Hybrid
value (fullstack-a-47)` with both deviations
accepted. 6 files, no stowaways.

@@Architect routed my `-a-48` scope question via
[`event-architect-fullstack-a.md`](../alex/event-architect-fullstack-a.md)
"2026-05-21 — @@Architect: routing on -a-48
scope question — option (B) + PARTIAL fold into
-a-53": option B selected. PARTIAL fix from
WebtestA's webtest-a-4 walk (HybridTerminalConfig
custom-TERM input rendering) folded into `-a-53`
rather than a standalone task.

### -a-48 complete (Hybrid back-side Task F option B)

Six-file change: SPA + chan-server wiring. No
chan-drive touched.

* `crates/chan-server/src/config.rs` — new
  `ReportsConfig { enabled: bool }`;
  `ServerConfig.reports` field with default ON.
* `crates/chan-server/src/routes/preferences.rs`
  — `PreferencesView.reports` field round-trips
  through `/api/config`.
* `web/src/api/types.ts` — new
  `ReportsPreferences { enabled: boolean }`;
  `Preferences.reports?` optional (back-compat).
* `web/src/components/HybridFileBrowserConfig.svelte`
  populated from stub with 3 toggles:
  - **Semantic search** migrated verbatim from
    SettingsPanel `-a-21` (state machine,
    polling, BuildInfo guard, formatModelSize).
  - **Multi-model picker** placeholder slot
    (disabled `<select>` with the default
    `BAAI/bge-small-en-v1.5`; Round-3 Track 2
    populates).
  - **chan-reports** toggle (new). Writes
    `editing.reports.enabled`; persists via the
    merge-against-current-server PATCH from
    `-a-45`/`-a-46`. Default ON. Help text
    flags backend gating is a follow-up task.
* `web/src/components/HybridFileBrowserConfig.test.ts`
  — 11 wiring pins + 4 negative pins.
* `web/src/components/SettingsPanel.svelte` —
  Semantic-search section, state machine,
  helpers, type imports, `onDestroy` import,
  and ~13 stale CSS rules all removed. After
  `-a-48`, SettingsPanel is the About section
  + GlobalConfig autosave plumbing only.

### Default ON

Option B's call: today's behaviour is
unconditional chan-report; the toggle defaults
to ON behaviourally matching that. When the
backend gating + destructive-on-disable modal
land (follow-up task), the default flips to OFF
per the round-2-plan §"Pre-flight feature
toggles" opt-in spec.

### Gate

* vitest **637 / 637** (+15 net from -a-47's
  622; 11 wiring + 4 negative pins).
* svelte-check 0 errors / 0 warnings across
  3989 files. CSS sweep cleared 14 warnings.
* npm build clean.
* cargo fmt --check clean.
* cargo clippy --all-targets -- -D warnings
  clean.
* cargo test -p chan-server: 205 / 205 pass.

### Follow-up needed

When `-a-48` lands, the next task should cover:
- Backend gating across 4 chan-server route files
  (`inspector` / `graph` / `report` / `storage`).
- chan-drive indexer-pass flag for the
  reports-off case.
- Destructive-on-disable confirmation modal.
- Default flip ON → OFF.

Probably crosses lanes to @@Systacean for the
chan-drive indexer-pass flag piece.

Impl note + commit subject at
[fullstack-a-48.md](fullstack-a-48.md). Outbound
poke fired; standing by. Queue waiting: `-a-53`
(theme architecture correction + bundled
custom-TERM PARTIAL fix) → `-a-54` (flip UX
redesign) → `-a-49..52` (graph overhaul) →
`-a-42` (About).

## 2026-05-21 — -a-48 committed; -a-53 ready for review

`-a-48` cleared + committed as `0391eae Migrate
Search/Indexing/Reports settings to Hybrid FB
back-side (fullstack-a-48 option B)`. 9 files,
no stowaways, pre/post audits matched.

### -a-53 complete (theme architecture correction + custom-TERM PARTIAL fix)

Six-file change. SPA-only; no Rust touched.

**Architecture decision flagged**: kept the
existing `pane.theme?: HybridTheme` field name
rather than renaming to `themeOverride`. The
field semantic was already 3-state
(`undefined | "light" | "dark"`) — the
`themeOverride` name in the task body reads as
descriptive of intent, not a literal rename.
The new 3-option UI (Inherit / Light / Dark)
writes to `pane.theme = undefined` / `"light"`
/ `"dark"`. Avoids 6-file rename churn for a
semantic that's stable. Flag if a literal
rename is wanted.

**Files touched**:

* `Pane.svelte` — `HybridTerminalConfig` +
  `HybridEditorConfig` now receive a `pane`
  prop so the override toggles can write to
  `pane.theme`.
* `HybridEditorConfig.svelte` — Appearance
  section + `setThemeChoice`/`ThemeChoice`
  imports removed; `editing.theme` from
  dirty/save scopes; new per-Hybrid override
  toggle section (3 radios) reading from
  `pane.theme`.
* `HybridTerminalConfig.svelte` — new
  per-Hybrid override toggle section
  alongside scrollback + TERM; `customMode`
  state machinery for the custom-TERM fix
  (init from persisted shape; flips on user
  dropdown choice; preserved across drive
  refreshes via `customModeInited` gate).
* `SettingsPanel.svelte` — Appearance section
  restored with `name="settings-appearance"`;
  `setThemeChoice`/`ThemeChoice`/`ui` imports
  added back; `.theme-row`+`.theme-opt` CSS
  restored.
* `HybridEditorConfig.test.ts` — 5 rewritten
  pins to match the new contract.
* `HybridTerminalConfig.test.ts` — 5 new pins
  for the override toggle + customMode +
  setTermSelection routing.

**Custom-TERM PARTIAL fix** (bundled per
@@Architect's option-B routing):

`-a-45`'s
`setTermSelection("__custom__")` seeded
`default_term=""`. The `currentTerm` derivation
then collapsed empty → DEFAULT_TERM, which
landed `termSelectValue` on a known entry and
hid the custom input. The fix tracks
"user picked Custom" in a separate
`customMode` state slot independent of the
persisted value. Persisted string is now left
alone when the user toggles Custom; toggling
Custom → known → Custom restores the user's
previous custom string in the input.

### Gate

* vitest **643 / 643** (+6 net).
* svelte-check 0 errors / 0 warnings across
  3990 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions flagged

* Kept `pane.theme` field name (vs literal
  rename to `themeOverride`). Existing
  semantic + wire compat preserved.
* `customMode` init gated on
  `customModeInited` to survive drive.info
  refreshes.
* Inherit represented as
  `pane.theme = undefined`; wire compat with
  existing `ht` field encoding.

Impl note + commit subject at
[fullstack-a-53.md](fullstack-a-53.md).
Outbound poke fired; standing by. Queue
waiting: `-a-54` (flip UX redesign — needs
`-a-53` first) → `-a-49..52` (graph overhaul)
→ `-a-42` (About).

## 2026-05-21 — -a-53 committed; -a-54 ready for review

`-a-53` cleared + committed as `8c65296 Hybrid
back-side theme architecture correction +
custom-TERM fix (fullstack-a-53)`. 10 files,
no stowaways, pre/post audits matched.

### -a-54 complete (Hybrid flip UX redesign)

Two-file change. SPA-only; no Rust touched.

`Pane.svelte` reshape:

* Tab strip rendered always (was hidden when
  `pane.showingBack=true` under `-a-43`).
  New `class:flipped={pane.showingBack}` flag.
* `hybridFamilyName` derived from
  `active?.kind`: "Hybrid Terminal" / "Hybrid
  Editor" / "Hybrid Graph" / "Hybrid File
  Browser" / "Hybrid".
* Family-name title element hosted inside the
  `.dead-zone` slot when flipped (un-mirrored,
  pointer-events: none).
* CSS rules: `.tabs.flipped .tab { transform:
  scaleX(-1) }` mirrors tabs; `.tabs.flipped
  .actions { order: -1 }` swaps hamburger to
  the left; `.tabs.flipped .dead-zone` becomes
  the title host with `display: flex;
  justify-content: center`.

`Pane.test.ts` updates:

* Two `-a-43` "tab strip hidden on back"
  pins rewritten to assert the new `-a-54`
  contract (tab strip present + `.flipped`
  class + `.hybrid-title` in dead-zone).
* New `describe("Pane flip UX redesign
  (fullstack-a-54)")` block with 3 pins.

### Design decisions

* **Title in dead-zone slot** vs absolute
  overlay. Dead-zone is the natural empty
  space between rightmost tab + hamburger.
  Overlay risked competing with tab
  click-targets.
* **Order swap** for hamburger (`order: -1`)
  rather than DOM reshuffle. Flex slot swap
  keeps the same `<HamburgerMenu>` instance;
  its anchor "just works" since the menu
  positions relative to DOM, not source
  order.
* **Title not mirrored**. While tabs mirror
  for the viewed-from-behind semantic, the
  title is the user's read-anchor. Flag if
  @@Alex wants the title mirrored too.
* **Dead-zone cursor reset to default** when
  flipped — the drag-to-NAV affordance from
  `-a-44` doesn't make sense from the back.
  Handlers still wire (no behavior change),
  just no longer visually advertised. A
  stricter handler-side gate (no-op on
  `showingBack`) is a small polish follow-up.

### Gate

* vitest **646 / 646** (+3 net from `-a-53`'s
  643).
* svelte-check 0 errors / 0 warnings across
  3990 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

Impl note + commit subject at
[fullstack-a-54.md](fullstack-a-54.md).
Outbound poke fired; standing by. Queue
waiting: `-a-49..52` (graph overhaul first
sub-wave) → `-a-42` (About; A+B+C+F all in
HEAD post -a-53).

## 2026-05-21 — -a-54 committed; -a-55 ready for review

`-a-54` cleared with all 5 shape decisions
accepted + committed as `714ec48 Hybrid flip
UX: preserve tab strip + mirror tabs + swap
hamburger + family-name title (fullstack-a-54)`.
6 files, no stowaways.

Architect cut `-a-55` to correct two design
issues @@Alex flagged post-`-a-54` ship + one
PARTIAL @@WebtestA's `webtest-a-5` walk
surfaced:

1. Remove family-name title from tab strip
   (architect-side misinterpretation of @@Alex's
   "tab area" framing; the title belongs in the
   back-side config view body, not the chrome).
2. Right-align tabs in flipped state (@@Alex
   2026-05-21: "tabs must be aligned to the
   right.. because we flipped").
3. Fix click-on-mirrored-tab swap (webtest-a-5
   check #6 PARTIAL).

### -a-55 complete

Two-file change. SPA-only.

**`Pane.svelte`**:

* Script: `hybridFamilyName` derived removed.
* Template: `<span class="hybrid-title">`
  removed from `.dead-zone` slot.
* CSS:
  - `.hybrid-title` rule + flex-centering of
    `.tabs.flipped .dead-zone` removed.
  - Whole-element transform on
    `.tabs.flipped .tab` removed; replaced with
    per-child `transform: scaleX(-1)` on
    `.tab-icon` + `.path` + `.dirty` +
    `.broadcast-marker` + `.marker`. Click
    target stays in natural coordinates → fixes
    PARTIAL #6.
  - `flex-direction: row-reverse` on
    `.tabs.flipped` + actions order swapped
    `-1` → `1`. Layout: `[≡] [slack] [tabN ...
    tab0]`.
  - Close button NOT mirrored (universally-
    readable `×` stays upright).

**`Pane.test.ts`**:

* Existing `-a-54` "title in tab area" pin
  inverted into a regression guard (`.hybrid-
  title` IS null + back-side config view IS
  rendered).
* `-a-54` raw-source CSS guard rewritten:
  pin per-child mirror selectors + row-reverse
  + actions order: 1; reject the old
  whole-tab transform + old order: -1.
* New click-swap pin: dispatches `mousedown`
  on a flipped-state tab; asserts
  `pane.activeTabId` swaps to the clicked
  tab's id.

### Architect-side lesson context

The architect's poke called out their own
misinterpretation of @@Alex's framing — "tab
area" was read as "tab strip chrome" but
@@Alex meant the back-side config view body
(which already had the title from `-a-43`'s
stubs). My `-a-54` implementation faithfully
followed the spec; the correction is on the
chrome side. No fault on the implementation
lane.

### Gate

* vitest **647 / 647** (+1 net).
* svelte-check 0 errors / 0 warnings across
  3990 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

Impl note + suggested commit subject at
[fullstack-a-55.md](fullstack-a-55.md).
Outbound poke fired; standing by. Queue
waiting: `-a-49..52` (graph overhaul) →
`-a-42` (About).

## 2026-05-21 — -a-55 committed; -a-49 ready for review (option C)

`-a-55` cleared + committed as `7cf6f8e
Hybrid flip UX: remove tab-strip title +
right-align tabs + fix mirrored-tab click
(fullstack-a-55)`.

Then picked up `-a-49` and the audit caught an
architect-side error in the task body: chan-
server's `merge_filesystem_layer` ALREADY emits
Directory nodes + contains edges (line 1131 in
routes/graph.rs). The SPA already CONSUMES them.
So G2 was never about adding ancestor data to
the wire; it's about the LAYOUT TRANSFORM in
GraphCanvas.svelte (1133 lines, d3-force).

Architect routed option C (layout transform
only; defer markdown-link / G5 to its own
follow-up).

### -a-49 complete

Two-file change. SPA-only.

`web/src/components/GraphCanvas.svelte`:

* `DNode` extended with `depth: number` +
  `parentId: string | null`.
* `FORCE` config gains `hierarchyYSpacing: 90`,
  `hierarchyYStrength: 0.45`,
  `parentXStrength: 0.18`.
* `nodeHierarchy(n)` helper derives depth +
  parentId from kind + path: non-hierarchical
  kinds (tag/mention/language) → depth -1; drive
  root → 0/null; folder/file via path-segment
  count.
* `rebuildWorkingSet` populates depth + parentId
  on both branches (mutate + fresh).
* `buildSim` replaces `forceY<DNode>(0)` with a
  depth-aware variant — hierarchical nodes
  target `depth * hierarchyYSpacing` with
  `hierarchyYStrength`; non-hierarchical keep
  `centerStrength` at y=0.
* New custom `parentXForce(strength)` factory
  added as `"parentX"` force — per-tick velocity
  push toward parent's X position. Skips
  non-hierarchical + null-parent + missing-parent
  edge cases.

`web/src/components/GraphCanvas.test.ts` (new):

* 11 raw-source pins for the wiring shape
  (DNode + FORCE config + nodeHierarchy
  branches + rebuildWorkingSet propagation +
  buildSim wiring + parentXForce shape).

### Layout strategy: (1) d3-force with depth
forces

Picked (1) over (2) hybrid d3-hierarchy + force
overlay or (3) full d3-hierarchy tree. (1) is
conservative blast radius — composes with the
existing simulation + preserves the drag /
interaction model. (2) adds a second layout
engine that has to reconcile with force
positions; (3) drops the force-based affordances
entirely. (1) keeps the existing UX intact while
adding the hierarchy backbone.

### Visual behavior

* Drive root at y=0 (depth 0).
* Top-level dirs (`docs`, `crates`, `web`) at
  y=90 (depth 1).
* `docs/journals/` at y=180.
* `docs/journals/phase-8/` at y=270.
* Files at their parent dir's depth + 1.

Architect's acceptance criterion (deep dir
below shallow dir below root) → vitest pins
lock the wiring; manual visual verification
recommended via `webtest-a-6` walk.

### Gate

* vitest **658 / 658** (+11 net).
* svelte-check 0 errors / 0 warnings across
  3990 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

Impl note + commit subject at
[fullstack-a-49.md](fullstack-a-49.md).
Outbound poke fired; standing by. Queue
waiting: `-a-50..52` (further graph overhaul)
+ G5 task (markdown-link overlay; architect to
cut as `-a-56` or `-a-57`) → `-a-42` (About).
