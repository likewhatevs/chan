# fullstack-a-67 — Right-click context menus revamp across 5 surfaces (Hybrid / Terminal / FB / Graph / Editor)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Revamp the right-click context menus across all 5 tab
surfaces per @@Alex's spec. Substantial UX refresh —
adds discoverable entries + new sub-menus + reorganizes
existing ones.

## Reference

[`../alex/addendun-a.md`](../alex/addendun-a.md)
"## Right-click menus and flows revisited" — verbatim
spec per surface. Includes screenshots in the same
folder (`image-2.png` Hybrid, `image-3.png` Terminal,
`image-4.png` FB, `image-5.png` FB selection,
`image-6.png` Graph, `image-9.png` Editor).

## Scope (5 surfaces)

Implementer's call on bundling vs splitting per
surface. The addendum lists each menu's desired state
verbatim — treat that as the source of truth.

### 1. Hybrid hamburger (global)

* New Draft Cmd+N
* Terminal Cmd+T
* Rich Prompt Cmd+P
* Graph the drive Cmd+Shift+M
* Separator
* Enter Hybrid Nav
* Separator
* Focus border colour (with planned merges)

### 2. Terminal

Substantial — see addendum section "### Terminal" for
full spec. Highlights:

* Name (editable, as today).
* `connected: size` text (colon not em dash).
* `Set MCP env vars` with info-button (opens dialog
  with explanation + show-in-terminal button).
* Restart (destructive).
* Separator
* Find / Copy / Paste / Copy path to $CWD / Copy
  scrollback.
* Separator
* "From $CWD" text section: New File / New Terminal /
  New File Browser / New Graph.
* Separator
* Broadcast on/off (as today) + Terminals
  ROLLED INTO DROPDOWN (Jitter + list of terminals
  inside the dropdown).
* Separator
* Settings (toggle).
* Reopen last tab Cmd+Shift / Close Cmd+W or Ctrl+D.

### 3. File Browser

Substantial — see addendum section "### File Browser":

* Drive name (editable like Terminal name).
* Full path (greyed, drive icon, fade-on-overflow,
  click → drive inspector).
* (Un)Dock left / right.
* Expand / Collapse all dirs.
* Reload.
* Import Contacts.
* Settings (toggle).
* Reopen last / Close.

Plus in-browser selection menu (addendum image-5):

* "From selection" text.
* New File or Directory (unified dialog accepts both;
  if dir → create + select; if file → create + open
  in Hybrid Editor).
* Search (selection scope).
* New Terminal (Cmd+T) with selection-aware placement.
* New Graph (Cmd+Shift+M).
* Settings (toggle).

### 4. Graph

* Full path (matching FB style; file/dir icon for
  the focused node; click → inspector).
* Existing depth / reload / colours.
* Settings (toggle).
* Reopen last / Close.

### 5. Editor

Substantial — see addendum section "### Editor":

* Name (editable, accepts paths like `../other/dir/`
  + extension changes).
* Show Source Code (Obsidian-style shortcut).
* Collapse Code Blocks.
* Search / Find / Copy / Paste / Copy path to file /
  Copy path to $CWD.
* "From $CWD" section: Duplicate File / New File /
  New Terminal / New File Browser / New Graph.
* Settings (toggle).
* Reopen last / Close.

## Implementer's choice on shape

**Bundle vs split**: if all 5 surface revamps fit in
one coherent commit, bundle. If any single surface is
large enough that a separate commit reads cleaner,
split. Per-surface task numbers (`-a-67a` /
`-a-67b` etc.) acceptable. Use judgment.

## Acceptance

Per-surface checklist against the addendum spec. Every
listed entry present + functional. Existing chord
bindings preserved.

### Tests

Vitest pins per substantive menu surface for the
final entry shape.

### Gate

* `npm test -- --run`, `npm run check`, `npm run build`
  green.

## Coordination

* @@FullStackA. SPA-heavy.
* Atomic-audit-commit (per-path; multiple agents may
  be in tree).
* Cross-references existing tasks:
  * Cmd+N → New Draft is `fullstack-a-66`.
  * "Set MCP env vars" dialog may already have an
    info button; check.

## Authorization

Yes for SPA menu / dialog files + tests + task tail +
outbound. If chan-desktop-side menu accelerators
need adjusting beyond what's already in flight,
scope-poke + I route the cross-lane piece.

## Numbering

This is `-a-67`.

## Out of scope

* Backend changes (chan-server / chan-drive).
* Drafts/folder backend (separate task `systacean-24`).
* Cmd+N keybinding wiring (separate task
  `fullstack-a-66`).
* Cmd+Shift+[/] tab focus bug (separate task
  `fullstack-a-64`).
