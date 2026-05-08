// One umbrella error type so the FFI surface stays a single enum.
// Variants map cleanly across uniffi (no nested non-uniffi types in
// Display/Debug payloads).

use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, ChanError>;

#[derive(Debug, Error)]
pub enum ChanError {
    #[error("path is empty")]
    PathEmpty,
    #[error("path escapes drive root")]
    PathEscape,
    #[error("path is not editable text: {0}")]
    NotEditableText(String),
    #[error("refusing to operate on non-regular file ({kind}): {path}")]
    SpecialFile { kind: String, path: PathBuf },
    #[error("path resolves through a symlink that escapes drive root: {0}")]
    SymlinkEscape(PathBuf),
    #[error("invalid blob key: {0}")]
    InvalidKey(String),
    #[error("drive not registered: {0}")]
    DriveNotRegistered(PathBuf),
    #[error("drive root does not exist: {0}")]
    DriveRootMissing(PathBuf),
    #[error("drive is locked by another process")]
    DriveLocked,
    #[error("write conflict: file changed on disk (current mtime: {current_mtime:?})")]
    WriteConflict { current_mtime: Option<i64> },
    #[error("config decode error in {path}: {message}")]
    ConfigDecode { path: PathBuf, message: String },
    #[error("config encode error: {0}")]
    ConfigEncode(String),
    #[error("search error: {0}")]
    Search(String),
    #[error("graph error: {0}")]
    Graph(String),
    #[error("watch error: {0}")]
    Watch(String),
    #[error("trash entry not found: {0}")]
    TrashEntryNotFound(String),
    #[error("trash entry corrupt ({id}): {message}")]
    TrashCorrupt { id: String, message: String },
    #[error("trash restore target already exists: {0}")]
    TrashOccupied(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("operation cancelled")]
    Cancelled,
}

impl From<std::io::Error> for ChanError {
    fn from(e: std::io::Error) -> Self {
        ChanError::Io(e.to_string())
    }
}

impl From<toml::de::Error> for ChanError {
    fn from(e: toml::de::Error) -> Self {
        ChanError::ConfigDecode {
            path: PathBuf::new(),
            message: e.to_string(),
        }
    }
}

impl From<toml::ser::Error> for ChanError {
    fn from(e: toml::ser::Error) -> Self {
        ChanError::ConfigEncode(e.to_string())
    }
}

impl From<rusqlite::Error> for ChanError {
    fn from(e: rusqlite::Error) -> Self {
        ChanError::Graph(e.to_string())
    }
}

impl From<notify::Error> for ChanError {
    fn from(e: notify::Error) -> Self {
        ChanError::Watch(e.to_string())
    }
}

impl From<crate::index::IndexError> for ChanError {
    fn from(e: crate::index::IndexError) -> Self {
        ChanError::Search(e.to_string())
    }
}
