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

## 2026-05-22 — slice 1 (Graph surface) ready for review

Per the architect's "use judgment" + "per-surface
splits acceptable" framing, splitting `-a-67` into
slices. **Slice 1: Graph hamburger header row only.**

Two-file change. SPA-only.

### What landed

`web/src/components/GraphPanel.svelte`:

* Imported `FileText`, `Folder`, `HardDrive`,
  `Hash` icons from `lucide-svelte` (existing
  pattern from `Pane.svelte`).
* Added a `graph-scope-row` at the TOP of the
  tab-menu bubble, ABOVE the depth slider.
  Renders a kind-appropriate icon + the
  current scope path (or kind-label for drive
  / global). Path fades at the right edge
  (1.25rem linear-gradient mask) per the
  `-a-62` FB-tree pattern; matches @@Alex's
  addendum spec ("apply the same shade effect
  we use for tab names").
* Followed by a `<div class="msep">` separator
  to delimit the header from the existing
  depth / reload / filter rows.
* Icon dispatch covers all ScopeOption kinds
  (drive / global / dir / tag / git_repo /
  group / file). `git_repo` + `group` route
  to the Folder icon as a sensible default
  (they're directory-like aggregates).
* Display-only in this slice: no click-to-
  inspector wiring yet. The @@Alex spec calls
  for "Clicking on this will open the inspector
  for the file or directory" — flag for the
  follow-up slice. Existing tab-menu rows
  (depth, reload, filters, Settings, Reopen,
  Close) untouched in this slice.

`web/src/components/graphScopeHeaderRow.test.ts`
(new): 5 raw-source pins covering the lucide
imports, header-row markup, icon dispatch
coverage, mask-image fade, and the separator
placement before depth-row.

### What's deferred to follow-up slices

* **`-a-67b` (suggested)**: Click-to-inspector
  wiring on the new graph-scope-row. Spec'd
  but not yet wired.
* **`-a-67c` (suggested)**: Hybrid hamburger
  revamp. Has cross-task dep on `-a-66`
  (New Draft Cmd+N entry); recommend
  scheduling after `-a-66` lands so the entry
  has a real handler.
* **`-a-67d` (suggested)**: Terminal right-
  click menu revamp (substantial: MCP env
  info-button dialog, Restart, From $CWD
  section, Terminals dropdown with Jitter,
  Settings toggle, Reopen/Close).
* **`-a-67e` (suggested)**: File Browser
  right-click menu revamp (Drive name editable,
  full path header, (Un)Dock entries,
  Expand/Collapse all, Import Contacts,
  selection-menu revamp).
* **`-a-67f` (suggested)**: Editor right-click
  menu revamp (editable Name w/ path-accept,
  Show Source Code shortcut, Collapse Code
  Blocks, Search/Find/Copy/Paste/Copy paths,
  From $CWD section, Settings toggle,
  Reopen/Close).

The `-a-67` parent task stays open as the
umbrella; closing it requires all 5 surfaces
landed. Architect's call on whether to
re-dispatch the follow-up slices as separate
tasks (-a-67b through -a-67f) or keep them
under the umbrella.

### Acceptance (slice 1 only)

* Graph tab-menu shows a scope-path header row
  at the top ✓.
* Icon matches the scope kind ✓ (HardDrive for
  drive/global, Folder for dir/git_repo/group,
  Hash for tag, FileText for file).
* Path label fades at the right edge for long
  paths ✓.
* Separator below the row ✓.

### Gate

* vitest **789 / 789** (+5 net from `-a-65`'s
  784).
* svelte-check 0 errors / 0 warnings across
  4006 files.
* npm build clean.
* Rust gate not re-run.

### Decisions

* **Split per surface** — `-a-67` parent
  authorized this in the task body. Five
  slices is a lot for one commit; surfacing
  partial value early matches the
  "I want to see progress now" framing from
  @@Alex's addendum.
* **Display-only header** — click wiring
  needs to map a scope kind to the right
  inspector helper (file → file inspector,
  dir → dir inspector, tag → tag inspector).
  Slicing the wiring out keeps this commit
  tight + lets the architect re-validate
  the click behaviour separately.
* **`git_repo` + `group` → Folder icon**.
  No dedicated icon in the spec; they're
  directory-aggregate scopes so Folder
  reads sensibly.
* **Icons via lucide-svelte** — same import
  pattern as `Pane.svelte`; no new dep.

### Suggested commit subject

```
Graph hamburger: scope-path header row with kind icon (fullstack-a-67 slice 1)
```

Single commit. Scope-row markup + CSS +
imports + test pin tightly coupled.

### Files for `git add` (per-path discipline)

* `web/src/components/GraphPanel.svelte`
* `web/src/components/graphScopeHeaderRow.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-67.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance + routing
on the remaining 5 surfaces (whether to keep
under `-a-67` umbrella or re-dispatch as
`-a-67b..f`).
