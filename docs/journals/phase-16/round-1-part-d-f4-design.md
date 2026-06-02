# F4 design: context-menu split + link affordances (@@LaneD)

Design-first deliverable for F4. Read `menu-inventory.md` (the full menu
catalog) and `lane-d.md` F4 spec alongside this. Nothing here is
implemented yet; this is for @@Host review via @@Lead before any code.

## 1. Problem (grounded in source)

There is ONE transient bubble, `tabMenu` (`web/src/state/tabMenu.svelte.ts`):

```
tabMenu = { openForTabId: string | null, anchor: AnchorRect | null }
```

`openTabMenu(tabId, anchor)` is called from THREE sites, all opening the
same bubble with the SAME item set:

```
caller                                     intent       today
-----------------------------------------  -----------  ----------------
Pane.svelte tab strip (:1008, :1076)       tab-name     full item set
FileEditorTab.onEditorContext (:501-510)   editor body  full item set
TerminalTab.onTerminalContextMenu (:1373)  term body    full item set
```

The bubble body is rendered by the widget (FileEditorTab / TerminalTab)
when `tabMenu.openForTabId === tab.id`. It does not know WHICH site
opened it, nor whether text is selected. Result: right-clicking selected
text in the editor shows the same rename / page-width / mode / outline /
spawn-band menu as clicking the tab name. The terminal already has a
selection-aware `copySelectionOrScrollback`, but its body and tab-name
menus are still identical.

## 2. Goal

- Body right-click -> a short, selection-aware menu (Cut/Copy/Paste +
  contextual link actions + Find), NOT the whole tab-config menu.
- Tab-name right-click -> the tab-config menu, PRUNED of body-only items.
- Link affordances: an external-link "open" action, and a markdown
  preview for link targets (terminal preview is read-only).

## 3. Proposed state change (small, central)

Add an open SOURCE to the shared bubble so the widget can branch:

```
tabMenu = {
  openForTabId: string | null,
  anchor: AnchorRect | null,
  source: "tab" | "body",      // NEW
}
openTabMenu(tabId, anchor, source)   // source defaults to "tab"
```

- Pane tab strip passes `"tab"`; the two body handlers pass `"body"`.
- Selection-awareness reuses what already exists: FileEditorTab tracks
  `selVer` (bumped on every Wysiwyg selection change) and holds
  `wysiwygRef`; the body branch reads the live selection from the editor
  at render time. No new global selection state.
- Backward compatible: a missing/`"tab"` source renders today's
  tab-config menu, so the change is additive.

## 4. Editor item split (FileEditorTab.svelte:501-1006)

```
item                       body-context      tab-context
-------------------------  ----------------  ----------------
Cut / Copy                 yes (if sel)      no (prune)
Paste                      yes               no (prune)
Copy link / Open link      yes (if on link)  no
Markdown preview           yes (if on link)  no
Find                       yes               no (prune)
Search (seed w/ selection) yes (if sel)      keep? (decision)
-------------------------  ----------------  ----------------
rename                     no                yes
page-width slider          no                yes
mode (source/tree/table)   no                yes
collapse code blocks       no                yes
outline / details toggle   no                yes
style toolbar              no                yes
syntax highlight           no                yes
trailing-whitespace        no                yes
copy path to file / $CWD   no                yes
reload from disk           no                yes
spawn band (dup/new *)     no                yes
settings                   no                yes
reopen closed tab / close  no                yes
```

Rationale: the body menu stays a tight "act on what I clicked" set; the
tab menu keeps document-view + lifecycle config. Conditionals already in
place (draft -> "Save to Workspace", plain-text hides mode toggles,
markdown-only code-block toggle, source-mode syntax highlight) stay on
the tab menu.

## 5. Terminal item split (TerminalTab.svelte)

The terminal body menu is mostly right already (selection-aware
`copySelectionOrScrollback` exists; F3 just moved broadcast to the top).
The split needed:

```
body-context (term):  Find, Copy (sel-or-scrollback), Paste,
                       Copy Scrollback, Open link (if URL under cursor),
                       Markdown preview (read-only, if link)
tab-context (term):    rename, group, status, MCP env, restart,
                       broadcast section, From $CWD spawn band,
                       settings, reopen, close
```

Today both show everything; pruning the tab-name menu of Find/Copy/Paste
mirrors the editor.

## 6. Link affordances

Infra that already exists (`web/src/editor/external_links.ts`):
`openExternalUrl(url)` (desktop OS browser / web new tab),
`externalUrlAtPos(state, pos)` (URL at a CM position), and a
click-to-open handler. So "open" is a one-call action; the new work is
the surfaces.

- **External-link "open" bubble.** Two options:
  - (A) Context-menu item only: body-context shows "Open link" +
    "Copy link" when `externalUrlAtPos` resolves at the click. Lowest
    risk, no new hover layer.
  - (B) Hover bubble: a small floating "Open" affordance on hover over a
    link, plus the menu item. More discoverable, more surface area
    (positioning, dismissal, mobile).
  - Recommend A first, B as a follow-up if @@Host wants the hover.
- **Markdown preview.** Fuzziest ask; needs scope from @@Host:
  - (P1) Internal `[[wiki]]` / relative md link -> a small preview of the
    target's rendered markdown (first N lines) on hover or via a menu
    item. Editor = interactive; terminal = read-only render.
  - (P2) Preview of the CURRENT external link's destination is out of
    scope (no fetch of arbitrary URLs from the workspace).
  - Recommend P1, internal-link preview, menu-item triggered first.

## 7. Implementation plan (after review)

Files (all @@LaneD-owned): `state/tabMenu.svelte.ts` (+source),
`components/FileEditorTab.svelte` (body branch + link items),
`components/TerminalTab.svelte` (body branch + link items),
`components/Pane.svelte` (pass source="tab"),
`editor/external_links.ts` (open-bubble helpers if B),
new `editor/link_preview.ts` (markdown preview render) if P1.

Steps: (1) state + source threading; (2) editor body/tab branch; (3)
terminal body/tab branch; (4) external "open" menu item; (5) markdown
preview. Each step is independently gateable and browser-smokeable
(Svelte-5 reactivity: selection -> menu items must be live).

Tests: source-pattern (`?raw`) tests asserting the source field + the
body/tab item partition, matching the existing menuTrims / editorRight
ClickRevamp / terminalRightClickRevamp test style. Browser-smoke the
selection-aware items and the link actions (static gates miss reactivity).

## 8. Decisions for @@Host (the review)

1. Body menu scope: tight (Cut/Copy/Paste + link + Find) or include a few
   view toggles? Recommend tight.
2. Does "Search (seed with selection)" belong on body-context, tab, or
   both? Recommend body-context when a selection exists.
3. External "open": context-menu item only (A) or also a hover bubble
   (B)? Recommend A first.
4. Markdown preview: internal-link preview (P1) only, and hover vs
   menu-item trigger? Recommend P1, menu-item first.
5. Split scope this round: editor + terminal both, or editor first and
   terminal as a follow-up? Recommend both (symmetry), staged commits.

## 9. Risks / scope

- [L] task; the state change is small but touches three render sites.
- The shared bubble is also used by dashboard / file browser / graph /
  search (menu-inventory.md). This design only re-points the EDITOR and
  TERMINAL bodies; the other widgets keep one menu (their bodies have no
  text-selection semantics). Confirm that boundary with @@Host.
- Pre-release: no back-compat needed, but the source field defaults to
  "tab" so unconverted callers are unaffected during staged commits.
