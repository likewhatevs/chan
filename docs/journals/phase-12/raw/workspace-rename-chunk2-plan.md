# Chunk 2 execution plan: the coordinated wire + frontend flip (@@LaneB)

Prepared while HELD (Alex: hold + prep; @@LaneA/@@LaneC not yet quiescent).
Chunk 2 is the LAST + biggest slice: it flips every "drive" wire surface to
"workspace" AND the full web/src frontend AND the folded chunk-1b internal
eradication AND CLI copy - all together, in @@Architect's web/src freeze window,
with BROWSER/runtime verification (the gate is blind to this whole class; it has
already bitten 3x this round).

## 0. Preconditions (do not start until ALL true)
- @@LaneA reports graph/FB quiescent + paused (it owns store/GraphPanel/scope/
  tabs/GraphCanvas/FileTree/App - my hottest files); @@LaneC web/src work parked.
- @@Architect opens the web/src + routes freeze window + announces it.
- Rebase phase-12-lane-b onto the post-quiescence main FIRST.
- Declare the web/src touch on event-lane-b-lane-a.md + event-lane-b-lane-c.md.

## 1. Backend wire flips (remove the chunk-1 "drive" pins, flip to "workspace")
- 4 serde enum tags: DELETE the `#[serde(rename = "drive")]` I added on
  GraphScope / InspectorKind / ResetModeView / CloseReason `Workspace` variants
  (graph.rs, inspector.rs, storage.rs, terminal_sessions.rs). They then
  serialize "workspace" - paired with the client.ts flip in this same chunk.
- HTTP routes (chan-server lib.rs route table + routes/): "/api/drive" ->
  "/api/workspace", "/api/drive/bootstrap" -> "/api/workspace/bootstrap",
  "/api/cloud-drives" -> "/api/cloud-workspaces". Handler fn names
  api_get_drive/api_patch_drive/api_drive_bootstrap/api_cloud_drives ->
  *_workspace. Rename routes/drive.rs -> routes/workspace.rs.
  CAUTION: host.rs prefix tests use "/driveway/api/drive" + "/drive/api/drive"
  to test prefix matching - HAND-EDIT, no find/replace.
- On-disk (clean break, no migration): registry.rs DELETE the
  `#[serde(rename="default_drive_root")]` (field already default_workspace_root)
  -> TOML key flips to default_workspace_root; rename the `drives` Vec field ->
  `workspaces` ([[drives]] -> [[workspaces]]); paths.rs `join("drives")` ->
  `join("workspaces")` (~/.chan/drives/ -> ~/.chan/workspaces/).

## 2. Frontend web/src flip (~1100 lines, ~50 files)
- File renames (+ update all imports): DriveInfoBody.svelte ->
  WorkspaceInfoBody.svelte; DriveWarningsModal.svelte ->
  WorkspaceWarningsModal.svelte; state/driveWarnings.test.ts ->
  workspaceWarnings.test.ts.
- api/client.ts: `drive()` -> `workspace()` (route -> /api/workspace); graph
  `scope: "drive"` -> "workspace" (lines 206/955/964); cloud-drives ->
  cloud-workspaces; ResetMode usage.
- api/types.ts: DriveInfo -> WorkspaceInfo; InspectorKind union "drive" ->
  "workspace" (l592); ResetMode "drive" -> "workspace" (l733); the /api/drive
  comments.
- Hot files (counts at prep time): store.svelte.ts (168), GraphPanel (96),
  tabs.svelte.ts (45), FileBrowserSurface (42), scope.svelte.ts (34), GraphCanvas
  (25), SettingsPanel (22), EmptyPaneCarousel (22), FileTree, App.svelte: flip
  "drive"->"workspace" idents + user-facing copy.
  PRESERVE in frontend: any cloud-drive product copy + the "dir scope / drive
  root" WS comments are cosmetic (the chan root IS the workspace, so "workspace
  root" is the right flip - but verify no cloud-product string is hit).

