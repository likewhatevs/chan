# fullstack-a-102 — Right-click menu nits: terminal double-separator + editor row order

Owner: @@FullStackA
Phase: 8, Round 3
Date cut: 2026-05-23
Priority: P2 — release polish for v0.13.0; small visual fixes

## Goal

Close two small polish nits @@Alex flagged 2026-05-23 in the right-click menus that `-a-98`'s per-surface audit didn't catch (the audit verified item presence; these are about order + separator structure).

## Background

@@Alex's two screenshots (2026-05-23):

### Nit 1: Terminal right-click — double separator after Name

The Terminal right-click menu shows the Name row followed by **two separators** before the `connected: ...` status row. Should be one separator.

Likely cause: two `<li class="separator">` elements (or equivalents) emitted in sequence, possibly from a guard branch that doesn't suppress the second when the first is already present.

### Nit 2: Editor right-click — page width / name order

Current order in the Editor right-click menu:
1. Page width slider
2. (separator)
3. Name (edit field)
4. (separator)
5. Show Source Code / Collapse Code Blocks
6. (separator)
7. Show Outline

Should be:
1. Name (edit field) **first**
2. (separator)
3. Page width slider
4. (rest unchanged)

The name row is the primary identity affordance; page width is a secondary view setting. Name should lead.

## Scope

### Fix 1: Terminal menu separator

Locate the Terminal right-click menu code in `web/src/components/TerminalTab.svelte` (or wherever the `-a-67` slice d shipped the menu). Remove the duplicate separator after the Name row.

### Fix 2: Editor menu row order

Locate the Editor right-click menu in `web/src/components/FileEditorTab.svelte` (or wherever the `-a-67` slice f shipped). Move the Page width row from position 1 to after the Name row + its trailing separator.

## Acceptance criteria

1. **Terminal right-click**: single separator between Name and the `connected: ...` status row.
2. **Editor right-click**: Name row first, then separator, then Page width, then existing rows below.
3. **Test pins**: update the existing menu pins (`menuTrims.test.ts` or per-surface pin files) to assert the new structure.
4. **Gate**: `npm run check` + `npm test -- --run` + `npm run build` green.

## How to start

1. `grep -n separator web/src/components/TerminalTab.svelte` + spot the double-separator near Name.
2. Editor: similar grep on `FileEditorTab.svelte`; reorder rows.
3. Update vitest pins to reflect the new structure (or remove pins that asserted the wrong order, if applicable).

## Coordination

* Time-boxed: small fix. Should be <30 min of work plus tests.
* Safety guardrail: do NOT touch @@Alex's running chan.app session. Throwaway drives or pure code inspection.
* This is post-`-98`-ship polish; `-98` was item-presence audit, this is structure/order.

## Authorization

Yes for SPA-side edits (`web/src/`) + vitest pin updates.

## Out of scope

* Further menu polish beyond these two specific nits.
* Restructuring other menus.

## Reference

* @@Alex's two screenshots in chat 2026-05-23.
* `-a-67` slices d (Terminal) + f (Editor) are the surfaces.
* `-a-98` audit report confirmed item presence; this task addresses order/separators.
