# journal-LaneC (new-team-1)

## 2026-06-02

- Woke as @@LaneC (CHAN_TAB_NAME). Poke from @@LaneA pointed to
  tasks/task-LaneA-LaneC-1.md: read-only inventory of the CURRENT
  chan-desktop launcher so @@LaneB can ground the redesign.
- Read desktop/src/{index.html,main.js,styles.css},
  desktop/src-tauri/src/{main.rs,serve.rs,config.rs}, tunnel/*,
  tauri.conf.json, capabilities/*, plus the redesign draft.
- Wrote new-team-1/launcher-inventory-LaneC.md: 5 sections
  (Open-workspace flow, Attach flow, row rendering, per-row gear,
  window machinery) + cross-cutting notes, all file:line backed.
- Key answers: (a) folder picker is the dialog plugin, not a Rust
  command; (b) the gear toggles exactly bge + reports (same pair as
  the Open pre-flight modal); (c) a real [New] WINDOW is supported -
  open_new_launcher_window already builds main-N index.html windows
  via WebviewWindowBuilder, and capabilities/default.json scopes
  ["main","main-*"] so it inherits picker+opener perms with no new
  capability file. Modal is the alternative; design call, not a
  blocker.
- Cut completion to tasks/task-LaneC-LaneA-1.md, poked @@LaneA.
  Read-only; no code edited. Did not block on @@LaneD.

## 2026-06-02 (task-2: gear-command deletion)

- Poke from @@LaneA: delete the now-dead gear Tauri commands
  (get/set_workspace_features). DELETE-ONLY, modal path chosen.
  Files granted: main.rs + permissions/app.toml. Grep-confirm scope
  FIRST; STOP if anything else references them.
- Grep result: PRODUCT scope is exactly as the task predicted (gear in
  main.js = @@LaneB removing; Rust defs+registration; 2 app.toml perms;
  no capabilities/*.json). Verified add-time toggles use add_workspace's
  `features` param, NOT set_workspace_features.
- BUT grep surfaced 2 guard tests in serve.rs (NOT in my granted files):
  invoke_handler_registers_workspace_features_ipcs (772) breaks from my
  main.rs delete; launcher_calls_workspace_features_ipcs (959) breaks
  from @@LaneB's JS delete. build+clippy stay green; cargo test goes RED.
  That hits the task's explicit STOP-and-report trigger.
- Also traced dead-helper cascade in main.rs (mine): deleting both cmds
  kills read_workspace_features_blocking (486) + resolve_workspace_for_
  features (504) -> must delete to keep clippy -D warnings green.
- Stale comment crates/chan/src/main.rs:1662 (prose only) flagged, no
  action proposed.
- HOLDING. Made NO edits. Cut scope-check to task-LaneC-LaneA-2.md asking
  @@LaneA to pick A (authorize me to also delete the 2 serve.rs tests,
  gate incl. cargo test) or B (reassign serve.rs). Poked @@LaneA.

## 2026-06-02 (task-2 cont.: OPTION A cleared, executing)

- @@LaneA cleared OPTION A: serve.rs (2 canary tests only) added to grant;
  full gate incl. cargo test; leave crates/chan:1662 comment alone.
- Applied: main.rs deleted get_workspace_features + set_workspace_features
  + 3 now-dead private helpers (read_workspace_features_blocking,
  resolve_workspace_for_features, AND apply_workspace_features_blocking -
  the 3rd wasn't in @@LaneA's list; found it reading set_'s body, it's the
  same dead-helper class, in-file/in-scope). Removed the 2 registration
  lines. app.toml: dropped set refs + 2 [[permission]] blocks. serve.rs:
  deleted the 2 canary tests.
- Kept WorkspaceFeatures struct + store features cache (per decision;
  add_workspace still writes it; config.rs serde tests still cover field).
- GATE so far: cargo fmt --check GREEN. clippy --all-targets -D warnings
  RED on ONE forced dead leaf: embedded.rs:77 EmbeddedServer::live_workspace
  (sole callers were the deleted cmds; doc says "Feature toggles use this";
  no further cascade; embedded.rs unmodified by anyone). embedded.rs is
  OUTSIDE my grant -> HOLDING. Appended the finding to task-LaneC-LaneA-2.md
  + poked @@LaneA for OK on A (delete the 4 lines) or B (#[allow]).
- NOTE: first append attempt hit a wrong-cwd relative-path miss (cat >>
  failed silently while echo printed "appended"); re-verified, re-appended
  with absolute path, confirmed landed (grep -c=1). No stray file created.
- NOT committing (round-closer/@@LaneA owns commits). NOT touching
  @@LaneB's index.html/main.js (their concurrent WIP shows as M in tree).

## 2026-06-02 (task-2 cont.: Decision 2 executed - DONE in scope)

- @@LaneA Decision 2: delete embedded.rs live_workspace + broadened grant
  to follow the forced dead-code cascade to completion ANYWHERE under
  desktop/src-tauri/ (stop at crate boundaries / live callers). Also valid
  critique: my earlier poke fired before the append landed (cwd miss) -
  now verifying writes land before poking.
- Deleted embedded.rs::live_workspace (Read the file first this time, not
  just sed). Verified Path/Arc imports still used -> no orphan. Cascade
  complete: that was the ONLY extra leaf.
- GATE: fmt --check GREEN; clippy --all-targets -D warnings GREEN; build
  -p chan-desktop GREEN; test -p chan-desktop = 78 pass / 1 FAIL.
- The 1 fail = serve.rs:797 pick_and_add_shows_preflight_dialog... an
  include_str! canary on main.js. PROVEN to be @@LaneB's concurrent main.js
  WIP (HEAD main.js had showPreflightDialog( x2 -> passed; LaneB 1150-line
  diff removed pickAndAdd -> fails; I never edited main.js). NOT gear-
  dead-code, OUTSIDE my grant per STOP rule. @@LaneB is frontend-only so
  won't fix the serve.rs canary.
- My src-tauri gear-removal work COMPLETE + green in scope. Full cargo test
  green is blocked until @@LaneB's main.js lands (serve.rs canaries pin old
  main.js patterns). Reported DONE-in-scope + asked @@LaneA to sequence the
  canary reconciliation (offered to own it post-LaneB). NOT committing.

## 2026-06-02 (task-3: serve.rs canary reconciliation - DONE green)

- @@LaneA: B landed/frozen -> proceed from prep to the fix; gate cargo test
  to 0 fail. Re-audited the FROZEN main.js (not the stale prep) per the note.
- Reconciled the 5 frontend (MAIN_JS) canaries:
  * #1 REPLACE (only red one): renamed -> new_workspace_modal_gates_add_
    workspace_with_features; old showPreflightDialog( / features:
    choice.features (both 0 in frozen file) -> showNewWorkspaceDialog( +
    invoke('add_workspace', { path: localPath, features } (main.js:617).
  * #5 RENAME -> new_workspace_modal_carries_add_time_feature_toggles;
    asserts (Semantic search/Reports/data-feat bge/reports) survive on the
    modal D1 toggles, retargeted name+comment off the dead gear panel.
  * #3 message fix (dropped dead "showPreflightDialog" ref).
  * #2, #4 KEEP-AS-IS (copy phrases + default-workspace flow survive verbatim).
  * No deletes (gear get/set canaries already gone in task-2).
- GATE all GREEN: fmt --check; clippy --all-targets -D warnings; build
  -p chan-desktop; test -p chan-desktop = 79 passed / 0 failed (+ tunnel_e2e
  7 passed). task-2 red resolved.
- Flagged (no action, @@LaneA's call): optional [New]-modal JS-wiring canary
  (offered); canary brittleness (pins exact spellings) -> possible follow-up.
- Cut completion task-LaneC-LaneA-3.md. NOT committing (round-close owns).
- TOOLING NOTE: a bundled grep+heredoc+poke command got truncated after the
  grep; journal append + poke did NOT run. Re-doing each separately +
  verifying (per the verify-writes-before-poke discipline).

## 2026-06-02 (task-3 ACCEPTED - HOLDING for post-smoke flag-1)

- @@LaneA accepted task-3 (cargo test 79/0; coverage moved with the feature).
- Flag 1 APPROVED but SEQUENCED POST-SMOKE: do NOT add yet. After @@Alex's
  smoke confirms frontend final, @@LaneA pokes me to add ONE minimal canary:
  assert #new-workspace -> showNewWorkspaceDialog + main.js contains
  invoke('add_outbound_workspace') + invoke('tunnel_start'). Anchor on
  function/invoke names (stable), NOT copy text. (Deferred to avoid churn
  if smoke finds a fix + a serve.rs race with @@LaneD's verify gate.)
- Flag 2 (canary brittleness rethink) DEFERRED to a possible later task.
- HOLDING for the post-smoke poke. Not committing (lead commits).
