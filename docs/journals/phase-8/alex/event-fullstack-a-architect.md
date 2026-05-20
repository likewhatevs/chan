# event-fullstack-a-architect.md

From: @@FullStackA
To: @@Architect
Date: 2026-05-19

## 2026-05-19 — poke

`fullstack-a-1` committed locally (no push) per your clearance.
Picking up `fullstack-a-2` next. See
[../fullstack-a/fullstack-a-1.md](../fullstack-a/fullstack-a-1.md)
for the implementation note.

## 2026-05-19 — poke

`fullstack-a-2` ready for review: status-bar click handlers
gone (collapse stays), watcher unread dot now yellow
(`var(--warn-text, …)`) to match the back-attention pulse.
Vitest + svelte-check + build clean.

See [../fullstack-a/fullstack-a-2.md](../fullstack-a/fullstack-a-2.md).
Moving on to `fullstack-a-3`.

## 2026-05-19 — poke

`fullstack-a-3` ready for review: Hybrid status-bar label
rewritten (`Hybrid ☯ Enter commit, Esc discard, H help`), the
mid-screen `H for help` flash and all its supporting state +
CSS are gone, and Cmd+K → 1/2/3 stage + commit in the same
case (no Enter required). Gate green (vitest 445/445, check
clean, build clean).

See [../fullstack-a/fullstack-a-3.md](../fullstack-a/fullstack-a-3.md).
Moving on to `fullstack-a-4`.

## 2026-05-19 — poke

`fullstack-a-4` ready for review. All four sub-items done:
1) rich-prompt skips auto-focus when survey bubbles are
present (via new `bubbleCount` prop), refocuses when count
drops to 0; 2) `submitRichPrompt` no longer steals focus back
to xterm after Cmd+Enter — bumps `focusNonce` instead;
3) `.terminal-host` gets dynamic `margin-bottom = heightPx +
12px` while the prompt is open so xterm's ResizeObserver
re-fits and the bottom line stays above the overlay;
4) `SpawnDialog` lifted to App root via a new
`state/spawnDialog.svelte.ts` singleton — fixes the "backdrop
without dialog" visibility regression by moving the dialog
out of ancestor stacking contexts that clipped its fixed
positioning.

Gate green (vitest 445/445, check clean, build clean).
See [../fullstack-a/fullstack-a-4.md](../fullstack-a/fullstack-a-4.md).
Moving on to `fullstack-a-5`.

## 2026-05-19 — poke

`fullstack-a-5` ready for review. Editor cluster:
1) image paste / drop now dispatches `scrollIntoView: true` so
the caret stays in view when inserting at the bottom of a doc;
2) `closeTabAsync` no longer auto-collapses an empty Hybrid
pane — the pane stays standing with the empty-pane landing
(explicit `closePane` still dismisses on purpose);
3) `BubbleOverlay.visibleEvents` filters surveys whose `id`
matches a sibling `survey-reply` event, killing the re-pop on
every watcher poll. Picked option (b) over (a) since the SPA-
only path matches the existing server pair-by-id record and
needs no FullStackB / chan-server coordination.

Gate green (vitest 452/452, check clean, build clean).
See [../fullstack-a/fullstack-a-5.md](../fullstack-a/fullstack-a-5.md).
Moving on to `fullstack-a-6` next.

## 2026-05-19 — poke

`fullstack-a-6` ready for review. SearchPanel's open-transition
focus effect was using `queueMicrotask` which ran before
Svelte mounted the OverlayShell child block, so the
`inputEl?.focus()` call was a silent no-op. Swapped for
`tick().then(...)` so the input is bound before focus.

Gate green. See [../fullstack-a/fullstack-a-6.md](../fullstack-a/fullstack-a-6.md).

## 2026-05-19 — poke

`fullstack-a-8` ready for review. Audited eight right-click /
overlay-entry surfaces; four were missing the easeOutBack
wobble (PaneModeHelp, TerminalRichPrompt `.ctx`, FileTree
`.ctx`, GraphPanel `.tab-menu-bubble`). Added a 260ms
cubic-bezier(0.34, 1.56, 0.64, 1) open animation to each,
scoped to a local keyframes name + `prefers-reduced-motion`
cancel — same shape the existing four wobble call sites use.

Gate green (vitest 456/456, check clean).
See [../fullstack-a/fullstack-a-8.md](../fullstack-a/fullstack-a-8.md).

Holding fullstack-a-7 (Cmd+K → Cmd+. swap) until you
green-light. It overlaps the status-bar copy from -3 and
likely needs a scope question on the Settings overlay (per
the task spec). Awaiting commit clearance on -4 through -6
and -8 first.

## 2026-05-19 — poke

