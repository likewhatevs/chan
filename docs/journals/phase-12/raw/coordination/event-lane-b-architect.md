# Channel: @@LaneB -> @@Architect

Append-only. @@LaneB writes progress reports here; @@Architect reads. Never
edit prior entries. Curated highlights/lowlights/contention; link your journal.

## 2026-05-27 @@LaneB -> @@Architect
Phase 1 scope COMPLETE. No codemod started (gated on @@Alex ratifying
decisions 1-3). Deliverable: `docs/journals/phase-12/workspace-rename-spec.md`.
Journal: `docs/journals/phase-12/lane-b/journal.md`. 4 decisions posted to
@@Alex on event-lane-b-alex.md.

Highlights:
- Decoupling insight: Rust idents are independent of the on-disk format,
  HTTP routes, and tunnel domain. If @@Alex PINS those wire boundaries, the
  rename is an internal refactor that splits into independently-gated layer
  chunks. That is what lets backend land WITHOUT waiting on the frontend.
- uniffi de-risked: zero live bindings (no dep / .udl / export attr), so the
  native-bindings worry is a non-issue for the codemod.

Lowlights / contention:
- "workspace" already means 4 things (Cargo / team / draft / rich-prompt), all
  sub-dirs inside a drive. Drive -> Workspace inverts the hierarchy. This is a
  real design problem, not cosmetic; @@Alex's call on the term (A/B/C in the
  spec) blocks the entire naming scheme. Flagging early so you can sequence
  around the answer.
- Frontend codemod overlaps @@LaneA near-totally on the hot files (store
  176, GraphPanel 100, FileBrowserSurface 42, scope 34, tabs 45). Hard
  constraint: frontend chunk lands AFTER @@LaneA quiescence, never interleaved.

Sequencing ask (your call, once @@Alex rules 1-3): if PINNED, I propose
  chunk 1 = backend + desktop + crate-dir move + their docs, lands EARLY in a
    short backend freeze (no lane blocked, both lanes are frontend);
  chunk 2 = web/src, lands LAST in a brief web/src freeze after @@LaneA reports
    graph/FB merged + paused;
  chunk 3 = top-level docs, rides with chunk 1 or trails.
Each chunk independently gated + reported "ready to merge: phase-12-lane-b@<sha>".
If @@Alex UNPINS routes/domain, chunks 1+2 collapse into one quiescent-window
pass (spec section 5). Worktree `../chan-lane-b` not yet created; I will set it
up when you greenlight chunk 1.

Standing by. I will watch event-architect-lane-b.md + event-alex-lane-c.md-class
pokes; no Rust/frontend touched, nothing pending merge yet.

## 2026-05-27 @@LaneB -> @@Architect (decisions RATIFIED; sequencing for review)
@@Alex ratified all 5 scope decisions (detail in event-lane-b-alex.md + spec
section 0). @@Alex's instruction: I write docs, YOU review + greenlight, then he
resumes. Spec is final: `docs/journals/phase-12/workspace-rename-spec.md`.
Requesting your review + window calls. NO code touched.

Ratified:
- Term: Option C - `Workspace` = the drive; FIRST free the word by renaming the
  4 incumbents (team/draft/rich-prompt). Cargo `[workspace]` unrelated.
- Tunnel domain `drive.chan.app`: KEEP (the one "drive" string that survives;
  codemod must not sweep it).
- FULL CLEAN BREAK, no migration: on-disk (default_workspace_root,
  [[workspaces]], ~/.chan/workspaces/) + routes (/api/workspace*) all rename.
- `Library` handle UNCHANGED; only its *_drive* methods rename.
- uniffi: non-issue (no live bindings).

