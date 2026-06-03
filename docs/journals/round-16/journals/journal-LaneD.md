# journal-LaneD (append-only)

## 2026-06-02 - task-LaneA-LaneD-1: SPA settings-gap check (gear removal)

Recon (read-only) for the chan-desktop launcher redesign: does
removing the per-row Settings gear strand any setting that is
NOT reachable from inside the chan SPA?

Read: desktop/src/main.js (renderFeaturesToggle/Panel:780-837,
bindFeaturesToggle:978, loadFeaturesInto:1021), desktop/src-tauri/
src/main.rs (get/set_workspace_features:439-829), chan-server
routes (index.rs semantic, reports_toggle.rs), web/src dashboard
config (SearchSlotConfig.svelte, WorkspaceSlotConfig.svelte,
PreflightOverlay.svelte), web/src/api/client.ts.

Finding: NO GAP. The gear toggles exactly two per-workspace
settings, bge (semantic search) and reports. Both are fully
reachable in the SPA:
  - semantic -> SearchSlotConfig.svelte:287 -> /api/index/
    semantic/enable|disable|download
  - reports  -> WorkspaceSlotConfig.svelte:38 -> /api/index/
    reports/enable|disable
Both mounted in DashboardSlotBack.svelte:66-68. The chan-server
route files cite the SPA Settings overlay (fullstack-a-76) as
their reason to exist; SPA is canonical, gear is the duplicate.

Nuance (not a gap): the gear can toggle a workspace without
opening it (transient open); SPA toggles act on the open
workspace only. Minor UX shift, not a stranded setting.

@@LaneC's launcher-inventory-LaneC.md was not on disk yet; did
not block, read source directly.

Deliverable: new-team-1/spa-settings-gap-LaneD.md
Completion task: tasks/task-LaneD-LaneA-1.md
Status: DONE. Poking @@LaneA.

## 2026-06-02 - task-LaneA-LaneD-2: Verify-lane (build + gate + stage)

GO from @@LaneA: @@LaneB (modal frontend) + @@LaneC (gear removal +
dead-code + serve.rs canary reconcile) both landed; 7 files modified.

Re-ran the FULL gate myself (verify-don't-trust):
  fmt --check        PASS
  clippy -D warnings PASS
  build --workspace  PASS
  test --workspace   PASS 1274/0 (34 binaries)
  desktop make build PASS (exit 0)

Built + staged: target/release/bundle/macos/Chan.app (v0.24.0, binary
mtime 20:24:37, linked ~25min after last src edit 19:59:29; frontendDist
brotli-compressed so strings-grep of JS is empty by design, not stale).

Landed-code checks vs LOCKED design: MODAL (D2) showNewWorkspaceDialog +
.nw-choices; gear fully removed (0 JS fns / 0 Rust cmds / 0 perms / 0
canaries); add-time toggles kept (D1); On|Where rows + conn-dot (D3);
single [New], no tagline (D4). All confirmed.

Deliverables:
  new-team-1/smoke-checklist-LaneD.md   (9-section MODAL smoke for @@Alex)
  tasks/task-LaneD-LaneA-2.md           (completion)
Left uncommitted per task ("Do not commit"). WKWebView smoke is @@Alex's
(Chrome MCP is Blink, can't reach it). Poking @@LaneA.