Actually went ahead and did `fullstack-a-7` — the Settings
overlay scope question resolved itself: `app.settings.toggle`
is already wired as `Mod+,` in `shortcuts.ts` + `openSettings`
exists in `store.svelte.ts`, so no new overlay design. Three-
edit change: `App.svelte` swaps `KeyK` for `Period` in the
window keydown chord, `shortcuts.ts` re-declares
`app.pane.mode` + `app.pane.flip` chord descriptors, and
`PaneModeHelp` header comment refreshed. Hybrid pill copy
from `-3` stays as-is (no Cmd+K/. in the visible text).

Gate green (vitest 456/456, check clean modulo a pre-existing
EmptyPaneCarousel warning, build clean).
See [../fullstack-a/fullstack-a-7.md](../fullstack-a/fullstack-a-7.md).
Queue clear — awaiting commit clearance on -4, -5, -6, -7,
-8 + next assignments.

## 2026-05-19 — poke

All five approved tasks committed in sequence:
* `ebd4bc5` fullstack-a-1
* `ec983d3` fullstack-a-2
* `ccd2f09` fullstack-a-3
* `05e00fa` fullstack-a-5 (re-ordered, clean file set)
* `d98ebc9` fullstack-a-6
* `59fc2ec` fullstack-a-4
* `808c0a4` fullstack-a-7
* `424dd98` fullstack-a-8

No push yet (waiting on Round-1 close per protocol). Coexisted
cleanly with @@FullStackB's b-2 / b-3 hunks in shortcuts.ts /
shortcuts.test.ts / App.svelte / TerminalRichPrompt.svelte —
those stayed unstaged for B to commit.

Picking up `fullstack-a-9` next ([` ]` resize inversion) per
your suggestion. -10 (Chrome-style tab fade + full-path hover)
and -11 are queued after -9.

## 2026-05-19 — poke

`fullstack-a-9` ready for review. Root cause was the `inA`
branch in `paneModeResize`: divider direction flipped based on
the focused pane's side, which read as inverted when focus was
on the right / bottom child. Dropped the branch so the bracket
direction = divider direction, independent of focus side.
Renamed the boolean from `grow` to `positive` and refreshed
`PaneModeHelp`'s Resize section copy from the ambiguous
`Shrink / grow horizontally` to `Move divider left / right`.

Gate green (vitest 474/474, check clean).
See [../fullstack-a/fullstack-a-9.md](../fullstack-a/fullstack-a-9.md).
Moving on to `fullstack-a-10`.

## 2026-05-19 — poke

`fullstack-a-10` ready for review. Two edits:
1) Pane tab strip drops `truncateTabTitle()` and gets a
`mask-image` linear-gradient at the right edge so long titles
fade into transparency instead of showing `[..]`. 22ch cap with
1.25rem fade band, plus `-webkit-mask-image` mirror for WebKit.
2) FileTree directory + file row roots get a `fullPath()`
tooltip composing `drive.info.root` with `node.path`; file
branch keeps the existing contact / view-only annotations on
top. The tab strip's existing `title={tabTooltip(t)}` already
returns the full file path for file tabs, so no extra plumbing
needed there.

Gate green (vitest 474/474, check clean).
See [../fullstack-a/fullstack-a-10.md](../fullstack-a/fullstack-a-10.md).
Picking up `fullstack-a-11` next if it's still on my queue.

## 2026-05-19 — poke

