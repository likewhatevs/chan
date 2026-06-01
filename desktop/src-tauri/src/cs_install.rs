//! First-run install of a `cs` wrapper into `~/.local/bin` for AppImage
//! users (round-3 @@Architect decision, option (a)).
//!
//! An AppImage is a single self-mounting file with no in-bundle symlink, so
//! a desktop-only Linux user has no `cs` on PATH and cannot drive the
//! running window from a terminal. On launch FROM an AppImage (the AppImage
//! runtime sets `$APPIMAGE` to the .AppImage path), chan-desktop drops a
//! tiny wrapper that re-execs the AppImage with argv[0]="cs", so
//! `chan_shell::invoked_as_cs` fires and `run_as_cs_if_requested` takes the
//! control-client path instead of the GUI.
//!
//! Why a wrapper and not a symlink: `std::env::current_exe()` inside an
//! AppImage points into the ephemeral `/tmp/.mount_*` squashfs that vanishes
//! on exit, and the AppImage `AppRun` shim can reset argv[0], so a symlink is
//! unreliable. `exec -a cs "$APPIMAGE"` pins both the stable path and the
//! argv[0] the detection keys on.
//!
//! Posture: best-effort + idempotent. A failure is logged, never fatal. A
//! stale wrapper (the AppImage moved / updated) self-heals on the next
//! launch. A `cs` the user installed themselves is never clobbered.

#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;

/// Marker line so we only ever rewrite a wrapper WE wrote, never a user's
/// own `cs` (a hand-made `cs -> chan` symlink, another tool, etc.).
const WRAPPER_MARKER: &str = "# chan-desktop cs wrapper";

/// The AppImage path from `$APPIMAGE`, or `None` when not running from an
/// AppImage (macOS, `cargo run`, a deb/rpm install) so the installer is a
/// no-op there.
pub fn appimage_path() -> Option<PathBuf> {
    std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

/// The wrapper script that re-execs `target` as `cs`. `exec -a` is a bash
/// builtin (bash is present on every AppImage-capable desktop), and it is
/// what guarantees argv[0]=="cs" regardless of how the AppImage `AppRun`
/// shim would otherwise rewrite it.
#[cfg(unix)]
fn wrapper_script(target: &Path) -> String {
    format!(
        "#!/usr/bin/env bash\n\
         {WRAPPER_MARKER}\n\
         # Re-exec the chan-desktop AppImage as `cs` so the control client\n\
         # runs instead of the GUI (argv[0] detection). Rewritten on launch\n\
         # if the AppImage path changes; delete this file to opt out.\n\
         exec -a cs {} \"$@\"\n",
        shell_single_quote(target),
    )
}

/// Single-quote a path for the wrapper so a space or shell metacharacter in
/// the AppImage path is safe. Embedded single quotes are escaped the POSIX
/// way (`'\''`).
#[cfg(unix)]
fn shell_single_quote(p: &Path) -> String {
    format!("'{}'", p.to_string_lossy().replace('\'', "'\\''"))
}

/// What the installer should do given the AppImage path and the current
/// contents of `~/.local/bin/cs` (if any). Pure, so the precedence is
/// unit-tested without touching the filesystem or the environment.
#[cfg(unix)]
#[derive(Debug, PartialEq, Eq)]
enum Plan {
    /// Leave `cs` alone: it is a user's own (no marker), or already current.
    Skip,
    /// Write this wrapper (cs is absent, ours-but-stale).
    Write(String),
}

#[cfg(unix)]
fn plan(target: &Path, existing: Option<&str>) -> Plan {
    let desired = wrapper_script(target);
    match existing {
        // A `cs` we did not write: never touch it.
        Some(content) if !content.contains(WRAPPER_MARKER) => Plan::Skip,
        // Our wrapper, already pointing at the right AppImage.
        Some(content) if content == desired => Plan::Skip,
        // Absent, or our wrapper but stale -> (re)write.
        _ => Plan::Write(desired),
    }
}

/// Install `~/.local/bin/cs` pointing at the running AppImage. No-op unless
/// running from an AppImage. Returns `Ok(true)` when it wrote/updated the
/// wrapper, `Ok(false)` when nothing was needed (not an AppImage, no home,
/// up-to-date, or a foreign `cs` left untouched).
#[cfg(unix)]
pub fn install_appimage_cs_wrapper() -> std::io::Result<bool> {
    use std::os::unix::fs::PermissionsExt;

    let Some(appimage) = appimage_path() else {
        return Ok(false);
    };
    let Some(home) = dirs::home_dir() else {
        return Ok(false);
    };
    let bin_dir = home.join(".local").join("bin");
    let cs_path = bin_dir.join("cs");

    // Read the existing wrapper as text. A non-UTF8 / unreadable entry that
    // nonetheless exists (a binary, a symlink to one) is treated as foreign
    // and left alone, so we never stomp something we did not author.
    let existing = match std::fs::read_to_string(&cs_path) {
        Ok(content) => Some(content),
        Err(_) if cs_path.symlink_metadata().is_ok() => return Ok(false),
        Err(_) => None,
    };

    match plan(&appimage, existing.as_deref()) {
        Plan::Skip => Ok(false),
        Plan::Write(script) => {
            std::fs::create_dir_all(&bin_dir)?;
            std::fs::write(&cs_path, script)?;
            let mut perms = std::fs::metadata(&cs_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&cs_path, perms)?;
            Ok(true)
        }
    }
}

/// Non-unix: chan-desktop ships AppImages only on Linux, and `exec -a` is a
/// unix builtin, so there is nothing to install elsewhere.
#[cfg(not(unix))]
pub fn install_appimage_cs_wrapper() -> std::io::Result<bool> {
    Ok(false)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn wrapper_script_pins_argv0_and_quotes_path() {
        let s = wrapper_script(Path::new("/home/u/Apps/Chan x86_64.AppImage"));
        assert!(s.starts_with("#!/usr/bin/env bash\n"));
        assert!(s.contains(WRAPPER_MARKER));
        // argv[0] forced to cs, AppImage path single-quoted (space-safe).
        assert!(s.contains("exec -a cs '/home/u/Apps/Chan x86_64.AppImage' \"$@\""));
    }

    #[test]
    fn shell_single_quote_escapes_embedded_quote() {
        assert_eq!(shell_single_quote(Path::new("/a/b")), "'/a/b'");
        assert_eq!(
            shell_single_quote(Path::new("/it's/here")),
            "'/it'\\''s/here'"
        );
    }

    #[test]
    fn plan_writes_when_absent() {
        let appimage = Path::new("/home/u/Chan.AppImage");
        assert!(matches!(plan(appimage, None), Plan::Write(_)));
    }

    #[test]
    fn plan_skips_a_foreign_cs() {
        // No marker -> someone else's cs. Hands off.
        let foreign = "#!/bin/sh\nexec /usr/local/bin/chan shell \"$@\"\n";
        assert_eq!(
            plan(Path::new("/home/u/Chan.AppImage"), Some(foreign)),
            Plan::Skip
        );
    }

    #[test]
    fn plan_skips_when_already_current_and_rewrites_when_stale() {
        let appimage = Path::new("/home/u/Chan.AppImage");
        let current = wrapper_script(appimage);
        assert_eq!(plan(appimage, Some(&current)), Plan::Skip);
        // Our wrapper, but the AppImage moved -> rewrite.
        let stale = wrapper_script(Path::new("/old/Chan.AppImage"));
        assert!(matches!(plan(appimage, Some(&stale)), Plan::Write(_)));
    }
}