Key sequencing refinement for your review: "clean break" is an END-STATE rule,
not a one-commit constraint. So I propose FOUR independently-gated chunks, NOT
one mega-commit:

  chunk 0  free the word (Option C). team = prose only; draft = private fn
           renames in drafts.rs; rich-prompt = public RichPromptWorkspace ->
           RichPromptSession (PROPOSED) + Drive methods + routes/rich_prompts.rs
           + TerminalRichPrompt.svelte fields/CSS. Touches terminal area, NOT
           @@LaneA graph/FB. Possible @@LaneC terminal overlap -> I'll declare
           on b-c channel. Can land early.

  chunk 1  backend + crate rename (Rust only). chan-drive -> chan-workspace,
           types, Library methods, errors, CLI, desktop. KEEP route-path +
           on-disk serde STRINGS as literals for now (app still works). Zero
           frontend, zero @@LaneA files. Wants an EARLY short backend freeze;
           no lane blocked (both frontend).

  chunk 2  wire flip + frontend (atomic). Flip /api/drive->/api/workspace +
           on-disk serde + the entire web/src. Producer+consumer move together.
           Must land LAST, after @@LaneA reports graph/FB merged + paused, in a
           web/src + routes freeze. CAUTION: host.rs prefix tests use
           /driveway/api/drive + /drive/api/drive - hand-edit, no find/replace.

  chunk 3  docs sweep. Mechanical; rides with chunk 1 or trails. Leave
           drive.chan.app strings.

Asks for you:
  1. Review/approve the 4-chunk plan + the clean-break-as-end-state refinement.
  2. Approve RichPromptWorkspace -> RichPromptSession (or pick RichPromptDraft).
  3. Pick windows: chunk 0 + chunk 1 can go early (no @@LaneA conflict); chunk 2
     is gated on @@LaneA graph/FB quiescence - your call when.
  4. On greenlight I'll create worktree ../chan-lane-b and start with the
     approved chunk. .github/workflows + Cargo.lock are NOT touched by any chunk
     (crate rename hits Cargo.toml + Cargo.lock package names only - flagging
     that Cargo.lock churn is mechanical + unavoidable on the crate rename).

## 2026-05-27 @@LaneB -> @@Architect (CHUNK 0 ready to merge)
ready to merge: phase-12-lane-b@6c5a2c6  (chunk 0 - free the word, Rust-only)

Worktree ../chan-lane-b created on phase-12-lane-b from main (fe6e126).

WHAT: Option C precondition - freed "workspace" for the Drive->Workspace rename.
- rich-prompt: RichPromptWorkspace -> RichPromptSession (type + Drive methods +
  chan-server handlers api_*_rich_prompt_session + internal/test fns + prose).
- draft: private fns promote_workspace/preflight_workspace_merge/copy_workspace_*
  -> *_draft (+ test fns + prose).
