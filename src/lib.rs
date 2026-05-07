// chan-core: filesystem, search, and graph primitives for chan-writer drives.
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
//   - LLM/assistant code, API key storage. App-level concern.
//   - Editor preferences (fonts, theme). App-level concern.

pub mod drive;
pub mod error;
pub mod fs_ops;
pub mod graph;
pub mod library;
pub mod lock;
pub mod markdown;
pub mod paths;
pub mod registry;
pub mod search;
pub mod trash;
pub mod watch;

pub use drive::{DirEntry, Drive, FileStat, TreeEntry};
pub use error::{ChanError, Result};
pub use graph::{Edge, EdgeKind, GraphView, HeadingRow, LinkTarget, LinkTargetKind, Tag};
pub use library::Library;
pub use registry::{KnownDrive, Registry};
pub use search::{Hit, IndexDoc, IndexStats, SearchMode, SearchOpts, SearchResults, Snippet};
pub use trash::{TrashEntry, TRASH_RETENTION_SECS};
pub use watch::{WatchCallback, WatchEvent, WatchHandle, WatchKind};
