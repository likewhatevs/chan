# followup-LaneA-LaneB-1: B4 ruling = Option (A)

From: @@LaneA  To: @@LaneB  Re: task-LaneB-LaneA-5 (B4 boundary)

Great recon. RULING: **Option (A)** - you own the `applyPaneExec` region for B4
and land it atomically. The wire dir string and the applyPaneExec check must
change together, so a single owner is the only race-free path.

## Authorization (task-spec, inline / on record)

@@LaneB may edit, for B4 only:
- web/src/state/store.svelte.ts - the `PaneExecOp` split variant (`dir`) +
  the `applyPaneExec` split case + the focus-preserve. (This is @@LaneC's file;
  authorized for the applyPaneExec region ~781-813 ONLY. Stay out of @@LaneC's
  B9 graph region ~1881-2052, which is done + will commit as C's.)
- web/src/state/tabs.svelte.ts `splitPane` (~2953) IF you take the `focusNew`
  arg approach - that's your pane region, fine.
- crates/chan-shell/src/{cli.rs, wire.rs} SplitDir Left->Right.
- control_socket.rs: NO change (confirmed - it forwards opaquely).

## Land it as ONE lockstep burst

chan-shell SplitDir (Left->Right) + store.svelte.ts PaneExecOp dir string
(`"left"`->`"right"`) + applyPaneExec split case (right = row/after) +
focus-preserve - all together, re-check `cargo check -p chan-shell` + svelte
typecheck green before pausing. Do NOT land the chan-shell half alone (breaks
the cs split until applyPaneExec matches), exactly as you flagged.

Focus mechanism: your choice (capture/restore activePaneId in applyPaneExec, OR
focusNew=false arg on splitPane). Pick whichever is cleaner given the call
sites. Transaction-mode: agreed it already looks satisfied for the cs path -
confirm on the browser smoke.

## Coordination

- @@LaneC is on R2-2 (editor list-paste); it does NOT touch store.svelte.ts. I'm
  sending it a defensive heads-up to stay clear of applyPaneExec. Re-check
  store.svelte.ts is quiescent (no concurrent edit / stable mtime) right before
  your burst.
- COMMIT NOTE: store.svelte.ts will carry C's B9 region + your B4 applyPaneExec
  region. I commit the merged file once at round close. In your B4 report, list
  WHICH store.svelte.ts lines are B4 so I attribute cleanly.

## Gate + report

Same as task-LaneA-LaneB-5 (cargo chan-shell + chan-server unaffected; web-check;
browser-smoke split RIGHT/BOTTOM + no focus-steal + no transaction-mode). Cut
task-LaneB-LaneA-6 + poke.
