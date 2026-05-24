// chan-drive: filesystem, search, and graph primitives for chan-writer drives.
//
// Public surface is path-based, all relative paths POSIX-style ("/" separator)
// and rooted at a Drive's `root`. Designed to be FFI-safe via uniffi later:
// no lifetimes on public types, owned strings only, all handle types are
// `Arc<Self>`-able.
//
// Two top-level handles:
//
//   Library: owns the per-machine registry (~/.chan/config.toml) of known
//   drives and resolves OS state/cache locations. Process-wide singleton
//   in practice; cheap to clone (Arc inside).
//
//   Drive: handle to one registered directory. Exposes filesystem
//   primitives (read/write/stat/list), search, graph, and watch. Holds a
//   per-drive cross-process lock for the index writer.
//
// What is intentionally NOT here:
//   - HTTP server, WebSocket transport, frontend bundle. Those live in
//     `chan` (the CLI + embedded editor).
//   - LLM/agent code, API key storage. App-level concern.
//   - Editor preferences (fonts, theme). App-level concern.

mod blob;
pub mod contacts;
pub mod drafts;
pub mod drive;
pub mod error;
pub mod fs_ops;
pub mod graph;
pub mod index;
pub mod indexer;
pub mod library;
pub mod lock;
pub mod markdown;
pub mod metadata_archive;
pub mod paths;
pub mod progress;
pub mod registry;
mod report;
pub mod teams;
pub mod trash;
pub mod vcs;
pub mod watch;

pub use chan_report::{
    CocomoModel, CocomoParams, CocomoSummary, FileBucket as ReportFileBucket,
    FileStats as ReportFileStats, LanguageStats as ReportLanguageStats, Report, ReportMeta,
    Scope as ReportScope, Totals as ReportTotals,
};

pub use contacts::{
    Contact, EmailAddress, ImportCounts, ImportOpts, ImportOutcome, ImportSummary, Organization,
    PhoneNumber, ProviderKind,
};
pub use drafts::{DraftInspection, DraftPromoteMode, DraftPromoteReport, DraftRef};
pub use drive::ReconcileReport;
pub use drive::{
    DirEntry, Drive, FileStat, RenameOutcome, ResolvedLink, SearchOpts, TreeEntry,
    BYTES_WRITE_LIMIT, TEXT_WRITE_LIMIT,
};
pub use error::{ChanError, Result};
pub use fs_ops::{
    classify, classify_path, FileClass, PathClass, PathKind, PathPermission, WalkFilter,
};
pub use graph::{
    ContactNode, Edge, EdgeKind, GraphView, HeadingRow, LinkTarget, LinkTargetKind, Mention,
    NodeKind, Tag,
};
pub use index::{
    BuildOptions, BuildSummary, Chunking, Hit, IndexConfig, IndexStats, Mode as SearchMode,
    ScreensaverTheme, SearchAggression, SearchBudget, SearchResult, DEFAULT_MODEL,
};
pub use indexer::{GraphIndexer, DEFAULT_DEBOUNCE_MS};
pub use library::{Library, ResetMode, ResetReport, SweepReport};
pub use metadata_archive::{
    validate_archive_entry_path, ArchiveEntryKind, MetadataArchivePathError, MetadataExportOptions,
    MetadataExportReport, MetadataManifest, MetadataSchema, ScmIdentity,
};
pub use progress::{
    eta_secs_from, progress_fn, NoProgress, ProgressCallback, ProgressEvent, ProgressStage,
};
pub use registry::{KnownDrive, Registry};
pub use teams::{Member, Position, TeamConfig, TeamRef};
pub use trash::{TrashEntry, TRASH_RETENTION_SECS};
pub use vcs::{detect_drive_vcs, detect_parent_vcs, is_vcs_control_path, VcsKind, VcsParent};
pub use watch::{WatchCallback, WatchEvent, WatchHandle, WatchKind};
