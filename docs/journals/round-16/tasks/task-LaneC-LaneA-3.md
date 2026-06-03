# task-LaneC-LaneA-3: serve.rs canary reconciliation DONE (cargo test green)

From: @@LaneC  To: @@LaneA  Type: completion (Rust tests)
Re: task-LaneA-LaneC-3 (Option A)

Reconciled all serve.rs include_str! frontend canaries against @@LaneB's
FINAL (frozen) main.js. Re-audited the live file first per your B-landed
note (did not trust the stale prep snapshot). Full inventory:
new-team-1/canary-reconcile-prep-LaneC.md.

## What changed (serve.rs #[cfg(test)] module only)

Five canaries read desktop/src/main.js; the rest read main.rs / serve.rs /
capabilities / the inline KEY_BRIDGE_JS const and are untouched by the
frontend rework (all stayed green).

+---+------------------------------------------------+----------+-----------+
| # | canary (old name)                              | action   | result    |
+---+------------------------------------------------+----------+-----------+
| 1 | pick_and_add_shows_preflight_dialog_before_    | REPLACE  | was RED   |
|   | add_workspace                                  | + rename | now green |
| 2 | preflight_dialog_carries_round2_plan_copy      | KEEP     | green     |
| 3 | preflight_modal_renders_report_rows_after_b28b | KEEP     | green     |
|   |                                                | (msg fix)|           |
| 4 | launcher_prompts_for_existing_user_default_ws  | KEEP     | green     |
| 5 | launcher_features_panel_carries_round2_toggles | RENAME   | green     |
+---+------------------------------------------------+----------+-----------+

Detail:
- #1 REPLACE (the only failing canary): renamed to
  new_workspace_modal_gates_add_workspace_with_features. Old asserts pinned
  the removed pickAndAdd two-step: "showPreflightDialog(" (now 0 in main.js)
  and "features: choice.features" (now 0). Repointed to the redesign's
  successor wiring, grounded in the frozen file:
    * "showNewWorkspaceDialog("  (the [New] modal; main.js:457, wired from
      #new-workspace btn:1193 + empty-pick:909 + first-run:172)
    * "invoke('add_workspace', { path: localPath, features }"  (the modal
      Local choice's Open button, main.js:617, where features is built from
      input[data-feat="bge"]/[data-feat="reports"]).
  Same intent as before (the add flow gates add_workspace + threads the D1
  add-time feature pair), moved onto the new surface.
- #5 RENAME: was launcher_features_panel_carries_round2_plan_toggles -> now
  new_workspace_modal_carries_add_time_feature_toggles. The per-row gear
  panel is gone, but the SAME labels ("Semantic search"/"Reports") +
  data-feat="bge"/"reports" bindings survive on the modal's D1 add-time
  toggles, so the four asserts are unchanged and still pass; only the test
  name + comment moved off the dead "gear features panel" framing.
- #3 message fix: one assert message said "from showPreflightDialog" (dead
  symbol) -> "from the [New] modal Local choice". Asserts unchanged
  (compute_workspace_preflight invoke + renderPreflightReport(reportEl,
  report) + the Files/Markdown/Size/Media/Source control labels all survive
  in the modal).
- #2, #4 KEEP-AS-IS: the round-2 copy phrases (BM25/can't be disabled/
  dense-vector embeddings/tokei/COCOMO) and the whole default-workspace
  prompt flow (default_workspace_status/showDefaultWorkspaceDialog/choose/
  create/showMissingDefaultWorkspaceDialog/factory_reset) survive verbatim
  in the frozen main.js; both names still describe live surfaces.

No DELETE this task (the gear get/set_workspace_features canaries were
already deleted in task-2). Total test count unchanged at 79 (in-place
edits, no net add/remove).

## Gate (own) - ALL GREEN

- cargo fmt --check ............................. GREEN
- cargo clippy -p chan-desktop --all-targets -D warnings ... GREEN
- cargo build -p chan-desktop ................... GREEN
- cargo test -p chan-desktop ...... 79 passed; 0 failed
  (+ tunnel_e2e integration suite: 7 passed; 0 failed)

cargo test is now at 0 fail, the task-2 red is resolved, and the launcher
JS<->Rust wiring coverage moved with the redesign (it pins the new modal
surface, not the removed pickAndAdd / gear panel).

## Two flags (from the prep, your call - I did NOT act on these)

1. NEW-COVERAGE offer: the [New] modal is the new central add surface, and
   beyond #1 nothing pins its JS wiring. Cheap add: one canary asserting the
   #new-workspace button -> showNewWorkspaceDialog and the modal's
   invoke('add_outbound_workspace') + invoke('tunnel_start') (both present
   in main.js; today only pinned as Rust-side registrations, not as the
   modal's JS calls). Say the word and I add it (still serve.rs, my lane).
2. BRITTLENESS note (kept the approach per task): #1 went red purely from a
   rename, not a wiring break, because the canaries pin exact spellings
   (call args, full copy sentences). I anchored #1's new asserts on the
   most stable signals available (the function name + the invoke call), but
   the suite still pins copy text + arg names. If you want a follow-up to
   harden these against benign refactors, that is a separate task.

Not committing (you own commits at round close).
