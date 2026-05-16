// API types: the JSON shapes returned by chan-core's HTTP handlers.
// Keep in lockstep with crates/chan-core/src/server.rs.

export type DriveInfo = {
  name: string | null;
  root: string;
  /// Mirror of GlobalConfig.preferences. Per-drive overrides
  /// were removed; settings are always per-device-global. Carried
  /// here so a single `/api/drive` round-trip is enough to
  /// render the editor with the right fonts / assistant config
  /// without a follow-up `/api/config` fetch.
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

export type AssistantBackendKind =
  | "claude_cli"
  | "gemini_cli"
  | "codex_cli";

/// Per-CLI override surface. The local `claude` / `gemini` / `codex` CLIs own
/// their own auth, so the only Settings-managed field is `enabled`;
/// `model` is written from the assistant overlay's inspector.
export type CliPrefs = {
  enabled: boolean;
  model?: string | null;
  cmd_override?: string | null;
};

export type AssistantPrefs = {
  /// Derived server-side: true when a default backend is set AND the
  /// matching provider's enable flag is on. Read-only on PATCH (the
  /// server recomputes it from the per-provider flags). Drives the
  /// assistant button / Cmd+I gate.
  effective_enabled: boolean;
  /// Which provider is the default assistant. Sticky across enable/
  /// disable toggles so user intent survives a "disable then re-
  /// enable" round-trip; null means no default picked.
  default_backend: AssistantBackendKind | null;
  answers_dir: string;
  claude_cli: CliPrefs;
  gemini_cli: CliPrefs;
  codex_cli: CliPrefs;
};

export type LlmModelEntry = {
  name: string;
  supports_tools: boolean;
  /// Whether the model accepts an extended-thinking `thinking`
  /// block on the wire. Today only Anthropic Opus / Sonnet 4.x
  /// return true here; the inspector hides the effort knob when
  /// false.
  supports_thinking: boolean;
};

export type LlmStatus = {
  backend: string;
  model: string | null;
  ready: boolean;
  /// Human-readable explanation of why `ready = false`. Absent
  /// when the assistant is ready.
  reason?: string | null;
  enabled: boolean;
  supports_tools: boolean;
};

export type CliDetectionView = {
  backend: AssistantBackendKind;
  ready: boolean;
  command: string[];
  reason?: string | null;
};

export type CliDetectionResponse = {
  detections: CliDetectionView[];
};

/// Mirror of chan-core's Message / ToolSpec / etc. Kept loose
/// (unknown JSON for tool inputs) since the schema is owned by
/// the backend.
export type LlmRole = "system" | "user" | "assistant" | "tool";

export type LlmMessage = {
  role: LlmRole;
  content: string;
  tool_call_id?: string;
  tool_calls?: LlmToolCall[];
  /// Optional multimodal payload on `Role::User` messages. Each
  /// entry is base64-encoded image bytes plus the MIME. The
  /// chan-llm backend that handles the request forwards them
  /// natively (Anthropic image content block, Gemini inline_data,
  /// Ollama images array); backends without image support drop
  /// them silently — the text content is still authoritative.
  images?: LlmImageInput[];
};

export type LlmImageInput = {
  mime_type: string;
  data: string;
};

export type LlmToolSpec = {
  name: string;
  description: string;
  input_schema: unknown;
};

export type LlmToolCall = {
  id: string;
  name: string;
  input: unknown;
};

export type LlmCompletionRequest = {
  messages: LlmMessage[];
  tools?: LlmToolSpec[];
  max_tokens?: number;
  temperature?: number;
  /// Client-generated correlation id echoed on every llm.* WS frame
  /// the server emits while this request is in flight. Lets the
  /// frontend filter the broadcast channel to its own turn so
  /// streaming deltas from a sibling window don't crosstalk.
  session_id?: string;
};

export type LlmStopReason =
  | "end_turn"
  | "max_tokens"
  | "tool_use"
  | "stop_sequence"
  | "cancelled"
  | "other";

export type UnknownAgentStatus = {
  kind: string;
  [key: string]: unknown;
};

/// Mirrors chan-llm's non-exhaustive `AgentStatus`; unknown kinds
/// must be tolerated by reducers and UI.
export type AgentStatus =
  | { kind: "spawned"; backend: string; pid: number | null }
  | {
      kind: "ready";
      backend: string;
      session_id: string | null;
      model: string | null;
      version: string | null;
    }
  | { kind: "thinking"; backend: string; status: string | null }
  | { kind: "heartbeat"; backend: string; idle_ms: number }
  | { kind: "turn_stopping"; backend: string; reason: string | null }
  | {
      kind: "rate_limit";
      backend: string;
      status: string;
      resets_at: string | null;
      rate_limit_type: string | null;
      in_overage: boolean;
    }
  | { kind: "exited"; backend: string; code: number | null; success: boolean }
  | { kind: "unhealthy"; backend: string; reason: string; detail: string | null }
  | { kind: "cancelled"; backend: string }
  | UnknownAgentStatus;

export type UnknownAgentActivity = {
  kind: string;
  [key: string]: unknown;
};

/// Mirrors chan-llm's non-exhaustive `AgentActivity`; JSON payloads
/// owned by backend tools stay unknown until a caller narrows them.
export type AgentActivity =
  | { kind: "session_started"; backend: string; session_id: string | null }
  | {
      kind: "message_started";
      backend: string;
      message_id: string | null;
      parent_id?: string | null;
    }
  | { kind: "thinking_started"; backend: string; parent_id?: string | null }
  | {
      kind: "thinking_delta";
      backend: string;
      text: string;
      parent_id?: string | null;
    }
  | {
      kind: "tool_started";
      backend: string;
      id: string;
      name: string;
      parent_id?: string | null;
    }
  | {
      kind: "tool_args_delta";
      backend: string;
      id: string | null;
      partial_json: string;
      parent_id?: string | null;
    }
  | {
      kind: "tool_finished";
      backend: string;
      id: string;
      name: string | null;
      output: unknown;
      is_error: boolean;
      parent_id?: string | null;
    }
  | {
      kind: "tool_denied";
      backend: string;
      id: string | null;
      name: string | null;
      reason: string | null;
      input: unknown;
      parent_id?: string | null;
    }
  | { kind: "agent_note"; backend: string; text: string; parent_id?: string | null }
  | { kind: "turn_usage"; backend: string; usage: unknown }
  | UnknownAgentActivity;

export type UserOption = {
  label: string;
  description?: string | null;
};

export type UserQuestion = {
  question: string;
  header?: string | null;
  multi_select: boolean;
  options: UserOption[];
};

export type UnknownUserRequest = {
  kind: string;
  [key: string]: unknown;
};

export type UserRequest =
  | {
      kind: "survey";
      backend: string;
      id: string;
      questions: UserQuestion[];
      parent_id: string | null;
    }
  | UnknownUserRequest;

export type LlmStatusFrame = {
  type: "llm.status";
  session_id: string;
  status: AgentStatus;
};

export type LlmActivityFrame = {
  type: "llm.activity";
  session_id: string;
  activity: AgentActivity;
};

export type LlmUserRequestFrame = {
  type: "llm.user_request";
  session_id: string;
  request: UserRequest;
};

export type LlmCompletionResponse = {
  content: string;
  tool_calls: LlmToolCall[];
  stop_reason: LlmStopReason;
  model: string;
};

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
  /// Width of the assistant overlay's right-side inspector pane.
  /// Optional on the wire so older servers (no `assistant` field in
  /// PaneWidths) still parse cleanly; the client fills the default.
  assistant?: number;
};

