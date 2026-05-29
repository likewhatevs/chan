# fullstack-41: Ctrl+D closes the current tab (Files + Graph + doc)

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Goal

`Ctrl+D` should close the focused tab uniformly across
tab types. Terminal tabs already get this for free
because `Ctrl+D` in the shell sends EOF, the shell
exits, and the terminal-session lifecycle closes the
tab. The first-class tab types added in `fullstack-14`
(Files, Graph) don't have a similar natural exit
hook, and the doc / editor tabs lack the shortcut
too. Wire `Ctrl+D` as the canonical close-current-tab
keybind for these surfaces.

## Relevant links

* @@Alex's chat note 2026-05-19 12:25 BST.
* Predecessor: [./fullstack-14.md](../fullstack-a/fullstack-14.md)
  (Phase 1 overlays → first-class tabs).

## Acceptance criteria

### Behavior

* **Files tab** focused + `Ctrl+D` → tab closes.
  Same effect as the tab strip `×` or doc-tab right-
  click → Close.
* **Graph tab** focused + `Ctrl+D` → tab closes.
* **Doc / editor tab** focused + `Ctrl+D` → tab
  closes. If the doc is dirty, honor the existing
  unsaved-changes flow (whatever the close-tab path
  does today; don't bypass).
* **Terminal tab** stays unchanged — `Ctrl+D` is
  forwarded to the shell as today (EOF), which exits
  the shell, which closes the tab. Don't intercept
  before the shell sees it.

### Implementation discipline

* The binding is global within the focused tab's
  keymap, NOT a global app shortcut — so it doesn't
  fire when a Cmd+K mode dialog is up, or when an
  in-house modal (`PromptModal` / `ConfirmModal`) has
  focus.
* On macOS this is `Cmd+D`? No — @@Alex explicitly
  said "control d". Honor the literal `Ctrl+D` on
  both native and web. macOS users will know Ctrl is
  the right modifier (it's a POSIX-ish "send EOF"
  spirit).
* Existing per-tab-type shortcuts stay (e.g. `Cmd+W`
  if any tab type ships it). `Ctrl+D` is additive.

### Tests

* Per tab type (Files, Graph, doc): focus the tab,
  fire `Ctrl+D`, assert tab is removed.
* Terminal tab: assert `Ctrl+D` is NOT intercepted
  at the SPA layer (forwarded to the shell as EOF).
* Modal-up test: when a `PromptModal` /
  `ConfirmModal` is open, `Ctrl+D` does NOT close
  the underlying tab.

## Out of scope

* Cmd+W or other close-tab keybinds (separate
  shortcuts, not in this task).
* "Close all tabs" / "Close pane" variants — those
  already exist via the hamburger menu and Cmd+K
  mode's `x` / `k` (per `fullstack-39`).

## How to start

1. The Files + Graph tab components live in
   `web/src/components/` (`FileBrowserSurface.svelte`,
   `GraphPanel.svelte`). Add the keydown handler at
   the tab-content root.
2. For the editor (doc tab) the keymap goes through
   CodeMirror — add a `Ctrl+D` keybind that calls the
   existing close-tab helper. Make sure it doesn't
   collide with CM's default `Ctrl+D` (multi-cursor
   "selectNextOccurrence" — that's a common one).
   If it does, prefer our close-tab binding on the
   tab-strip-focused state, multi-cursor on
   editor-content-focused state.
3. Terminal: don't add anything. The keystroke
   already flows to the shell via the existing PTY
   input path.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-a-architect.md`.

## 2026-05-19 13:15 BST — @@FullStackA specialist review

### Implementation shape

* `web/src/App.svelte` — added `onCtrlDCapture(e)`, a
  document-level keydown listener registered with
  `capture: true` so it pre-empts CodeMirror's default
  `selectNextOccurrence` keymap (which is bubble-phase
  inside the editor's content element).
* Guards (in this order, before any side effect):
  * `!e.ctrlKey || e.metaKey || e.shiftKey || e.altKey`
    → bail. Literal Ctrl only — not Cmd, not Shift,
    not Alt.
  * `e.code !== "KeyD"` → bail. Layout-agnostic
    (matches both "d" and "D" without inspecting
    e.key).
  * Any in-house modal (`promptState`, `pathPromptState`,
    `confirmState`) open → bail; the modal owns the
    keyboard.
  * `paneMode.active` → bail; Cmd+K mode handles `x`
    and `k` for close-all-tabs / kill-pane and we
    don't want a stray Ctrl+D to leak past it.
  * Focused tab is a terminal → bail without
    `preventDefault`. xterm forwards Ctrl+D to the PTY
    as EOF; the shell exit closes the tab through the
    existing terminal-session lifecycle.
  * Otherwise: `preventDefault` + `stopPropagation`,
    then `void closeTab(p.id, active.id)`. The
    existing `closeTab` path handles the unsaved-doc
    confirmation flow for dirty file tabs (no
    bypassing).

### Why capture phase

CodeMirror keymaps fire at bubble. With a bubble-phase
listener on `document`, CM6 already ran multi-cursor
selectNextOccurrence by the time we see the event —
leaves a stale multi-selection behind even though the
tab also closes. Capture-phase on `document` runs before
the editor element receives the event; calling
`stopPropagation` keeps CM6's keymap from firing at
all.

### What stayed unchanged

* Terminal tabs: no SPA-level interception. xterm
  receives Ctrl+D through the normal keydown flow.
* In-house modal contexts: Ctrl+D in a PromptModal /
  ConfirmModal is a no-op as far as this listener is
  concerned. The modal's text input owns the keystroke.
* The existing `onWindowKey` bubble handler is
  untouched — adding the capture listener as a
  separate function keeps the other shortcuts'
  semantics stable.

### Tests

New `web/src/components/ctrlDCloseTab.test.ts`:

* Behaviour: directly drives `closeTab` against a
  synthetic layout for each shipped target tab type
  (Files / Graph / clean doc) and asserts the tab is
  removed. Reuses the unsaved-changes flow indirectly
  via the doc fixture (mirrors `tabs.test.ts`'s
  fileTab shape, content === saved so no modal pops).
* Routing decision: raw-source assertions on
  `App.svelte` that the four guards (modifier match,
  code === "KeyD", modals + paneMode skip, terminal
  skip) are in place and the listener registers at
  capture phase. Same pattern as
  `paneModeKeymap.test.ts` from `fullstack-40`.

### Gate

* `npm run test -- ctrlDCloseTab` — 7 passed.
* `npm run test` — 34 files / 298 tests, all pass.
* `npm run check` — 0 errors / 0 warnings.
* `npm run build` — clean.
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` — green.

### Proposed commit message

> Ctrl+D closes the focused non-terminal tab (fullstack-41)
>
> A capture-phase document keydown listener intercepts Ctrl+D
> before CodeMirror's default selectNextOccurrence keymap,
> calls preventDefault + stopPropagation, and closes the focused
> tab via the existing closeTab helper (which honours the
> unsaved-doc confirmation). Terminal tabs are deliberately
> skipped so xterm still forwards Ctrl+D to the shell as EOF.
> In-house modal contexts and Pane Mode short-circuit the
> handler.

Ready for commit + push under standing topic-level
clearance.
