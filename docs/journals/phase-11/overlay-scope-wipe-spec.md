# Overlay-state + scope-concept wipe (next-round spec)

OWNER: graph lane. From @@Alex (2026-05-27), during the GI-8 OverlayShell
cleanup. This is the "plan the rest" follow-up to the safe OverlayShell
removal (GraphPanel dead overlay branch, landed 1d64380).

## Decision (@@Alex)

- The "SCOPE" concept is OLD and is WIPED ENTIRELY. We are pre-release, so
  there is no migration/back-compat burden - delete it.
- "panes form scope" is GONE: the machinery that derives graph scope OPTIONS
  from the open layout panes/tabs (`availableGraphScopes`) is removed.
- The new equivalent of the old "scope" is simply a DIRECTORY from the
  filesystem spine. A graph is rooted at a directory (or the drive root);
  "re-root" / "graph from here" = navigate to another directory. This matches
  the fs-graph endpoint, which already takes `scope=directory&path=<dir>`.

## Why this is NOT inert dead-code deletion (the blocker found in C3/C4)

graphOverlay / browserOverlay look like leftover overlay state but are
load-bearing TODAY:
- `graphOverlay.scopeId` is a live MIRROR of the active graph tab's scope,
  written by `mirrorGraphTabToOverlay` (store ~2005) and READ by
  `availableGraphScopes` (store ~1644-1684) which GraphPanel calls to resolve
  `currentScope` (GraphPanel scopeOptions/currentScope). 61 store refs incl.
  the HASH_GRAPH persistence (encode/decode graph filters+scope+mode+depth).
- `browserOverlay` backs the DOCK's state: `browserState = tab ?? browserOverlay`
  (FileBrowserSurface ~95); the dock (no tab) falls back to it. 14 store refs.
- `graphOverlay.open` / `browserOverlay.open` are still SET by the legacy
  `?graph=` / `?files=` hash restore (store ~1229) - so the `if(.open)` branches
  are not dead until that hash is retired.

So deleting the state outright breaks graph scope-resolution + the dock. The
wipe must re-architect first, then delete.

## Wipe plan (each step independently gated + merge-ready)

W1. Kill "panes form scope": delete `availableGraphScopes` + the `ScopeOption`
    option list + the pane-derived scope enumeration. GraphPanel stops calling
    it. The graph tab already carries its own `scopeId` string; what to fetch
    is derived from THAT (file:/dir:/drive), not a global option list.
W2. Reduce graph "scope" to a filesystem directory. The fs-graph already keys
    on `scope=directory&path=`. Decide the fate of the non-dir scope KINDS
    (drive/dir/file/tag/group/git_repo/global) - see OPEN QUESTIONS. Likely:
    drive = drive root dir; dir = that dir; file = its parent dir (the file is
    a node within); tag/group/global = removed or reframed.
W3. GraphPanel: replace `graphState = tab ?? graphOverlay` with `graphState =
    tab` (tab is always set), `visible = true`, drop the graphOverlay import +
    the dead `{#if !tab}` bar + close()'s graphOverlay branch. Replace
    `currentScope` (ScopeOption) with the directory the tab is rooted at +
    the special drive-root case. Drop `synthesizeScope` if it falls out.
W4. Dock: give FileBrowserSurface's dock variant its own local `browserState`
    ($state) instead of `tab ?? browserOverlay`. The dock doesn't render the
    inspector (isWideSurface gates it to overlay/tab), so a minimal local
    default suffices.
W5. Retire the legacy `?graph=` / `?files=` hash: on restore, degrade OLD
    bookmarks gracefully - convert to opening a graph/browser TAB at the
    path/dir, or ignore - NEVER crash. Stop WRITING HASH_GRAPH/HASH_BROWSER
    (graph/browser tabs already persist via the layout `t` array). After this
    nothing sets graphOverlay.open / browserOverlay.open.
W6. DELETE the now-dead state: `graphOverlay` + `browserOverlay` $state defs,
    `mirrorGraphTabToOverlay`, the HASH_GRAPH encode/decode, the App overlay-
    persistence $effect (App ~215-268 void-reads), the scope.svelte.ts
    browserOverlay reads (~162/298), and any remaining FileTree
    `if(browserOverlay.open)` lines. End state: no graphOverlay/browserOverlay;
    OverlayShell only in Search + Settings (already true after 1d64380).
W7. Re-gate + IN-BROWSER: graph still renders + navigates directories
    (re-root = another dir); reveal still opens an FB tab; an OLD `?graph=` /
    `?files=` bookmark loads without crashing.

## Root kinds (RESOLVED by @@Alex 2026-05-27)

The base scope is a filesystem-spine DIRECTORY. TWO cross-cutting "layers above
the spine" stay rootable, with specific semantics; everything else is dropped.

- DIRECTORY (base): root at a directory (or drive root). Depth walks the
  containment spine forward, like File Browser expand (this is GI-9's spine).
  The default + the target of "graph from here" on a dir.
- TAG (layer above the spine - connects across MARKDOWN documents): rootable.
  First-degree edges go to the documents the tag is used on. DEPTH ALWAYS 1
  (just the tag -> its docs; no deeper walk). Tags are a markdown-only layer.
- LANGUAGE (layer above the spine - connects across DIRECTORIES): rootable.
  First-degree edges go to nodes that are DIRECTORIES with the majority of
  code in that language; depth walks FORWARD like the File Browser expand-
  directory (i.e. depth>1 reveals deeper directory structure under those dirs).
- FILE: "graph from here" on a file re-roots to the file's PARENT directory
  (file pinned) - current GI-6 behavior; keep. A file is not its own root kind.
- DROPPED: group, global, git_repo as rootable scopes. (git_repo may fold into
  directory; confirm if it resurfaces.)

So `availableGraphScopes` / "panes form scope" is replaced by: directory
navigation (base) + a TAG root entry point (depth-1 -> docs) + a LANGUAGE root
entry point (depth-walk -> dirs by majority language). The fs-graph endpoint
covers directory; tag + language rooting need their own first-degree queries
(tag -> docs using it; language -> dirs by majority language) - likely small
backend additions or existing graph/language endpoints reused.

## Surfaces
web/src/components/GraphPanel.svelte, GraphCanvas.svelte;
web/src/state/store.svelte.ts (availableGraphScopes, graphOverlay,
mirrorGraphTabToOverlay, HASH_GRAPH, synthesizeScope),
web/src/state/scope.svelte.ts, web/src/App.svelte,
web/src/components/FileBrowserSurface.svelte, FileTree.svelte;
crates/chan-server/src/routes/fs_graph.rs (already directory-keyed - likely no
change). ~95 web refs total.
