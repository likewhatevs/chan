# @@LaneB journal (drive -> workspace rename, SCOPE FIRST)

Append-only, self-documenting. @@LaneB = scoping architect. Phase 1 = scope.

## 2026-05-27 -- Phase 1 scoping pass complete

Read: bootstrap.md, phase-12-backlog.md, lane-b-plan.md, all four lane-b
channels, coordination README; `crates/chan-drive/design.md` + root `design.md`
in full; ground-truthed every "drive" surface with rg sweeps + a live
`~/.chan/config.toml`. Wrote `docs/journals/phase-12/workspace-rename-spec.md`.

Findings that shaped the spec:

- THE collision (key): "workspace" already means FOUR things (Cargo
  `[workspace]`, team workspace `teams.rs`, draft workspace `drafts.rs`, Rich
  Prompt workspace `rich_prompts.rs::RichPromptWorkspace`). All three non-Cargo
  meanings are SUB-dirs inside a drive (under Drafts metadata), so `Drive` ->
  `Workspace` inverts the hierarchy (a Workspace would contain team
  workspaces). This changes the whole rename and is decision 1 to @@Alex.
- uniffi is ASPIRATIONAL: no uniffi dep, no .udl, no `#[uniffi::export]`. All
  mentions are "uniffi later" doc comments. Zero live bindings to break. The
  native-bindings decision is de-risked to "future-shell expectations + comment
  text only".
- The Rust idents are DECOUPLED from the on-disk format (serde), HTTP routes
  (strings), and tunnel domain (string). Pinning those three wire boundaries
  turns the rename into an internal-only refactor that can land in
  independently-gated layer chunks (backend early, frontend last). This is the
  spine of the sequencing proposal.
- Scale: ~420 `Drive` idents, ~432 `chan_drive::` paths, 147 files touch the
  crate name, ~1145 frontend lines, ~331 CLI lines, ~700 desktop lines.
- Frontend overlap with @@LaneA is near-total on the hot files (store.svelte.ts
  176, GraphPanel 100, FileBrowserSurface 42, scope 34, tabs 45). Frontend
  codemod chunk MUST land after @@LaneA quiescence.
- CAUTION recorded for the codemod: `host.rs` has `/driveway/api/drive` and
  `/drive/api/drive` prefix tests that naive find/replace would corrupt.

Decisions surfaced to @@Alex on event-lane-b-alex.md (standing gate). Progress
reported to @@Architect on event-lane-b-architect.md. NO codemod started; phase
2 gated on @@Alex ratifying decisions 1-3 + @@Architect picking the windows.

Worktree note: `../chan-lane-b` not yet created. Not needed for phase-1 (docs
only, written in main checkout). Will request/create it when chunk 1 is
greenlit.

## 2026-05-27 -- All 5 scope decisions RATIFIED by @@Alex

Via event-lane-b-alex.md + chat:
1. Term: Option C - `Workspace` = the drive; free the word by renaming the 4
   incumbents (team/draft/rich-prompt) first. Cargo `[workspace]` unrelated.
2. Tunnel domain `drive.chan.app`: KEEP as-is (the one surviving "drive"
   string; codemod must not sweep it).
