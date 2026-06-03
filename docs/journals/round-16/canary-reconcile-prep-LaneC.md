# canary-reconcile-prep-LaneC: serve.rs include_str! canary inventory

Task: task-LaneA-LaneC-3 (Option A). PREP pass = read-only inventory +
KEEP/UPDATE/REPLACE/DELETE tag per canary against the design doc's new
wiring. FIX IS HELD pending @@LaneA's "B-landed" poke (fixing against WIP
risks re-breaking). No edits made.

Status grounding: read serve.rs #[cfg(test)] module + cross-checked each
canary's exact strings against @@LaneB's CURRENT WIP main.js/index.html
(read-only) and the design doc (new-team-1/desktop-redesign-design.md).
WIP is not yet declared stable, so all "still present in WIP" notes are
provisional and MUST be re-verified against the LANDED main.js before the
fix.

## Headline

Only ONE canary actually FAILS today: pick_and_add_shows_preflight_dialog_
before_add_workspace (the task-2 cargo-test red). The other four frontend
canaries currently PASS on @@LaneB's WIP because the redesign preserved
their pinned strings. So the reconciliation is small and surgical:
- 1 REPLACE (the failing #1: pickAndAdd/showPreflightDialog -> the [New]
  modal Local choice).
- 3 KEEP-AS-IS (verify the strings survive once B lands).
- 1 KEEP-but-RENAME (#5: asserts survive via the D1 add-time modal toggles,
  but its NAME/comment still say "features panel" = the removed gear).
No frontend canary needs a pure DELETE (the gear get/set canaries were
already deleted in task-2).

## Census: every include_str! canary in serve.rs

Rust/internal canaries (read main.rs / serve.rs / capabilities / the inline
KEY_BRIDGE_JS const) are UNAFFECTED by @@LaneB's frontend rework -> all
KEEP-AS-IS. Only the five reading desktop/src/main.js are in scope.

+----+--------------------------------------------------+-----------+--------+
| #  | test fn (serve.rs:line)                          | reads     | verdict|
+----+--------------------------------------------------+-----------+--------+
|    | invoke_handler_registers_reload_window... (715)  | main.rs   | KEEP   |
|    | key_bridge_wires_zoom_chords_to_ipc (732)        | KBJS const| KEEP   |
|    | key_bridge_wires_cmd_shift_i... (752)            | KBJS const| KEEP   |
|    | invoke_handler_registers_zoom_commands (761)     | main.rs   | KEEP   |
|    | invoke_handler_registers_outbound_attach (773)   | main.rs   | KEEP   |
|    | invoke_handler_registers_default_workspace (784) | main.rs   | KEEP   |
| 1  | pick_and_add_shows_preflight_dialog... (797)     | main.js   | REPLACE|
| 2  | preflight_dialog_carries_round2_plan_copy (814)  | main.js   | KEEP   |
|    | invoke_handler_registers_compute_preflight (844) | main.rs   | KEEP   |
| 3  | preflight_modal_renders_report_rows (855)        | main.js   | KEEP   |
|    | registry_and_feature_commands_in_process (886)   | main.rs   | KEEP   |
|    | bin_status_machinery_is_gone (918)               | main.rs/  | KEEP   |
|    |                                                  | serve.rs  |        |
| 4  | launcher_prompts_for_existing_default_ws (947)   | main.js   | KEEP   |
| 5  | launcher_features_panel_carries_toggles (976)    | main.js   | RENAME |
|    | new_window_accelerator_uses_cmd_shift_n (1001)   | main.rs   | KEEP   |
|    | key_bridge_wires_reload_and_devtools_ipc (1019)  | KBJS const| KEEP   |
|    | embedded_url_prefix_parser_* (1040, 1047)        | logic     | KEEP   |
|    | key_bridge_invokes/drops/keeps_* (1055/68/85)    | KBJS const| KEEP   |
|    | workspace_title_is_the_path_verbatim (1117)      | logic     | KEEP   |
|    | workspace_capability_* (1194, 1220)              | caps json | KEEP   |
|    | app_acl_allows_workspace_window_commands (1237)  | app.toml  | KEEP   |
|    | default_capability_covers_extra_launchers (1255) | caps json | KEEP   |
+----+--------------------------------------------------+-----------+--------+

Note: the Rust-side IPC-registration canaries (773 outbound, 784 default-ws,
844 compute_preflight, 886 set_semantic/reports) stay valid because the
redesign KEEPS those commands (the [New] modal's three add-flows still call
add_workspace / add_outbound_workspace / tunnel_start, and register_and_boot
still calls set_semantic_enabled/set_reports_enabled for the D1 add-time
toggles). They passed in the task-2 run; no change.

## The five frontend canaries in detail

### #1 pick_and_add_shows_preflight_dialog_before_add_workspace (797) REPLACE
Current asserts:
  - MAIN_JS.contains("showPreflightDialog(")     -> WIP: only a COMMENT
    remains ("modeled on showPreflightDialog"); the call is gone. FAILS.
  - MAIN_JS.contains("features: choice.features") -> WIP: 0 occurrences.
    The modal threads features differently now. FAILS.
