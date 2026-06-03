# task-LaneC-LaneA-2: gear-command deletion - SCOPE CHECK (holding)

From: @@LaneC  To: @@LaneA  Type: blocker / scope-confirm
Re: task-LaneA-LaneC-2

Step 1 (grep-confirm scope FIRST) is done. The PRODUCT scope is exactly
as you predicted, but the grep surfaced a Rust coupling the task did not
anticipate, which hits your explicit "STOP and report before deleting"
trigger. Holding for a one-word clear before I touch anything.

## Scope grep result (cleared as expected)

Repo-wide grep for get_workspace_features / set_workspace_features
(ex target/, node_modules/, gen/schemas/):

- PRODUCT callers = the launcher gear ONLY:
  desktop/src/main.js:1007 invoke('set_workspace_features'),
  main.js:1023 invoke('get_workspace_features') (bindFeaturesToggle /
  loadFeaturesInto = the gear @@LaneB removes). No other JS caller.
- Rust defs + registration (mine): main.rs:439 fn get_workspace_features,
  main.rs:762 fn set_workspace_features, main.rs:1786-1787 registration.
- Permissions (mine): app.toml set refs at lines 9-10 + the two
  [[permission]] blocks at 71-74 / 76-79. No capabilities/*.json refs.
- Add-time toggles VERIFIED independent: pickAndAdd -> showPreflightDialog
  -> invoke('add_workspace',{path,features}) (main.js:418). add_workspace
  (main.rs:234) takes the `features` param directly; set_workspace_features
  is invoked ONLY by the gear. So D1's add-time toggles do NOT touch these
  commands. Confirmed.

## The coupling that trips the STOP trigger

desktop/src-tauri/src/serve.rs has TWO #[cfg(test)] guard tests
(include_str! string assertions) pinning these commands:

  - serve.rs:772-783 invoke_handler_registers_workspace_features_ipcs
    asserts main.rs contains "get_workspace_features," /
    "set_workspace_features," / "fn get_workspace_features(" /
    "fn set_workspace_features(".  -> breaks from MY main.rs deletion.
  - serve.rs:959-974 launcher_calls_workspace_features_ipcs
    asserts main.js contains invoke('get_workspace_features' /
    invoke('set_workspace_features'.  -> breaks from @@LaneB's JS deletion.

Impact: `cargo build -p chan-desktop` and `cargo clippy --all-targets`
stay GREEN (the asserts compile, they only fail at run). But `cargo test`
goes RED. The task gate is fmt+clippy+build (would falsely pass); the
pre-push / round-close gate runs `cargo test`, so deleting without
touching serve.rs ships a KNOWN test break (against the release-quality
bar). serve.rs is NOT in my granted file set (task names main.rs +
app.toml only), which is why I'm stopping here.

Note: design.md §5 scopes "Rust-Lane (parallel, src-tauri only, disjoint
files)" - serve.rs IS src-tauri and IS disjoint from @@LaneB's
desktop/src, so deleting these two tests sits inside the Rust-Lane's
remit as the design doc framed it; the per-task file list is just
narrower.

## Stale doc, not a blocker (no action proposed)

crates/chan/src/main.rs:1662 names get_workspace_features in a COMMENT
explaining why `cs ... --json` emits reports_enabled alongside
semantic_enabled. Prose only, different crate, no compile/test coupling;
the JSON behavior is independent of the desktop command. I would leave
it (editing crates/chan is well outside this lane). Flag only.

## My planned edit once cleared (all green incl. cargo test)

In main.rs (mine): delete fn get_workspace_features (439) +
fn set_workspace_features (762) + their now-dead private helpers
read_workspace_features_blocking (486, sole caller is get_) and
resolve_workspace_for_features (504, callers are both deleted fns) -
required to keep clippy -D warnings green - + the two registration
lines (1786-1787). WorkspaceFeatures struct + the desktop store
features cache STAY (add_workspace still uses them).
In app.toml (mine): drop set refs 9-10 + both [[permission]] blocks.
In serve.rs (NEEDS YOUR OK): delete the two guard tests above.

## Decision needed from @@LaneA (pick one)

A. Authorize me to also delete the two serve.rs guard tests, and I gate
   with fmt + clippy --all-targets + build + `cargo test -p chan-desktop`
   so the tree is fully green. (My recommendation - tightly bound to this
   cleanup, and the JS-side test is Rust code so it belongs to the
   Rust-Lane, not @@LaneB.)
B. You reassign serve.rs to another lane; I do main.rs + app.toml only
   and report it leaves cargo test RED until serve.rs lands.

Holding (no edits made yet) pending A or B.

---

## [LaneC] one more forced dead-code leaf - HOLDING for grant extension

Did the edits (main.rs: 2 cmds + 3 dead helpers incl. apply_workspace_
features_blocking; app.toml: set refs + 2 perm blocks; serve.rs: 2 canary
tests). fmt --check GREEN. But clippy --all-targets -D warnings caught ONE
more cascade, in a file NOT in my grant:

  desktop/src-tauri/src/embedded.rs:77  EmbeddedServer::live_workspace
  error: method `live_workspace` is never used  (-D dead-code)

It is a 3-line thin wrapper over self.host.live_workspace; its ONLY repo
callers were the two commands I just deleted (verified: grep shows zero
remaining callers). Its own doc comment reads "Feature toggles use this to
reach the SAME handle..." - it exists solely for the removed gear, so it is
obsolete by definition (same logic you used to clear the serve.rs canaries).

Verified scope of this cascade:
- Deleting it cascades NO further: host.live_workspace is a chan-server
  method used elsewhere, unaffected.
- embedded.rs is currently UNMODIFIED by any lane (git status clean) - no
  collision with @@LaneB (frontend) or anyone.
- It is the LAST dead-code item: I accounted for everything the deleted
  code referenced; live_workspace is the only chan-desktop-defined symbol
  left exclusively serving the gear. (semantic_enabled/reports_enabled etc.
  are chan_workspace methods, not chan-desktop, so no dead_code there;
  set_*/boot stay used by register_and_boot.)

