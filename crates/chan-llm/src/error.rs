// One umbrella error so the FFI surface stays a single tagged enum.
// Variants are uniffi-friendly: primitive payloads only, no nested
// non-uniffi types in the Display strings.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, LlmError>;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("backend not implemented yet: {0}")]
    NotImplemented(String),
    #[error("api key missing for backend {0}")]
    MissingApiKey(String),
    #[error("config decode error: {0}")]
    ConfigDecode(String),
    #[error("config encode error: {0}")]
    ConfigEncode(String),
    #[error("http error: {0}")]
    Http(String),
    #[error("backend error: {status}: {message}")]
    BackendError { status: u16, message: String },
    #[error("tool error: {0}")]
    Tool(String),
    #[error("chan-drive: {0}")]
    Core(String),
    #[error("io: {0}")]
    Io(String),
    #[error("subprocess: {0}")]
    Subprocess(String),
    #[error("keychain: {0}")]
    Keychain(String),
    #[error("mcp: {0}")]
    Mcp(String),
}

impl From<std::io::Error> for LlmError {
    fn from(e: std::io::Error) -> Self {
        LlmError::Io(e.to_string())
    }
}

impl From<reqwest::Error> for LlmError {
    fn from(e: reqwest::Error) -> Self {
        LlmError::Http(e.to_string())
    }
}

impl From<chan_drive::ChanError> for LlmError {
    fn from(e: chan_drive::ChanError) -> Self {
        LlmError::Core(e.to_string())
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

impl From<keyring::Error> for LlmError {
    fn from(e: keyring::Error) -> Self {
        LlmError::Keychain(e.to_string())
    }
}
