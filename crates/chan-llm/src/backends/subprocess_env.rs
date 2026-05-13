//! Subprocess env sanitization for the CLI-shaped backends.
//!
//! The agentic CLI backends (`claude` / `gemini`) spawn external
//! binaries. Inheriting the full parent environment hands the
//! child every secret the host process happens to hold:
//! `OPENAI_API_KEY`, `GH_TOKEN`, `AWS_SECRET_ACCESS_KEY`, anything
//! else the user's shell exported. On Linux those values become
//! readable via `/proc/<pid>/environ` to any process the same uid
//! can see; on macOS they're visible to `ps -E` and friends. They
//! are also persisted into the child's argv-equivalent env table
//! the child may pass on to its own subprocesses.
//!
//! `sanitize_env` wipes the inherited env and forwards a small,
//! explicit allowlist: the POSIX shell basics needed to run a
//! tool (`PATH`, `HOME`, `LANG`, ...) plus any vendor prefixes
//! the caller opts into. Everything else is dropped. Callers
//! still set vendor-specific values (`GEMINI_API_KEY`,
//! `GEMINI_CLI_HOME`, etc.) explicitly via `command.env(...)`
//! after this runs.

use tokio::process::Command;

/// POSIX shell / locale / tmpdir vars every CLI we shell out to
/// expects to be present. Anything outside this list is dropped.
const BASE_ALLOWLIST: &[&str] = &[
    "PATH",
    "HOME",
    "USER",
    "LOGNAME",
    "SHELL",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "LC_MESSAGES",
    "LC_TIME",
    "LC_COLLATE",
    "LC_MONETARY",
    "LC_NUMERIC",
    "TZ",
    "TERM",
    "TMPDIR",
    "TMP",
    "TEMP",
];

/// Replace the spawned command's env with the POSIX basics plus
/// any var whose name starts with one of `forwarded_prefixes`.
/// Use the prefixes to carry vendor-specific config the child
/// genuinely needs: e.g. `&["ANTHROPIC_"]` for the claude CLI,
/// `&["GOOGLE_", "GEMINI_"]` for the gemini CLI. An empty
/// `forwarded_prefixes` is fine when the caller plans to set
/// every vendor var explicitly itself.
pub(crate) fn sanitize_env(command: &mut Command, forwarded_prefixes: &[&str]) {
    command.env_clear();
    for (key, value) in std::env::vars_os() {
        let Some(name) = key.to_str() else {
            // Non-UTF-8 env var names are exceedingly rare and would
            // not match any allowlist entry anyway; skipping them is
            // safer than forwarding unknown bytes to a child process.
            continue;
        };
        let keep = BASE_ALLOWLIST.contains(&name)
            || forwarded_prefixes.iter().any(|p| name.starts_with(p));
        if keep {
            command.env(&key, &value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The helper mutates `Command` state we cannot read back, so we
    // exercise the filter predicate directly to keep the test
    // deterministic against whatever the host shell exports.
    fn keeps(name: &str, prefixes: &[&str]) -> bool {
        BASE_ALLOWLIST.contains(&name) || prefixes.iter().any(|p| name.starts_with(p))
    }

    #[test]
    fn forwards_base_vars() {
        assert!(keeps("PATH", &[]));
        assert!(keeps("HOME", &[]));
        assert!(keeps("LANG", &[]));
        assert!(keeps("LC_ALL", &[]));
    }

    #[test]
    fn drops_unrelated_secrets() {
        assert!(!keeps("OPENAI_API_KEY", &["ANTHROPIC_"]));
        assert!(!keeps("GH_TOKEN", &["ANTHROPIC_"]));
        assert!(!keeps("AWS_SECRET_ACCESS_KEY", &["GEMINI_", "GOOGLE_"]));
    }

    #[test]
    fn forwards_vendor_prefixes() {
        assert!(keeps("ANTHROPIC_API_KEY", &["ANTHROPIC_"]));
        assert!(keeps("ANTHROPIC_BASE_URL", &["ANTHROPIC_"]));
        assert!(keeps("GEMINI_API_KEY", &["GEMINI_", "GOOGLE_"]));
        assert!(keeps(
            "GOOGLE_APPLICATION_CREDENTIALS",
            &["GEMINI_", "GOOGLE_"]
        ));
        // Cross-prefix isolation: ANTHROPIC_ caller does not forward
        // GOOGLE_ vars, and vice versa.
        assert!(!keeps("GOOGLE_API_KEY", &["ANTHROPIC_"]));
        assert!(!keeps("ANTHROPIC_API_KEY", &["GEMINI_", "GOOGLE_"]));
    }
}
