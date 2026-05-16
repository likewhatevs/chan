//! Subprocess env sanitization + stderr drainer for the CLI-shaped
//! backends.
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
//!
//! `spawn_stderr_drainer` runs the child's stderr into a capped
//! shared buffer concurrently with the stdout loop. Without it,
//! a child that writes more than the OS pipe buffer to stderr
//! (~64 KiB on Linux, ~16 KiB on macOS) blocks on `write`, which
//! stalls stdout, which triggers our inactivity timeout. The
//! user sees "subprocess wedged" instead of the actual stderr
//! content. The drainer keeps stderr flowing; on non-zero exit
//! the backend reads up to `STDERR_CAP_BYTES` from the buffer to
//! surface in `on_error`.

use std::sync::{Arc, Mutex};

use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::task::JoinHandle;

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
///
/// Prefix matching is convenient when the user owns the parent
/// environment (their shell), but widens the attack surface when
/// chan-llm runs under a service account that imports tainted env:
/// `GOOGLE_APPLICATION_CREDENTIALS` (path to a service-account JSON),
/// `ANTHROPIC_BEDROCK_BASE_URL` (could redirect to a hostile
/// endpoint), and similar non-API-key vars survive the filter. Use
/// `sanitize_env_strict` instead when the caller is a long-lived
/// service rather than the user's interactive shell.
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

/// Replace the spawned command's env with the POSIX basics plus an
/// explicit set of var names. Tighter than `sanitize_env`: each var
/// the child receives must appear in the allowlist verbatim. Use
/// this when the parent environment is not under the user's direct
/// control (a long-lived service host, a future remote runner) so
/// vendor-prefix matches don't accidentally forward things like
/// `GOOGLE_APPLICATION_CREDENTIALS`.
///
/// Callers still set vendor-specific values explicitly via
/// `command.env(...)` after this runs; the allowlist controls only
/// what survives the inheritance.
///
/// Wired by the per-backend `sanitize_env_for_*` helpers below; the
/// `hardened_subprocess_env` flag on `LlmConfig` controls which
/// variant runs.
pub(crate) fn sanitize_env_strict(command: &mut Command, allowed_names: &[&str]) {
    command.env_clear();
    for (key, value) in std::env::vars_os() {
        let Some(name) = key.to_str() else {
            continue;
        };
        let keep = BASE_ALLOWLIST.contains(&name) || allowed_names.contains(&name);
        if keep {
            command.env(&key, &value);
        }
    }
}

/// Strict allowlist for the `claude_cli` backend. Just the primary
/// credential vars; `ANTHROPIC_BEDROCK_BASE_URL`,
/// `ANTHROPIC_CUSTOM_HEADERS`, etc are dropped because they can
/// redirect requests to a hostile endpoint or inject headers an
/// untrusted parent env could weaponize. Users running under
/// hardened mode who need Bedrock / Vertex routing must set those
/// values via the chan-llm config or extend the allowlist here.
const CLAUDE_CLI_STRICT_ALLOWLIST: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
];

/// Strict allowlist for the `gemini_cli` backend. Just the primary
/// API-key vars; `GOOGLE_APPLICATION_CREDENTIALS` (a path to a
/// service-account JSON) is dropped under hardened mode so a
/// tainted parent env can't redirect gemini-cli at an attacker-
/// controlled credential file.
const GEMINI_CLI_STRICT_ALLOWLIST: &[&str] = &["GEMINI_API_KEY", "GOOGLE_API_KEY"];

/// Strict allowlist for the `codex_cli` backend. Just the primary
/// credential vars Codex consumes. Anything matching the broader
/// `OPENAI_` / `CODEX_` prefixes (org IDs, project IDs, alternate
/// base URLs) is dropped under hardened mode; users who need them
/// configure them via the chan-llm config.
const CODEX_CLI_STRICT_ALLOWLIST: &[&str] = &["OPENAI_API_KEY", "CODEX_API_KEY"];

/// Per-backend wrapper: pick the loose or strict variant based on
/// `hardened`. Centralized so each backend doesn't have to repeat
/// the if/else; also makes the strict allowlist for each vendor
/// the single source of truth (instead of scattered string literals).
pub(crate) fn sanitize_env_for_claude_cli(command: &mut Command, hardened: bool) {
    if hardened {
        sanitize_env_strict(command, CLAUDE_CLI_STRICT_ALLOWLIST);
    } else {
        sanitize_env(command, &["ANTHROPIC_", "CLAUDE_"]);
    }
}

