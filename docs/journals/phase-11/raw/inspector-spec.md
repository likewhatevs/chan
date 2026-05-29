# Phase 11 inspector consistency + layout (new task)

New work item from @@Alex (2026-05-26), captured so it does not drift
again. The inspector is a single panel reused on three surfaces (File
Browser, editor, Graph); the Graph surface drifted from the prior spec.
This doc reconciles the recovered prior spec with @@Alex's new
requirements and assigns ownership.

## The drift @@Alex called out

- Graph inspector "Open" button does NOT open the document in the
  editor. It must, for any editable/source-code file, reusing the
  editable-file rules the File Browser already uses.
- "Graph from here" picks a new starting point (file or folder) and must
  always show its own parent folder, or the drive root.
- The folder inspector in the Graph must match the File Browser's folder
  inspector.

## Recovered prior spec (from past-phase journals)

- Consistency contract (phase-7 fullstack-a-29): inspectors live on Graph
  tabs, File Browser tabs, and any future surface; same contract for all.
  A reveal action ("Show File/Directory") spawns/focuses a File Browser
  tab for that path on non-browser surfaces.
- Shared transfer actions (phase-10 Track C): File Browser, Graph, and
  editor detail surfaces expose the same Upload/Download controls.
- Current components: `Inspector.svelte` (chrome) -> `InspectorBody.svelte`
  (dispatcher) -> `FileInfoBody.svelte` (file + dir) /
  `DirectoryInfoBody.svelte` (graph dir) / `TagInfoBody.svelte`.
- Lazy loading already exists: base metadata eager via `/api/inspector/
  {path}`; backlinks (`/api/backlinks`), report (`/api/report/file|prefix`
  streaming), and tags/links/mentions (from `graphData`) load on demand.
- Editable-file rules already exist and must be reused for "Open":
  backend `crates/chan-drive/src/fs_ops.rs::is_editable_text`; frontend
  `web/src/state/fileTypes.ts::isEditableText`. EditableText (.md, .txt)
  + Text (source code) open in the editor; Image/Pdf/Other do not.
- Drift root cause (phase-7 fullstack-a-33): per-node "Graph from here"
  was dropped in favor of an ancestor breadcrumb, but the breadcrumb is a
  top-level control, not in the inspector body, so a selected node shows
  no re-root button. fs-mode file nodes also lost the "Open" action.

## Target spec (reconciled)

One inspector, identical section model on all three surfaces; only the
contextual actions differ by surface.

Layout, top to bottom:
1. Filename / kind header.
2. ACTIONS section, directly under the filename: Open, Upload, Download,
   plus a toggle to reveal the full path. (New: these move up here.)
   - Open: enabled for any editable file per `is_editable_text`; opens it
     in the editor (via the shared tab-open path). Disabled/absent for
     non-editable kinds.
   - For media: keep the existing View + Zoom controls in this section.
   - Graph surface adds "Graph from here" (re-root on the selected file or
     folder; the graph then always shows that node's parent folder, or
     the drive root if it is top-level).
3. LAZY content below, loaded on demand and hidden when empty: chan-report
   for the file/dir, links, backlinks, tags, contacts, dates.

Folder inspector parity: the Graph folder inspector must render the same
body as the File Browser folder inspector (today Graph only shows
`DirectoryInfoBody` when `onSetAsScope` is bound; make it unconditional
and aligned with FB).

## Ownership and sequencing

- OWNER: @@LaneA owns this inspector feature end-to-end. It already owns
  the Inspector host, the File Browser, the Graph, and the lazy/
  progressive loading that this depends on (its core theme). Single owner
  is the fix for the drift.
- @@LaneB contributes ONLY the desktop-native download mechanism
  (progress indicator) and the native drag-in/out removal (its bug 2);
  @@LaneA wires the Download button into the new actions section and
  consumes @@LaneB's download-progress hook. The Upload/Download button
  PLACEMENT and the inspector body are @@LaneA's. This avoids two lanes
  editing `FileInfoBody.svelte` / `Inspector.svelte`.
- SEQUENCE: downstream of @@LaneA's File Browser + Graph slices (the
  per-instance reshape and scoped pub/sub must land first). Not started
  yet; queued. Validate the implementation against THIS doc so it does
  not drift a third time.

## Key files
- `web/src/components/{Inspector,InspectorBody,FileInfoBody,DirectoryInfoBody,TagInfoBody}.svelte`
- `web/src/components/GraphPanel.svelte` (the drifted inspector wiring)
- `web/src/state/fileTypes.ts` (`isEditableText`), `crates/chan-drive/src/fs_ops.rs` (`is_editable_text`)
- `crates/chan-server/src/routes/inspector.rs` + report/backlinks routes
