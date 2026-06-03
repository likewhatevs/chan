# task-LaneA-LaneC-2: Rust cleanup - delete the unused gear commands

From: @@LaneA (lead)  To: @@LaneC  Type: build (Rust)

The launcher redesign removes the per-row gear (@@LaneB is removing the JS/
UI). The two Tauri commands the gear used become dead. You delete them. You
did the inventory, so you have the context.

## Source of truth

new-team-1/desktop-redesign-design.md (LOCKED block + §3 gear removal + §5).
@@Alex chose the MODAL path: do NOT add open_new_workspace_window or any new
command/capability (that was the rejected WINDOW path). This task is
DELETE-ONLY.

## Files - single owner (disjoint from @@LaneB's desktop/src)

desktop/src-tauri/src/main.rs, desktop/src-tauri/permissions/app.toml.

## Steps

1. FIRST, confirm scope. Grep the whole repo for get_workspace_features and
   set_workspace_features. Expected: the only callers are the launcher gear
   (desktop/src/main.js, which @@LaneB is removing) + the Tauri registration.
   The add-time pre-flight toggles set features via add_workspace's
   `features` param, NOT via set_workspace_features - verify that holds. If
   ANYTHING else references these commands (Rust, or JS @@LaneB is NOT
   removing), STOP and report to @@LaneA before deleting.
2. Delete the two `#[tauri::command]` fns get_workspace_features (main.rs
   ~438) and set_workspace_features (~761).
3. Remove both from the generate_handler!/invoke_handler list (main.rs
   ~1781).
4. Remove their permission entries (allow-get-workspace-features /
   allow-set-workspace-features) from desktop/src-tauri/permissions/app.toml,
   and from any capability set that lists them.

## Gate (own)

cargo fmt --check; cargo clippy --all-targets -- -D warnings; cargo build
-p chan-desktop. All green after the deletions (a dangling reference = you
missed a registration/permission line). Report the exact commands + green.

## Coordination

@@LaneB removes the JS invokes in parallel (disjoint files). No compile
coupling (JS is not cargo-compiled), but both must land before the verify
smoke so we never ship a half-state. Pre-release: no back-compat, delete
outright.

## On completion

Cut a completion task to @@LaneA at tasks/task-LaneC-LaneA-2.md
(append-only) - what was deleted, the grep result that cleared scope, gate
green - + poke.

---

## [LaneA DECISION] re task-LaneC-LaneA-2 scope hold: OPTION A - CLEARED

Excellent scope check + correct stop. Decision: OPTION A.

AUTHORIZED to also edit desktop/src-tauri/src/serve.rs to delete the two
obsolete guard tests:
- serve.rs:772-783 invoke_handler_registers_workspace_features_ipcs
- serve.rs:959-974 launcher_calls_workspace_features_ipcs

Rationale: these are canary tests pinning the exact IPCs we are
INTENTIONALLY removing - obsolete by definition once the gear is gone.
serve.rs is src-tauri (inside the design-doc Rust-Lane remit "src-tauri
only, disjoint files") and disjoint from @@LaneB's desktop/src; the
JS-asserting test is RUST code, so it belongs to the Rust-Lane, not
@@LaneB. @@LaneB is frontend-only and does not touch serve.rs - no
collision. Do NOT add replacement guard tests; this is a straight removal.

EXPANDED granted file set for this task:
- desktop/src-tauri/src/main.rs
- desktop/src-tauri/permissions/app.toml
- desktop/src-tauri/src/serve.rs  (ONLY the two tests above; leave the
  rest of serve.rs untouched)

Proceed with your planned main.rs edit (the two fns + the dead private
helpers read_workspace_features_blocking + resolve_workspace_for_features +
the registration lines; KEEP WorkspaceFeatures + the store cache, since
add_workspace still uses them) and the app.toml edit.

GATE (full, since cargo test is now in scope): cargo fmt --check; cargo
clippy --all-targets -- -D warnings; cargo build -p chan-desktop; cargo
test -p chan-desktop. All GREEN before you report.

crates/chan/src/main.rs:1662 stale comment: LEAVE IT. Different crate,
prose only, no compile/test coupling. Do NOT touch crates/chan; @@LaneA
will handle that one-line comment at commit time. Good catch flagging it.

---

## [LaneA DECISION 2] embedded.rs dead leaf: CLEARED + broadened grant

Verified independently (git status + grep, NOT relying on the append your
poke referenced - that append is NOT on disk; the task file was unchanged,
so re-check your file writes land before poking). The dead leaf is real:
embedded.rs:77-78 `pub fn live_workspace` only delegated to
self.host.live_workspace and its sole callers were the deleted commands ->
clippy -D warnings correctly flags it. Deleting the wrapper is safe; the
host method it called lives in chan-server and stays.

DECISION: delete it (your recommended A).

BROADENED GRANT (to stop the per-leaf ping-pong): you are authorized to
delete any FORCED dead-code leaf ANYWHERE under desktop/src-tauri/ that is
a direct consequence of this gear-command removal (clippy -D warnings
dead_code), following the cascade to completion. This matches the design
doc's "Rust-Lane = src-tauri only" remit. desktop/src-tauri/ is fully
disjoint from @@LaneB's desktop/src/ - no collision.

STOP rule still applies for:
- Any edit OUTSIDE desktop/src-tauri/ (a different crate: chan-server,
  chan-workspace, crates/chan, web/, etc.). Report, do not touch.
- Anything that still has OTHER live callers (i.e. NOT purely forced dead
  by this removal). If a deletion is ambiguous, report.

GATE unchanged: fmt --check; clippy --all-targets -- -D warnings; build -p
chan-desktop; cargo test -p chan-desktop. All green before you report DONE.

---

## [LaneA] task-LaneA-LaneC-2 ACCEPTED (DONE, green in scope)

Gear removal + dead-code cascade accepted. Clean diagnosis + proof on the 1
cargo-test fail: it is @@LaneB's WIP main.js tripping a serve.rs include_str!
canary (showPreflightDialog), NOT your deletion. Verified your changeset is
src-tauri-only. This task is closed.

The serve.rs canary reconciliation = a NEW task (gated on @@LaneB's final
frontend): task-LaneA-LaneC-3.md. Decision = your Option A (you own it,
Rust-Lane). See that file. I commit; you do not.
