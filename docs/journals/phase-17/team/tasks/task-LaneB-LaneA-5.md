# task-LaneB-LaneA-5: B4 cross-lane boundary - needs store.svelte.ts (@@LaneC)

From: @@LaneB  To: @@LaneA  Re: task-LaneA-LaneB-5 (B4, unblocked)

## STOP + route: B4 pulls into @@LaneC's store.svelte.ts (not listed in the task)

I recon'd the full `cs pane split` flow against HEAD before editing. B4 cannot
be done within the files the task listed (cli.rs, tabs.svelte.ts pane region,
Pane.svelte, control_socket.rs pane-exec) - both parts land in
`web/src/state/store.svelte.ts::applyPaneExec`, which is @@LaneC's lane (and
@@LaneC is likely in it now for B9 graph). Per the bootstrap "STOP + route if a
fix pulls you into another lane's file", I'm holding B4 edits for your call.

## The actual flow (grounded)

`cs pane split` -> control socket -> chan-server handle_pane_exec (forwards the
PaneOp OPAQUELY via WindowCommand::PaneExec, no dir interpretation) -> SPA
store.svelte.ts `applyPaneExec` -> `splitPane`.

- control_socket.rs pane-exec region: NO change needed (it serde-forwards the
  op; it never reads SplitDir). So the only Rust change is the chan-shell
  enum.
- The CLI hamburger already does right/bottom via app.pane.splitRight/Down
  (splitActive) - that path is fine. The BUG is the `cs pane` path
  (applyPaneExec), which still does left/bottom + steals focus.

## Part 1 (RIGHT|BOTTOM) - lockstep across two lanes

- MINE (chan-shell): cli.rs `SplitDirArg { Left, Bottom }` -> `{ Right, Bottom }`
  (+ help text + the From impl), wire.rs `SplitDir { Left, Bottom }` ->
  `{ Right, Bottom }`.
- @@LaneC (store.svelte.ts): `PaneExecOp` split variant `dir: "left" | "bottom"`
  -> `"right" | "bottom"` (line ~781); `applyPaneExec` split case (line ~813)
  `if (op.dir === "left") splitPane(p.id, "row", "before")` ->
  `if (op.dir === "right") splitPane(p.id, "row", "after")` (new pane on the
  RIGHT = row/after); `else` stays `splitPane(p.id, "column", "after")` (bottom).
These MUST land together (the wire dir string must match what applyPaneExec
checks), so this is a true cross-lane lockstep, not interleave-safe.

## Part 2 (no focus steal / no transaction)

- Focus steal: `applyPaneExec` calls `splitPane` (tabs.svelte.ts:2953), which
  hardcodes `layout.activePaneId = newPane.id` (2971) -> the cs split moves
  focus to the new empty pane (@@Alex's "took focus away from the terminal").
  splitPane is shared by keyboard/UI splits that SHOULD focus the new pane, so
  the fix belongs in the cs path: `applyPaneExec` should preserve the sender's
  focus for split (capture activePaneId before, restore after) UNLESS the op
  targets focus. That edit is in store.svelte.ts (@@LaneC). (Alternative: add a
  `focusNew=false` arg to splitPane in tabs.svelte.ts [MINE] and have
  applyPaneExec pass it [@@LaneC] - still needs store.svelte.ts.)
- Transaction mode: applyPaneExec uses splitPane/closePane (NOT paneModeSplit /
  enterPaneModeTransaction), so the cs path does NOT enter hybrid-nav
  transaction mode today. @@Alex's "stuck perhaps in transaction mode" looks
  already-satisfied for the cs path; I'll confirm on the browser smoke. No
  paneModeSplit/enterPaneModeTransaction change appears needed.

## Decision for you (pick one)

(A) Authorize me the `applyPaneExec` REGION of store.svelte.ts for B4 (PaneExecOp
    split variant + the split case + the focus-preserve), coordinating the
    store.svelte.ts merge with @@LaneC the same way I share tabs.svelte.ts
    regions. I land B4 atomically (chan-shell wire/cli + applyPaneExec in one
    lockstep). RECOMMENDED - the wire dir + applyPaneExec must change together,
    so one owner avoids a cross-lane broken window. Needs @@LaneC clear of the
    applyPaneExec region during my burst.
(B) Reassign the store.svelte.ts::applyPaneExec edits to @@LaneC; I supply the
    exact spec above + land the chan-shell wire/cli half in lockstep with
    @@LaneC's store.svelte.ts half (you sequence the two so neither lands alone).

Either way: do NOT land the chan-shell SplitDir change alone - it breaks the cs
split until applyPaneExec matches.

## Status

Holding ALL B4 edits pending your ruling. The chan-shell wire/cli half is ready
to land the moment the store.svelte.ts ownership is decided. Everything else of
mine (B8, B1, B12, cs mcp_env) is done + gate-green + verified.
