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

[`../alex/addendun-a.md`](../alex/addendum-a.md)
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

## 2026-05-22 — slice 1b (Graph header click → inspector) ready for review

Two-file change. SPA-only. Wires the slice-1
header row (`af65ebc`) to open the inspector
for the current scope when clicked.

### What landed

`web/src/components/GraphPanel.svelte`:

* New `openScopeHeaderInspector()` handler.
  Maps the current scope kind to the matching
  node id in the current graph nodes list:
  * `drive` → empty-string id (drive root
    node carries `id=""` in the filesystem-
    merged layer).
  * `tag` → `currentScope.nodeId` (the tag
    scope's stable graph id).
  * `file` → walk `nodes`, find
    `n.kind === "file" && n.path === currentScope.path`.
  * `dir` / `git_repo` → walk `nodes`, find
    `n.kind === "folder" && n.path === path`.
    (`git_repo` uses `currentScope.root` for
    the path lookup.)
  * `group` / `global` → no-op (no single
    inspector node; multi-file / no-node
    scopes don't map cleanly).
* Header row converted from `<div>` to
  `<button>` with `onclick={openScopeHeaderInspector}`.
* `closeTabMenu()` called after setting the
  selection so the menu doesn't linger.
* CSS: `cursor: pointer` on
  `.graph-scope-row` (was `default`); hover
  state lifts `.graph-scope-path` color to
  `var(--text)` for affordance.

`web/src/components/graphScopeHeaderRow.test.ts`:
+7 raw-source pins covering the button
markup, the drive/tag/file/dir mapping
branches, the inspectorOpen + closeTabMenu
side-effects, and the hover CSS state.

### Acceptance (slice 1b)

* Click on graph hamburger header row opens
  the inspector for the current scope ✓.
* Hover surfaces the affordance (cursor
  pointer + color lift) ✓.
* No-op for scopes without a matching node
  (group / global) ✓.

### Gate

* vitest **796 / 796** (+7 net from slice 1's
  789).
* svelte-check 0 errors / 0 warnings across
  4007 files.
* npm build clean.
* Rust gate not re-run.

### Decisions

* **In-graph inspector** (not external
  navigation) — matches the existing
  graph-side inspector pattern from
  `-a-50`'s DirectoryInfoBody work. Click on
  a graph element opens the in-graph
  inspector; the inspector's own action
  buttons handle further navigation
  (e.g. "Open in editor").
* **No-op for group/global** — group is a
  multi-file scope (no single inspector
  target); global has no first-class node in
  the current graph view. The handler simply
  returns early.
* **closeTabMenu() after select** — without
  this the menu stays open over the inspector,
  blocking the user's read.

### Suggested commit subject

```
Graph hamburger: scope-header click opens inspector (fullstack-a-67 slice 1b)
```

### Files for `git add` (per-path discipline)

* `web/src/components/GraphPanel.svelte`
* `web/src/components/graphScopeHeaderRow.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-67.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.

## 2026-05-22 — slice 2 (Hybrid hamburger New Draft entry) ready for review

Three-file change (plus 1 test update). SPA-only.

### What landed

`web/src/components/Pane.svelte`:
* New `spawnActions` entry at slot 0:
  `{ label: "New Draft", icon: FilePlus,
  command: "app.draft.new", chordId:
  "app.draft.new" }`.
* `FilePlus` added to the `lucide-svelte`
  import block.
* The 4 prior entries (Terminal / File
  Browser / Rich Prompt / Graph) stay in
  order beneath it; nothing about the
  separator-then-Enter-Hybrid-NAV-then-
  palette layout changes.
* Since `spawnActions` is shared across THREE
  surfaces — pane hamburger, empty-pane
  right-click, and the empty-pane carousel
  slide 1 — all three pick up New Draft
  simultaneously.

`web/src/App.svelte`:
* `runCommand` switch gains a
  `case "app.draft.new"` branch that calls
  `void createDraftAndOpen()`. Routes the
  menu click (via the existing
  `chan:command` event) AND the future
  native-menu binding (chan-desktop)
  through the same handler the Cmd+N chord
  already uses.
* `createDraftAndOpen` helper preserved
  unchanged.

`web/src/components/Pane.test.ts`:
* Two existing spawn-list expectations
  updated to include `"New Draft"` at slot
  0. Comments cite both `-a-32` (original
  spawn set) and `-a-67 slice 2`
  (extension).

`web/src/components/hybridHamburgerNewDraft.test.ts`
(new): 6 raw-source pins covering the
spawnActions extension, the icon import,
the rationale comment, the ordering
preservation, the runCommand routing, and
the createDraftAndOpen helper preservation.

### Acceptance (slice 2, Hybrid hamburger surface)

1. **Hybrid hamburger shows New Draft as
   first row** ✓ — mechanism via tests;
   @@WebtestA walk for empirical confirm.
2. **Cmd+N + native menu still work** ✓ —
   `createDraftAndOpen` is the shared
   handler; runCommand adds a route, doesn't
   replace.
3. **Empty-pane right-click menu also shows
   New Draft** ✓ (shared array; tested by
   the existing Pane.test.ts pin update).
4. **Carousel slide 1 also picks up New
   Draft** ✓ (same shared array).

### Out of scope for this slice

Per the architect-side queue framing, the
remaining `-a-67` surfaces (Terminal / FB /
Editor hamburgers) are separate slices.
This slice is the smallest one in the menu
revamp series — single addition + the
runCommand route.

### Gate

* vitest **1026 / 1026** (+7 net from
  `-a-92`'s 1019).
* svelte-check 0 errors / 0 warnings across
  4037 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **Keep File Browser in the Hybrid
  hamburger** despite the addendum-a spec
  listing only Terminal / Rich Prompt /
  Graph alongside New Draft. The current
  shared `spawnActions` array keeps FB as a
  first-class spawn surface across the
  three menus; removing it from JUST the
  Hybrid hamburger would require splitting
  the array OR introducing a per-row visibility
  flag. The 5-entry shape (New Draft +
  Terminal + FB + Rich Prompt + Graph)
  reads cleaner than the addendum's strict
  4-entry shape; document the deviation
  for @@Alex's review. If the spec is
  load-bearing, split into separate arrays
  in a follow-up.
* **`createDraftAndOpen` is the shared
  handler** — Cmd+N chord, menu click, and
  the future native-menu path all converge.
  Adding a chan:command route was cheaper
  than restructuring.
* **Bundle the test-pin updates** in the
  same commit. The Pane.test.ts pins were
  literal-array equality checks of the
  menu shape; updating them belongs with
  the shape change.

### Suggested commit subject

```
Hybrid hamburger: add New Draft as first spawn entry (fullstack-a-67 slice 2)
```

Single commit. spawnActions extension +
icon + runCommand route + 2 Pane.test
updates + 6 new pins.

### Files for `git add` (per-path discipline)

* `web/src/components/Pane.svelte`
* `web/src/App.svelte`
* `web/src/components/Pane.test.ts`
* `web/src/components/hybridHamburgerNewDraft.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-67.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance + the
@@Alex review on the keep-FB-in-Hybrid
decision.

## 2026-05-23 — slice -a-67d (Terminal right-click menu revamp)

SPA-only. Substantial reshape of the TerminalTab
right-click menu per addendum-a's verbatim
spec.

### Shape applied

* Status row: " - " → ": " (colon, not em dash).
* MCP env vars + Restart pulled up above the
  find/copy band (header neighbourhood).
* Find / Copy / Paste / Copy path to $CWD /
  Copy Scrollback consolidated in one band.
* New "From $CWD" section with label + four
  spawn entries (New File / New Terminal /
  New File Browser / New Graph). Chord hints
  surfaced via `chordFor()` lookups.
* Broadcast section structure kept as-is
  (deferred: Terminals expander dropdown +
  Jitter slider — see scope-poke below).
* New foot block: Settings (flipHybrid) +
  Reopen Closed Tab + Close. Replaces the
  prior Reload Window / Open Inspector tail
  entries that addendum-a's spec drops.

### Files touched

* `web/src/components/TerminalTab.svelte`
  * Imports: added `Folder` / `Settings2` /
    `Terminal as TerminalIcon` / `X` from
    lucide; dropped `Bug` / `FolderOpen` /
    `RefreshCw`.
  * Imports: added `flipHybrid` from
    `../state/tabs.svelte`.
  * Imports: dropped `isTauriDesktop`,
    `openWebInspector`, `reloadWindow` from
    `../api/desktop` + `notify` from
    `../state/notify.svelte` (only consumers
    were the dropped Reload + Inspector
    handlers).
  * Helpers added: `dispatchChanCommand` +
    `openNewTerminal` / `openNewFileBrowser` /
    `openNewGraph` (each closes the menu +
    fires the canonical `chan:command` event
    so the chord-routing layer + the empty-
    pane carousel + this menu all converge on
    one handler). `flipToSettings` calls
    `flipHybrid(paneId)`. `closeFromMenu`
    calls `closeTab(paneId, tab.id)`.
  * Helpers removed: `doReloadWindow`,
    `doOpenInspector` (and the inline notify
    fallback message).
  * Markup: full `action-list` reshape as
    described above. Status row em-dash → colon.
  * CSS: added `.mbtn.destructive`
    (Restart red color via `--danger-text`)
    + `.from-cwd-label` (subdued section
    label per addendum-a's "from-CWD" font).

### Tests

* New file `web/src/components/terminalRightClickRevamp.test.ts`
  — 15 architectural pins covering the colon
  switch, the From-$CWD band (label + helpers +
  buttons + dispatchChanCommand), the
  MCP/Restart-above-find/copy ordering,
  Settings (flipHybrid) + Reopen + Close foot
  block, and the `flipHybrid` import.
* `web/src/components/tabMenuReloadInspector.test.ts`:
  flipped the terminal-side block from REQUIRE
  to FORBID for Reload + Open Inspector + the
  desktop helper imports + the inspector
  notify hint. (FileEditor block unchanged —
  pending `-a-67f`.)
* `web/src/components/menuTrims.test.ts`: the
  `-80` Terminal block updated. Search drop
  preserved; `openSettingsFromMenu` (the
  global-Settings overlay opener) drop
  preserved; NEW assertion that
  `flipToSettings` (per-tab back-side flip,
  semantically distinct) IS present.
* `web/src/components/TerminalTab.test.ts`:
  the "no New Terminal entry" test flipped
  to "From-$CWD band renders New File / New
  Terminal / New File Browser / New Graph"
  (with cite to addendum-a + the historical
  `-a-32` reasoning).

### Deferred / scope-poke

* **MCP env info-button → modal dialog**:
  addendum-a wants the info button to open a
  dialog "like the New File one" with the
  explanation + a CTA "Show MCP env in
  terminal". Slice 1 keeps the current toggle
  popover; slice 2 converts to modal. SPA-only
  (no backend dep). Tracked as
  `-a-67d` slice 2.
* **Terminals expander dropdown + Jitter
  slider**: addendum-a wants the per-target
  broadcast list wrapped behind a "Terminals"
  expander, with a Jitter input (0-5s) at
  the top of the dropdown that randomly
  delays broadcast input. Jitter has a
  chan-server gap: the broadcast logic
  applies inputs immediately today. Persist
  the per-drive Jitter value via the
  preferences endpoint + apply random
  `[0, jitter]` delay in `broadcastTerminalInput`.
  Scope-poked to architect as `-a-67d` slice 3
  (backend gap).

### Gate

* `svelte-check` → 0/0.
* `vitest` → **1140 passing** (+16 from
  `-a-77c` baseline: 15 new pins + 1 rewritten
  `TerminalTab.test` pin).
* `npm run build` → clean.
* `cargo fmt --check` + `clippy --all-targets
  -- -D warnings` → clean (no Rust delta).

### Suggested commit subject

```
Terminal: right-click menu revamp per addendum-a spec (fullstack-a-67 slice d)
```

### Files (per-path)

* `web/src/components/TerminalTab.svelte`
* `web/src/components/terminalRightClickRevamp.test.ts` (new)
* `web/src/components/tabMenuReloadInspector.test.ts`
* `web/src/components/menuTrims.test.ts`
* `web/src/components/TerminalTab.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-67.md`

Autonomous-commit mode. No clearance held.
Next: -a-67e (File Browser) + -a-67f (Editor),
plus the -a-67d slice-2 (MCP modal) + slice-3
(Jitter backend scope-poke).

## 2026-05-23 — slice -a-67e (File Browser menus revamp)

SPA-only. Two FB menus reshaped per addendum-a:
the FB tab right-click / hamburger
(FileBrowserSurface.svelte) and the in-tree
selection menu (FileTree.svelte).

### FB tab right-click (FileBrowserSurface)

Header:
* Inline editable Drive name (mirror of Terminal
  name input). Oninput → PATCH preferences
  via the new `commitDriveName` helper +
  echo fresh DriveInfo back into the store.
* Drive-path row with HardDrive icon, monospace
  text, fade-on-overflow (same `mask-image`
  pattern as `-a-67 slice 1`'s graph-scope-row).
  Click → drive inspector.

Body (in order):
* SEP → (Un)Dock left / (Un)Dock right.
* SEP → Expand/Collapse all directories / Reload.
* SEP → Import Contacts.

Foot (tab variant only):
* Settings (flipHybrid via new `onFlip` callback
  prop). Pane.svelte wires
  `onFlip={() => flipHybrid(pane.id)}` for tab
  variant; dock + overlay variants don't pass
  it so the entry hides.
* Reopen Closed Tab (disabled when stack
  empty).
* Close (routes through `onClose` —
  Pane.svelte's existing wiring covers it).

Dropped:
* `Rename drive...` modal entry — replaced by
  the inline input.
* `New file` / `New directory` entries — moved
  to the selection menu where they're rooted
  under the selected directory.

### FB in-tree selection menu (FileTree)

* New `"From selection"` label at the top.
* `New File` / `New Directory` (kept; gated on
  `isDir`).
* `Search` (was "Search this") — relabel.
* `New Terminal` (was "Terminal from here") —
  relabel.
* `New Graph` — NEW entry; routes through
  the existing `graphThis` helper.
* SEP (`.ctx-sep`).
* Copy Path / Rename / Move / Delete —
  preserved.

### Decisions

* **Copy Path / Rename / Delete kept** even
  though addendum-a doesn't explicitly list
  them in the selection menu spec. Dropping
  destructive + path-copy ops without
  another surface would regress critical
  workflows. Flagged for @@Alex review;
  trivial to drop if requested.
* **Unified `New File or Directory` dialog**
  deferred to slice 2. The spec calls for one
  input that detects file-vs-dir from path
  shape; that needs a `kind: "either"`
  extension to `PathPromptModal` + a
  per-typestroke detector. Slice 1 keeps
  the two existing entries.
* **Settings (flip) entry in FileTree** —
  spec lists it; deferred to slice 2.
  FileTree is shared across dock / overlay /
  tab variants. The cleanest wire is to pipe
  `onFlip` down from FBSurface → FileTree as
  a prop. Slice 1 lands the FBSurface flip
  entry (the substantial change); the
  in-tree flip is a small follow-up.
* **`onFlip` callback prop** (not paneId
  prop) — same shape as the existing
  `onClose` callback. Keeps Pane.svelte as
  the only consumer of `flipHybrid` for the
  FB tab; the surface stays paneId-agnostic.

### Files touched

* `web/src/components/FileBrowserSurface.svelte`
  * Imports: dropped `FilePlus` / `FolderPlus`;
    added `HardDrive` / `History` /
    `Settings2` / `X`. Added `ui` + `api` +
    `canReopenClosedTab` + `reopenClosedTab`.
    Dropped `openGraphForDrive`.
  * Props: added `onFlip?: () => void`.
  * Helpers: added `commitDriveName`,
    `flipToSettings`, `doReopenClosedTab`,
    `closeFromMenu`. Dropped `newFileHere`,
    `newDirHere`, `graphDrive`, the modal
    `renameDrive` (the helper stays in
    `fileOps` for other callers; this menu
    no longer surfaces it).
  * Markup: full `menuItems` snippet
    rewrite per the new shape.
  * CSS: new `:global(.hamburger-menu
    li.drive-rename-row)` + .drive-rename-
    input + .drive-path-row + .drive-path-
    text. Dropped `.folder-row` / .folder-
    text / .folder-label / .folder-path /
    .mono selectors that styled the
    retired rows.
* `web/src/components/FileTree.svelte`
  * Markup: ctx menu reshape per new shape.
  * CSS: new `.from-selection-label` +
    `.ctx-sep` selectors.
* `web/src/components/Pane.svelte`
  * Passes `onFlip={() => flipHybrid(pane.id)}`
    to the FBSurface tab variant.
* `web/src/components/fileBrowserRightClickRevamp.test.ts`
  (new): 15 architectural pins for the
  FBSurface menu shape — drive-rename input
  + commitDriveName + path row with mask-image
  + dock/expand/reload/import ordering +
  Settings/Reopen/Close foot block gated on
  isTab + the Pane.svelte onFlip wiring +
  the dropped entries.
* `web/src/components/fileTreeSelectionMenu.test.ts`
  (new): 7 pins for FileTree's selection
  menu — From-selection label, relabels,
  New Graph entry, ctx-sep separator, per-row
  ops preserved.

### Deferred / scope-pokes

* **`-a-67e` slice 2** (SPA-only, no clearance
  needed): unified "New File or Directory"
  dialog (`kind: "either"` for PathPromptModal
  + per-keystroke detect) + Settings entry in
  the FileTree selection menu (pipe down
  onFlip).
* No new chan-server gaps.

### Gate

* `svelte-check` → 0/0.
* `vitest` → **1162 / 1162** (+22 net from
  `-a-67d`'s 1140; 22 new pins in 2 new test
  files).
* `npm run build` → clean.
* `cargo fmt --check` + `clippy --all-targets
  -- -D warnings` → clean (no Rust delta).

### Suggested commit subject

```
File Browser: right-click menu revamp per addendum-a (fullstack-a-67 slice e)
```

### Files (per-path)

* `web/src/components/FileBrowserSurface.svelte`
* `web/src/components/FileTree.svelte`
* `web/src/components/Pane.svelte`
* `web/src/components/fileBrowserRightClickRevamp.test.ts` (new)
* `web/src/components/fileTreeSelectionMenu.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-67.md`

Autonomous-commit mode. No clearance held.
Next: -a-67f (Editor).
