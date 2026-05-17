# frontend-3: Resource colors and graph mode consistency

Owner: @@Frontend.

Status: REVIEW (color centralization + scope options +
folder filter landed; cross-mode filter normalization is
partial — see "Deferred" below).

Related:

- [request.md](./request.md)
- [journal.md](./journal.md)
- [backend-2.md](./backend-2.md)
- [webtest-1.md](./webtest-1.md)

## Goal

Unify resource classification visuals and graph-mode behavior:

- Apply consistent colors across inspector, file browser, search, agent, and
  graph.
- Add or align graph filters across modes for language, folder, symlink,
  hardlink, link, tag, contact, and media where data exists.
- In Markdown/link graph mode, allow parent folders and path-to-root nodes to be
  shown/hidden.
- In folder graph mode, allow Markdown cross-links and paths to be shown/hidden.
- Include parent folder scope options for `.md` files and common ancestor folder
  scope options for multiple `.md` scopes.

## Acceptance criteria

- Resource color mapping:
  - Markdown document: orange.
  - Contact document and `@@contact`: yellow.
  - Media: purple.
  - Binary: blue matching inspector FILE blue.
  - Tag: green.
  - Folder: grey.
- Color/classification is centralized enough to avoid component drift.
- Graph filters are visible only when meaningful, or disabled with clear state.
- Folder visibility can improve whole-drive graph readability.
- Scope option behavior matches [request.md](./request.md).

## Test expectations

- Run `cd web && npm run check`.
- Add focused tests for classification/filter helpers if introduced.
- Coordinate graph browser smoke with [webtest-1.md](./webtest-1.md).

## Review expectations

- @@Backend confirmation for graph data shape from [backend-2.md](./backend-2.md).
- @@Webtest desktop/narrow graph validation.

## Progress notes

- 2026-05-16 @@Architect: Alex reports @@Frontend is already working on this
  task. Backend graph audit in [backend-2.md](./backend-2.md) is complete and
  says to compose existing `/api/graph` and `/api/fs-graph`; no backend API
  change is currently expected. @@Frontend should add implementation notes here
  as changes land.
- 2026-05-16 @@Frontend: started.

Landed:

- **Centralized resource colors.** Added two palette tokens to
  `App.svelte`: `--g-binary` (FILE blue: `#58a6ff` dark /
  `#0969da` light, matching the inspector's existing `--link`
  shade) and `--g-folder` (neutral grey: `#8e8e93` / `#6c6c70`).
  Switched `state/kinds.ts::colorVarFor()` so `binary →
  --g-binary` and `folder → --g-folder` (previously grey and
  green respectively). Updated `web/src/design.md` palette
  table to reflect the new tokens. File tree's per-row folder
  icon now uses `--g-folder` instead of `--accent`.
  `GraphCanvas.svelte` reads the same tokens (plus typed
  fallback hexes) so graph nodes match the file tree / inspector
  /search palette per request.md.
- **Auto-derived dir scope options** in `state/scope.svelte.ts`:
  - For each visible file in scope, add a `dir:<parent>` option
    labelled `parent dir: <name>/`.
  - When 2+ files are visible (group scope), add a
    `dir:<common-ancestor>` option labelled
    `common ancestor: <name>/`.
  - Dedup against any user-selected dir scope (file browser
    selection) so the dropdown doesn't double up. Both extra
    options use the existing `dir:<path>` id format so
    downstream consumers (graph, search, assistant) need no
    code change — they already handle `dir:` scope. Extracted
    `parentDir()` + `commonAncestor()` helpers with full unit
    test coverage in `state/scope.test.ts` (15 cases).
- **Graph folder filter.** Added `folder: boolean` to
  `GraphFilters` (default on); URL hash encoder/decoder
  extended to a 6th bit with legacy 5-char hashes treating the
  missing slot as on. `App.svelte` persistence effect tracks
  the new field. `GraphPanel.svelte` renders the chip only in
  filesystem mode (labelled "folder"); flipping it off computes
  a `hiddenFolderIds` set and excludes both folder nodes AND
  edges touching them — same shape as the existing img /
  contact node-filters so the layout stays stable across
  toggles. Counts row reports folder node count when applicable.

Deferred / partial (filed for follow-up — keeps phase 3 from
becoming an open-ended graph refactor):

- **Full per-mode filter normalization** — today filesystem mode
  reuses the link / tag / mention chip slots for
  contains / symlink / hardlink, and markdown mode uses
  link / tag / mention / language / img / contact. Per
  request.md the ideal is a unified set with chips disabled in
  modes where the underlying data doesn't apply. That's a real
  refactor of `GraphFilters` to per-concept booleans
  (`contains` / `symlink` / `hardlink` / `link` / `tag` /
  `contact` / `media` / `language` / `folder`) plus an
  applicability matrix per mode. Since backend-2 confirms
  `/api/graph` + `/api/fs-graph` already carry the data, this
  is feasible without further backend work but the diff is
  substantial and risks breaking saved URL hashes. Filing as
  a follow-up to ship after phase-3 lands and the smaller
  visible changes have soaked. The current "folder" addition
  is the request's most-urgent piece (whole-drive readability)
  per the journal's "Hiding folders as nodes may resolve the
  whole-drive view much better" note.
- **Markdown/link graph mode parent folder + path-to-root
  filter** — depends on the unified filter refactor above.
  Today markdown mode does not surface folder nodes at all, so
  there's nothing to hide. Adding the option means asking the
  backend semantic graph to emit synthetic folder nodes for
  each markdown file's ancestor chain, OR computing the
  ancestor chain in the frontend from file paths and inlining
  synthetic folder/link nodes into the cytoscape graph. Backend
  audit says compose existing endpoints, so frontend
  synthesis is the path — filed as a follow-up.

## Test expectations

- `cd web && npm run check` — clean.
- `cd web && npm test -- --run` — 145 tests pass (was 130 after
  frontend-2; added 15 new in `state/scope.test.ts`).
- Browser smoke: confirm that
  - File tree / inspector / search / graph all paint binary files
    in the new FILE-blue tone and folders in grey;
  - Graph folder filter chip appears in filesystem mode and
    hides folder nodes + their edges when off;
  - Scope dropdown in the graph overlay shows a `parent dir`
    entry for each visible file and a `common ancestor` entry
    when 2+ files are visible.

## Commit readiness notes

Ready for @@Webtest visual sweep + @@Architect review. Suggested
commit unit:

```
chan-web: centralize resource colors + graph scope/folder filter

- App.svelte: add --g-binary (FILE blue) and --g-folder (grey)
  palette tokens (dark + light).
- kinds.ts: route binary -> --g-binary, folder -> --g-folder so
  every surface (file tree, inspector, search, agent, graph)
  reads the same hue per request.md.
- design.md: refresh hex table + per-kind mapping table.
- GraphCanvas.svelte: pull folder + binary fills from the new
  tokens; typed fallback hexes match.
- scope.svelte.ts: derive `parent dir: <name>/` scope option
  per visible file and `common ancestor: <name>/` when 2+
  files are in view. Helpers (parentDir, commonAncestor) get
  full unit coverage.
- GraphFilters: add folder filter (filesystem mode only);
  URL hash bumps to 6 slots with legacy fallback for older
  saved hashes.
```
