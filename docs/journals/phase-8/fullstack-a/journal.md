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