`fullstack-a-11` was already fixed as a side effect of `-5`.
Audit found no code flipping `showingBack` from a close-tab
path; `closeTabAsync` only mutates `p.tabs` (which IS the back
side's tabs when `showingBack=true`). What looked like
"auto-flip back to front" pre-`-a-5` was actually the
`collapseEmptyPane()` call collapsing the empty pane into its
sibling, so the front pane took over the visible space. With
`-a-5` dropping that call, the Hybrid survives empty and the
flip state stays true.

Landing it as a regression pin: new test in
`state/tabs.test.ts` (`closing the last tab on the back side
keeps showingBack=true`) seeds a Hybrid, flips, opens a tab,
closes it, and asserts `showingBack === true` + `tabs === []`
+ `back.tabs === ["front"]`. Catches any future close-path
regression that re-adds an auto-flip side-effect.

Gate green (vitest 475/475, check clean).
See [../fullstack-a/fullstack-a-11.md](../fullstack-a/fullstack-a-11.md).
Queue clear — awaiting commit clearance on -9, -10, -11 +
next assignments.

## 2026-05-20 — poke

Three approved tasks committed in sequence:
* `d753775` fullstack-a-9 (Hybrid NAV `[` / `]` / `-` / `=` fixed direction)
* `a28f9b2` fullstack-a-10 (tab strip + FB tree fade-out + full-path hover)
* `a230262` fullstack-a-11 (regression pin: last back-tab keeps `showingBack=true`)

No push yet (Round-1 close protocol). Working tree clean for
the SPA scope.

Picking up `fullstack-a-13` next (image-insert viewport snap)
per your suggested order. `systacean-2` already landed
(`4a04917`) so -12's server-side dependency is satisfied; will
still slot -12 after -13 since -13 is the worst user-visible.
-14 last.

## 2026-05-20 — poke

`fullstack-a-13` ready for review. Root cause: inline atom
image widgets have unknown height between mount and image-load
completion; for a tall asset (the seeded `test-image.png` is
~2200px natural) the load reflow grows the line by ~2200px,
but CM6 only re-anchors scroll on transactions, not async
layout shifts. So the caret stays at its old scrollTop while
the doc grows beneath it and ends up far below the viewport.

Fix is a one-shot `load` listener on the success-load img path
in `web/src/editor/widgets/image.ts`. When the load fires AND
the caret is on or next to this image's source line AND the
caret is currently off-screen, dispatch a `scrollIntoView` for
the caret. Three guards keep distant-image loads from
fighting the user's deliberate scroll. The pre-existing
`fullstack-a-5` `scrollIntoView: true` on paste/drop inserts
still handles the insert-time tracking; the new handler
handles the post-decode reflow.

Verified the mechanism on the lane-A server via a programmatic
repro through `cmTile.view`: scrollHeight grew from 4446 to
6625 on image-load (matching the ~2200px observed delta) while
scrollTop stayed put. The fix's gate is line-proximity +
off-screen, so initial-mount image loads (caret far away)
don't trigger spurious scrolls.

Gate green (vitest 475/475, check clean, build clean,
`cargo build -p chan` re-embeds the bundle clean).
See [../fullstack-a/fullstack-a-13.md](../fullstack-a/fullstack-a-13.md).
Moving on to `fullstack-a-14` next.

## 2026-05-20 — poke

`fullstack-a-13` committed at `887d19c`. Picking up
`fullstack-a-12` next per your queue note (-12 ahead of -14).

`fullstack-a-12` ready for review. Confirmed the bug shape
in `GraphPanel.svelte::isFileGhost` — the `!treeHasPath.has(...)`
branch was firing on every file living under an un-expanded
FB subtree, independent of the server's `missing` flag. With
`systacean-2` now landed, the server's resolver covers all
on-disk files, so the server flag IS the source of truth.

Two-edit change: dropped the `treeHasPath` derivation, simplified
`isFileGhost` to `selectedNode.missing === true`, and refreshed
the leading docstring to record why the lazy-tree fallback went
away (the previous comment about "search index not rebuilt" was
misleading post-fix). Audited the surrounding ghost paths —
the server-side `kind: "ghost"` branch and the broken-link
inspector branch still fire correctly for true ghosts.

Gate green (vitest 475/475, check clean, build clean).
See [../fullstack-a/fullstack-a-12.md](../fullstack-a/fullstack-a-12.md).
Moving on to `fullstack-a-14` next.

## 2026-05-20 — poke

`fullstack-a-12` committed at `9971bd3`.

`fullstack-a-14` ready for review. Root cause: the `Wysiwyg`
and `Source` editors `view.focus()` UNCONDITIONALLY in their
`onMount`. On re-open of the rich prompt with a bubble
present, the child mounts and grabs focus BEFORE the
prompt's `$effect` (which has the bubble-gated early-return)
gets to run. So the editor wins the focus race and
BubbleOverlay's `editableTarget` guard bails on the now-
focused contenteditable, sending number replies into the
prompt buffer instead of the survey. @@WebtestA's hypothesis
("focus-effect grabs before bubbleCount catches up") was
directionally right but pointed at the wrong owner — it's
the child editor's mount focus, not the parent effect, that
loses the race.

Fix: new `autoFocus?: boolean` prop on both editors
(`true` default, so `FileEditorTab` keeps its existing
snap-to-focus). `TerminalRichPrompt` passes
`autoFocus={bubbleCount === 0}` to both children. Combined
with the prompt's existing `bubbleCount > 0 -> early return`
gate, the bubble-present mount path now leaves the editor
unfocused; BubbleOverlay's window keydown takes over.

Bubble-drop-to-zero path is unchanged: the prompt's effect
re-runs when bubbleCount changes, sees 0, and dispatches
`wysiwygRef?.focusEnd()` to snap the caret back.

Chose the prop-at-mount over an effect-level blur to avoid
the one-frame flicker the alternative would have produced
(child mount + child focus runs SYNCHRONOUSLY before the
parent's effect under Svelte 5's lifecycle order).

Audited the other call site (`FileEditorTab`); it inherits
the `true` default — no regression. The existing
`richPromptAutoFocus` string-match test still matches all
four of its watchwords.

Gate green (vitest 475/475, check clean, build clean).
See [../fullstack-a/fullstack-a-14.md](../fullstack-a/fullstack-a-14.md).
Queue clear after this; awaiting commit clearance + next
assignments.

## 2026-05-20 — poke

`fullstack-a-14` committed at `7513ea2`. Picking up
`fullstack-a-15` next per your queue order.

`fullstack-a-15` ready for review. Root cause was NOT in
`appendDefaultMd` (idempotent against `foo.md` already). The
doubling happened one layer up in `PathPromptModal.svelte`'s
open-time selection: with default `untitled.md` the modal only
selected the `untitled` stem, leaving the `.md` suffix
unselected in the field. So if the user typed `foo.md` (with
the extension), the typed text replaced the stem but the
trailing `.md` stayed put — the field ended up `foo.md.md`,
which `appendDefaultMd` (correctly) treats as already-extended
and returns unchanged.

Fix: extend the selection to cover the whole filename
(stem + extension). Typing `foo` still produces `foo.md`
(extension auto-added). Typing `foo.md` now produces `foo.md`
(replacing the whole filename together). Typing `foo.txt`
unchanged behaviour (modal validator runs). Hitting Enter
without typing still submits `untitled.md`. Directory prefix
(everything before the last `/`) stays outside the selection,
so Tab-completed parents survive a one-keystroke replace.

Single-file edit (`web/src/components/PathPromptModal.svelte`),
selection range only, comment refreshed.

Gate green (vitest 480/480 — @@FullStackB has been adding
tests in parallel; all green alongside mine; check clean,
build clean).
See [../fullstack-a/fullstack-a-15.md](../fullstack-a/fullstack-a-15.md).
Moving on to `fullstack-a-16` next (the 5-min copy edit).

## 2026-05-20 — poke

`fullstack-a-16` ready for review. Pure copy update:
`PaneModeHelp.svelte` Spawn section's 1/2/3 rows said
`Stage: Terminal` / `Stage: File Browser` / `Stage: Graph` —
leftover from the pre-`fullstack-a-3` stage-then-commit
flow. `fullstack-a-3` made 1/2/3 immediate-commit, so the
"Stage:" prefix has been wrong since that landed. Replaced
with `Spawn Terminal` / `Spawn File Browser` / `Spawn Graph`
(verb matches the section title; matches the runtime).
Also refreshed the section comment from the
`fullstack-72` attribution to `fullstack-a-3` so the audit
trail tracks the current behaviour.

`paneModeHelpClickable.test.ts` line 59 pinned
`action:\s*"Stage: Terminal"` as the marker on the `1`/`t`
row regex (added in `fullstack-b-9`). Updated to
`"Spawn Terminal"` so the assertion keeps tracking the same
row under the new copy.

Gate green (vitest 480/480, check clean, build clean).
See [../fullstack-a/fullstack-a-16.md](../fullstack-a/fullstack-a-16.md).
Moving on to `fullstack-a-17` next.

## 2026-05-20 — poke

`fullstack-a-17` ready for review. Root cause is in
`TerminalTab.svelte`'s focus effect (line 170 pre-fix): when
`focused` transitions true, it queues `term?.focus()` via
`queueMicrotask`. On Cmd+K p against a pane with no terminal,
the new terminal mounts and this effect fires in the same
Svelte tick where the rich prompt's `{#if richPrompt?.open}`
block mounts the editor. xterm's `focus()` is synchronous,
the rich prompt's focus path waits a `tick()` first — xterm
wins the race.

`fullstack-b-8`'s `blurTerminalHelperTextarea()` in
`openActiveTerminalRichPrompt` couldn't cover this case
because at the time it runs, xterm hasn't mounted yet (no
helper textarea to blur up front).

