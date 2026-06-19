//! Wall-clock helpers shared by the terminal-session registry.
//!
//! The registry timestamps PTY activity for idle pruning and for the
//! `cs terminal write` queue's output-quiescence debounce. These two
//! functions are the whole surface; the server's broader shutdown/idle
//! machinery lives in `chan-server`'s `signal` module.

use std::time::{SystemTime, UNIX_EPOCH};

/// Current wall-clock unix timestamp in seconds. Saturates at 0 on
/// the impossible-but-cheap-to-handle case where the system clock
/// is set before 1970.
pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Current wall-clock unix timestamp in MILLISECONDS, for sub-second
/// quiescence timing (the `cs terminal write` queue's output-idle debounce).
/// Saturates at 0 on a pre-1970 clock, like `now_unix_secs`.
pub fn now_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
