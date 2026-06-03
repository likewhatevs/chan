# task-LaneA-LaneB-1: chan-desktop launcher redesign - DESIGN doc

From: @@LaneA (lead)  To: @@LaneB  Type: design (no code)

## Context

The chan-desktop launcher redesign was DRAFTED in phase-16 but never
built. @@Alex wants it built now, design-first (phase-16's clearest
lesson: design-first prevents bad builds). You own the design doc; once
@@Alex signs off, @@LaneA dispatches the build across lanes.

The loose draft + 3 reference screenshots:
- docs/journals/phase-16/desktop-redesign-draft/draft.md
- docs/journals/phase-16/desktop-redesign-draft/image.png   (current launcher)
- docs/journals/phase-16/desktop-redesign-draft/image-1.png (today's Open workspace = native folder picker)
- docs/journals/phase-16/desktop-redesign-draft/image-2.png (today's Attach panel: Open-by-URL outbound + Receive-remote inbound)

Current code (read, do not change):
- desktop/src/index.html  (header markup, ~43 lines)
- desktop/src/main.js      (~1400 lines: handlers, row render, tunnel panel)
- desktop/src/styles.css

## What the draft asks for

1. Header: merge [Open workspace] + [Attach] into a single [New] button.
   Final header row: `[enso] Workspaces ... [New] [sun/moon]`. Keep the
   theme toggle. (No Settings button in the header today; the draft's
   "remove SETTINGS" refers to the per-ROW gear, see #2.)
2. Per-row redesign: columns `ON | WHERE` (today `ON | PATH`). Row =
   `[on/off toggle] [computer|home|network icon] [path or URL] ... [Open + dropdown]`.
   Remove the per-row Settings gear (the draft: those settings will only
   exist inside chan's SPA). For URL workspaces, show a clear INBOUND vs
   OUTBOUND indication (we listen vs we connect to).
3. The [New] button opens a NEW WINDOW presenting 3 choices:
   a. Local directory (a git repo / any directory)
   b. Remote attached OUTBOUND (we connect to a URL)
   c. Remote attached INBOUND (we listen for an incoming connection)
   @@Alex wants this widget to "resemble the Team Work one": selecting the
   real estate and switching tabs vs split panes changes the layout and
   options; each of the 3 choices shows a different layout. Specify this
   interaction model CONCRETELY, mapping each choice to the existing flow
   it replaces (folder picker / Open-by-URL outbound / port-listen inbound
   from image-1/image-2).

## Inputs arriving from peers (fold these in; @@LaneA re-pokes when ready)

- @@LaneC: current-launcher code inventory -> new-team-1/launcher-inventory-LaneC.md
  (what each handler does, which Tauri commands the flows invoke, the gear's
  exact toggles, any existing Tauri window-creation machinery).
- @@LaneD: SPA settings-gap check -> new-team-1/spa-settings-gap-LaneD.md
  (do the settings the per-row gear exposes already exist inside the SPA?
  any GAP the redesign must cover before we remove the gear?).

Start now on the parts that do not depend on C/D (read the draft + the 3
images + skim the current code). Integrate C's inventory and D's gap
finding when @@LaneA points you to them.

## Deliverable

new-team-1/desktop-redesign-design.md, covering:
- Header redesign: final markup + CSS approach.
- Row redesign: ON|WHERE, icon set, INBOUND/OUTBOUND indication, gear removal
  (and where its toggles land per D's finding).
- The [New] window: a NEW Tauri window vs an in-launcher modal (recommend
  one, with rationale grounded in C's window-machinery finding); the 3-choice
  layout + the Team-Work-like tabs/split-pane interaction model, concretely.
- Implementation plan: which files change, any new Tauri command/window,
  a build + browser-smoke test plan, and a proposed lane split for the build.
- Open questions for @@Alex (e.g. new-window-vs-modal if you can't decide it
  yourself; any gear-removal SPA gap from D).

## Constraints

- Design only. No code edits this task.
- Writing rules: no em dashes; ASCII tables target 80 cols; factual, no
  marketing. Comments/docs explain WHY.
- Keep it buildable: someone should be able to implement from this doc
  without re-deriving the draft.

## On completion

Cut a completion task back to @@LaneA at tasks/task-LaneB-LaneA-1.md
(append-only, recipient = @@LaneA) pointing at the design doc + listing the
open questions for @@Alex, then poke @@LaneA.

---

## [LaneA append] Peer recon is IN - both findings ready

@@LaneC and @@LaneD both finished. Read both before finalizing:
- new-team-1/launcher-inventory-LaneC.md  (current-code map, file:line)
- new-team-1/spa-settings-gap-LaneD.md     (gear-removal gap check)

Recon resolved the gating question and surfaced specific design calls:

1. GEAR REMOVAL: confirmed SAFE. Gear toggles EXACTLY bge (semantic
   search) + reports; both reachable in the SPA Dashboard config (D:
   SearchSlotConfig.svelte:287, WorkspaceSlotConfig.svelte:38, mounted
   DashboardSlotBack.svelte:66-68). No new SPA surface needed.
   - NUANCE (D): the gear toggles these WITHOUT opening the workspace;
     SPA toggles only act on the OPEN workspace. Removing the gear drops
     out-of-workspace toggling. Minor local-first UX shift, not a
     stranded setting. Note it in the design; not a blocker.
   - DECISION FOR YOU (C): the SAME bge/reports toggles ALSO appear in
     the [Open workspace] add-time PRE-FLIGHT modal (main.js:440). If the
     principle is "these settings live only in the SPA," the pre-flight
     toggles arguably leave too. Decide + state it (consistency call).

2. [New] WINDOW vs MODAL: a real second window IS supported with NO new
   capability file - WebviewWindowBuilder / open_new_launcher_window
   (main.rs:1947), `main-N` label, capabilities scoped ["main","main-*"]
   (default.json:5), `?w=<label>` per-window state (serve.rs:340). So
   [New] can be a real window (Team-Work-style layout = pure frontend)
   OR an in-launcher modal (existing pattern main.js:199/271/440). This
   is YOUR design call - recommend one with rationale (the draft says
   "open a new window," which the machinery already supports).

3. The 3 choices map cleanly to existing flows (reuse, do not reinvent):
   - Local directory -> tauri-plugin-dialog open() picker + add_workspace
     (main.rs:234), today's pickAndAdd path (main.js:402).
   - Remote OUTBOUND -> add_outbound_workspace (main.rs:1048), today's
     "Open by URL" form.
   - Remote INBOUND -> tunnel_start (main.rs:946), today's "Receive a
     remote workspace" port-listen form.
   Rows already carry a `kind` field (local / outbound / tunneled,
   main.rs:185/200) to drive the INBOUND/OUTBOUND icon; today there is
   only a text tag, no directional icon (C3).

Proceed to finalize the design doc.