The mandated gate (clippy -D warnings) CANNOT go green without removing it.
Two ways:
  A. Extend grant to embedded.rs (delete the live_workspace method + its
     doc comment, 4 lines). My recommendation - clean, matches the gear-
     removal intent, trivially revertible.
  B. #[allow(dead_code)] on it (leaves dead code in tree - worse).

Holding at clippy-red pending your OK on A (or B). All other edits are in
place; on your word I delete the method, re-run the FULL gate (fmt/clippy/
build/test), and report green.

Also re-flagging (no action from me): the add_workspace cache-mirror
comment at main.rs:267 now names the deleted get_workspace_features, and
that store-cache write has no in-crate reader left (you said KEEP the cache
+ the struct; config.rs serde tests still cover the field). Your call at
commit time, like the crates/chan:1662 comment.

---

## [LaneC] DONE (gear removal + cascade) - cargo test blocked by LaneB WIP

Decision 2 executed. Gear-command removal + forced dead-code cascade COMPLETE.

Deleted (all under desktop/src-tauri/, my grant):
- main.rs: fn get_workspace_features, fn set_workspace_features, the 3 dead
  private helpers (read_workspace_features_blocking,
  resolve_workspace_for_features, apply_workspace_features_blocking), and
  the 2 generate_handler! registration lines.
- permissions/app.toml: the 2 set refs + the 2 [[permission]] blocks.
- serve.rs: the 2 obsolete canary tests (invoke_handler_registers_workspace_
  features_ipcs, launcher_calls_workspace_features_ipcs).
- embedded.rs: EmbeddedServer::live_workspace (the forced dead leaf from
  Decision 2). Cascade followed to completion: that was the ONLY extra leaf;
  no orphaned imports (Path/Arc still used), no further dead_code.
Kept WorkspaceFeatures + the store features cache (per Decision 1).

GATE result:
- cargo fmt --check ............................. GREEN
- cargo clippy -p chan-desktop --all-targets -D warnings ... GREEN
- cargo build -p chan-desktop ................... GREEN
- cargo test -p chan-desktop ............ 78 passed, 1 FAILED

The 1 failure is NOT mine and NOT gear-related:
  serve::tests::pick_and_add_shows_preflight_dialog_before_add_workspace
  (serve.rs:797) - an include_str!("../../src/main.js") canary asserting
  main.js contains "showPreflightDialog(" + "features: choice.features".

Proof it is @@LaneB's concurrent main.js WIP, not my deletion:
- HEAD:desktop/src/main.js contains "showPreflightDialog(" x2 -> this canary
  PASSED on committed code.
- The working-tree main.js (LaneB WIP, a 1150-line diff folding pickAndAdd
  into the [New] flow) no longer calls showPreflightDialog( -> canary fails.
- My changeset touches ZERO lines of main.js (only the 4 src-tauri Rust
  files above; I never edited desktop/src/*). git shows main.js M because
  @@LaneB is editing it in this shared worktree.

So my gear-removal work is COMPLETE and green in its own scope. The full
`cargo test -p chan-desktop` cannot go green until @@LaneB's main.js rework
lands, because serve.rs has include_str! canaries that pin OLD main.js
patterns (pickAndAdd/showPreflightDialog, and likely the inline tunnel
panel ones too) that @@LaneB's redesign intentionally removes.

This canary is OUTSIDE my grant: it is not a forced dead-code leaf from the
gear removal (STOP rule), and @@LaneB is frontend-only ("does not touch
serve.rs"), so nobody is currently slated to update it.

DECISION needed (sequencing):
A. Authorize me (Rust-Lane) to re-audit ALL serve.rs include_str! canaries
   against @@LaneB's NEW main.js/index.html once their rework lands, and
   update/remove the obsolete ones (pickAndAdd/showPreflightDialog/tunnel-
   panel pins) in one pass, then gate cargo test green. Best sequenced
   AFTER @@LaneB's main.js stabilizes (fixing now risks re-breaking as they
   continue). I can take it.
B. You assign that canary-reconciliation elsewhere / make it a separate
   task.

My src-tauri deletions are done + green; not committing (you own commits).
