// API types: the JSON shapes returned by chan-server's HTTP handlers.
// Keep in lockstep with crates/chan-server/src/routes.

export type WorkspaceInfo = {
  root: string;
  /// Path-derived display label from the server. This is not
  /// persisted user metadata; full root remains authoritative.
  label: string | null;
  metadata_key: string | null;
  /// Mirror of GlobalConfig.preferences. Per-workspace overrides
  /// were removed; settings are always per-device-global. Carried
  /// here so a single `/api/workspace` round-trip is enough to
  /// render the editor with the right fonts without a follow-up
  /// `/api/config` fetch.
  preferences: Preferences;
  /// Non-fatal boot warnings, currently used for broken draft
  /// workspaces under metadata.
  warnings: WorkspaceWarning[];
};

export type WorkspaceWarning = {
  kind: string;
  path: string;
  message: string;
};

export type MetadataExportDownload = {
  blob: Blob;
  filename: string;
  files: number | null;
  bytes: number | null;
};

export type MetadataImportReport = {
  manifest: MetadataManifest;
  imported_subtrees: string[];
  files: number;
  bytes: number;
  rescanned: boolean;
};

export type MetadataManifest = {
  archive_format_version: number;
  chan_version: string;
  created_at: string;
  source_root: string;
  source_metadata_key: string;
  metadata_schema: {
    path_key_scheme: string;
    index_schema_version: number;
    graph_user_version?: number | null;
    vector_shard_format_version?: number | null;
    report_schema_version?: number | null;
  };
  scm?: {
    remotes: string[];
    head?: string | null;
  } | null;
  included_subtrees: string[];
  excluded_subtrees: string[];
};

/// Global per-user config. Lives at `paths::global_config_path()`
/// on the server side and applies to every workspace (no per-
/// workspace override anymore; settings are always device-global).
export type GlobalConfig = {
  preferences: Preferences;
  /// When set, the resolver's fallback path becomes this; when
  /// unset, it falls back to the platform convention
  /// (`~/Documents/Chan` on macOS, `$XDG_DATA_HOME/chan/default`
  /// on Linux, `%USERPROFILE%\Documents\Chan` on Windows).
  default_workspace_root?: string | null;
  /// Known workspaces the user has opened on this machine. Updated
  /// by the server on every spawn (touch existing or append).
  /// Sorted most-recent first.
  workspaces?: KnownWorkspace[];
};

export type KnownWorkspace = {
  path: string;
  metadata_key: string;
  /// RFC3339 timestamp.
  last_seen_at: string;
};

/// Editor theme. Workspaces the markdown renderer + source view
/// typography and chrome. Light/dark variants are selected from
/// the active ThemeChoice; density from LineSpacing. App chrome
/// (toolbar, panes, status bar) is not affected.
export type EditorTheme = "github" | "google_docs" | "word";

export type ThemeChoice = "system" | "light" | "dark";
export type SurfaceThemeChoice = "light" | "dark";
export type HybridSurfaceKind =
  | "editor"
  | "terminal"
  | "browser"
  | "graph"
  | "dashboard";
export type HybridSurfaceThemes = Partial<Record<HybridSurfaceKind, SurfaceThemeChoice>>;

export type PaneWidths = {
  inspector: number;
  graph: number;
  browser: number;
  search: number;
  /// Width of the left-side outline pane in the file editor tab.
  /// Optional on the wire so older servers (no `outline` field in
  /// PaneWidths) still parse cleanly; the client fills the default.
  outline?: number;
};

export type BrowserSidePanes = {
  left: boolean;
  right: boolean;
};

/// Vertical density for paragraphs and lists in the editor.
/// `standard` is the default; `compact` is denser. `tight` is a
/// legacy read alias accepted from older persisted configs.
export type LineSpacing = "standard" | "compact" | "tight";

export type SearchAggression = "conservative" | "balanced" | "aggressive";