/// Vertical density for paragraphs and lists in the editor.
/// `tight` matches Google Docs spacing; `standard` keeps the older
/// roomier spacing. Default is `tight`.
export type LineSpacing = "tight" | "standard";

export type Preferences = {
  editor_theme: EditorTheme;
  assistant: AssistantPrefs;
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
  /// Editor density for paragraphs and lists.
  line_spacing: LineSpacing;
  /// Default format used by @date / @today and as the initial
  /// selection in the calendar picker's format dropdown.
  /// Format ids are defined in `web/src/editor/dateFormats.ts`.
  date_format: string;
};

export type TreeEntry = {
  path: string;
  is_dir: boolean;
  mtime: number | null;
  size: number;
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
  | { kind: "tag"; id: string; label: string }
  | { kind: "mention"; id: string; label: string }
  | { kind: "date"; id: string; label: string };

export type GraphViewEdgeKind = "link" | "tag" | "mention" | "date";

export type GraphViewEdge = {
  source: string;
  target: string;
  kind: GraphViewEdgeKind;
  /// Only meaningful for `link` edges; missing/false for the others.
  broken?: boolean;
};

export type GraphView = {
  nodes: GraphViewNode[];
  edges: GraphViewEdge[];
};

export type FsGraphScope = "file" | "folder";
export type FsGraphNodeKind = "folder" | "file" | "symlink" | "ghost";
export type FsGraphEdgeKind = "contains" | "symlink" | "hardlink";

export type FsGraphNode = {
  id: string;
  kind: FsGraphNodeKind;
  name: string;
  path: string;
  size: number;
  mtime?: number | null;
  target?: string | null;
  outside?: boolean;
  broken?: boolean;
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
/// inspector renders the per-file row; the folder inspector renders
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
  code: number;
  comments: number;
  blanks: number;
  complexity: number;
};

export type ReportTotals = {
  files: number;
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