pub(crate) fn sanitize_env_for_gemini_cli(command: &mut Command, hardened: bool) {
    if hardened {
        sanitize_env_strict(command, GEMINI_CLI_STRICT_ALLOWLIST);
    } else {
        sanitize_env(command, &["GOOGLE_", "GEMINI_"]);
    }
}

pub(crate) fn sanitize_env_for_codex_cli(command: &mut Command, hardened: bool) {
    if hardened {
        sanitize_env_strict(command, CODEX_CLI_STRICT_ALLOWLIST);
    } else {
        sanitize_env(command, &["CODEX_", "OPENAI_"]);
    }
}

/// Hard cap on stderr we hold in memory per child. 8 KiB covers any
/// reasonable single error message (claude / gemini both emit
/// kilobyte-class diagnostics at worst). The drainer keeps reading
/// past the cap to keep the pipe flowing; bytes past the cap are
/// dropped on the floor. This is the budget we're willing to spend
/// to surface a wedged-child's last words.
pub(crate) const STDERR_CAP_BYTES: usize = 8 * 1024;

/// Shared buffer the drainer fills and the backend reads from on
/// error paths. Wrapped in `Arc<Mutex<_>>` because both the drainer
/// task and the backend read after `child.wait()`.
pub(crate) type SharedStderrBuf = Arc<Mutex<Vec<u8>>>;

/// Drainer handle returned by `spawn_stderr_drainer`. The backend
/// keeps it for the lifetime of the child; on early-exit paths it
/// awaits the join handle so the drainer flushes its in-flight read
/// before the backend reads the buffer.
pub(crate) struct StderrDrainer {
    pub(crate) buf: SharedStderrBuf,
    pub(crate) handle: JoinHandle<()>,
}

/// Soft grace window we give the drainer to finish naturally after
/// the child has been killed / waited. In practice the drainer
/// returns within a few milliseconds (stderr EOFs on child exit),
/// but on macOS with current_thread tokio runtimes we've observed
/// reads that wait past the child reap before the kqueue event
/// propagates. Past this window we abort the task and return what
/// the buffer captured so far; the snippet is already
/// surface-bounded so a slightly-truncated read is fine.
const DRAINER_FINISH_GRACE: std::time::Duration = std::time::Duration::from_millis(200);

impl StderrDrainer {
    /// Drain whatever's left in the read loop, then return a
    /// human-readable string from the captured buffer.
    ///
    /// Race the natural completion of the drainer task against a
    /// short grace window. If the task completes first (the common
    /// case: child died, stderr pipe EOF'd, read returned Ok(0)),
    /// we get a full flush. If the window elapses first (observed
    /// on macOS during the oversize-line test), we abort the task
    /// and read whatever was captured. Either way the caller gets
    /// a bounded snippet without risking a deadlock against the
    /// reactor.
    pub(crate) async fn finish(self) -> String {
        let StderrDrainer { buf, handle } = self;
        let abort = handle.abort_handle();
        let _ = tokio::time::timeout(DRAINER_FINISH_GRACE, handle).await;
        // Abort is a no-op if the task already completed. After
        // this returns the task is no longer mutating `buf`.
        abort.abort();
        let b = buf.lock().expect("stderr buf poisoned");
        String::from_utf8_lossy(&b).into_owned()
    }
}

