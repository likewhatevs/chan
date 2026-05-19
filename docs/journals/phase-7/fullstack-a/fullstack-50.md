# fullstack-50: Cmd+K p — show/spawn rich prompt + close button + menu cleanup

Owner: @@FullStackA
Cut by: @@Architect
Date: 2026-05-19

## Goal

Add `p` as a Pane Mode keybind that shows the rich
prompt. Behavior depends on whether a terminal exists:

* If the focused pane contains a terminal tab → show
  the rich prompt on that terminal (focus it +
  reveal the prompt UI if hidden).
* If the focused pane has a terminal that's NOT
  focused → focus that terminal first, then show
  the rich prompt on it.
* If there's no terminal in the focused pane → create
  a new terminal tab in the focused pane, with the
  rich prompt enabled on first show.

Additionally:

* The rich prompt grows a **close button** (`×`) so
  the user can dismiss it cleanly.
* Drop "Show rich prompt" / "Toggle rich prompt" /
  similar items from any menu where they exist
  (right-click menus, hamburger, etc.). `Cmd+K p` is
  the canonical entry; the close button is the exit.

## Relevant links

* @@Alex's chat note 2026-05-19 14:00 BST.
* Rich prompt component lives at
  `web/src/components/TerminalRichPrompt.svelte`.
* Predecessor (Pane Mode keymap surface):
  [./fullstack-42.md](../fullstack-a/fullstack-42.md).

## Acceptance criteria

### `p` binding behavior

* Inside Pane Mode + focused pane is empty → commits
  draft, creates a new Terminal tab in that pane,
  shows the rich prompt on it.
* Inside Pane Mode + focused pane has a Terminal
  tab (active or not) → commits draft, focuses the
  Terminal, shows the rich prompt.
* Inside Pane Mode + focused pane has tabs but no
  Terminal → commits draft, creates a new Terminal
  tab in that pane, shows the rich prompt.
* "Show the rich prompt" = if hidden, reveal it; if
  already visible, focus it.
* Outside Pane Mode: no change (Cmd+K p only works
  inside the mode).

### Close button on the rich prompt

* The rich prompt header gains a small `×` close
  affordance (matches the bubble overlay's existing
  visual chrome — small, subtle).
* Click closes the rich prompt (same effect as the
  current "hide" path — bubbles + survey stay
  alive on disk, just the UI hides).
* Esc on the rich prompt input area should also
  close (or already does — verify).

### Menu cleanup

* Audit every menu (right-click, hamburger, tab
  right-click) for items like "Show rich prompt",
  "Toggle rich prompt", "Open rich prompt", "Rich
  prompt visibility", etc.
* Drop all of them. `Cmd+K p` is the canonical
  entry; the close button is the exit.
* Rich prompt's OWN context menu (toggle source,
  toggle style toolbar, watcher controls — added
  in `fullstack-17` polish) stays inside the rich
  prompt itself. Those control the prompt's
  contents, not its visibility.

### Update Pane Mode help cheatsheet

* If `fullstack-42`'s help overlay has shipped by
  the time you land this, add the new `p` binding
  to the Spawn group of the cheatsheet.

## Out of scope

* Reworking the rich prompt's bubble overlay (lives
  in `fullstack-13/18/23/24/48`).
* Per-terminal rich-prompt visibility memory
  beyond what already exists.

## How to start

1. `web/src/App.svelte:handlePaneModeKey` — new
   case for `p`. Resolve focused pane, find
   terminal tab (or spawn one), commit the draft,
   reveal + focus the rich prompt.
2. `TerminalRichPrompt.svelte` — add the close
   button to the prompt header. Wire to the
   existing hide path.
3. Audit menus for the "Show rich prompt" items;
   drop where found.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-a-architect.md`.

## 2026-05-19 14:38 BST — @@FullStackA implementation note

Implementation:

* New `showOrSpawnRichPromptInFocusedPane()` in
  `tabs.svelte.ts`. Finds the first terminal tab in the
  focused pane and focuses it; if no terminal, calls
  `openTerminalInPane()` to spawn one; finally calls
  `openActiveTerminalRichPrompt()` to reveal the prompt.
  Caller commits Pane Mode first so the spawned terminal
  lands in the live layout instead of evaporating on Esc.
* `App.svelte` `handlePaneModeKey`: new `case "p" / "P"`
  that commits the draft, schedules a session save, then
  calls the helper. Outside Pane Mode `p` has no special
  meaning (the focused editor / terminal absorbs it as
  normal input).
* The close button is **already present** at
  `TerminalRichPrompt.svelte:284-286` (the `X` icon with
  `class="icon-btn"` calling `onClose`, sitting next to
  the `Send` button in the header). Esc also closes via
  the existing `onKeydown` (line 78). Both behaviors
  match the spec; no change needed there.
* `TerminalTab.svelte` hamburger menu: dropped the
  `Rich prompt` entry + the corresponding
  `MessageSquareText` import. Alt+Space global shortcut
  is preserved as the out-of-Pane-Mode keystroke (still
  in `SHORTCUTS` + `App.svelte`'s direct handler) for
  muscle-memory, but the menu surface no longer
  duplicates it.
* `PaneModeHelp.svelte`: added the new `p` row under
  the Spawn group of the cheatsheet so the in-overlay
  hint reflects the binding.

Menu audit:

* Hamburger menus: `TerminalTab.svelte` "Rich prompt" —
  DROPPED.
* Right-click menus: the rich prompt's own context menu
  (mode toggle, style toolbar, watcher controls, spawn
  agent, bubble mode) is preserved — these control the
  prompt's CONTENTS, not its visibility. Spec explicitly
  carves them out.
* Tab right-click menus: no rich-prompt entries today.
* Doc / file / browser / graph contexts: no rich-prompt
  entries today.

Files touched:

* `web/src/state/tabs.svelte.ts` —
  `showOrSpawnRichPromptInFocusedPane`.
* `web/src/App.svelte` — `case "p" / "P"` in pane-mode
  dispatch + helper import.
* `web/src/components/TerminalTab.svelte` — dropped
  hamburger entry + `MessageSquareText` import.
* `web/src/components/PaneModeHelp.svelte` — cheatsheet
  row for `p`.
* `web/src/state/tabs.test.ts` — 3 new tests covering
  empty-pane spawn / existing-terminal focus / prompt
  preserves buffer + mode on re-show.
* `web/src/components/paneModeKeymap.test.ts` — raw-
  source assert on the `case "p" / "P"` ordering.

Gate green:

* `npm run test -- tabs paneModeKeymap` (71 passed),
* `npm run test` (342 passed),
* `npm run check`, `npm run build`,
* `bash -lc 'ulimit -n 4096; scripts/pre-push'` (green).

Proposed commit message:

> Cmd+K p shows or spawns rich prompt (fullstack-50)
>
> New Pane Mode key `p` reveals the rich prompt on the
> focused pane's terminal: focuses an existing terminal
> tab (active or not) and shows the prompt there, or
> spawns a new terminal in the pane when none is
> present. Commits the Pane Mode draft first so any
> in-flight layout edits seal and a freshly-spawned
> terminal survives. Drops the "Rich prompt" hamburger
> entry on TerminalTab; Cmd+K p is now the canonical
> entry, the rich prompt's existing `×` button (and Esc)
> is the exit. Alt+Space global shortcut preserved for
> muscle memory.