export type TerminalPreferences = {
  idle_timeout_secs: number;
  session_cap: number;
  ring_bytes: number;
  /// Per-terminal scrollback budget in MB. Consumed at xterm.js
  /// construction time; spawn-time only (existing terminals keep
  /// their current scrollback until the session restarts). Server
  /// clamps to [10, 500]; default 50.
  scrollback_mb?: number;
  /// Default TERM env var on the spawned PTY. Optional on the wire
  /// so older servers (no field) deserialize cleanly; the SPA
  /// treats `undefined` as the default `xterm-256color`.
  default_term?: string;
  /// Terminal-font preference. Optional on the wire so older
  /// servers (no field) deserialize as the default `os-default`
  /// (per-OS native mono). `source-code-pro` opts into Source
  /// Code Pro; the SPA triggers the download endpoint when needed.
  font?: TerminalFontChoice;
};

export type TerminalFontChoice = "os-default" | "source-code-pro";

export type BubbleOverlayMode = "stack" | "tray";

export type TerminalSpawnRequest = {
  name: string;
  command: string;
  env?: Record<string, string>;
  orchestrator_session?: string;
};

export type TerminalSpawnResponse = {
  session: string;
  tab_label: string;
};

export type TerminalRestartRequest = {
  name?: string;
  /// Broadcast group for the respawned shell. Sets `$CHAN_TAB_GROUP`
  /// and the registry's per-session `tab_group`. Defaults to "default".
  group?: string;
  window_id?: string;
  /// Optional command override for the restarted PTY. When set,
  /// the new shell runs this command instead of the original
  /// spawn command. Used by the team-bootstrap orchestrator to
  /// flip the host's terminal into the lead's session.
  command?: string;
  /// Optional env override for the restarted PTY. Entries are
  /// merged into the restart options' env so per-member env
  /// (e.g. CHAN_TAB_NAME = lead handle) lands before respawn.
  env?: Record<string, string>;
};

export type Preferences = {
  editor_theme: EditorTheme;
  /// Where image uploads land (relative to workspace root). Default
  /// `attachments/`. Not exposed in the Settings UI; round-tripped
  /// here so save() doesn't accidentally reset the value when the
  /// user has overridden it via the global config.
  attachments_dir: string;
  /// Editor theme. Lives server-side so changes propagate to every
  /// open window over the WS config_changed event.
  theme: ThemeChoice;
  /// Optional body-theme overrides for Hybrid element families.
  /// Missing entries inherit the global `theme` above.
  hybrid_surface_themes?: HybridSurfaceThemes;
  /// Sidebar widths shared across all panes (file editor inspector,
  /// graph details, file browser). Per-machine.
  pane_widths: PaneWidths;
  /// Docked file-browser panes attached outside the workspace.
  browser_side_panes: BrowserSidePanes;
  /// Editor density for paragraphs and lists.
  line_spacing: LineSpacing;
  /// Default format used by @date / @today and as the initial
  /// selection in the calendar picker's format dropdown.
  /// Format ids are defined in `web/src/editor/dateFormats.ts`.
  date_format: string;
  /// When true, saves strip trailing spaces and tabs before writing
  /// text buffers to disk.
  strip_trailing_whitespace_on_save: boolean;
  /// Search indexer resource profile. Not surfaced in Settings yet,
  /// but round-tripped by /api/config so CLI/server config changes
  /// remain visible to clients.
  search_aggression: SearchAggression;
  /// Terminal PTY session retention settings. Not surfaced in
  /// Settings yet; round-tripped for config preservation.
  terminal: TerminalPreferences;
  /// Watcher bubbles display mode: show all inline, or collapse
  /// to a count tray until expanded.
  bubble_overlay_mode: BubbleOverlayMode;
  /// Auto-rotate the empty-pane carousel. Optional on the wire so
  /// older servers that don't ship the field don't trip the type
  /// contract; the UI treats `undefined` as the default-true.
  empty_pane_carousel_cycling?: boolean;
};

export type TreeEntry = {
  path: string;
  is_dir: boolean;
  mtime: number | null;
  size: number;
  path_class?: PathClass;
  /// File-kind discriminator from the server. Present for every
  /// regular file; absent on directory entries (frontends key off
  /// `is_dir` for those). Values mirror the unified taxonomy in
  /// `web/src/state/kinds.ts`:
  ///   - `document`: markdown-class (.md / .txt) without contact
  ///     frontmatter.
  ///   - `contact`: markdown-class with `chan.kind: contact`
  ///     frontmatter.
  ///   - `text`: any other text file (.py, .json, Makefile, ...)
  ///     the editor can round-trip through a UTF-8 buffer.
  ///   - `media`: images.
  ///   - `binary`: PDFs, archives, audio/video, and everything else
  ///     opaque to the editor.
  kind?: "document" | "contact" | "text" | "media" | "binary";
};