/// Spawn a tokio task that drains `stderr` into a capped buffer
/// concurrently with the main stdout loop. Returns `None` if the
/// caller passed `None` (no stderr piped) so call sites can keep a
/// uniform shape.
pub(crate) fn spawn_stderr_drainer(
    stderr: Option<tokio::process::ChildStderr>,
) -> Option<StderrDrainer> {
    let stderr = stderr?;
    let buf: SharedStderrBuf = Arc::new(Mutex::new(Vec::with_capacity(1024)));
    let buf_drain = buf.clone();
    let handle = tokio::spawn(async move {
        let mut reader = stderr;
        let mut chunk = vec![0u8; 4096];
        loop {
            match reader.read(&mut chunk).await {
                Ok(0) => break,
                Ok(n) => {
                    let mut held = buf_drain.lock().expect("stderr buf poisoned");
                    let room = STDERR_CAP_BYTES.saturating_sub(held.len());
                    let take = n.min(room);
                    if take > 0 {
                        held.extend_from_slice(&chunk[..take]);
                    }
                    // Bytes past the cap are dropped on the floor so
                    // the pipe never blocks; the cap exists because we
                    // surface a snippet on error and don't want
                    // unbounded growth from a misbehaving child.
                }
                Err(_) => break,
            }
        }
    });
    Some(StderrDrainer { buf, handle })
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

    // Mirror the strict-mode filter for direct testing. Same shape
    // as `keeps` but uses verbatim name matching instead of prefix
    // matching.
    fn keeps_strict(name: &str, allowed: &[&str]) -> bool {
        BASE_ALLOWLIST.contains(&name) || allowed.contains(&name)
    }

    #[test]
    fn strict_allowlist_forwards_exact_names_only() {
        assert!(keeps_strict("ANTHROPIC_API_KEY", &["ANTHROPIC_API_KEY"]));
        assert!(keeps_strict("GEMINI_API_KEY", &["GEMINI_API_KEY"]));
        // Strict mode rejects vendor-prefix matches: GOOGLE_APPLICATION_CREDENTIALS
        // and ANTHROPIC_BEDROCK_BASE_URL no longer leak when the
        // caller intended "just the API key, nothing else".
        assert!(!keeps_strict(
            "GOOGLE_APPLICATION_CREDENTIALS",
            &["GEMINI_API_KEY"]
        ));
        assert!(!keeps_strict(
            "ANTHROPIC_BEDROCK_BASE_URL",
            &["ANTHROPIC_API_KEY"]
        ));
        // POSIX base allowlist still forwards.
        assert!(keeps_strict("PATH", &["ANTHROPIC_API_KEY"]));
        assert!(keeps_strict("HOME", &[]));
    }

    // The per-vendor strict allowlists are the single source of truth
    // for what each `sanitize_env_for_*` wrapper forwards under
    // hardened mode. Locking them down via tests catches accidental
    // widening (someone adding `ANTHROPIC_BEDROCK_*` "to be helpful"
    // re-introduces the redirect-style attack surface the strict
    // allowlist exists to prevent).

    #[test]
    fn claude_cli_strict_allowlist_pinned() {
        let allow = CLAUDE_CLI_STRICT_ALLOWLIST;
        // What a hardened-mode caller MUST get through:
        assert!(allow.contains(&"ANTHROPIC_API_KEY"));
        assert!(allow.contains(&"ANTHROPIC_AUTH_TOKEN"));
        assert!(allow.contains(&"CLAUDE_CODE_OAUTH_TOKEN"));
        // What it MUST NOT get through (these would be loose-mode-only):
        assert!(!allow.contains(&"ANTHROPIC_BEDROCK_BASE_URL"));
        assert!(!allow.contains(&"ANTHROPIC_CUSTOM_HEADERS"));
        assert!(!allow.contains(&"CLAUDE_CODE_USE_VERTEX"));
    }

    #[test]
    fn gemini_cli_strict_allowlist_pinned() {
        let allow = GEMINI_CLI_STRICT_ALLOWLIST;
        assert!(allow.contains(&"GEMINI_API_KEY"));
        assert!(allow.contains(&"GOOGLE_API_KEY"));
        // Path-pointing var: if a tainted parent env sets this to
        // an attacker-controlled JSON, gemini-cli would try to use
        // those credentials. Strict mode drops it.
        assert!(!allow.contains(&"GOOGLE_APPLICATION_CREDENTIALS"));
    }

    #[test]
    fn codex_cli_strict_allowlist_pinned() {
        let allow = CODEX_CLI_STRICT_ALLOWLIST;
        assert!(allow.contains(&"OPENAI_API_KEY"));
        assert!(allow.contains(&"CODEX_API_KEY"));
        // Same reasoning as the others: dropping OPENAI_BASE_URL
        // prevents a tainted parent from redirecting Codex to a
        // hostile endpoint that captures the key.
        assert!(!allow.contains(&"OPENAI_BASE_URL"));
        assert!(!allow.contains(&"OPENAI_ORG_ID"));
    }
}
