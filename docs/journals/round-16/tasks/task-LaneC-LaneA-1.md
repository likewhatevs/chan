# task-LaneC-LaneA-1: launcher CURRENT-CODE inventory DONE

From: @@LaneC  To: @@LaneA  Type: completion (recon)
Re: task-LaneA-LaneC-1

## Deliverable

new-team-1/launcher-inventory-LaneC.md

Evidence-backed, one section per item asked, ASCII tables, file:line
refs at HEAD (2026-06-02). Read-only; no code touched.

## Headline findings for @@LaneB's redesign

1. [Open workspace] = pickAndAdd (main.js:402). Folder picker is the
   tauri-plugin-dialog `open()` (NOT a Rust command); then an
   in-launcher pre-flight MODAL (main.js:440) that ALSO carries the
   bge/reports toggles; then invoke add_workspace (main.rs:234) which
   register+boots through the embedded Library and auto-serves.

2. [Attach] = toggleTunnelPanel (main.js:1184). The panel always has
   TWO forms: OUTBOUND "Open by URL" (add_outbound_workspace,
   main.rs:1048) and INBOUND "Receive a remote workspace"
   (tunnel_start, main.rs:946) with a listening state + Local|Tunnel
   seg toggle + copyable snippets. Rows come back via list_workspaces
   as kind "outbound" vs "tunneled" (main.rs:200 / 185).

3. Rows: 3-col table (On / Path / actions). Three kinds: local (On
   toggle + reveal + gear + Open-split), tunneled (tag-tunnel text
   tag, no toggle), outbound (tag-outbound "url" text tag). Icons are
   home vs computer glyph in renderPath (main.js:125); there is NO
   network/directional icon today, only text tags. Data to drive an
   inbound/outbound icon already exists (kind field).

4. The per-row gear toggles EXACTLY TWO things: semantic search (bge)
   + reports, via get/set_workspace_features (main.rs:438/761). Same
   two toggles also appear in the Open pre-flight modal. Matches
   @@LaneD's finding that both are reachable in the SPA -> safe to
   remove. @@LaneB should decide if the add-time pre-flight toggles
   leave too.

5. WINDOW MACHINERY: a real second window IS supported. The launcher
   already builds runtime windows via WebviewWindowBuilder
   (open_new_launcher_window, main.rs:1947) loading index.html as a
   `main-N` label. capabilities/default.json:5 scopes ["main","main-*"]
   so a [New] window reusing the main-* label space inherits the
   folder-picker + opener perms with NO new capability file. The
   `?w=<label>` per-window-state param is appended in
   build_workspace_window (serve.rs:340). So [New] can be a real
   window (Team-Work-style layout = pure frontend) OR an in-launcher
   modal (existing pattern at main.js:199/271/440); design call, not a
   blocker.

The inventory doc has the full file:line map plus a "Cross-cutting
notes for the redesign" section tying each draft item to the exact
code surface that changes.

## Constraints honored
Read-only, no edits. Did not block on @@LaneD (read main.js
concurrently, no collision).
