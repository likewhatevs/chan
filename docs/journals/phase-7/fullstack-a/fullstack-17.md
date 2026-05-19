# fullstack-17: polish bundle (rename-restart prompt + cosmetic carry-overs)

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Small polish bundle folded from carry-over items. Three
separate UX nits that don't deserve their own tasks but
do deserve attention before they pile up.

## Acceptance criteria

### Rename-restart prompt

* When the user renames a terminal and presses Enter,
  don't close the edit silently. Immediately offer to
  restart in-place ("restart now?" affordance — small
  inline button or a confirm dialog).
* The existing out-of-sync indicator stays as a
  secondary affordance for users who dismiss the prompt
  but want to come back to it.
* The restart confirmation prompt (already requested in
  E4) still fires on actual restart click since the
  session gets reset.

Spec source: [../request.md](../request.md) — E4 sub-
bullet "Addendum (2026-05-18 21:30 BST)".

### Light-mode terminal contrast polish

* `\e[37m` "white" in light mode is currently
  `rgb(110, 119, 129)` — readable but right at the
  AA-large threshold. Bump to a slightly darker gray
  that clears AA cleanly (target ≥ 4.5:1 against white).
* `\e[97m` "bright white" in light mode currently
  collapses to the same color as `\e[30m` "black"
  (`rgb(36, 41, 47)`). Pick a distinct lighter shade so
  bright-white is visibly different from regular black.
* Dark mode untouched.

Spec source: @@WebtestB's `fullstack-7` PASS verdict +
@@WebtestA's wave-2b note on `\e[37m` borderline.

### Watch directory dialog: accept absolute paths

* The current Watch directory dialog (added in `fullstack-13`)
  rejects absolute paths with `× absolute paths are not
  allowed`. `systacean-9`'s API spec accepts both drive-
  relative and absolute paths. Loosen the dialog to match —
  external watch dirs are valid use cases.

Spec source: @@WebtestA's `webtest-a-6` item-7 minor side
observation.

### Drop unknown-type events in SPA

* The chan-server watcher logs + ignores unknown event
  types per `systacean-9` spec. The SPA, however, still
  renders a bubble showing the raw type name (e.g.
  `futuristic-thing from @@TestAgent`). Match backend
  behavior: silently drop unknown types in the SPA reader.

Spec source: @@WebtestA's `webtest-a-6` second minor side
observation.

### Watcher staleness on session reload

* On session reload the SPA still shows the "Stop
  watching" affordance, but the server-side watcher has
  already dropped. First reply attempt fails with
  `watcher is no longer attached` (409). Either:
  * Server-side: auto-reattach the watcher when a
    session resumes with a known watch dir (preferred,
    matches "watcher follows the terminal" mental
    model). Belongs in chan-server if you take this
    path — coordinate with @@Systacean.
  * SPA-side: on `409` from any event-reply call,
    clear the local watcher state, show "Watch
    directory" affordance again, and surface a brief
    "watcher detached on reload" hint.

Pick whichever lands cleaner; lean SPA-side if you want
this in the polish task alone.

Wider repro per @@WebtestB's `webtest-b-4` follow-up: the
bug also fires on **URL-hash navigation between terminal
tabs** (e.g. hash moves from `B19v3` to `@@BubbleTab`)
without explicit "Stop watching". SPA persists
`watching events` + the Stop affordance; server has no
watcher for the new session id; all reply POSTs 409.
"Stop watching" itself also fails (`terminal watcher not
found`). So the fix needs to handle: (a) reload of the
same session, (b) navigation to a different session.
SPA-side detection on 409-from-any-reply + clearing
local state is the simplest seam that covers both.

Spec source: @@WebtestA's `webtest-a-6` item-12 follow-
up minor + @@WebtestB's `webtest-b-4` follow-up.

### Answered survey bubbles auto-dismiss

* Once a survey is fully answered (single-topic on click
  or multi-topic on last-tab-answered) the bubble should
  auto-dismiss after a brief delay (e.g. 600ms — long
  enough to see the answered-state `*` confirmation,
  short enough to not clutter).
* Skip / Esc dismissal stays immediate.

Spec source: @@WebtestA's `webtest-a-6` second follow-up
minor.

### Hamburger ↔ right-click menu auto-dismiss

* Opening the pane hamburger menu while the pane right-
  click menu is open should auto-dismiss the right-
  click menu (and vice versa). Currently both render
  simultaneously.
* Pressing Esc should dismiss whichever menu is open;
  if both are open (today's state), Esc dismisses both
  cleanly.

Spec source: @@WebtestA's `webtest-a-5` wave-2b note
("Minor side observation: opening the hamburger menu
while a prior pane-right-click menu is open does not
auto-dismiss the other").

## Out of scope

* Bigger menu-system refactors.
* Theme variants beyond fixing the two ANSI nits.

## How to start

* Rename prompt: extend the terminal-name edit completion
  handler with the restart affordance. Place the prompt
  inline near the name input or as a small follow-up
  dialog.
* Light-mode contrast: the palette table from
  `fullstack-7` is in `web/src/styles/` somewhere. Adjust
  the two specific slot values; rerun @@WebtestA/B's
  computed-color checks.
* Menu auto-dismiss: shared `closeAllMenus()` helper or
  a single global menu-state store; opening any menu
  closes the others.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-architect.md`.

## 2026-05-19 04:36 BST — implementation note

Implemented the `fullstack-17` polish bundle.

Changed behavior:
* Terminal rename Enter now keeps the tab menu open and reveals the existing restart-now affordance instead of silently closing the edit; live-session restart now goes through the in-app confirmation modal.
* Light terminal ANSI `white` is darker for AA contrast, and `brightWhite` is distinct from regular black.
* Watch directory path prompt opts into absolute paths, matching the terminal watcher API.
* SPA watcher event parsing now drops unknown event types, matching chan-server.
* Survey replies keep the answered state visible for 600ms before auto-dismiss; skip/Esc dismissal stays immediate.
* Reply/stop 409-style stale watcher errors clear local watcher state and surface a short reload-detached hint.
* Pane hamburger and pane right-click menus now dismiss each other; Escape closes any open pane menu.

Verification so far:
* `npm run test -- BubbleOverlay TerminalRichPrompt watcherEvents pathValidate`
* `npm run check`
* `npm run build`

## 2026-05-19 04:37 BST — commit readiness

Ready to land.

Changed files:
* web/src/components/BubbleOverlay.svelte
* web/src/components/BubbleOverlay.test.ts
* web/src/components/HamburgerMenu.svelte
* web/src/components/Pane.svelte
* web/src/components/PathPromptModal.svelte
* web/src/components/TerminalRichPrompt.svelte
* web/src/components/TerminalRichPrompt.test.ts
* web/src/components/TerminalTab.svelte
* web/src/state/pathValidate.ts
* web/src/state/pathValidate.test.ts
* web/src/state/store.svelte.ts
* web/src/state/watcherEvents.ts
* web/src/state/watcherEvents.test.ts
* docs/journals/phase-7/fullstack-a/fullstack-17.md

Verification:
* `npm run test -- BubbleOverlay TerminalRichPrompt watcherEvents pathValidate`
* `npm run check`
* `npm run build`
* `scripts/pre-push`

Known risk: watcher-detached recovery is SPA-side only; server-side auto-reattach remains a future improvement if Alex wants watcher state to survive session id churn transparently.

Proposed commit: Polish watcher and terminal UX (fullstack-17).
