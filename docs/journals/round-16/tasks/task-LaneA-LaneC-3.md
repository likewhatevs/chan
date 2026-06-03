# task-LaneA-LaneC-3: reconcile serve.rs frontend canaries (gated on @@LaneB)

From: @@LaneA (lead)  To: @@LaneC  Type: build (Rust tests)  Decision: Option A

@@LaneB's launcher redesign rewrites desktop/src/main.js + index.html. The
serve.rs include_str! canary tests pin OLD frontend patterns (pickAndAdd /
showPreflightDialog, the inline tunnel panel, the old open-workspace/
tunnel-btn buttons, etc.) that the redesign intentionally removes or moves.
Those canaries are Rust tests in YOUR lane (src-tauri) and are the ONLY
automated coverage of the launcher's JS<->Rust wiring (desktop/src has no
JS test harness). Reconcile them so cargo test goes green AND the wiring
coverage stays meaningful.

## Sequencing - HOLD then prep

- The FIX must wait for @@LaneB's main.js/index.html to STABILIZE - fixing
  against WIP risks re-breaking. Do NOT do the fix until @@LaneA pokes you
  that @@LaneB has landed.
- You MAY do a read-only PREP pass NOW: inventory every include_str! canary
  in serve.rs (test name, file it reads, the exact string patterns it
  asserts), and tag each as KEEP-AS-IS / UPDATE / DELETE against the design
  doc's new wiring. This makes the post-B reconciliation fast. Read-only,
  no edits.

## Files - single owner

desktop/src-tauri/src/serve.rs (the #[cfg(test)] canary module only).
@@LaneB is frontend-only and will not touch serve.rs - no collision.

## Reconciliation rules (after B lands)

For each include_str! canary asserting on main.js / index.html:
- Pattern B KEEPS or MOVES (e.g. invoke('add_workspace'), invoke('add_
  outbound_workspace'), invoke('tunnel_start'), compute_workspace_preflight,
  renderPreflightReport now inside the modal): UPDATE the assertion to the
  NEW pattern/location so the wiring check survives. Do NOT just delete a
  still-valid wiring check.
- Feature B REMOVES with a successor (pickAndAdd -> the [New] modal Local
  choice; showPreflightDialog -> the in-modal scan; the inline tunnel panel
  -> the modal Inbound/Outbound bodies; #open-workspace/#tunnel-btn -> the
  single #new-workspace button): REPLACE with a canary pinning the NEW
  equivalent (showNewWorkspaceDialog, #new-workspace, the modal's add_*
  invokes) so coverage moves with the feature.
- Feature B removes with NO successor (the per-row gear get/set_workspace_
  features - already handled in task-2): DELETE.
- Goal: green cargo test + meaningful, non-obsolete wiring coverage. If you
  judge the include_str! string-canary approach itself should be rethought
  (it is brittle), FLAG it to @@LaneA as a note - but for THIS task, keep
  the existing approach and just reconcile.

## Gate (own, after the fix)

cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo build
-p chan-desktop; cargo test -p chan-desktop -> ALL GREEN (78+ pass, 0 fail).
Report the new pass count.

## On completion

Cut a completion task to @@LaneA at tasks/task-LaneC-LaneA-3.md
(append-only) - the canary inventory + what you updated/replaced/deleted +
the green test count - + poke. (Post the PREP inventory whenever it is
ready, even before the B-landed poke, so @@LaneA can sanity-check the plan.)

---

## [LaneA] @@LaneB HAS LANDED - proceed with the fix

@@LaneB's frontend is DONE + make build green; @@LaneA verified the tree
(7 files modified, all expected) and B's main.js/index.html are now the
STABLE final shape. Treat them as frozen for this round. Proceed from your
PREP inventory to the actual reconciliation now: reconcile ALL serve.rs
include_str! canaries against B's FINAL main.js/index.html per the rules
above, then gate `cargo test -p chan-desktop` to 0 fail. Report the green
count. (If B's final differs from what your prep assumed, re-audit against
the live file - do not trust a stale prep snapshot.)

---

## [LaneA] task-3 ACCEPTED (cargo test 79/0) + flag decisions

Reconciliation accepted. Exactly the right approach: coverage MOVED with the
feature (#1 repointed to the modal wiring, #5 renamed, #3 msg fixed, #2/#4
kept, 0 deletes), not just made-to-compile. cargo test 79/0 + tunnel_e2e
7/0, clippy/build green. Task-2 red resolved.

FLAG DECISIONS:
- Flag 1 (new-coverage canary for the modal's outbound/inbound JS wiring):
  APPROVED, but SEQUENCED POST-SMOKE. Do NOT add it yet. After @@Alex's
  smoke confirms the frontend is final (no fix needed), @@LaneA pokes you to
  add ONE minimal canary: assert #new-workspace -> showNewWorkspaceDialog +
  main.js contains invoke('add_outbound_workspace') + invoke('tunnel_start').
  Anchor on the function name + invoke calls (stable signals), NOT copy text.
  Deferring avoids churn if the smoke finds a frontend fix + a serve.rs race
  with @@LaneD's verify gate.
- Flag 2 (brittleness rethink of the include_str! canaries): DEFERRED, not
  this round. Noted as a known limitation; a separate hardening task later
  if @@Alex wants it.

HOLD for the post-smoke poke (flag 1). Do not commit (lead commits).
