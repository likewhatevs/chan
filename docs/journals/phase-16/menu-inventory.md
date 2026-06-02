# SPA menu inventory (F4 review deliverable)

Architect-produced for @@Host's context-menu review (task F4). Covers every
hamburger (kebab / ⋮) and right-click menu across the SPA, with file:line
and current contextual logic. The core finding: several widgets reuse ONE
shared `tabMenu` bubble for BOTH the tab-name click and the body
right-click — that conflation is what F4 untangles.

## Editor — `FileEditorTab.svelte:501-1006`
One bubble serves the tab-name click AND the body right-click. Items:
rename, page-width slider, mode (source | pretty tree | table), collapse
code blocks, outline toggle, details toggle, style toolbar, syntax
highlight, trailing-whitespace (highlight / remove), search, find, copy path
to file, copy path to $CWD, reload from disk, spawn band (duplicate, new
file, new terminal, new file browser, new graph), settings, reopen closed
tab, close.
- NOT selection-aware: identical items whether or not text is selected.
  This is F4's primary editor fix (add Copy/Cut/Paste + contextual items on
  selection; keep the rest on the tab menu).
- Conditionals today: rename only for non-draft files (draft shows
  "Save to Workspace"); mode toggles hidden for plain text; code-block
  toggle markdown-only; syntax highlight source-mode-only.

## Terminal — `TerminalTab.svelte:1370+`
Body right-click bubble. Items: rename, group, group-change notice, status
row, stale-env prompt, find, copy-selection-or-scrollback, copy-scrollback,
paste clipboard, copy $CWD, spawn band, broadcast targets (per-group-tab
checkboxes), MCP env toggles, settings, reopen closed tab, close.
- Selection-aware copy already exists: `copySelectionOrScrollback` (:1061)
  copies selection first, else full scrollback.
- The broadcast block is the F3 reorder target (move it to the top, after
  the Group row).

## Tab strip — `Pane.svelte`
The tab-name right-click routes through the SAME per-widget bubble as the
body (editor / terminal / dashboard / file browser all mirror their body
menu onto the tab). F4 wants the tab-name menu PRUNED of body-only items.

## Pane ⋮ — `Pane.svelte`
New draft, terminal, file browser, team work, graph, search, dashboard,
split horizontally, split vertically, close pane, nav mode.

## File Browser — `FileBrowserSurface.svelte` (hamburger) + `FileTree.svelte:540+` (row right-click)
Hamburger: expand/collapse all, maximize, reload, settings,
open/search/graph from selection. Row menu (file-vs-dir aware): new
file/dir, rename, duplicate, copy path / full path, delete, download /
export tar, upload (dir only), graph from here, terminal from here, open in
file browser, settings. Clipboard-aware (paste only when a cut/copy is
pending).

## Graph — `GraphPanel.svelte` / `GraphCanvas.svelte`
Canvas right-click forwards to the panel hamburger: scope picker, filter
chips (link / tag / mention / img), depth slider, inspector toggle, reload,
settings.

## Dashboard — `DashboardTab.svelte:75-189`
About / Workspace / Search slot toggles (at least one must stay on),
settings, reload. Hamburger and body right-click share `openAtCursor`.

## Search — `SearchPanel.svelte`
Hamburger: maximize, reload, inspector toggle, settings. No row context
menu.

## Inspector — `Inspector.svelte`
No context menu today. (F2 adds section separators, not a menu.)

## EmptyPaneWelcome
Intentionally no right-click (click-grid + the pane ⋮ cover it).

## Global — `App.svelte` / `shortcuts.ts`
No app-level hamburger; chords only (reload Cmd+R, settings Cmd+,, nav
Cmd+.).

## Link handling (for the F4 link-preview feature)
- Internal `[[...]]` wiki-links: click-to-navigate, NO hover/preview.
- External URLs: clickable, no preview. `openExternalUrl`
  (`web/src/editor/external_links.ts`) -> Tauri opener on desktop,
  `window.open(_, "_blank")` on web. No user choice today.
- Terminal: xterm `WebLinksAddon` detects URLs -> same `openExternalUrl`.
- No preview bubble and no internal "open in new tab" affordance exist.
