# spa-settings-gap-LaneD: launcher gear vs SPA settings

From: @@LaneD  To: @@LaneA  Type: recon finding (read-only)
Gates: @@LaneB design (task-LaneA-LaneB-1)

## Bottom line

NO GAP. The per-row gear exposes exactly two per-workspace
settings (semantic search, reports). BOTH are fully reachable
and configurable from inside the chan SPA today, through their
own chan-server routes and live Dashboard config UI. Removing
the launcher gear strands nothing. The gear is redundant with
the SPA, not the sole home of any setting.

Both chan-server route files even name the SPA as their reason
for existing ("Unblocks fullstack-a-76, SPA Settings overlay's
Features", reports_toggle.rs:11 and index.rs context). The SPA
is the canonical home; the launcher gear is the duplicate.

## What the gear toggles (confirmed from source)

desktop/src/main.js renderFeaturesPanel() (main.js:802) + the
add-workspace pre-flight modal (main.js:489) expose the same two
checkboxes, data-feat="bge" and data-feat="reports". The gear
handler bindFeaturesToggle() (main.js:978) wires them to Tauri
IPC set_workspace_features (main.rs:762), which calls through to:

  bge     -> chan_workspace Workspace::set_semantic_enabled
  reports -> chan_workspace Workspace::set_reports_enabled (+boot)

(main.rs:814-827, apply_workspace_features_blocking). Both are
live and functional today, not a dead stub: loadFeaturesInto()
clears the disabled attr on first panel open (main.js:1027).

## Per-setting table

  setting  | launcher path             | SPA location           | verdict
  ---------+---------------------------+------------------------+--------
  Semantic | gear checkbox bge ->       | SearchSlotConfig       | IN SPA
  search   | set_workspace_features ->  | .svelte:287 toggle ->  |
  (bge)    | Workspace::               | api.semanticEnable/    |
           | set_semantic_enabled      | Disable -> POST /api/  |
           |                           | index/semantic/enable| |
           |                           | disable (index.rs)     |
  ---------+---------------------------+------------------------+--------
  Reports  | gear checkbox reports ->   | WorkspaceSlotConfig    | IN SPA
           | set_workspace_features ->  | .svelte:38 toggle ->   |
           | Workspace::               | api.reportsEnable/     |
           | set_reports_enabled       | Disable -> POST /api/  |
           |                           | index/reports/enable|  |
           |                           | disable                |
           |                           | (reports_toggle.rs)    |

Both SPA toggles are mounted in DashboardSlotBack.svelte:66-68
(the Dashboard slot flip-back config surface), so they are
reachable UI, not orphaned components.

## Routes + client cites

Semantic (chan-server lib.rs:861-864, routes/index.rs):
  POST /api/index/semantic/enable   set_semantic_enabled(true)
  POST /api/index/semantic/disable  set_semantic_enabled(false)
  POST /api/index/semantic/download  (BGE-small model fetch)
  PATCH /api/index/semantic/model
  api client: web/src/api/client.ts:1035-1036

Reports (chan-server lib.rs:870-871, routes/reports_toggle.rs):
  POST /api/index/reports/enable    set_reports_enabled(true)+boot
  POST /api/index/reports/disable   set_reports_enabled(false)
  api client: web/src/api/client.ts:1045-1047

SPA UI:
  web/src/components/dashboard/SearchSlotConfig.svelte
    line 287 "Enable semantic search (Hybrid mode)" checkbox;
    calls semanticEnable/Disable/Download (136,155,168)
  web/src/components/dashboard/WorkspaceSlotConfig.svelte
    line 38 setReportsEnabled -> reportsEnable/Disable (43)
  web/src/components/PreflightOverlay.svelte
    onboarding path also offers semantic Download & enable
    (180-213, 342)

## One nuance (not a gap, worth a design note)

The launcher gear can toggle bge/reports for a workspace WITHOUT
opening it: set_workspace_features does a transient
open_workspace if the workspace is not already mounted
(resolve_workspace_for_features, main.rs:504). The SPA toggles
only act on the workspace you are currently inside. So removing
the gear shifts these from "configure any registered workspace
from the launcher" to "configure each workspace from inside its
own SPA". Given chan is local-first single-user, and you open a
workspace to work in it, this is a minor UX shift, not a
stranded setting. The redesign does not need to add anything to
the SPA; it may optionally note that out-of-workspace bulk
toggling goes away. No back-compat concern (pre-release).

## Recommendation for @@LaneB

Proceed with removing the per-row gear. No new SPA settings
surface is required; both settings already live in the SPA
Dashboard config (Search slot + Workspace slot). If you want to
preserve discoverability, a one-line pointer in the launcher
("feature toggles live in the workspace Dashboard") is optional
polish, not a required gap fill.