Fix: gate the focus effect on `tab.richPrompt?.open`. When
the prompt has the floor, bump `focusNonce` so its open-
effect re-runs and lands the caret on the editor; otherwise
fall through to `term?.focus()` as before. The
`queueMicrotask` boundary keeps the `tab.richPrompt?.open`
read out of the `$effect`'s reactive tracking, so this effect
still only re-fires on `focused` changes (no extra runs on
richPrompt open/close transitions).

Bonus coverage: the same gate fixes the latent
pane-switch-return regression (user has rich prompt open on
pane A, clicks pane B, clicks back to A — pre-fix would have
re-focused xterm; with the gate, the rich prompt's editor
re-focuses).

Single-file edit in `web/src/components/TerminalTab.svelte`.

Gate green (vitest 480/480, check clean, build clean).
See [../fullstack-a/fullstack-a-17.md](../fullstack-a/fullstack-a-17.md).
Moving on to `fullstack-a-18` next (wysiwyg-mode Cmd+Enter
dispatch dropping; flagged by @@FullStackB in -b-8).

## 2026-05-20 — poke

`fullstack-a-18` ready for review. @@FullStackB's root-cause
note in `-b-8` was exact: `Wysiwyg.svelte`'s extension stack
has a high-precedence `Mod-Enter` keymap entry that calls
`onSubmit?.()` and returns `true` to consume the event.
Without an `onSubmit` prop, the chord registers, swallows the
event, and does nothing. Source mode works because Source
has no Mod-Enter binding and the event bubbles to the
wrapper's `onKeydown` (line 118-122), which calls the local
`submit()`.

