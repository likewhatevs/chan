// One umbrella error so the FFI surface stays a single tagged enum.
// Variants are uniffi-friendly: primitive payloads only, no nested
// non-uniffi types in the Display strings.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, LlmError>;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("backend not implemented yet: {0}")]
    NotImplemented(String),
    #[error("no backend configured; set one in chan settings before sending")]
    BackendNotConfigured,
    #[error("{backend} CLI unavailable: {command}: {reason}")]
    CliNotFound {
        backend: String,
        command: String,
        reason: String,
    },
    #[error("config decode error: {0}")]
    ConfigDecode(String),
    #[error("config encode error: {0}")]
    ConfigEncode(String),
    #[error("backend error: {status}: {message}")]
    BackendError { status: u16, message: String },
    #[error("tool error: {0}")]
    Tool(String),
    /// Catch-all for chan-drive errors that don't have a typed
    /// variant here. Prefer the narrow variants below when matching;
    /// they preserve the kind so hosts can branch (e.g. "show
    /// reload prompt" for `WriteConflict`, "show too-large dialog"
    /// for `WriteTooLarge`). The narrow variants used to flatten
    /// into this string and broke host UX.
    #[error("chan-drive: {0}")]
    Core(String),
    /// chan-drive's `WriteConflict` passthrough. The assistant's
    /// write was a CAS against `current_mtime_ns` and the file
    /// changed under it; the host should re-read and retry.
    #[error("write conflict: file changed on disk (current mtime ns: {current_mtime_ns:?})")]
    WriteConflict { current_mtime_ns: Option<i64> },
    /// chan-drive's `WriteTooLarge` passthrough.
    #[error("write too large: {size} bytes exceeds {limit} byte cap for {kind}")]
    WriteTooLarge { kind: String, size: u64, limit: u64 },
    /// chan-drive's `ListingTooLarge` passthrough.
    #[error("listing too large: {observed} entries (cap {limit})")]
    ListingTooLarge { observed: u64, limit: u64 },
    /// chan-drive refused a path: sandbox escape, special file,
    /// non-editable extension, or empty rel. The string carries the
    /// chan-drive Display so the host can show it directly.
    #[error("path refused: {0}")]
    PathRefused(String),
    #[error("io: {0}")]
    Io(String),
    #[error("mcp: {0}")]
    Mcp(String),
}

impl From<std::io::Error> for LlmError {
    fn from(e: std::io::Error) -> Self {
        LlmError::Io(e.to_string())
    }
}

impl From<chan_drive::ChanError> for LlmError {
    fn from(e: chan_drive::ChanError) -> Self {
        // Preserve the kind so hosts can branch on the cause.
        // Variants not enumerated here flatten into Core(_) - they
        // are operational (DriveLocked, DriveAlreadyOpen, etc.) or
        // already string-shaped (Search, Graph, Watch).
        match e {
            chan_drive::ChanError::WriteConflict { current_mtime_ns } => {
                LlmError::WriteConflict { current_mtime_ns }
            }
            chan_drive::ChanError::WriteTooLarge { kind, size, limit } => LlmError::WriteTooLarge {
                kind: kind.to_string(),
                size,
                limit,
            },
            chan_drive::ChanError::ListingTooLarge { observed, limit } => {
                LlmError::ListingTooLarge {
                    observed: observed as u64,
                    limit: limit as u64,
                }
            }
            chan_drive::ChanError::PathEmpty
            | chan_drive::ChanError::PathEscape
            | chan_drive::ChanError::NotEditableText(_)
            | chan_drive::ChanError::SpecialFile { .. }
            | chan_drive::ChanError::SymlinkEscape(_) => LlmError::PathRefused(e.to_string()),
            other => LlmError::Core(other.to_string()),
        }
    }
}

impl From<toml::de::Error> for LlmError {
    fn from(e: toml::de::Error) -> Self {
        LlmError::ConfigDecode(e.to_string())
    }
}

impl From<toml::ser::Error> for LlmError {
    fn from(e: toml::ser::Error) -> Self {
        LlmError::ConfigEncode(e.to_string())
    }
}
