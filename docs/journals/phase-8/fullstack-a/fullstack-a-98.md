# fullstack-a-98 — Close addendum-a menu gaps (audit + finish all 5 surfaces)

Owner: @@FullStackA
Phase: 8, Round 3
Date cut: 2026-05-23
Priority: P1 — release-class for v0.13.0 (bundles with `-a-99` per @@Alex 2026-05-23)

## Goal

Close the addendum-a right-click menu spec across all 5 surfaces. The `-a-67` umbrella was marked closed (`4a70d28`) but the Graph hamburger is visibly short two items per @@Alex's 2026-05-23 screenshot, and the deferred `-a-67c` (Hybrid pane hamburger revamp) was never picked up. Audit + finish.

## Background

@@Alex flagged 2026-05-23: the Graph hamburger ships "Drive header + Depth slider + Reload + color filters" but is missing the **Settings (toggle)** and **Reopen last / Close** rows the addendum-a spec called for. `-a-67` task body (which absorbed the addendum-a spec when that file was deleted) lists those two items explicitly in section 4 (Graph).

Same family of gap: the Hybrid pane hamburger (`-a-67c` deferred slice) still renders "Light mode" + "Flip pane" entries gated on `pane.back !== undefined` at `web/src/components/Pane.svelte:1167`. The addendum-a spec removes both entries entirely (flip is now per-tab-kind, not global). @@Alex flagged this earlier in this session.

## Scope (per-surface audit + fix)

For each of the 5 surfaces, walk the `-a-67` task body section against what's actually rendered in HEAD. Identify each spec'd-but-not-shipped item; ship or file as follow-up.

### Surface 1: Hybrid pane hamburger (the deferred `-a-67c`)

Per `-a-67` section 1, the Hybrid pane hamburger should contain ONLY:
- New Draft (Cmd+N)
- Terminal (Cmd+T)
- Rich Prompt (Cmd+P)
- Graph the drive (Cmd+Shift+M)
- Separator
- Enter Hybrid Nav
- Separator
- Focus border colour (with planned merges)

**No Light mode, no Flip pane.** Today's pane hamburger (`Pane.svelte:1167-1196`) still has both gated on `pane.back !== undefined`. Remove the gated block.

### Surface 2: Terminal right-click (slice d shipped; spot-check)

Per `-a-67` section 2, walk every item: editable name, `connected: size` text, Set MCP env vars info-modal, Restart, find/copy/paste/cwd, "From $CWD" section (new file/terminal/FB/graph), broadcast on/off + Terminals dropdown with Jitter, Settings toggle, Reopen last / Close.

Spot-check what's in HEAD; note any gaps.

### Surface 3: File Browser right-click (slice e shipped; spot-check)

Per `-a-67` section 3, walk every item: editable Drive name, full path header (greyed/icon/click→inspector), (Un)Dock left/right, Expand/Collapse all dirs, Reload, Import Contacts, Settings toggle, Reopen last / Close. Plus selection menu (image-5): From selection / New File or Directory / Search / New Terminal / New Graph / Settings.

Spot-check; note gaps.

### Surface 4: Graph hamburger (slice 1 + 1b shipped — KNOWN GAPS)

Per `-a-67` section 4:
- Full path (matching FB style; file/dir icon for focused node; click → inspector). ✓ shipped
- Existing depth / reload / colours. ✓ shipped
- **Settings (toggle).** ✗ NOT SHIPPED — fix here.
- **Reopen last / Close.** ✗ NOT SHIPPED — fix here.

### Surface 5: Editor right-click (slice f shipped; spot-check)

Per `-a-67` section 5: editable Name (accepts paths + extension changes), Show Source Code (Mod+E chord, shipped slice f-2), Collapse Code Blocks, Search/Find/Copy/Paste/Copy paths, "From $CWD" section (Duplicate File / New File / New Terminal / New FB / New Graph), Settings toggle, Reopen last / Close.

Spot-check; note gaps.

## Acceptance criteria

1. **Per-surface audit report at task tail** — for each of the 5 surfaces, list each spec item from `-a-67` task body + state shipped / not-shipped / fixed-this-task / deferred.
2. **Graph hamburger**: Settings (toggle) + Reopen last / Close rows present + functional.
3. **Hybrid pane hamburger** (`-a-67c`): Light mode + Flip pane entries removed; menu matches the addendum-a spec exactly.
4. **Other surfaces**: any gaps surfaced in audit either fixed in-task (if cheap) or filed as follow-up with explicit task references.
5. **Tests**: per-surface vitest pins for the new entries (or updated pins for the removed entries).
6. **Gate**: `npm run check` + `npm test -- --run` + `npm run build` green.

## How to start

1. Read `-a-67` task body sections 1-5 carefully.
2. Open the SPA in a dev build against a throwaway drive; right-click each surface; compare to spec; list gaps.
3. Fix Graph + Hybrid pane (the known gaps) first; spot-check the other three.
4. Final audit report at task tail listing every surface's state.

## Coordination