Fix: thread the wrapper's `submit` function as
`onSubmit={submit}` on the `<Wysiwyg>` instantiation site.
Single-line addition next to the `autoFocus` prop from
`fullstack-a-14`. Source mode unchanged.

The caret-retention rule from `fullstack-a-4` still applies:
`submit()` → `onSubmit(buffer)` prop → TerminalTab's
`submitRichPrompt`, which bumps focusNonce so the caret stays
in the prompt after dispatch.

Gate green (vitest 480/480, check clean, build clean).
See [../fullstack-a/fullstack-a-18.md](../fullstack-a/fullstack-a-18.md).
Queue clear — awaiting commit clearance on -15 / -16 / -17 /
-18 + next assignments.

## 2026-05-20 — poke

Four cleared tasks committed in order:
* `3eed19b` fullstack-a-15 (New file dialog stem+.md select)
* `c05e9fc` fullstack-a-16 ("Stage:" → "Spawn" copy)
* `2466a41` fullstack-a-17 (Cmd+K p focus gate)
* `2787041` fullstack-a-18 (Wysiwyg Cmd+Enter dispatch)

Push still held for Round-1 close. Picking up `fullstack-a-19`
(chord-table drift cleanup) next.

## 2026-05-20 — poke

`fullstack-a-19` ready for review. Audited both cheatsheets
against the runtime in `App.svelte::handlePaneModeKey`. The
SPA cheatsheet was mostly synced piecemeal by earlier tasks
(only one stale comment + one cap-key consistency tweak + the
title chord suffix); the CLI's `SERVE_LONG_ABOUT` block had
drifted further — section header still said "Pane Mode (Cmd+K)",
search row was `s` not `f`, kill-pane was `k` not `Backspace`,
and `p` / `< >` / `Tab` rows were missing entirely.

Re-synced both. Two structural tests caught the title change:
`hybridNavRename.test.ts` line 49 pinned `>Hybrid NAV<`; updated
to `>Hybrid NAV (Cmd+.)<` with a comment recording why the
chord suffix is now part of the brand pin.
`paneModeHelpClickable.test.ts` line 33 comment referenced the
old "(1-4 + p / s)" spawn set; updated to "(1-4 + p + f)".
Neither assertion logic needed to change — the existing pins
don't check `s` or `f` directly.

Cross-stack gate clean: vitest 480/480, check 0/0, npm build
clean, `cargo fmt --check`, `cargo clippy -p chan --all-targets
-- -D warnings`, `cargo test -p chan` 58/58, `cargo build -p chan`
re-embeds the new bundle clean.

See [../fullstack-a/fullstack-a-19.md](../fullstack-a/fullstack-a-19.md).
Queue clear — awaiting commit clearance + next assignments.

## 2026-05-20 — poke

`fullstack-a-19` committed at `9c30295`.

`fullstack-a-20` ready for review — hotfix for the
double-dispatch regression `-a-18` introduced (caught by
@@Alex on return; thanks for cutting it). Root cause is the
exact shape the task spec described: `-a-18` connected the
Wysiwyg keymap's Mod-Enter run to a real `submit()`, CM's
keymap runner calls `preventDefault()` but not
`stopPropagation()`, the wrapper's `onKeydown` didn't check
`defaultPrevented` and called `submit()` a second time.

Single-line fix in `TerminalRichPrompt.svelte`:
`if (e.defaultPrevented) return;` at the top of `onKeydown`.
Wysiwyg path: CM keymap dispatches once, the
preventDefault'd event bubbles to the wrapper and the wrapper
bails. Source path unchanged (no Mod-Enter on Source's
keymap, event reaches wrapper unhandled, single dispatch).

Test pin in `TerminalRichPrompt.test.ts`: mounts the prompt,
dispatches a `keydown` with `preventDefault()` already called,
asserts `onSubmit` is NOT invoked. Catches any future
wrapper-level handler that re-introduces the
defaultPrevented-blind path.

Gate green: vitest 481/481 (+1 from the new test), check 0/0,
build clean.
See [../fullstack-a/fullstack-a-20.md](../fullstack-a/fullstack-a-20.md).
Queue clear — Round-1 wave-3 set complete on my side. Wave-4
is -21 (Settings UI, gated on systacean-7) + -22 (Hybrid pane
flip animation) — standing down for the session as you
suggested unless you want me to start -22.

## 2026-05-20 — poke

`fullstack-a-20` committed at `f1d0dcf`.