export type PathKind =
  | "directory"
  | "symlink"
  | "regular_file"
  | "fifo"
  | "socket"
  | "block_device"
  | "char_device"
  | "other";

export type PathPermission = "read_write" | "read_only";

export type PathClass = {
  kind: PathKind;
  permission: PathPermission;
  link_count: number;
  target?: string | null;
  target_escapes_workspace?: boolean;
};

/// Response from POST /api/move. The rename itself always succeeds
/// when `renamed` is non-empty; per-source rewrite failures land in
/// `conflicts` and do not abort the move.
export type MoveResponse = {
  /// (old_path, new_path) for every file the rename moved. Single
  /// entry for a file rename; one per descendant file for a directory.
  renamed: Array<[string, string]>;
  /// Source files whose contents were rewritten to point at the new
  /// locations. Workspace-rooted POSIX paths (post-rename).
  rewritten: string[];
  /// Source files where the rewrite was abandoned because the file
  /// changed between read and CAS-write. The on-disk rename stands.
  conflicts: string[];
};

/// FB clipboard + multi-drag multi-entry move/copy (POST /api/fs/transfer).
export type TransferOp = "move" | "copy";

export type TransferResponse = {
  /// Per-source final destination (after collision suffixing), in
  /// request order.
  moved: Array<{ from: string; to: string }>;
  /// Sources skipped (no-op move into the same parent, or escaped workspace).
  skipped: string[];
  /// Link-rewrite CAS conflicts accumulated across moved entries.
  conflicts: string[];
};

export type DraftInspectResponse = {
  path: string;
  name: string;
  file_count: number;
  dir_count: number;
  total_size: number;
  has_attachments: boolean;
};

export type DraftPromoteResponse = {
  path: string;
  name: string;
  mode: "file" | "directory_created" | "directory_merged";
};

export type FileResponse = {
  path: string;
  content: string;
  mtime: number | null;
  mtime_ns?: string | null;
  path_class?: PathClass;
  /// Path of the enclosing git repo, relative to the workspace root.
  /// Absent when the file is not inside a git repo (or when the
  /// repo coincides with the workspace root). Workspaces the per-file
  /// scope indicator in the overlay picker.
  repo_root?: string | null;
  /// Filesystem-level writability: true when the underlying file
  /// has user-write bits set on disk, false otherwise. Workspaces the
  /// per-tab read-only lock that overrides the user's lamp toggle.
  /// Optional for forward-compat with older servers; absent =
  /// treat as writable to match prior behavior.
  writable?: boolean;
};

export type SearchHit = { path: string; score: number };

export type LinkTarget = {
  kind: "File" | "Heading";
  path: string;
  title?: string | null;
  heading?: string | null;
  anchor?: string | null;
  level?: number | null;
  mtime?: number | null;
};

export type LinkEdge = {
  source: string;
  target: string;
  resolved: string | null;
  wiki: boolean;
};

/// Graph edge as returned by /api/backlinks/{path}. Mirrors
/// chan-workspace's graph::Edge: `kind` is "link" / "mention" / "tag";
/// `anchor` is the heading slug or block id (with leading `^`)
/// when the link points inside a file, else null.
export type GraphEdge = {
  src: string;
  dst: string;
  kind: "link" | "mention" | "tag";
  anchor: string | null;
};

export type GraphSnapshot = {
  edges: LinkEdge[];
  broken: LinkEdge[];
  file_count: number;
};

