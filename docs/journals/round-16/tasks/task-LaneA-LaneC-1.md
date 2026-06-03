# task-LaneA-LaneC-1: chan-desktop launcher - CURRENT-CODE inventory

From: @@LaneA (lead)  To: @@LaneC  Type: recon (read-only, no code)

## Context

We are redesigning the chan-desktop launcher (merge [Open workspace] +
[Attach] into a single [New] button opening a 3-choice new-workspace
window; redesign the rows; remove the per-row gear). @@LaneB owns the
design doc and needs a precise inventory of what the CURRENT launcher
does so the design is grounded, not guessed. This is your job.

The draft (for context only): docs/journals/phase-16/desktop-redesign-draft/draft.md

## Scope (read-only)

Inventory the current chan-desktop launcher:
- desktop/src/index.html
- desktop/src/main.js  (~1400 lines)
- desktop/src/styles.css
- desktop/src-tauri/src/*.rs (the Tauri commands the launcher invokes:
  start with main.rs, serve.rs, registry.rs, tunnel/*, config.rs)

Report, with file:line evidence:
1. The [Open workspace] flow: handler (pickAndAdd), which Tauri command it
   invokes, the native folder-picker path, how the chosen dir is registered.
2. The [Attach] flow: toggleTunnelPanel -> the Open-by-URL OUTBOUND form and
   the Receive-a-remote INBOUND port-listen form; which Tauri commands each
   invokes; how each result becomes a workspace row.
3. Row rendering: how rows are built in main.js, the columns, the on/off
   toggle, the icon logic (computer vs home), and the URL / outbound tags
   (the `tag-tunnel` / `tag-outbound` spans).
4. The per-row gear (Settings): exactly what it toggles (semantic search?
   reports? anything else?), the features-panel markup, and the Tauri
   command/route each toggle calls.
5. Tauri window machinery: is there any existing code that opens a NEW
   window (WebviewWindow / tauri window config)? This decides whether the
   [New] window can be a real second window or should be an in-launcher
   modal. Note window labels / the `w=<window-label>` URL param machinery.

## Deliverable

new-team-1/launcher-inventory-LaneC.md - concise, evidence-backed, one
section per item above. ASCII tables ok (80 col), no em dashes.

## Constraints

- Read-only. No code edits.
- Do not block on @@LaneD; you may both read main.js concurrently (read-only,
  no collision).

## On completion

Cut a completion task back to @@LaneA at tasks/task-LaneC-LaneA-1.md
(append-only) pointing at the inventory, then poke @@LaneA.