## 3. Rich-prompt FIELD flip (paired carry-over from chunk 0)
Chunk 0 renamed the TYPE RichPromptWorkspace->RichPromptSession but DEFERRED the
data field names. Flip them now, producer+consumer together:
- chan-drive rich_prompts.rs struct fields workspace_path/workspace_abs ->
  session_path/session_abs; chan-server routes/rich_prompts.rs response struct +
  mapping; web api/types.ts (l175/178) workspace_path/abs; tabs.svelte.ts
  (l396/399) workspacePath/Abs -> sessionPath/Abs; TerminalTab.svelte (~20 hits)
  + TerminalRichPrompt.svelte workspace* state fields + `.workspace-row` CSS ->
  session*. (These were the files that entangled @@LaneA tabs + @@LaneC
  TerminalTab - now safe inside the freeze.)

## 4. Folded chunk-1b internal eradication (the gate-invisible class - CAREFUL)
Do AFTER the wire flips, with the same producer+consumer discipline:
- serde/IPC fields named drive/drives -> workspace/workspaces, EACH with both
  sides: desktop IPC arg `drive` (main.rs:135/941/1305) + the JS invoke `{ drive }`
  args; desktop config.drives + registry.drives (persisted) ; `chan list` JSON
  `drives` key (main.rs:1954 + any client reading it).
- Tauri command fn names: flip the REMAINING *_drive commands (add_drive,
  set_drive_on, get/set_drive_features, compute_drive_preflight,
  default_drive_status, choose/create/factory_reset_default_drive, open_local_drive)
  -> *_workspace, TOGETHER with their app.toml perms + main.js invoke() strings.
  PRESERVE the tunnel commands (open_tunneled_drive, open/add/remove_outbound_drive).
- local `drive` vars / fn params -> workspace (internal; safe last pass).
- module file chan-workspace/src/drive.rs -> workspace.rs (+ `mod drive` ->
  `mod workspace`, internal `crate::drive::` paths).

## 5. CLI copy
main.rs help/about "drive" -> "workspace"; `--drive` flag (contacts import) ->
`--workspace`. PRESERVE drive.chan.app + the --tunnel-drive flag + tunnel help.

## 6. STILL PRESERVED after chunk 2 (the non-chan-dir "drive" meanings)
- cloud products: GoogleDrive/iCloudDrive/OneDrive/CloudDriveJson + "Google
  Drive"/"iCloud Drive"/"My Drive" detection strings.
- tunnel/domain: ALL chan-tunnel-* crates, drive.chan.app domain, --tunnel-drive,
  MAX_DRIVE_NAME_LEN, TUNNELED_DRIVE_READY, *-outbound-drive, list_drives_for,
  open_tunneled_drive.

## 7. Gate + VERIFY (the gate is blind to the wire/serde/IPC class)
- cargo: fmt / clippy -D warnings / test / build --no-default-features.
- web: npm run check + npm run build.
- BROWSER smoke (mandatory - this is the class that shipped broken 3x): serve a
  scratch drive, exercise graph (scope=workspace now), inspector, storage reset,
  file browser, link nav; check the WS frames + console for serde errors.
- Desktop smoke: launcher list/add/remove workspace (IPC perms), default-workspace
  flow, terminal env (CHAN_WORKSPACE_*), CLI->desktop handoff (rebuild BOTH).
- grep audit post-flip: zero `"/api/drive"`, zero `[[drives]]`, zero
  `default_drive_root` TOML, zero un-pinned "drive" serde tag the frontend reads;
  and confirm cloud/tunnel "drive" strings UNCHANGED.

## 8. Report
Each coherent slice gated + `ready to merge: phase-12-lane-b@<sha>`. Given the
size, may split: (2a) backend wire + routes + on-disk; (2b) frontend; (2c)
folded internal eradication + desktop IPC + CLI copy - but 2a/2b MUST land
together-or-adjacent so the running app is never wire-skewed mid-merge.
@@Architect serializes + announces the freeze close.