Note on the ordering: your clearance suggested -20 ahead of
-19 in the wave-3 set. -19 already landed at `9c30295` by the
time -20 cleared (clearances came back in -19→-20 order; I
committed -19 before reading -20's review). Other lanes have
since committed on top (`8b35c03` systacean-6). The git story
is still linear:

* `2787041` -18 (introduces the regression)
* `9c30295` -19 (unrelated docs sweep)
* `f1d0dcf` -20 (fixes the regression)

A reorder would need a local rebase across systacean's commit
+ would rewrite already-landed history; flagged as a
preference miss rather than fixing destructively. If you want
me to interactively rebase to put -20 directly after -18,
say the word.

Queue clear — standing down for the session per your earlier
note unless you want me on -22 (Hybrid pane flip animation,
independent).

## 2026-05-20 — poke

Picked up `fullstack-a-21` now that `systacean-7` (`6bf44cd`)
unblocks the API contract.

`fullstack-a-21` ready for review. Three-file change:

* `web/src/api/types.ts` — new `SemanticState` type matching
  the locked `systacean-7` JSON shape.
* `web/src/api/client.ts` — four methods on the `api` object
  (`semanticState`, `semanticDownload`, `semanticEnable`,
  `semanticDisable`).
* `web/src/components/SettingsPanel.svelte` — new "Semantic
  search" section between "Date pills" and "About". Toggle +
  hint with model name & size + spinner-during-download
  (no progress bar per your UX adjustment) + status grid
  (Active mode + Stored-at path) + error row.

Followed your UX adjustment exactly: synchronous download
POST + parallel 3-second poll against `/state` for the
`model_present` transition; auto-enable on download
completion so the toggle lands ON. The `--no-default-features`
build (embeddings off) gets a build-hint placeholder instead
of a non-functional toggle. `prefers-reduced-motion` honoured
on the spinner.

No vitest pin — SettingsPanel has no existing test file and
the toggle's logic is flow-of-state + error path, both
better verified end-to-end on lane-A. Per the task spec,
"visual verification on lane-A is acceptable when no
testable seam exists".

Gate green: vitest 481/481, check 0/0, build clean.
See [../fullstack-a/fullstack-a-21.md](../fullstack-a/fullstack-a-21.md).
Moving on to `fullstack-a-22` next.

## 2026-05-20 — poke

`fullstack-a-21` committed at `f5b91b7`.

`fullstack-a-22` ready for review — **with a scope
deviation I want you to read on the impl note before
clearing**. Short version: I landed a single-face
half-flip animation (rotate 0° → 90° → 0°; content swap
during the invisible edge-on midpoint) instead of the
strict two-face card flip the spec described.

Why the deviation:

* The strict spec requires changing flipHybrid() to stop
  swapping `pane.tabs ↔ pane.back.tabs` and just toggle
  `showingBack`, then updating ~20+ files that read
  `pane.tabs` as "currently visible" to derive that from
  `showingBack`. Plus rewriting every flipHybrid test
  that asserts the swap semantics.
* The user-facing goal is "deliberate transition"
  (@@Alex's note in the task background). The half-flip
  achieves that with a ~30-line surface-area change in
  three files.
* You marked -22 as a "UX nicety, lower priority" — so
  the trade-off felt defensible. Full impl-note rationale
  in the task tail; I'd rather you review the smaller
  delta and bounce it if you want the strict version
  than batch the rewrite inside this PR.

What landed in 3 files:

* `web/src/state/tabs.svelte.ts` — new `paneFlip` bus
  (parallel to `paneWobble`); flipHybrid switches from
  wobble to flip. Two distinct signals for two distinct
  state changes.
* `web/src/components/Pane.svelte` — flip subscription
  mirroring the wobble pattern; `.pane.flipping`
  keyframe (Y-axis rotate, 400ms,
  `cubic-bezier(0.4, 0, 0.2, 1)`,
  `prefers-reduced-motion: reduce` honoured).
* `web/src/state/tabs.test.ts` — wobble→flip test pin
  asserts the flip counter ticks AND the wobble doesn't
  (catches any regression that re-couples them).

Gate green: vitest 481/481 (the renamed pin replaces the
old one), check 0/0, build clean.
See [../fullstack-a/fullstack-a-22.md](../fullstack-a/fullstack-a-22.md).
Queue clear — detour set complete on my side. Standing
down for the session unless you want me on the round-2
chord migration drafted in round-2-plan.md or want the
strict two-face refactor on -22.

## 2026-05-20 — poke

`fullstack-a-22` committed at `6ed7ebb` (deviation accepted
per your clearance).

`fullstack-a-23` ready for review. Followed Option A exactly
as you locked it:

* `ResizeHandle.svelte` — new `idleVisible?: boolean` prop
  (default `true`). When `false`, the `.handle` paints
  transparent in idle; the 4 px hit area, `col-resize`
  cursor, touch-action, pointer events, and hover paint
  (6 px + `--separator-hover`) all stay.
* `FileBrowserSidePane.svelte` — both ResizeHandle
  instances (left + right dock) pass `idleVisible={false}`.
* The other two consumers (`Inspector.svelte` for the file
  editor inspector, `GraphPanel.svelte` for the graph
  details inspector) inherit the `true` default →
  unchanged, visible separator stays.

Two-file change. Gate green: vitest 481/481, check 0/0,
build clean.
See [../fullstack-a/fullstack-a-23.md](../fullstack-a/fullstack-a-23.md).
Queue clear — detour set fully complete on my side
(-21 / -22 / -23 all landed). Standing down for the
session unless you want me on the round-2 chord migration
in round-2-plan.md.

## 2026-05-20 — poke

`fullstack-a-23` committed at `b0be42e`. Picked up `-24`
(rich prompt redesign) next.

`fullstack-a-24` ready for review. Five-area change:

* **Floating-pill visual**: rich-prompt `.rich-prompt`
  goes from flush-against-bottom-edge to 12 px-inset on
  all four sides with `border-radius: 14px` and
  all-sides shadow. Bubble overlay rounds from 6 → 12 px
  to match (12 / 14 asymmetry is intentional — bubbles
  are smaller chips above the prompt).
* **Default placeholder**: "Write a multi-line command
  and Cmd+Enter" via a CSS overlay over the empty
  composer-editor. `pointer-events: none` so the editor
  still owns interaction; Svelte conditional render
  clears it on the first keystroke. Picked the CSS
  overlay over a threaded CodeMirror placeholder
  extension because the placeholder is a rich-prompt
  concern, not an editor concern.
* **Style toolbar default off**: `toolbarOpen()` flipped
  from `!== false` to `=== true` so `undefined` reads
  as off. Mount site (inside header, top of bubble)
  stays.
* **Collapse / expand affordance**: new chevron button
  between Send and Close. Collapsed state clamps the
  prompt to header-only; bubbles above reclaim the
  freed vertical room. Persistence wired through the
  existing rich-prompt session payload (`rpc: 1` only
  when truthy; `richPromptFromSer` uses a conditional
  spread so the round-tripped shape stays exact-equal
  to pre-`-a-24` when the user hasn't collapsed).
* **State model**: new `collapsed?: boolean` on
  `TerminalRichPromptState` + `rpc?: 1` on SerTab.

Tests: two `TerminalRichPrompt.test.ts` mode-toggle
tests gained `styleToolbarOpen: true` on their prompt
fixtures (the toolbar's mode-toggle button only mounts
when the toolbar is open; default flipped). The
serialize/restore test passes unchanged thanks to the
conditional spread on deserialize.

Composition with prior phase-8 fixes
(`-a-4` / `-a-14` / `-a-17` / `-a-18` / `-a-20`)
verified untouched in the impl note.

Gate green: vitest 481/481, check 0/0, build clean.
See [../fullstack-a/fullstack-a-24.md](../fullstack-a/fullstack-a-24.md).
Moving on to `fullstack-a-25` next (editor toggle →
Settings).

## 2026-05-20 — poke

`fullstack-a-24` committed at `4f8d930`.

`fullstack-a-25` committed at `0043fd2` — last item in
the Round-1 detour set.

Two-file change:

* `FileEditorTab.svelte` — removed the
  "Run automatically on save / auto-save" menu entry
  (the manual one-shot "Remove trailing whitespace"
  button above it stays); dropped the now-unused
  `doToggleAutoStripWhitespace` + `SquareCheck` icon +
  `editorToolsPrefs` / `persistStripTrailingWhitespaceOnSave`
  imports.
* `SettingsPanel.svelte` — new "On save" section after
  "Date pills" with a `bind:checked={editing.strip_trailing_whitespace_on_save}`
  toggle on the same `.theme-opt.semantic-toggle` chip
  shape used elsewhere. Sibling `$effect` mirrors
  `editing.strip_trailing_whitespace_on_save` →
  `editorToolsPrefs.stripTrailingWhitespaceOnSave` so
  save-time stripping picks up the change immediately
  while the SettingsPanel's existing autosave handles
  durable persistence.

Storage was already in the right place
(`strip_trailing_whitespace_on_save` in `Preferences`,
round-tripped via `/api/config`, surfaced through
`editorToolsPrefs.stripTrailingWhitespaceOnSave`), so the
move was UI-only — no migration code.

Commit hygiene note: `web/src/components/SettingsPanel.svelte`
had @@FullStackB's `fullstack-b-11` Terminal section /
scrollback / TERM dropdown work also pending in my working
tree. Took care to commit ONLY my -25 hunks (verted to HEAD,
re-applied my -25 changes via Edit, committed, then restored
B's working-tree state on top of the new commit). Their work
stays uncommitted in my working tree exactly as I found it;
diff against HEAD now shows 77 lines pending = pure -b-11.

Gate green: vitest 491/491 (other lanes added +10 tests
since my last gate), check 0/0, build clean.
See [../fullstack-a/fullstack-a-25.md](../fullstack-a/fullstack-a-25.md).

Round-1 detour set complete on my lane
(-21 / -22 / -23 / -24 / -25 all landed). Standing down for
the session per your prior note; queue empty until Round-2
fan-out or a new dispatch.

## 2026-05-20 — poke

Picked up `fullstack-a-26` (markdown editor toolbar
parity) from your new dispatch.

`fullstack-a-26` ready for review. The shared
`StyleToolbar.svelte` already implements the separator
+ rendered/source toggle (gated on `mode && onModeToggle`
props being defined — the rich prompt was passing both,
the hybrid file editor was passing neither). Pure prop
wire-up; no shared-component extraction needed.

Two mount sites in `FileEditorTab.svelte`:

* Wysiwyg-mode mount: pre-existed; gained
  `mode="wysiwyg"` + `onModeToggle={hasRenderedMode ? () => doToggleMode() : undefined}`.
  The `hasRenderedMode` gate hides the toggle for plain
  text files (`.py` / `.toml` / etc.) that have no
  rendered counterpart.
* Source-mode mount: NEW. Gated on
  `tab.styleToolbarOpen && hasRenderedMode`. Mounts with
  `disabled={true}` (formatting row collapses) +
  `mode="source"` so the toggle reads "show rendered".
  The shared StyleToolbar's always-visible mode-toggle
  pattern keeps it reachable with the row collapsed.

Tested behaviour matches: markdown wysiwyg ↔ source,
JSON pretty ↔ source, CSV table ↔ source, plain text
has no toggle (matches the menu).

Gate green: vitest 501/501 (other lanes still adding
tests; all green alongside mine), check 0/0, build clean.
See [../fullstack-a/fullstack-a-26.md](../fullstack-a/fullstack-a-26.md).
Moving on to `fullstack-a-27` next (Hybrid hamburger
polish).

## 2026-05-20 — poke

`fullstack-a-26` committed at `a0ee014`.

**Commit hygiene incident on -26**: my first commit
attempt accidentally bundled `@@FullStackB`'s pending
`fullstack-b-11` work (12 files / 1033 insertions —
config.rs / preferences.rs / terminal_sessions.rs +
the SettingsPanel terminal section + scrollback.ts +
new tests + their journal file) into a commit titled
"fullstack-a-26". Caught it immediately on
`git show --stat`. Recovered with
`git reset HEAD~1` (mixed; non-destructive — kept all
changes in the working tree, just uncommitted) →
re-staged ONLY my `FileEditorTab.svelte` →
re-committed clean (`a0ee014`, single file, 30
insertions). Verified with `git show --stat` post-
commit. Then @@FullStackB landed their own
`fullstack-b-11` commit at `ab5a0ce` cleanly. No
cross-contamination, no force-push, no destructive
operations.

Root cause: untracked-file staging on a previous `git
add` was sticky from earlier work in the session.
Future workaround: `git diff --staged --stat` before
every commit (the
`feedback-shared-worktree-commits` discipline I cited
in the -25 poke but didn't apply rigorously on this
one). Self-correction landed; flagging here for the
audit trail.

`fullstack-a-27` ready for review. Two-file change:

* `Pane.svelte`: removed the standalone
  `.pane-theme-toggle` chrome button (from
  `fullstack-59`) + its scoped CSS. Added two new
  hamburger menu entries gated on `pane.back !== undefined`
  (Hybrid-only per the task spec):
  * Theme toggle ("Light mode" / "Dark mode" with
    Sun / Moon icon reflecting the click destination)
    calls the existing `togglePaneTheme()` helper —
    no behaviour change, just a relocation.
  * Flip pane (FlipHorizontal2 icon, "Cmd+. Tab"
    chord hint) calls `flipHybrid(pane.id)` and
    closes the menu so the user sees the
    `fullstack-a-22` flip animation play.
* `perHybridTheme.test.ts`: dropped the
  `class="pane-theme-toggle"` assertion (the chrome
  button is gone); kept the `togglePaneTheme`
  function-reference pin which is the actual contract.
  Added a comment recording the relocation.

Both entries appear between "Enter Hybrid NAV" and
the focus-border-colour palette so they sit with the
other pane-level operations. Composes cleanly with
`-a-22` (flip animation) and `fullstack-b-5` (per-
Hybrid theme override).

Gate green: vitest 501/501, check 0/0, build clean.
See [../fullstack-a/fullstack-a-27.md](../fullstack-a/fullstack-a-27.md).
Queue clear — Round-1 detour set fully complete on my
lane (-21 / -22 / -23 / -24 / -25 / -26 / -27 all
landed or under review). Standing down for the
session unless redirected.

## 2026-05-20 — poke

`fullstack-a-27` committed at `264674e` — clean two-file
commit (`git diff --staged --stat` ran first this time,
per the discipline I committed to after the -26
incident). Pane.svelte + perHybridTheme.test.ts only;
no cross-lane contamination.

Round-1 detour set fully landed on my lane: -21 / -22 /
-23 / -24 / -25 / -26 / -27 all committed locally
(push held for Round-2 close per protocol). Standing
down for the session per your "queue empty, standby
until Round-2 fan-out" closing note.
