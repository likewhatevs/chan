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