/// Typed nodes returned by GET /api/graph. The discriminated union
/// matches `chan-workspace::graph::GraphNode`; `path` is only present
/// on file nodes (clicking them opens the file in the active pane).
export type GraphViewNode =
  | {
      kind: "file";
      id: string;
      label: string;
      path: string;
      path_class?: PathClass | null;
      /// `chan.kind` discriminator from the indexer. "contact" for
      /// notes flagged with `chan.kind: contact` frontmatter; absent
      /// for regular markdown so the canvas falls back to the doc
      /// shape. Image files keep `node_kind` absent and are routed via
      /// the frontend's classifyFile extension check instead.
      node_kind?: "contact";
      /// True for ghost nodes synthesized as the target of a broken
      /// link. Rendered muted; clicking is a no-op (the file doesn't
      /// exist yet).
      missing?: boolean;
    }
  | {
      kind: "media";
      id: string;
      label: string;
      path: string;
      path_class?: PathClass | null;
      missing?: boolean;
    }
  | { kind: "tag"; id: string; label: string }
  | { kind: "mention"; id: string; label: string }
  | {
      kind: "language";
      id: string;
      label: string;
      language: string;
      files: number;
      code: number;
    }
  | {
      kind: "folder";
      id: string;
      label: string;
      path: string;
      path_class?: PathClass | null;
      files: number;
      code: number;
      /// Per-directory indexing status used by the Dashboard
      /// indexing slide to colour the spine read-only. Undefined
      /// for the normal graph view; the main graph leaves folder
      /// fills on the standard `--g-folder` palette.
      indexState?: "pending" | "indexed" | "indexing";
    }
  | {
      kind: "directory";
      id: string;
      label: string;
      path: string;
      files: number;
      code: number;
    }
  | { kind: "date"; id: string; label: string };

export type GraphViewEdgeKind =
  | "link"
  | "tag"
  | "mention"
  | "contains"
  | "language"
  | "date"
  /// Distinguished edge from workspace-root to Drafts-root.
  /// Emitted by chan-server's `synthesize_drafts_layer` when any
  /// indexed file lives under the `Drafts/` unified-keyspace
  /// prefix. Styled distinctly in the canvas (yellow tint) so the
  /// drafts surface reads as "different category" at a glance.
  | "drafts_link";

export type GraphViewEdge = {
  source: string;
  target: string;
  kind: GraphViewEdgeKind;
  /// Only meaningful for `link` edges; missing/false for the others.
  broken?: boolean;
  rank?: number;
  files?: number;
  code?: number;
};

export type GraphView = {
  nodes: GraphViewNode[];
  edges: GraphViewEdge[];
};

export type LanguageGraphEdge = GraphViewEdge & {
  kind: "language";
  rank: number;
  files: number;
  code: number;
};

export type LanguageGraphResponse = {
  max_depth: number;
  nodes: Array<Extract<GraphViewNode, { kind: "language" | "folder" | "directory" }>>;
  edges: LanguageGraphEdge[];
};

export type FsGraphScope = "file" | "directory";
export type FsGraphNodeKind = "directory" | "file" | "symlink" | "ghost";
export type FsGraphEdgeKind = "contains" | "symlink" | "hardlink";

export type FsGraphNode = {
  id: string;
  kind: FsGraphNodeKind;
  name: string;
  path: string;
  size: number;
  path_class?: PathClass | null;
  permission?: PathPermission | null;
  link_count?: number;
  mtime?: number | null;
  target?: string | null;
  outside?: boolean;
  broken?: boolean;
  target_escapes_workspace?: boolean;
};

export type FsGraphEdge = {
  source: string;
  target: string;
  kind: FsGraphEdgeKind;
};

export type FsGraphResponse = {
  root: string;
  scope: FsGraphScope;
  path: string;
  depth: number;
  nodes: FsGraphNode[];
  edges: FsGraphEdge[];
  truncated: boolean;
  /// Cursor-paged delivery (a request carrying `limit` or `cursor`):
  /// `cursor` is the opaque continuation token for the next batch, null
  /// on the final batch; `done` is true on the final batch. Absent on a
  /// whole-scope (non-paged) response, which returns everything at once.
  cursor?: string | null;
  done?: boolean;
};

// New-workspace pre-flight (GET /api/preflight). chan-server derives the
// snapshot from live state on every poll; the SPA renders it on a locked
// surface until `phase === "ready"`.
export type PreflightPhase = "running" | "needs_decision" | "ready" | "failed";
export type PreflightStepState =
  | "pending"
  | "running"
  | "done"
  | "needs_decision"
  | "failed";