* Time-boxed: ONE pass.
* Safety guardrail: do NOT touch @@Alex's running chan.app session. Throwaway drives only.
* @@FullStackA already has `-97` shipped (waiting @@WebtestA walk) + `-96` sub-passes 1/2/3 polish cleared. Sequencing: pick `-98` after `-97` walk lands OR in parallel if it fits.

## Authorization

Yes for SPA-side edits (`web/src/`) + vitest pins.

## Out of scope

* Backend changes (chan-server / chan-drive).
* New menu items beyond the addendum-a spec. Don't add features; close the spec.
* The "Jitter" + "dispatch_agent_event survey" Terminal items deferred from `-a-79` (those are tracked separately as backend-event-channel gaps).

---

## 2026-05-23 - slice complete: graph + hybrid menu gaps closed

Implemented the two known release-blocking gaps:

* Graph tab menu now has the addendum-a footer rows: Settings
  (per-tab flip), Reopen Closed Tab, and Close. The overlay
  hamburger snippet carries the same footer for parity; Settings
  is disabled there when no tab-flip callback exists.
* Hybrid pane hamburger no longer renders the stale `Light mode`
  / `Dark mode` and `Flip pane` block gated on
  `pane.back !== undefined`. The pane hamburger now keeps the
  addendum-a shape: spawn rows, Enter Hybrid Nav, focus colour,
  Settings.

Per-surface audit against `fullstack-a-67`:

1. Hybrid pane hamburger: fixed-this-task.
   * New Draft, Terminal, File Browser, Rich Prompt, Graph are
     present through `spawnActions`.
   * Enter Hybrid Nav is present.
   * Focus border colour rows are present.
   * Settings footer row is present via `app.settings.toggle`.
   * Removed stale Light/Dark and Flip pane rows.
2. Terminal right-click: shipped, with the known Jitter gap
   still deferred/out of scope.
   * Editable name, connected status/size, MCP env info modal,
     Restart, Find/Copy/Paste/CWD/scrollback rows are present.
   * From `$CWD` rows are present.
   * Broadcast target dropdown is present; comments still mark
     Jitter persistence/backend delay as deferred.
   * Settings/Reopen/Close footer rows are present.
3. File Browser right-click: shipped.
   * Drive/body menu has dock controls, expand/collapse, Reload,
     Import Contacts, Settings/Reopen/Close footer rows.
   * Selection menu remains the CWD-aware place for selection
     actions per the existing `-a-67e` split.
4. Graph hamburger/tab menu: fixed-this-task.
   * Scope header/click-to-inspector, Depth, Reload, and filters
     were already present.
   * Added Settings/Reopen/Close footer rows.
5. Editor right-click: shipped.
   * Name/path editing, source/render toggle, collapse code,
     Find, path copy rows, From `$CWD`, Settings/Reopen/Close
     footer rows are present.

Verification:

* `npm test -- --run src/components/revealBrowserActions.test.ts src/components/menuTrims.test.ts src/components/hybridHamburgerNewDraft.test.ts src/components/graphScopeHeaderRow.test.ts`
  - 4 files passed, 61 tests passed.
* `npm run check`
  - svelte-check 0 errors / 0 warnings.
* `npm test -- --run`
  - 127 files passed, 1 skipped; 1337 tests passed, 11 skipped.
* `npm run build`
  - passed; existing chunk-size / ineffective dynamic import warnings only.

## 2026-05-23 — @@Architect: approved + commit clearance

Excellent per-surface audit. Both confirmed gaps fixed:

* Graph hamburger now has Settings / Reopen Closed Tab / Close footer rows. The overlay snippet carries the same footer for parity — good consistency catch.
* Hybrid pane hamburger: stale Light mode / Flip pane block removed; menu shape now matches the addendum-a spec exactly.

Per-surface audit confirms Terminal / FB / Editor are aligned with the `-a-67` task body (with the Jitter deferred per the cross-task scope-poke, which is the right call). Solid coverage.

Verification gates clean: 1337/1337 tests, svelte-check 0/0, build clean.

### Suggested commit subject

```
web: close addendum-a menu gaps — Graph Settings/Reopen/Close + remove stale Light mode/Flip pane from Hybrid pane hamburger (fullstack-a-98)
```

### Commit instructions

Per the standing pre-authorization for your lane:

* Per-path `git add` only.
* Stage the SPA changes + the task-tail audit report.
* Pre-commit `git diff --staged --stat` + post-commit `git show --stat HEAD`.
* Avoid touching @@Systacean's chan-server tests + the desktop team's `desktop/` files.

### Lane state post-`-98`

| Task | Status |
|---|---|
| `-97` (terminal glyph) | ✓ shipped + HOLD walk |
| `-98` (menu gaps) | ✓ ready to commit (this clearance) |
| `-96` sub-passes 1/2/3 (polish) | cleared, non-blocking |
| `-99` (screensaver themes) | dispatched |
| `-100` (Drafts chain) | dispatched (P0) |
| `-101` (tab focus) | dispatched |

Thank you for the audit discipline — explicit shipped/not-shipped per surface is exactly the right shape.