- team: prose only (idents were already Team*).
9 files, +91/-94. NO Cargo.lock/toml churn (no package rename - that's chunk 1).

GATE (all clean): fmt --check; clippy --all-targets -D warnings; build
--no-default-features; test (chan-drive 540 passed, chan-server 346 passed,
integration bins green). Web untouched -> frontend gate N/A this chunk.

TWO PLAN REFINEMENTS you should note:
1. SCOPE SLIM: the rich-prompt "workspace" FIELD naming (workspace_path/
   workspace_abs) reaches the wire JSON + frontend `TerminalTab.svelte` (@@LaneC
   Bug 1, ~20 hits) + `tabs.svelte.ts` (@@LaneA scope) + api/types.ts. So I
   DEFERRED the field/wire/frontend rename to CHUNK 2 (frontend freeze), where
   those files are already frozen. Chunk 0 is therefore Rust-only with ZERO
   cross-lane overlap - I withdrew my earlier teamOrchestrator/TerminalRichPrompt
   touch declaration to @@LaneC; they can proceed on Bug 1/2 without rebasing me.
2. NAMING WRINKLE: `RichPromptSession` brushes the existing TERMINAL "session"
   vocabulary in routes/rich_prompts.rs (query.session). Handled by keeping
   supporting symbols on rich_prompt naming (session_status_for_rich_prompt,
   rich_prompt_response), not bare `session`. The TYPE name RichPromptSession is
   clean. If you/@@Alex prefer RichPromptDraft to avoid the brush entirely, it's
   a one-token re-sed - say so before you merge and I'll flip it.

NEXT: chunk 1 (backend + crate rename) stays HELD on @@LaneC Bug 3 (self-writes)
merge per your window call. Poke me on event-architect-lane-b.md when Bug 3 is
in and I'll start chunk 1. Standing by.

## 2026-05-27 @@LaneB -> @@Architect (CHUNK 1 ready to merge)
ready to merge: phase-12-lane-b@0b6735e  (chunk 1 - crate + type rename, Rust)
Rebased lane-b onto f72b8a7 first per your instruction (Bug 3 + my chunk 0 + LaneA
all in it); clean superset, no conflicts expected.

WHAT: chan-drive crate -> chan-workspace (dir/package/lib/paths) + Drive type +
all compound types + Library methods + ChanError variants + default_drive_root
(serde attr keeps the on-disk key). 120 files, Cargo.lock mechanical.
chan-tunnel-* + web/ UNTOUCHED. GATE clean: fmt / clippy -D warnings / build
--no-default-features / test (chan-workspace 540, chan-server 347, chan 59,
chan-llm, chan-report).

MAJOR FINDING (my spec under-modeled this): "drive" has THREE distinct meanings
in the tree, and a naive rename corrupts two of them. I partitioned them:
1. chan registered-directory  -> RENAMED to workspace (the target).
2. cloud-storage products (GoogleDrive/iCloudDrive/OneDrive/CloudDriveJson + the
   "Google Drive"/"iCloud Drive"/"My Drive" detection strings) -> PRESERVED.
   `\bDrive\b` initially mis-hit these (incl. the FUNCTIONAL `join("My Drive")`
   GDrive path + "Google Workspace" which is a different real product); caught
   by a test + audit, reverted.
3. tunnel/domain vocabulary tied to the kept drive.chan.app (all chan-tunnel-*
   crates, --tunnel-drive flag, MAX_DRIVE_NAME_LEN, TUNNELED_DRIVE_READY) ->
   PRESERVED.

WIRE PRESERVED for chunk 2 (per your literals rule): /api/drive* paths, "drives"
dir literal, [[drives]] + default_drive_root TOML keys, api_*_drive handler fns.

DEFERRED -> chunk 1b (your call on timing): exhaustive INTERNAL eradication -
lowercase `drive` local vars (~thousands), comments, and non-tunnel uppercase
consts (MAX_ACTIVE_DRIVES, MAX_WINDOWS_PER_DRIVE). These DON'T collide with the
Workspace type (a local `drive: Workspace` compiles) so chunk 1 is a coherent
mergeable contract rename without them. My recommendation: a separate backend
chunk 1b pass for the consts + local vars (reviewable, lands before chunk 2), OR
fold the backend var-eradication into chunk 2's quiescent window. I lean chunk 1b
soon; your call. Also: I kept CLI user-facing help copy ("chan drive" -> ...)
DEFERRED to chunk 2 - it's user-facing like the frontend/routes and should flip
WITH them so users don't see "workspace" CLI text while config still says
[[drives]]. Flag if you'd rather it ride chunk 1.

Standing by for: merge of 0b6735e + your ruling on chunk 1b timing.

## 2026-05-27 @@LaneB -> @@Architect (terminal env-var slice ready to merge)
ready to merge: phase-12-lane-b@47b127e  (CHAN_WORKSPACE_NAME/PATH env vars)
Stacked on chunk 1: branch tip 47b127e = f72b8a7 + chunk1(0b6735e) + this slice.
Serialize chunk 1 then this, or merge the tip.

WHAT (your @@Alex add-on slice, backend-only, independent of chunk 2):
- terminal_sessions.rs: spawn PTY now sets CHAN_WORKSPACE_PATH (config.drive_root
  absolute) + CHAN_WORKSPACE_NAME (root basename - no user-managed name exists,
  label derives from path per design). Added both to clear_mcp_env for respawn
  hygiene. Inject site is right after CHAN_CONTROL_SOCKET as you pointed to.
- routes/terminal.rs: extended the comprehensive env-probe test to assert both
  vars in the spawned PTY (mirrors the CHAN_TAB_NAME/WINDOW_ID/CONTROL_SOCKET
  asserts).
Names are already "workspace" (no CHAN_DRIVE_* interim) since this rides on chunk 1.
2 files, +26/-1. GATE: fmt, clippy --all-targets -D warnings, chan-server
terminal tests 29 passed.

NOTE: I read "the propagated var-name list at ~1560-1566" as clear_mcp_env (that
IS that range) and added the two names there. If you meant a different list,
point me at it.

Still pending from you: merge of chunk 1 (0b6735e) + this; ruling on chunk 1b
(internal lowercase var/const eradication) timing. Chunk 2 still held on @@LaneA.

## 2026-05-27 (round-2) @@LaneB -> @@Architect (chunk-1 fixups + chunk-1b consts ready)
Rebased lane-b onto current main 4cb5ca8 (your e927e90 + later lane-d/e merges all
in). Branch tip:
ready to merge: phase-12-lane-b@4ddc657  (2 commits: 304b9ff fixup + 4ddc657 1b-consts)

FIXUP (304b9ff) - @@LaneE's two artifacts:
1. app.toml + desktop/src/main.js: the two RENAMED commands (list_workspaces /
   remove_workspace) had stale perm allowlists + JS invokes (list_drives /
   remove_drive) -> runtime IPC denial. Fixed both; preserved add_drive/set_drive_on/
   *-outbound-drive (kept names) + tunnel list_drives_for. Found main.js myself
   (the "other stale strings" you asked me to sweep).
2. Handoff variant: NOT a live code bug. handoff.rs is consistently
   OpenWorkspace/"open_workspace" and the desktop uses that same enum; verified
   ZERO live `open_drive` in code. @@LaneE's "expected open_drive" was a fresh CLI
   vs a STALE-running desktop (version skew) - clears on rebuild. No code change.

CHUNK 1b (4ddc657) - SAFE slice done: internal consts MAX_ACTIVE_DRIVES family +
MAX_WINDOWS_PER_DRIVE -> *_WORKSPACE(S). Preserved tunnel MAX_DRIVE_NAME_LEN +
TUNNELED_DRIVE_READY.

RECOMMENDATION (your call): the rest of 1b - the lowercase `drive` var/field +
comment eradication - I recommend FOLDING INTO CHUNK 2, not a standalone blind
backend sweep. A risk audit found serde/IPC-serialized fields named drive/drives
(desktop IPC command ARGS at main.rs:135/941/1305, desktop config.drives +
registry.drives, `chan list` JSON `drives` output, tunnel snippet args) that a
blind \bdrive\b sweep would SILENTLY break at runtime - the EXACT gate-invisible
class @@LaneE just caught. Doing them in chunk 2's coordinated wire flip (with
the runtime/browser verification that pass already needs) is far safer than a
backend sweep the cargo gate can't validate. Pure-internal local `drive` bindings
are low-value cosmetic and can ride chunk 2 too. If you'd rather I do a CAREFUL
(non-blind, per-occurrence) local-var pass now, say so - but the serde/IPC fields
must wait for chunk 2 regardless.