export type PreflightDecisionChoice = { id: string; label: string };
export type PreflightDecision = {
  prompt: string;
  choices: PreflightDecisionChoice[];
};
export type PreflightStep = {
  id: string;
  label: string;
  state: PreflightStepState;
  /// Progress counters for a running step (the index build); the locked
  /// surface reads these as the single source of truth for its bar.
  current?: number;
  total?: number;
  /// Present when the step blocks on a user choice (`needs_decision`).
  decision?: PreflightDecision;
};
export type PreflightError = { step: string; message: string };
export type PreflightSnapshot = {
  phase: PreflightPhase;
  /// True until `phase === "ready"`. The single signal the locked surface
  /// keys on: while true it shows with no close affordance and ignores ESC.
  locked: boolean;
  steps: PreflightStep[];
  error?: PreflightError | null;
};
export type PreflightDecisionRequest = { step: string; choice: string };

// ---------------------------------------------------------------------------
// /ws message-type catalog.
//
// The watcher socket carries both directions. Server -> client frames are a
// tagged union on `type`; client -> server frames are the scope sub/unsub
// path. The legacy global `watch` frame stays for the editor's open-document
// external-edit toast (a single-file concern); the scoped `fs` frame serves
// the per-directory File Browser / Graph tree (two frames, two consumers).
// Server-side serialization in chan-server must stay in lockstep with these
// shapes; both sides pin them with a test.
// ---------------------------------------------------------------------------

/// A single filesystem change as chan-workspace's watcher serializes
/// it on the wire. Capitalized kinds plus the rename destination `to`,
/// matching the verbatim `chan_workspace::WatchEvent` serialization the
/// store dispatcher reads (it branches on `"Removed"` / `"Renamed"`).
/// Distinct from the older, narrower `WatchEvent` type below (lowercase
/// kinds, no rename destination); new code should use `WatchEventWire`.
export type WatchEventWire = {
  kind: "Created" | "Modified" | "Removed" | "Renamed";
  path: string;
  to?: string | null;
};

/// Workspace-relative POSIX directory path used as a watcher scope key. The
/// empty string is the workspace root (always implicitly watched). Mirrors the
/// server-side `ScopeRegistry` keyspace.
export type WatchScopeDir = string;

/// Server -> client: the legacy global filesystem frame. Fans out to every
/// connected socket regardless of scope. Kept for the editor external-edit
/// toast; the tree should prefer the scoped `fs` frame.
export type WsWatchFrame = { type: "watch"; event: WatchEventWire };

/// Server -> client: a scoped filesystem frame, delivered only to sockets
/// subscribed to `dir`. Carries the originating directory so a client that
/// subscribed to several dirs can route the event to the right pane / node.
export type WsFsFrame = { type: "fs"; dir: WatchScopeDir; event: WatchEventWire };

/// Client -> server: subscribe / unsubscribe this socket to a directory
/// scope. `dir: ""` is the workspace root (idempotent no-op refcount the server
/// accepts). The server routes these to its `ScopeRegistry` against this
/// socket's subscriber id; a socket close implicitly unsubscribes all.
export type WsSubFrame = { type: "sub"; dir: WatchScopeDir };
export type WsUnsubFrame = { type: "unsub"; dir: WatchScopeDir };

/// The client -> server frame union. Other server -> client frames
/// (`progress`, `window_command`, `config_changed`, ...) are handled
/// structurally in the store dispatcher and are intentionally not enumerated
/// here; this union is only the outbound scope-control path the transport
/// stub serializes.
export type WsClientFrame = WsSubFrame | WsUnsubFrame;

export type InspectorKind =
  | "workspace"
  | "directory"
  | "markdown"
  | "text"
  | "media"
  | "binary"
  | "special";

export type InspectorReportSummary = {
  totals: ReportTotals;
  by_language: ReportLanguageStats[];
};

export type InspectorSubtree = {
  files: number;
  directories: number;
  bytes: number;
  file_kinds: Record<string, number>;
};

export type InspectorPayload = {
  path: string;
  kind: InspectorKind;
  is_dir: boolean;
  size: number;
  mtime: number | null;
  path_class: PathClass;
  frontmatter_kind: string | null;
  report_file?: ReportFileStats | null;
  report_summary?: InspectorReportSummary | null;
  subtree?: InspectorSubtree | null;
};

export type WatchEvent =
  | { kind: "created"; path: string }
  | { kind: "modified"; path: string }
  | { kind: "deleted"; path: string };

/// One heading row from GET /api/headings/{path}. Mirrors
/// chan-workspace's graph::HeadingRow: `anchor` is the slug used in
/// `[link](file.md#anchor)` markdown URLs.
export type HeadingRow = {
  level: number;
  text: string;
  anchor: string;
  ord: number;
};

