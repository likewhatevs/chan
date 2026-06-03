# task-LaneB-LaneA-6: B4 DONE - cs pane split/close (Option A, atomic)

From: @@LaneB  To: @@LaneA  Re: task-LaneA-LaneB-5 + followup-LaneA-LaneB-1 (A)

## Result: cs pane split is now RIGHT|BOTTOM and one-shot cs pane commands no
   longer steal focus or enter transaction mode. Landed atomically, verified.

## Landed as ONE lockstep burst (per your Option A ruling)

- chan-shell wire.rs: `SplitDir { Left, Bottom }` -> `{ Right, Bottom }` (+ doc;
  snake_case serde -> wire string "right"|"bottom").
- chan-shell cli.rs: `SplitDirArg` Left->Right (+ From impl, ShellAction::Pane
  + PaneAction::Split help text, 2 tests).
- store.svelte.ts `applyPaneExec` (your authorized region ~781-813):
  - PaneExecOp split `dir: "left"|"bottom"` -> `"right"|"bottom"` (L781).
  - split case (L813): `op.dir === "right"` -> splitPane(p, "row", "after")
    (new pane RIGHT); else -> splitPane(p, "column", "after") (bottom).
  - FOCUS-PRESERVE (L813-822): capture `keepActive = layout.activePaneId`
    before splitPane, restore after - so a one-shot `cs pane split` does NOT
    yank focus to the new empty pane. Chose capture/restore over a splitPane
    `focusNew` arg: most contained, no ripple to splitPane's other callers.
- control_socket.rs: NO change (forwards the op opaquely - confirmed).

Transaction-mode: the cs path already uses splitPane/closePane (not
paneModeSplit / enterPaneModeTransaction), so it never enters hybrid-nav
transaction mode. Confirmed on the smoke (no "Hybrid Enter commit" bar after
split OR close).

## store.svelte.ts attribution (for your merged-file commit)

  - B4 (mine):   lines 781 + 813-822 (PaneExecOp dir + applyPaneExec split case
                 + focus-preserve).
  - B9 (@@LaneC): lines ~2004-2014 (openFsGraphForDirectory). NO overlap with
                 my B4 region.
Pre-burst quiescence check passed: store.svelte.ts mtime was stable and
@@LaneC's only WIP was the ~2004 graph region, clear of applyPaneExec.

## Files changed (blob fingerprints)

  crates/chan-shell/src/wire.rs   blob 188d8e345cceb16a76b6b136536d244499fabc36
  crates/chan-shell/src/cli.rs    blob d5394976229d3893f565b3e3fb7e0064b224801e
        (also carries the mcp_env work; B4 = the SplitDir bits)
  web/src/state/store.svelte.ts   blob bf5bf1dc8047b94c8c5acef9e19fa366aeaef37d
        (B4 = lines 781 + 813-822 ONLY)

## Own-gate (scoped) - GREEN

  cargo fmt -p chan-shell --check                    PASS
  cargo clippy -p chan-shell --all-targets -D warn   PASS
  cargo test -p chan-shell                           PASS (37)
  cargo check -p chan-server                         PASS (unaffected)
  npm test (full vitest)                             PASS (1656)
  npm run build                                      OK

svelte-check NOTE: the full-tree run shows 8 errors, but NONE are in my files.
They are other lanes' in-flight WIP: TeamDialogConfig.mcpEnv missing in the team
orchestrator test fixtures (your B5 DIALOG half) + a SearchPanel.svelte union
type. store.svelte.ts itself is svelte-check-clean. Reporting scoped-green per
the isolated-gate model; flagging so you know those reds aren't B4.

## Empirical smoke (Chrome, fresh binary :8794, cs run FROM the SPA terminal)

- `cs pane split right`  -> new pane to the RIGHT; focus STAYS on the sending
  Terminal-1; no transaction bar.
- `cs pane split bottom` -> new pane BELOW; focus stays.
- `cs pane split left`   -> rejected: "invalid value 'left' [possible values:
  right, bottom]".
- `cs pane` query -> pane-1 (the sender) still ACTIVE after both splits (focus
  not stolen).
- `cs pane close-pane --pane pane-3` (a NON-active pane, @@Alex's exact case)
  -> closed; Terminal-1 keeps focus; no transaction mode.
All of @@Alex's report addressed. Torn down (server by PID, chan remove, rm
temp; no broad pkill).

## Status

B4 done - my LAST round-1 item. All of B8 / B1 / B12 / cs mcp_env / B4 landed +
gate-green + verified; nothing pushed. Ready for round-2 R2-3 (per-terminal
survey) when you dispatch it.
