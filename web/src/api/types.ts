// API types: the JSON shapes returned by chan-core's HTTP handlers.
// Keep in lockstep with crates/chan-core/src/server.rs.

export type DriveInfo = {
  name: string | null;
  root: string;
  /// Mirror of GlobalConfig.preferences. Per-drive overrides
  /// were removed; settings are always per-device-global. Carried
  /// here so a single `/api/drive` round-trip is enough to
  /// render the editor with the right fonts without a follow-up
  /// `/api/config` fetch.
  preferences: Preferences;
};

/// Global per-user config. Lives at `paths::global_config_path()`
/// on the server side and applies to every drive (no per-
/// drive override anymore — settings are always device-global).
export type GlobalConfig = {
  preferences: Preferences;
  /// When set, the resolver's fallback path becomes this; when
  /// unset, it falls back to the platform convention
  /// (`~/Documents/Chan` on macOS, `$XDG_DATA_HOME/chan/default`
  /// on Linux, `%USERPROFILE%\Documents\Chan` on Windows).
  default_drive_root?: string | null;
  /// Known drives the user has opened on this machine. Updated
  /// by the server on every spawn (touch existing or append).
  /// Sorted most-recent first.
  drives?: KnownDrive[];
};

export type KnownDrive = {
  path: string;
  /// User-editable display name from the registry. Null when the
  /// drive was registered without one (e.g. legacy entries) or
  /// after the user explicitly cleared it.
  name?: string | null;
  /// RFC3339 timestamp.
  last_opened: string;
};

/// Editor theme. Drives the markdown renderer + source view
/// typography and chrome. Light/dark variants are selected from
/// the active ThemeChoice; density from LineSpacing. App chrome
/// (toolbar, panes, status bar) is not affected.
export type EditorTheme = "github" | "google_docs" | "word";

export type ThemeChoice = "system" | "light" | "dark";

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
};

export type Preferences = {
  editor_theme: EditorTheme;
  /// Where image uploads land (relative to drive root). Default
  /// `attachments/`. Not exposed in the Settings UI; round-tripped
  /// here so save() doesn't accidentally reset the value when the
  /// user has overridden it via the global config.
  attachments_dir: string;
  /// Editor theme. Lives server-side so changes propagate to every
  /// open window over the WS config_changed event.
  theme: ThemeChoice;
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
  /// Search indexer resource profile. Not surfaced in Settings yet,
  /// but round-tripped by /api/config so CLI/server config changes
  /// remain visible to clients.
  search_aggression: SearchAggression;
  /// Terminal PTY session retention settings. Not surfaced in
  /// Settings yet; round-tripped for config preservation.
  terminal: TerminalPreferences;
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
  target_escapes_drive?: boolean;
};

/// Response from POST /api/move. The rename itself always succeeds
/// when `renamed` is non-empty; per-source rewrite failures land in
/// `conflicts` and do not abort the move.
export type MoveResponse = {
  /// (old_path, new_path) for every file the rename moved. Single
  /// entry for a file rename; one per descendant file for a directory.
  renamed: Array<[string, string]>;
  /// Source files whose contents were rewritten to point at the new
  /// locations. Drive-rooted POSIX paths (post-rename).
  rewritten: string[];
  /// Source files where the rewrite was abandoned because the file
  /// changed between read and CAS-write. The on-disk rename stands.
  conflicts: string[];
};

export type FileResponse = {
  path: string;
  content: string;
  mtime: number | null;
  path_class?: PathClass;
  /// Path of the enclosing git repo, relative to the drive root.
  /// Absent when the file is not inside a git repo (or when the
  /// repo coincides with the drive root). Drives the per-file
  /// scope indicator in the overlay picker.
  repo_root?: string | null;
  /// Filesystem-level writability: true when the underlying file
  /// has user-write bits set on disk, false otherwise. Drives the
  /// per-tab read-only lock that overrides the user's lamp toggle.
  /// Optional for forward-compat with older servers; absent =
  /// treat as writable to match prior behavior.
  writable?: boolean;
};

export type SearchHit = { path: string; score: number };

export type LinkEdge = {
  source: string;
  target: string;
  resolved: string | null;
  wiki: boolean;
};

/// Graph edge as returned by /api/backlinks/{path}. Mirrors
/// chan-core's graph::Edge: `kind` is "link" / "mention" / "tag";
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
/// matches `chan-core::link_index::GraphNode`; `path` is only present
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
  | "date";

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
  target_escapes_drive?: boolean;
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
};

export type InspectorKind =
  | "drive"
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
/// chan-core's graph::HeadingRow: `anchor` is the slug used in
/// `[link](file.md#anchor)` markdown URLs.
export type HeadingRow = {
  level: number;
  text: string;
  anchor: string;
  ord: number;
};

/// Snapshot returned by GET /api/index/status. Field set matches
/// chan-core::index::indexer::IndexStatus.
export type IndexStatus =
  | { state: "idle"; indexed_docs: number; indexed_vectors: number; model: string }
  | { state: "building"; current: number; total: number; file: string }
  | { state: "reindexing"; file: string }
  | { state: "error"; message: string };

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

/// Drive reset modes, in increasing destructiveness. See
/// `crates/chan-core/src/storage.rs` for the per-mode contract.
export type ResetMode = "drive" | "everything";

export type ResetResponse = {
  removed_entries: number;
};

/// chan-report shapes. Mirror `crates/chan-core/crates/chan-report/src/summary.rs`
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