3. On-disk + routes: FULL CLEAN BREAK, no migration ("as if drive never
   existed"). default_drive_root/[[drives]]/drives-dir + /api/drive routes all
   rename; existing registries + bookmarks break, accepted pre-release.
4. uniffi: non-issue (no live bindings). FYI only.
5. `Library` handle: UNCHANGED ("Library is the right name"). Only its
   *_drive* methods rename.

Spec updated to final (workspace-rename-spec.md sections 0-6). Refinement
captured: "clean break" is an END-STATE rule, NOT one-commit. The Rust
crate/type rename (chunk 1) can land EARLY with route-path + on-disk serde
STRINGS left as literals; those flip with the frontend consumer in chunk 2.
End state is 100% clean, sequencing stays splittable.

Scoped Option C chunk 0 (free the word): team = prose only (idents already
Team*); draft = private fn renames in drafts.rs; rich-prompt = public type
`RichPromptWorkspace` -> proposed `RichPromptSession` + Drive methods +
chan-server routes + TerminalRichPrompt.svelte state fields/CSS (terminal area,
NOT @@LaneA graph/FB). Outstanding minor sub-decision: RichPromptWorkspace
target name (default RichPromptSession).

Final 4-chunk sequencing proposed to @@Architect (chunk 0 free-the-word; chunk 1
backend+crate early; chunk 2 wire+frontend last after @@LaneA quiescence; chunk
3 docs). @@Alex: write docs, architect reviews + greenlights, then resume.
STANDING BY for @@Architect window calls. No code touched.

## 2026-05-27 -- Phase 2 begins: CHUNK 0 done (greenlit by @@Architect)

@@Architect green-lit (event-architect-lane-b.md 12:05): 4-chunk plan APPROVED,
RichPromptSession APPROVED, worktree + chunk-0-may-start-now with a cross-lane
declaration precondition. @@Alex poked; proceeded on the greenlight.

Setup: created worktree ../chan-lane-b on phase-12-lane-b from main (fe6e126).
Declared chunk-0 touches on event-lane-b-lane-c.md, then SLIMMED the declaration
after discovering the rich-prompt field naming entangles @@LaneC's
TerminalTab.svelte + @@LaneA's tabs.svelte.ts -> deferred field/wire/frontend
rename to chunk 2; chunk 0 became Rust-only with ZERO cross-lane overlap.

CHUNK 0 committed: phase-12-lane-b@6c5a2c6 (9 files, +91/-94).
- rich-prompt: RichPromptWorkspace -> RichPromptSession (type/methods/handlers/
  internal+test fns/prose). KEPT data fields workspace_path/workspace_abs (a
  session owning a workspace_path dir is coherent + non-colliding) + wire +
  frontend -> chunk 2.
- draft: promote_workspace/preflight_workspace_merge/copy_workspace_* -> *_draft.
- team: prose only.
Naming wrinkle handled: RichPromptSession brushes terminal "session" vocab in
routes/rich_prompts.rs (query.session); kept supporting symbols on rich_prompt
naming (session_status_for_rich_prompt, rich_prompt_response). Flagged to
architect in case they prefer RichPromptDraft (one-token flip).

GATE clean: fmt --check / clippy -D warnings / build --no-default-features /
test (chan-drive 540, chan-server 346 + integration bins). Web untouched.
Reported ready-to-merge on event-lane-b-architect.md.

Missed-then-caught during gate: two re-export sites outside my initial file set
(chan-drive/src/lib.rs:92, chan-server/src/routes/mod.rs:68) - cargo check
surfaced them; fixed + a tree-wide rg confirmed zero old symbols. Lesson for
chunk 1: grep re-export lists (lib.rs pub use, routes/mod.rs) up front.

NEXT: chunk 1 HELD on @@LaneC Bug 3 (self-writes) merge per architect window.
Awaiting poke on event-architect-lane-b.md. STANDING BY.

## 2026-05-27 -- CHUNK 1 PREP (read-only; chunk 1 still HELD on Bug 3 MERGE)

State on poke: @@LaneC Bug 3 is COMMITTED on phase-12-lane-c (daed33a) + Bug 2
(cb1f113), but NOT merged to main (still fe6e126). My chunk-1 hold is on Bug 3
MERGE, so still correctly blocked; no architect poke yet. @@LaneC acked my
chunk-0 slim on event-lane-c-lane-b.md (no overlap). Did read-only prep so I can
execute chunk 1 the instant Bug 3 merges + I rebase lane-b.

CHUNK 1 EXECUTION CHECKLIST (after Bug 3 merges; rebase lane-b onto new main first):
1. Crate rename: `git mv crates/chan-drive crates/chan-workspace`;
   chan-workspace/Cargo.toml name="chan-workspace"; root Cargo.toml member +
   [workspace.dependencies] entry (note version string "0.15.4"); dependents
   (chan-server, chan, chan-llm, fetch-models, desktop) dep lines + feature
   passthroughs (chan-drive/embeddings|metal|cuda -> chan-workspace/...).
2. Symbol renames (all .rs):
   - chan_drive -> chan_workspace (global; covers use/paths/re-exports).
   - Drive -> Workspace (WHOLE-WORD). Separately: DriveCell->WorkspaceCell,
     DrivePaths->WorkspacePaths, DriveLock->WorkspaceLock (whole-word Drive
     won't catch these compounds).
   - Library STAYS; its methods: open/register/unregister/move/reset_drive ->
     *_workspace; list_drives->list_workspaces; drive_paths_for->
     workspace_paths_for; (set_/effective_)default_drive_root METHODS+paths::
     free fn -> *_workspace_root.
   - ChanError::DriveNotRegistered/DriveLocked/DriveAlreadyOpen -> Workspace*.
   - chan-server handler fns api_get_drive/api_patch_drive/api_drive_bootstrap/
     api_cloud_drives -> *_workspace* (CODE symbols; route PATH strings stay).
   - CLI main.rs help/about copy + --drive flag -> --workspace. desktop:
     git mv default_drive.rs default_workspace.rs + refs.
3. PRESERVE as literals (wire; flip in CHUNK 2, do NOT touch now):
   - route path strings "/api/drive", "/api/drive/bootstrap", "/api/cloud-drives"
   - paths.rs "drives" dir literal: config_dir().join("drives")
   - registry.rs Registry serde FIELD names default_drive_root + drives (these
     -> config.toml keys/tables; methods rename, field names stay). SUBTLE:
     read registry.rs at exec to confirm field-vs-method split.
   - drive.chan.app domain (ruling 2; permanent).
4. Re-export gotchas to grep UP FRONT (chunk-0 lesson): every `pub use`,
   routes/mod.rs, `use chan_drive::{...}` (confirmed sites: chan-server routes/
   {inspector,metadata,report,search,graph}.rs + more).
5. Gate: fmt / clippy -D warnings / test / build --no-default-features. Frontend
   untouched in chunk 1 -> npm N/A (chunk 2 owns web/src). Cargo.lock churn
   expected + mechanical; commit it.

## 2026-05-27 -- CHUNK 1 done (crate + type rename) -> phase-12-lane-b@0b6735e

Unblocked: @@Architect released chunk 1 (Bug 3 merged into f72b8a7; chunk 0 +
LaneA W1/W3/W4 also in). Rebased lane-b onto f72b8a7 (reset --hard; chunk 0
already in it). Executed crate + type contract rename.

THE big discovery: "drive" = THREE meanings. Naive `\bDrive\b` corrupted two:
- cloud products: "Google Drive"->"Google Workspace" (DIFFERENT product!),
  "My Drive"->"My Workspace" (a FUNCTIONAL GDrive detection path in paths.rs).
  Caught by tunnel_drive_flag test + a string-literal audit; reverted all.
- tunnel/domain (drive.chan.app): excluded chan-tunnel-* entirely + preserved
  --tunnel-drive/MAX_DRIVE_NAME_LEN/TUNNELED_DRIVE_READY.
Lesson: word-boundary rename still hits product-name + path STRING literals;
audit `"..Workspace.."` string literals + uppercase consts after a broad rename,
not just compile errors. The compile + most tests were GREEN while "Google
Workspace" / `join("My Drive")` were silently wrong - only one test caught it.

Method: staged seds (crate path -> compile -> compound types + Library methods
-> bare \bDrive\b -> default_drive_root+serde -> compile -> cloud/tunnel reverts
-> gate). Re-export gotchas (chan-drive lib.rs pub use, routes/mod.rs) caught by
compile as in chunk 0.

DEFERRED chunk 1b: internal lowercase `drive` vars + comments + non-tunnel
uppercase consts (MAX_ACTIVE_DRIVES, MAX_WINDOWS_PER_DRIVE). CLI user-facing
help copy deferred to chunk 2 (flips with routes/on-disk so users don't see
mixed terminology). Both flagged to @@Architect for timing.

GATE clean (see architect report). 120 files; chan-tunnel-* + web/ untouched.

## 2026-05-27 -- Terminal env-var slice -> phase-12-lane-b@47b127e

@@Architect appended an @@Alex add-on task (event-architect-lane-b.md): expose
CHAN_WORKSPACE_NAME + CHAN_WORKSPACE_PATH to the Hybrid Terminal. Backend-only,
independent of chunk 2, land after chunk 1. Did it on top of chunk 1 (0b6735e).

- terminal_sessions.rs spawn: CHAN_WORKSPACE_PATH = config.drive_root (abs),
  CHAN_WORKSPACE_NAME = root basename (no user-managed name; path-derived label
  per design). Added both to clear_mcp_env (respawn hygiene).
- routes/terminal.rs: extended the env-probe test (RegistryConfig.drive_root=cwd
  in TestTerminal::spawn, so asserted PATH=cwd.display(), NAME=cwd basename).
Names already "workspace" (rides chunk 1; no CHAN_DRIVE_* interim).
Gate: fmt, clippy -D warnings, chan-server terminal tests 29 passed. 2 files.

OUTSTANDING (pending @@Architect): merge chunk 1 (0b6735e) + this (47b127e);
ruling on chunk 1b timing; chunk 2 held on @@LaneA quiescence.

## 2026-05-27 (round-2) -- chunk-1 fixups + chunk-1b consts -> phase-12-lane-b@4ddc657

Rebased onto main 4cb5ca8. Two commits:
- 304b9ff FIXUP (@@LaneE's chunk-1 artifacts): app.toml perms + desktop/src/main.js
  invokes for the two renamed commands (list_workspaces/remove_workspace) - they
  were renamed in main.rs but the runtime-checked perm allowlist + JS invoke
  strings still said list_drives/remove_drive (IPC denial; gate-invisible). Found
  main.js myself. Handoff #2 = NO live bug (zero live open_drive; the enum is
  consistently OpenWorkspace; LaneE hit a fresh-CLI-vs-stale-desktop skew).
- 4ddc657 chunk-1b consts: MAX_ACTIVE_DRIVES family + MAX_WINDOWS_PER_DRIVE ->
  *_WORKSPACE(S). Preserved tunnel MAX_DRIVE_NAME_LEN/TUNNELED_DRIVE_READY.

KEY CALL: deferred the lowercase `drive` var/field eradication to chunk 2. Risk
audit found serde/IPC fields named drive/drives (desktop IPC args, desktop config/
registry, `chan list` JSON, tunnel snippets) - a blind sweep SILENTLY breaks the
wire (LaneE's exact gate-invisible class). Recommended chunk-2 fold to @@Architect.

Lesson reinforced: the cargo gate (fmt/clippy/test) CANNOT catch runtime IPC/serde
string mismatches. After any rename touching Tauri commands/perms or serde tags,
manually verify command-name strings (app.toml, *.js invokes) + serde field/tag
names against the renamed symbols. Two such artifacts shipped to main on chunk 1
because the gate was green.

## 2026-05-27 (round-2) -- graph-scope serde HOTFIX -> phase-12-lane-b@2256aa8

@@Architect flagged the /api/graph scope variant (chunk 1's \bDrive\b renamed
GraphScope::Drive->Workspace under rename_all, flipping the wire tag; frontend
sends scope:"drive" -> 400 on main). Swept ALL serde enums: SAME slip in FOUR
(GraphScope, InspectorKind, ResetModeView, CloseReason) - inspector + reset were
ALSO broken on main. Pinned all 4 with #[serde(rename = "drive")] (idents stay
Workspace; wire tag "drive" until chunk 2). Confirmed these are the only 4
Workspace variants in non-tunnel crates; all frontend "drive" wire values now map
to a pinned tag.

SMOKED fresh-binary: /api/graph?scope=drive -> 200 nodes; scope=workspace -> 400.
Gate: fmt/clippy/chan-server 347. Reported MERGE-ASAP (unbreaks main).

PATTERN (3rd gate-invisible wire slip from chunk 1): cargo cannot catch renamed
serde enum tags / Tauri perm strings / JS invoke names. After ANY rename, audit:
serde enum variants under rename_all, serde field names, Tauri app.toml command
allowlists + *.js invoke() strings, route path strings. The cargo gate being
green means nothing for these. Saved to memory.

## 2026-05-27 (round-2) -- HOLDING chunk 2; full execution plan written

@@Alex said "unblocked to continue", but I verified the bus: @@LaneA's own report
says "Still not quiescent (chunk 2 waits on me)" (mid graph loading-state, touches
store/GraphPanel/scope), @@LaneC has more web/src queued (drag-drop, terminal),
@@Architect = "idle until chunk 2 window". Surfaced the reconciliation to @@Alex
(didn't silently start chunk 2 -> would detonate the cross-lane rebase the
sequencing avoids; didn't silently sit). @@Alex chose HOLD + PREP.

Wrote `docs/journals/phase-12/workspace-rename-chunk2-plan.md`: the full chunk-2
token-map + execution + verification plan. Key points: chunk 2 flips ALL the
"drive" wire surfaces (4 serde tags I pinned, /api/drive routes, [[drives]]/
default_drive_root/drives-dir on-disk) + full web/src (~1100 lines) + the
deferred rich-prompt field flip + the folded chunk-1b internal eradication
(serde/IPC fields, Tauri command names + app.toml + main.js together) + CLI copy
- all in @@Architect's freeze window with MANDATORY browser/desktop smoke (the
gate is blind to this class; it shipped broken 3x already). Preserved post-chunk-2:
cloud products + tunnel/domain (drive.chan.app).

IDLE, holding for @@Architect's chunk-2 window. Plan ready -> fast execution.

## 2026-05-27 (round-2) -- CHUNK 2 in progress: 2a+2b done+validated (in-freeze)

@@Architect opened the web/src + routes freeze (A+C+E quiescent). Executing
chunk 2 per workspace-rename-chunk2-plan.md, atomic in-freeze. Rebased onto main
22621db; declared web/src touches to LaneA/LaneC.

- 2a (da32e50) backend wire: routes /api/drive*->/api/workspace*, handlers,
  routes/drive.rs->workspace.rs, UNPINNED the 4 hotfix serde tags (now serialize
  "workspace"), on-disk drives->workspaces field + default_workspace_root key +
  ~/.chan/workspaces/ dir, module drive.rs->workspace.rs. 540+347 tests green.
- 2b (0c36d6f) frontend: web/src ~all flipped with word-boundary + camelCase
  guards. CAUGHT false-positives: preserved driven/driving/driveable (English) +
  the lucide HardDrive ICON (reverted HardWorkspace->HardDrive). web/src had NO
  cloud/tunnel "drive" strings (provider names come from backend at runtime).
  Component renames + api client/types + suffix compounds. target_escapes_drive
  ->_workspace both sides. Validated: svelte-check 0/4111, vite build clean.

REMAINING 2c (in-freeze): rich-prompt session field rename (workspace_*->session_*),
backend local-var eradication (workspace.rs/fs_ops/indexer + chan-server locals),
desktop IPC command rename (add_drive etc. + app.toml + main.js), CLI copy +
--drive flag. THEN mandatory browser + desktop SMOKE (the gate-blind class).
Then squash/report phase-12-lane-b@<sha> as the atomic chunk-2 (2a+2b+2c).

LESSON (4th gate-blind near-miss): the frontend sweep would've corrupted the
lucide HardDrive icon + English "driven"/"driving" - word-boundary + camelCase
guards + svelte-check caught them. Third-party identifiers are another preserve-
class alongside cloud-products + tunnel.

## 2026-05-27 (round-2) -- CHUNK 2 COMPLETE + smoked -> phase-12-lane-b@814d3987

Squashed 2a+2b+2c into one atomic commit on freeze-point 22621db (222 files).
Full wire+frontend+CLI+desktop drive->workspace flip; app serves as "workspace"
end-to-end. Verified: full rust gate + vitest 1620 + svelte-check + vite build +
a fresh-binary SMOKE (/api/workspace 200, scope=workspace 200, /api/drive 404,
[[workspaces]] on disk).

Gate-blind near-misses caught this chunk (the recurring class): lucide HardDrive
icon (svelte-check), cloud "Google Drive"/"My Drive" detection strings, tunnel
ClientConfig.drive + TunneledDrive.drive + Registration.drive fields, the
drive-window Tauri capability (build.rs), serde scope-tag tests, tunnel-slug
test data. Word-boundary + camelCase + per-crate cloud/tunnel restore + the
smoke were essential; cargo green alone proved nothing for the wire/IPC/cap class.

DEFERRED -> 2d (flagged to @@Architect): deep snake-case `drive` compounds
(drive_root/cell/dir/try_drive/walk_drive - hundreds, internal/non-wire/consistent)
+ rich-prompt session field rename (overloaded vs tab workspaceName). Left to
avoid a silent serde break for cosmetic internal naming at freeze tail.

Reported ready-to-merge on event-lane-b-architect.md. Awaiting @@Architect
re-gate/re-smoke + merge + 2d sequencing.

## 2026-05-27 (round-2) -- chunk 2 re-gated after @@Architect caught 2 defects -> @adcee898

@@Architect did NOT merge 814d3987 - re-audit caught 2 gate-blind defects I shipped:
(1) config wire mismatch (frontend default_drive_root vs backend default_workspace_root
-> silent UI break), (2) desktop compile break (default_drive.rs set_default_drive_root).
ROOT CAUSE: my final default_drive_root flip excluded web/src + desktop, and I ran
clippy --all-targets BEFORE that flip + cargo check on a crate-SUBSET (not desktop)
after. Committed without a full re-gate. My gap.

Fixed both (frontend + desktop default_workspace_root), closed the gate gaps:
clippy --all-targets -D warnings (incl chan-desktop, compiles) + full test incl
-p chan-desktop (RUST 0) + vitest 1610 + svelte-check 0 + CONFIG ROUND-TRIP SMOKE
(PATCH+GET+config.toml all default_workspace_root). Amended to phase-12-lane-b@adcee898.

LESSON: a crate-subset `cargo check` is NOT a gate; `clippy --all-targets` is the
only thing that compiles chan-desktop. Re-run the FULL gate after EVERY late edit,
never commit on a partial. (5th gate-blind incident this round; this one self-
inflicted by skipping the full re-gate.)

## 2026-05-27 (round-2) -- CHUNK 3 (docs) + 2d (internal+tunnel) done; rename COMPLETE

chunk 3 (306d9c45): docs sweep, 40 tracked .md files. (Stuck-commit incident:
backgrounded `git commit -F - <<heredoc` hung on detached stdin; killed pid,
cleared worktree index.lock, re-committed via message file. Lesson: don't pipe
heredoc to a committed command that may background.)

chunk 2d (2ec65f39): @@Alex expanded scope mid-flight - "everything drive->
workspace except the drive.chan.app hostname". So tunnel (previously preserved)
is now flipped: chan-tunnel-* crates, --tunnel-drive->--tunnel-workspace-name
(@@Alex: it sets the published name), TunneledDrive, drive_name, error codes,
{drive} URL, drive-proxy, default_drive.rs->default_workspace.rs. PLUS the
internal snake_case + the tests/ dirs my src-only globs had MISSED (big gap:
hundreds of drive_root test locals, DRIVE_GATE consts, drive2, max_drives_per_user).

Preserved: drive.chan.app hostname (caught+restored a case-variant "ALICE.Drive.
Chan.App" the lowercase protect flipped), cloud products, English driver/driven/
driveway, webdriverio dep, HardDrive icon.

GATE (disciplined, full clippy --all-targets + test --workspace this time - the
chunk-2 gate-gap lesson applied) + SMOKE (--tunnel-workspace-name in --help;
workspace routes + on-disk). All green. Reported both slices ready-to-merge.

drive->workspace RENAME COMPLETE across chunks 0,1,1b,fixups,hotfix,2,3,2d. Only
the drive.chan.app hostname + cloud products survive in code (both intentional).
SKIPPED rich-prompt workspace_*->session_* (separate, optional, overloaded).

LESSONS this round (6 gate-blind/scope incidents): (1) cargo-green is blind to
serde-tag/Tauri-perm/route-string/IPC-arg/Tauri-capability runtime mismatches -
audit+smoke them. (2) full `clippy --all-targets` is the only thing that compiles
chan-desktop; never commit on a crate-subset check. (3) word-boundary sweeps miss
snake_case compounds AND src-only globs miss tests/. (4) preserve-classes: cloud
products, hostname case-variants, third-party idents (HardDrive/webdriverio),
English (driven/driveway). (5) backgrounded heredoc-commit hangs.

## 2026-05-27 (round-2) -- web-marketing + docs completion -> d43da40c (LAST item)

@@Alex follow-up (web-marketing) + the docs staleness it surfaced. home.html +
build.mjs copy flipped (drive.chan.app kept). web-marketing BUILD caught a broken
link: chunk-3 flipped the manual index link drives.md->workspaces.md but missed
the file rename -> git mv docs/manual/drives.md->workspaces.md. Plus folded the
tracked-docs completion (chunk-3 preserved tunnel + missed snake code-symbol refs,
stale after 2d): --tunnel-workspace-name, {workspace} URL, design-doc code refs.
0 non-preserve drive left in docs. web-marketing build green.

drive->workspace COMPLETE: code (0-2,2d) + docs (3 + this) + marketing. Only
drive.chan.app hostname + cloud products survive. Reported ready-to-merge; last
item before round close.
