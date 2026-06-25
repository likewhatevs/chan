//! Named process exit codes for the `cs` client, and the typed error that
//! carries one up to the dispatch edge.
//!
//! Only `cs terminal survey` needs a non-trivial code today: a survey that
//! gets no reply within its `--timeout` window exits [`SURVEY_TIMEOUT`] (124),
//! so a caller can tell "no answer in time" apart from a real error (exit 1)
//! and from a delivered answer (exit 0). The server answers such a survey with
//! `ControlResponse::Timeout`; [`crate::control::send_control_request`] turns
//! that into a [`ControlTimeout`], and `cmd_shell_survey` downcasts it, prints
//! the elapsed-window line, and exits with [`SURVEY_TIMEOUT`].

use std::error::Error;
use std::fmt;

/// `cs terminal survey --timeout` elapsed with no reply. Matches GNU
/// `timeout(1)`, which exits 124 when the command it guards times out.
pub(crate) const SURVEY_TIMEOUT: i32 = 124;

/// A control request the server answered with `ControlResponse::Timeout`
/// (today only a blocking `cs terminal survey` whose `--timeout` window
/// elapsed). Carried as an `anyhow` error so the dispatch edge can downcast it
/// to a distinct exit code instead of the generic exit-1 failure path. The
/// message is the server's elapsed-window line (e.g. `no reply within 600s`).
#[derive(Debug)]
pub(crate) struct ControlTimeout {
    pub(crate) message: String,
}

impl fmt::Display for ControlTimeout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for ControlTimeout {}