/// Snapshot returned by GET /api/index/status. Field set matches
/// chan-server::indexer::IndexStatus.
export type IndexStatus =
  | {
      state: "idle";
      indexed_docs: number;
      indexed_vectors: number;
      model: string;
      /// Background embedding progress, per the IDX wire-shape contract
      /// (idx-wire-shape.md). A `{done,total}` object (done <= total)
      /// while embeddings are still generating after the index reached
      /// BM25-ready - preflight unlocks on idle regardless - and `null`
      /// (the backend emits an explicit null) or absent once settled. The
      /// status bar renders it as a passive "embedding done/total" chip,
      /// never the active reindexing pill.
      embedding?: { done: number; total: number } | null;
    }
  | { state: "building"; current: number; total: number; file: string }
  | { state: "reindexing"; file: string }
  | { state: "error"; message: string };

export type IndexingDirectoryState = "indexed" | "indexing" | "pending";

export type IndexingStateNode = {
  path: string;
  state: IndexingDirectoryState;
  children_count: number;
};

export type IndexingStateResponse = {
  root: string;
  nodes: IndexingStateNode[];
};

export type HealthIndexerStatus = "idle" | "settling" | "rebuilding" | "error";

export type HealthResponse = {
  indexer?: {
    status: HealthIndexerStatus;
    queue_depth: number;
    last_event_at?: string | null;
    last_settled_at?: string | null;
    coalesced_rebuild?: boolean;
  } | null;
};

/// Hybrid / BM25 / semantic content search hit.
export type ContentHit = {
  path: string;
  chunk_id: string;
  heading: string;
  start_line: number;
  snippet: string;
  score: number;
};

export type ContentSearchResponse = {
  ready: boolean;
  mode: "hybrid" | "bm25" | "semantic";
  hits: ContentHit[];
};

/// Compile-time identity of the running chan binary. Powers the
/// Settings "About" footer so users can tell at a glance which
/// version they're on and whether semantic search is available.
export type BuildInfo = {
  version: string;
  features: {
    embeddings: boolean;
  };
};

/// Semantic-search state surface. Consumed by the Settings UI to
/// render the opt-in toggle and status row. `mode` is derived
/// server-side as `"hybrid"` iff `semantic_enabled AND
/// model_present`; the flag-on-but-model-deleted case falls back
/// to `"bm25"`. `model_size_bytes` is null pre-download (the
/// resolver only knows the size after the bundle lands on disk).
export type SemanticState = {
  mode: "bm25" | "hybrid";
  model_present: boolean;
  model_name: string;
  model_path: string;
  model_size_bytes: number | null;
  semantic_enabled: boolean;
};

export type SemanticModelEntry = {
  id: string;
  label: string;
  dim: number;
  size_label: string;
  note: string;
  default: boolean;
  downloaded: boolean;
  current: boolean;
};

export type SemanticModelRegistry = {
  current_model: string;
  models: SemanticModelEntry[];
};

/// Workspace reset modes, in increasing destructiveness. See
/// `crates/chan-server/src/routes/storage.rs` for the per-mode contract.
export type ResetMode = "workspace" | "everything";

export type ResetResponse = {
  removed_entries: number;
};

/// chan-report shapes. Mirror `crates/chan-report/src/summary.rs`
/// and the server's `routes::report::PrefixReport`. The file
/// inspector renders the per-file row; the directory inspector renders
/// the prefix roll-up (totals + by_language + COCOMO).

export type ReportFileStats = {
  path: string;
  language: string;
  code: number;
  comments: number;
  blanks: number;
  complexity: number;
  bytes: number;
  mtime?: string | null;
};

export type ReportLanguageStats = {
  name: string;
  files: number;
  bytes?: number;
  code: number;
  comments: number;
  blanks: number;
  complexity: number;
};

export type ReportTotals = {
  files: number;
  bytes?: number;
  code: number;
  comments: number;
  blanks: number;
  complexity: number;
};

export type ReportCocomoSummary = {
  model: string;
  effort_person_months: number;
  schedule_months: number;
  developers: number;
  estimated_cost_usd: number;
};

export type ReportPrefix = {
  totals: ReportTotals;
  by_language: ReportLanguageStats[];
  cocomo: ReportCocomoSummary;
};