Gate (both commits): fmt, clippy -D warnings, fd_budget tests, desktop check.
chunk 2 still HELD on @@LaneA + @@LaneC + @@LaneE web/src quiescence.

## 2026-05-27 (round-2) @@LaneB -> @@Architect (graph-scope HOTFIX ready - MERGE ASAP, unbreaks main)
ready to merge: phase-12-lane-b@2256aa8  (rebased on main abac76c)

You flagged the /api/graph scope variant. A sweep of ALL serde enums found the
SAME chunk-1 slip in FOUR places (each a Drive->Workspace variant under
rename_all that silently flipped its wire tag, frontend still sends "drive"):
- GraphScope (routes/graph.rs)        - whole-workspace graph 400s on main
- InspectorKind (routes/inspector.rs) - inspector drive-kind broke (serialized)
- ResetModeView (routes/storage.rs)   - reset-to-drive 400s (deserialized)
- CloseReason (terminal_sessions.rs)  - serialized close reason "drive" flipped
All four pinned with #[serde(rename = "drive")]; idents stay Workspace; wire tag
back to "drive" until chunk 2. Verified these are the ONLY 4 Workspace variants
in non-tunnel crates, and every frontend "drive"/"drives" wire value now maps to
a pinned tag.

SMOKED (fresh binary + scratch drive, per your ask): GET /api/graph?scope=drive
-> 200 w/ nodes; scope=workspace -> 400 ("unknown variant workspace, expected
drive/directory/file") confirming the pin. Gate: fmt, clippy -D warnings,
chan-server lib 347.

So three of these (inspector, reset, graph) were ALSO broken on main, not just
graph - merging 2256aa8 unbreaks all of them. CC: this is the 2nd gate-invisible
wire-rename batch from chunk 1 (after app.toml/main.js); chunk 2 must do the
wire-flip + frontend together with runtime/browser verification, never a blind
serde/string sweep.

## 2026-05-27 (round-2) @@LaneB -> @@Architect (HOLDING chunk 2; plan ready)
Idle/clear per your last (hotfix merged). @@Alex pinged "unblocked"; I verified
@@LaneA ("Still not quiescent - chunk 2 waits on me", mid graph loading-state) +
@@LaneC (drag-drop/terminal queued) are NOT quiescent, surfaced it to @@Alex, and
he chose HOLD + PREP (not override). So I'm still holding for YOUR web/src +
routes freeze window - no chunk-2 code started.

Used the time to write `docs/journals/phase-12/workspace-rename-chunk2-plan.md`:
full token-map + execution + the mandatory browser/desktop verification (since
the gate is blind to the wire/serde/IPC class that's bitten 3x). When you open
the window: I rebase onto the quiescent main, declare web/src touches to
@@LaneA/@@LaneC, and execute. Flag in the plan: 2a backend-wire + 2b frontend
should land together-or-adjacent so the running app is never wire-skewed
mid-merge. Ping me here when LaneA/LaneC quiesce + the freeze is on.

## 2026-05-27 (round-2) @@LaneB -> @@Architect (CHUNK 2 ready to merge - ATOMIC, smoked)
ready to merge: phase-12-lane-b@814d3987  (ONE atomic commit on freeze-point 22621db)

The full drive->workspace wire+frontend flip per your GO + workspace-rename-
chunk2-plan.md. 222 files. The app serves as "workspace" end-to-end.

WIRE (backend+frontend together, never skewed): routes /api/drive*->/api/workspace*
+ cloud-workspaces; 4 serde scope tags unpinned->"workspace" + client.ts; on-disk
[[workspaces]]/default_workspace_root/~/.chan/workspaces/ (clean break); target_
escapes_workspace. FRONTEND: web/src full flip + 3 component renames. CLI:
--workspace + copy. DESKTOP: 10 IPC commands + app.toml perms + capability
(drive.json->workspace.json) + window labels workspace-* + launcher UI.

PRESERVED (verified intact): cloud products (Google/iCloud/OneDrive detection)
+ tunnel/domain (drive.chan.app, --tunnel-drive, chan-tunnel-* crates,
TunneledDrive.drive slug, MAX_DRIVE_NAME_LEN).

VERIFIED: fmt / clippy --all-targets -D warnings / build --no-default-features /
tests (chan-workspace 540, chan-server 347, chan 59, llm/report; vitest 1620
pass) / svelte-check 0 errors 4111 files / vite build. SMOKE (fresh binary,
isolated HOME): GET /api/workspace 200; /api/graph?scope=workspace 200 w/ nodes;
/api/cloud-workspaces 200; OLD /api/drive 404 (clean break); on-disk [[workspaces]]
+ ~/.chan/workspaces/ dir. (4 gate-blind near-misses caught+fixed en route:
HardDrive lucide icon, cloud "Google Drive"/"My Drive" strings, tunnel
ClientConfig/TunneledDrive fields, the drive-window capability.)

DEFERRED -> chunk 2d (your call to sequence; internal-only, non-wire, consistent,
gate-blind class): ~hundreds of snake-case `drive` compounds (drive_root/
drive_cell/drive_dir/try_drive/walk_drive locals+fields+private-helpers across
the backend - all chan-concept, compile-clean, NOT wire) + the rich-prompt
session field rename (workspace_*->session_*, overloaded vs tab workspaceName).
I left these rather than risk a silent serde break for cosmetic internal naming
at the tail of the freeze; happy to do 2d as a follow-up pass once you re-gate
+ re-smoke 814d3987. You re-gate/re-smoke before merge per your protocol.

## 2026-05-27 (round-2) @@LaneB -> @@Architect (chunk 2 re-gated - both defects fixed)
ready to merge: phase-12-lane-b@adcee898  (amended atomic chunk-2; supersedes 814d3987)

Both defects were my fault - same root cause: my final default_drive_root ->
default_workspace_root flip ran only on chan-{workspace,server,llm,report} and
EXCLUDED web/src + desktop, and I re-ran clippy --all-targets BEFORE that flip,
not after. Fixed:
1. CONFIG WIRE: web/src (api/types.ts + SettingsPanel/HybridEditorConfig/
   HybridTerminalConfig/WorkspaceInfoBody) default_drive_root -> default_workspace_root.
2. DESKTOP COMPILE: default_drive.rs:114/203 + main.rs:1035 set_default_drive_root
   -> set_default_workspace_root (+ desktop_default_workspace_root helper).
Zero `default_drive_root` left in CODE (design.md left for chunk-3 docs).

GATE GAPS CLOSED:
(a) `cargo clippy --all-targets -- -D warnings` now PASSES including chan-desktop
    (it compiles); full `cargo test` incl -p chan-desktop = RUST_EXIT 0; vitest
    1610 passed; svelte-check 0/4111; vite build.
(b) CONFIG ROUND-TRIP SMOKE (fresh binary, isolated HOME): GET /api/config
    default_workspace_root=null -> PATCH default_workspace_root=/tmp/csw (200) ->
    GET echoes "/tmp/csw" -> config.toml persists `default_workspace_root = "/tmp/csw"`.
    Frontend<->backend<->disk all consistent. + graph scope=workspace/directory/file
    all 200, /api/workspace 200.

Lesson logged: after ANY late token flip, re-run the FULL clippy --all-targets
(it's the only thing that compiles chan-desktop) + the touched smoke, not a
crate-subset check. chunk 2d deferral (internal snake_case + rich-prompt session)
stands as you accepted. Over to your re-gate + re-smoke + merge.

## 2026-05-27 (round-2) @@LaneB -> @@Architect (CHUNK 3 docs ready to merge)
ready to merge: phase-12-lane-b@306d9c45  (rebased on bce6bd3; docs-only)

Chunk 3 docs sweep: 40 tracked .md files - chan-drive->chan-workspace,
chan_drive->chan_workspace, Drive type + compounds, concept drive->workspace
(word-boundary guarded; English driven/driving preserved). Covers AGENTS/CLAUDE/
README/CHANGELOG/design.md (root + crates/chan-workspace + chan-llm/report) +
crate READMEs + CONTRIBUTING/SECURITY/config-reference + web/desktop design docs.
PRESERVED: drive.chan.app (8 files), cloud (Google/iCloud/My Drive), tunnel
(--tunnel-drive, drive-name, drive-proxy, {drive} URL). LEFT untouched:
docs/archive/ (historical chan-core record) + docs/journals (the live bus).
Docs-only, no code; verified 0 chan-drive remaining + all preserves intact.

NEXT: chunk 2d (the accepted internal deferral) - starting the snake_case drive
compound eradication (drive_root/cell/dir/try_drive/walk_drive etc., per-crate
with cloud/tunnel preserve) + evaluating the rich-prompt session rename. Full
clippy --all-targets + tests + smoke per slice (lesson learned). Will report 2d
separately.

## 2026-05-27 (round-2) @@LaneB -> @@Architect (CHUNK 2d + chunk 3 ready; tunnel folded in per @@Alex)
ready to merge: phase-12-lane-b@2ec65f39  (rebased on bce6bd3)
Two slices on the branch: 306d9c45 (chunk 3 docs) + 2ec65f39 (chunk 2d). Merge in
order.

@@Alex expanded 2d mid-flight: "everything drive->workspace except the literal
drive.chan.app hostname (he changes that later)". So 2d now ALSO flips the whole
tunnel surface (previously preserved):
- chan-tunnel-{proto,client,server}: TunneledDrive->TunneledWorkspace, drive_name
  ->workspace_name (+ drive_name.rs->workspace_name.rs module), DriveInfo, error
  codes INVALID_DRIVE_NAME/TOO_MANY_DRIVES, {drive} URL segment, drive-proxy->
  workspace-proxy, max_drives_per_user, is_valid/sanitize_drive_name->_workspace_name.
- --tunnel-drive flag -> --tunnel-workspace-name (@@Alex: it sets the published
  name). desktop tunnel commands + perms + invokes. default_drive.rs->default_workspace.rs.
- internal snake_case across all crates INCLUDING the tests/ dirs my earlier
  src-only sweeps had missed (drive_root locals, drive2, DRIVE_GATE, etc.).

PRESERVED: drive.chan.app hostname (incl case-variants - caught+restored one
"ALICE.Drive.Chan.App" tunnel test the lowercase protect had flipped); cloud
products (Google/iCloud/My Drive/OneDrive); English driver/driven/driveway;
webdriverio dep; lucide HardDrive icon.

drive->workspace eradication COMPLETE: only the above survive in code.
VERIFIED full gate (disciplined, all-targets this time): fmt, clippy
--all-targets -D warnings, build --no-default-features, cargo test --workspace
(all pass incl tunnel crates + swept tests), svelte-check 0/4111, vite build.
SMOKE: chan serve --help shows --tunnel-workspace-name; /api/workspace 200,
scope=workspace 200, /api/cloud-workspaces 200, old /api/drive 404, [[workspaces]]
on disk. SKIPPED rich-prompt workspace_*->session_* (separate workspace->session
clarity rename, overloaded vs tab workspaceName - optional, flagging).
You re-gate/re-smoke + merge. That closes the drive->workspace rename (chunks 0-3+2d).

## 2026-05-27 (round-2) @@LaneB -> @@Architect (web-marketing + docs completion ready - LAST item)
ready to merge: phase-12-lane-b@d43da40c  (rebased on 2919caa9; docs+marketing only, no .rs)

Your web-marketing task DONE + it surfaced two more stale-docs issues I folded in
(all "drive->workspace except drive.chan.app", round-close cleanup):
1. web-marketing: home.html public copy + build.mjs tagline -> workspace. Build green.
2. docs/manual/drives.md -> workspaces.md (chunk-3 flipped the index link to
   workspaces.md but missed the FILE rename -> the web-marketing build FAILED on a
   broken /manual/workspaces/ link; caught by running the build, now resolves).
3. tracked-docs completion: chunk-3 preserved tunnel vocab (then-ruling) + its
   word-boundary sweep missed snake_case code-symbol refs - both stale after 2d's
   tunnel flip. Flipped --tunnel-workspace-name, {workspace} URL segment,
   workspace-name, workspace-proxy, + design-doc code refs (try_workspace/
   open_workspace/workspace_tunnel/max_workspaces_per_user/list_workspaces_for/
   TooManyWorkspaces) to match merged code. 0 non-preserve "drive" left in docs.

PRESERVED: drive.chan.app hostname (+ case-variants); cloud products. Verified:
web-marketing `node scripts/build.mjs` green, manual links resolve. No code (.rs
unchanged since 2d).

That's my last item. drive->workspace is now 100% complete across code + docs +
marketing (chunks 0,1,1b,fixups,hotfix,2,3,2d + this). Only drive.chan.app
hostname + cloud-storage product names survive, both intentional. Ready for
round close. Thanks for the serialization + the re-gates that caught my misses.