Design (LOCKED D2 = MODAL; §4.3): pickAndAdd is removed (WIP: `function
pickAndAdd` = 0); its job folds into showNewWorkspaceDialog()'s Local
choice, which runs the in-modal scan + the D1 feature toggles, then calls
add_workspace { path, features }.
Planned REPLACE (verify exact strings against LANDED main.js):
  - assert main.js contains "showNewWorkspaceDialog("  (WIP: 4 occurrences)
  - assert the Local choice invokes "invoke('add_workspace'" (WIP: present)
    AND threads the chosen feature pair (find the modal's real features
    payload spelling; "features: choice.features" is dead - likely a
    "features: { bge..." / "features:" object built in the modal).
  - RENAME the test (e.g. new_workspace_modal_gates_add_with_features) +
    rewrite the comment (no pickAndAdd).
This is the ONLY hard fix and the only currently-red canary.

### #2 preflight_dialog_carries_round2_plan_explanatory_copy (814) KEEP
Asserts copy phrases: "BM25 keyword search is", "can't be disabled",
"dense-vector embeddings", "tokei", "COCOMO".
WIP: ALL FIVE present (1 each). Design §4.3 reuses the .preflight-toggle
copy in the modal Local body; MODAL path keeps it in main.js (LOCKED block
forbids launcher-common.js). KEEP-AS-IS; re-verify the 5 phrases survive
post-land. Optional comment tweak (s/dialog/modal Local choice/).

### #3 preflight_modal_renders_report_rows_after_b28b_iv (855) KEEP
Asserts: "invoke('compute_workspace_preflight'", "renderPreflightReport(
reportEl, report)", and labels 'Files'/'Markdown'/'Size'/'Media'/'Source
control'.
WIP: invoke present (1), renderPreflightReport(reportEl, report) present (1).
Currently PASSES. Design §4.3 keeps the scan + renderer in the modal Local
body. KEEP-AS-IS; re-verify the exact "renderPreflightReport(reportEl,
report)" call survives (if @@LaneB renames the element var, downgrade to
"renderPreflightReport(" + the labels).

### #4 launcher_prompts_for_existing_user_default_workspace (947) KEEP
Asserts: invoke('default_workspace_status'), showDefaultWorkspaceDialog,
invoke('choose_default_workspace'), invoke('create_default_workspace'),
showMissingDefaultWorkspaceDialog, invoke('factory_reset_default_workspace').
WIP: showDefaultWorkspaceDialog present (2); currently PASSES. The default-
workspace prompt flow is NOT part of the launcher redesign (§4.7 only
reroutes the empty-state + first-run to open the [New] modal). KEEP-AS-IS;
re-verify post-land.

### #5 launcher_features_panel_carries_round2_plan_toggles (976) RENAME
Asserts: "Semantic search", "Reports", data-feat="bge", data-feat="reports".
WIP: data-feat="bge" (2), data-feat="reports" (2), "Semantic search" (1),
"Reports" (1) -> all present; currently PASSES. The per-row GEAR panel
(renderFeaturesPanel/renderFeaturesToggle) is removed (WIP: both = 0), BUT
the SAME labels + data-feat bindings now live on the D1 add-time toggles in
the [New] modal Local choice. So the asserts stay valid - they just cover a
different surface. Action: KEEP the asserts, but RENAME the test (e.g.
new_workspace_modal_carries_add_time_feature_toggles) + rewrite the comment
so coverage clearly targets the modal toggles, not the deleted gear. No
assert change expected; re-verify post-land.

## Flags for @@LaneA (not blocking)

1. NEW-COVERAGE GAP (recommend adding one canary): the [New] modal is the
   new central add surface and has NO direct JS-wiring canary beyond #1.
   The three add-flow IPCs are pinned only on the Rust side (registration
   canaries). Suggest one new canary pinning the modal's JS invokes:
   #new-workspace button -> showNewWorkspaceDialog, and the modal calling
   invoke('add_workspace') + invoke('add_outbound_workspace') +
   invoke('tunnel_start'). Cheap, and it moves the "launcher wiring"
   coverage onto the new surface. Will add iff you want it (else I keep the
   task scoped to reconciling the existing five).

2. BRITTLENESS NOTE (keeping the approach per task): several asserts pin
   over-specific internals - exact call-arg spellings ("renderPreflight-
   Report(reportEl, report)", "features: choice.features") and whole copy
   sentences. #1 went red purely from a rename, not a wiring break. During
   the fix I'll anchor on stable signals (function names, invoke('cmd')
   names) where possible to cut future false-reds, without weakening the
   wiring intent. Flagging only; no redesign of the canary approach in
   this task.

## Post-B plan + gate

When @@LaneA pokes "B landed/stable":
1. Re-grep the LANDED main.js for every string above; confirm #2/#3/#4/#5
   still pass and lock #1's new strings.
2. REPLACE #1 (asserts + name + comment); RENAME #5 (+ comment); touch
   #2/#3 comments only if copy/args moved; (optionally) add the new [New]-
   modal wiring canary if authorized.
3. Gate (own): cargo fmt --check; cargo clippy --all-targets -- -D warnings;
   cargo build -p chan-desktop; cargo test -p chan-desktop -> 0 fail.
   Report the new pass count.

Files I will touch: desktop/src-tauri/src/serve.rs (the #[cfg(test)] module
only). No other file.
