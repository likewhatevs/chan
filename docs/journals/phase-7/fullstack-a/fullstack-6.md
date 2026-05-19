# fullstack-6: pane menu reorg + click + color + nav cluster

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

One cohesive pass through the pane interaction surface that
folds together: B15 click-semantics fix, the pane menu reorg,
the per-pane focus-border color option, the next/prev pane
nav, AND building a doc-tab right-click menu (since @@WebtestB
found the doc tab has no menu at all today, only the terminal
does).

This is wave 1.5. All five pieces touch the same pane click
handlers and right-click menus, so one task lands them as a
coherent UX pass rather than five churny ones.

## Relevant links

* [../request.md](../request.md) Enhancements + Bugfixes
  (multiple bullets — the pane-related ones).
* [../webtest-b/webtest-b-1.md](../webtest-b/webtest-b-1.md)
  — @@WebtestB's finding that the doc tab has no right-click
  menu (terminal has a 22-item one).

## Acceptance criteria

### B15 click semantics

* Left-click on an empty pane: **selects the pane** only. Does
  NOT open the right-click menu.
* Left-click on a pane tab: **selects the tab** only. Does NOT
  open the right-click menu. (Currently blocks tab D&D.)
* Right-click is the ONLY way to open the pane / tab context
  menu.

### Pane right-click menu reorg

* Right-click on pane shows: Split (left/right/up/down), Close,
  Next pane, Previous pane, Focus border color (blue/green/
  pink). Reload + toggle web inspector are MOVED OUT of this
  menu.
* Pane hamburger menu now hosts: Reload, Toggle Web Inspector,
  and any existing hamburger items that should stay.

### Doc tab right-click menu (NEW)

* Right-click on a doc tab opens a menu, distinct from the
  terminal tab menu. Initial items (mirror the terminal where
  it makes sense; omit terminal-only actions):
  * Close
  * Close others
  * Close all
  * Copy path
  * Show in file browser
  * Reopen closed tab (already implemented by `fullstack-5`)
* Out of scope: full feature parity with the 22-item terminal
  menu. Build what makes sense for docs.

### Per-pane focus-border color

* Right-click menu item "Focus border color" with three
  options: blue (default), green, pink.
* Selection persists per pane through server preferences.
* The focus border on the active pane uses the selected color.
* Default stays blue for users who don't pick.

### Next / previous pane

* Right-click items "Next pane" / "Previous pane" focus the
  next or previous pane in the workspace's pane order.
* Keyboard:
  * Native (Chan.app desktop): `Cmd+]` (next), `Cmd+[`
    (previous).
  * Web: `Cmd+Alt+]` (next), `Cmd+Alt+[` (previous). Browsers
    reserve `Cmd+[/]` for back/forward.
  * Detect at runtime (Tauri presence) and register
    accordingly. Same pattern you used for `Cmd+T` in
    `fullstack-5`.

## Out of scope

* Pane drag-handle keyboard a11y (separate follow-up noted
  during fullstack-1 walkthrough).
* Tab D&D fixes beyond what's already in `fullstack-5`.

## How to start

The five pieces share machinery. Suggested order:

1. B15 click handler fix first (cheap, fixes a known
   confusing state).
2. Pane right-click menu refactor: define the new menu shape;
   move Reload + inspector to hamburger.
3. Doc tab right-click menu: new component, mirror terminal's
   shared menu primitives where applicable.
4. Color option: small preferences extension + CSS variable
   on the pane focus border.
5. Next/prev pane + shortcuts: leverage `fullstack-5`'s
   native-vs-web shortcut detection helper.

## Hand-off

Standard. Pre-push gate green before commit-readiness append.
Fire `alex/event-fullstack-architect.md` (type `poke`).

## 2026-05-18 17:28 BST — specialist review requested

Implemented the pane/menu pass for `fullstack-6`.

Changed files:

* `crates/chan/src/main.rs`
* `web/src/App.svelte`
* `web/src/components/FileEditorTab.svelte`
* `web/src/components/FileTree.svelte`
* `web/src/components/Pane.svelte`
* `web/src/components/TerminalRichPrompt.svelte`
* `web/src/state/shortcuts.ts`
* `web/src/state/store.svelte.ts`
* `web/src/state/tabs.svelte.ts`
* `web/src/state/tabs.test.ts`

What changed:

* B15 click semantics: left-click selects only; right-click opens pane/tab menus.
* Pane context menu now owns structural actions: split left/right/up/down,
  close pane, next/previous pane, and focus-border color.
* Pane hamburger now owns reload and web-inspector toggle.
* Doc tab right-click menu now includes close, close others, close all, copy
  path, show in file browser, and reopen closed tab.
* Per-pane focus-border color is stored with the pane layout state and restored
  with the serialized layout.
* Web next/previous pane shortcuts moved to `Cmd+Alt+[` / `Cmd+Alt+]`; native
  remains `Cmd+[` / `Cmd+]`.
* Rich prompt right-click menu can toggle rendered/source mode and the style
  toolbar.
* B22 defensive cleanup clears stale directory loading state after Copy Path.

Verification:

* `npm run check` from `web/`
* `npm run test -- tabs TerminalRichPrompt` from `web/`
* `npm run build` from `web/`
* `cargo check -p chan`
* `scripts/pre-push`

Notes:

* No manual desktop walkthrough performed in this lane.
* Focus color persists with the serialized pane layout/session state. If
  Architect requires this specifically in global server preferences rather than
  pane layout state, that needs a follow-up design call because pane ids are
  session-local.
