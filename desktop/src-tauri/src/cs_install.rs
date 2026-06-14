//! First-run install of the `chan` and `cs` bin shims into `~/.local/bin`, so a
//! chan-desktop install also gives you the `chan` / `cs` command line with
//! nothing extra to download. Both names resolve to the running chan-desktop
//! binary; the argv[0] dispatch (`chan_shell::invoked_as_chan` /
//! `invoked_as_cs`) selects the CLI / control-client path before any GUI init.
//!
//! The shape of the shim depends on how chan-desktop was installed:
//!
//! - **macOS `.app`** and **Linux deb/rpm**: real symlinks to the installed
//!   binary (`current_exe()`). The path is stable across an in-place
//!   self-upgrade, and a symlink WE wrote is re-pointed on boot if the binary
//!   moved (e.g. the `.app` was dragged into /Applications).
//! - **Linux AppImage**: tiny wrapper scripts (`exec -a chan "$APPIMAGE" "$@"`),
//!   not symlinks. `std::env::current_exe()` inside an AppImage points into the
//!   ephemeral `/tmp/.mount_*` squashfs that vanishes on exit, and the `AppRun`
//!   shim can reset argv[0]; `exec -a <name> "$APPIMAGE"` pins both the stable
//!   path and the argv[0] the detection keys on.
//! - **dev build / `cargo run` / anything else**: no-op (never pollute
//!   `~/.local/bin` from an un-packaged build).
//!
//! Posture: best-effort + idempotent + self-healing. A failure is logged, never
//! fatal. A shim WE wrote self-heals on the next launch when it goes stale (the
//! binary moved, the AppImage updated). A `chan` / `cs` the user installed
//! themselves — a real binary from install.sh, a hand-made symlink — is never
//! clobbered.

#[cfg(unix)]
use std::ffi::OsStr;
#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;

/// The two names we own. Both point at the same chan-desktop binary; argv[0]
/// dispatch picks the behavior.
#[cfg(unix)]
const SHIM_NAMES: [&str; 2] = ["chan", "cs"];

/// File stem of the chan-desktop binary. A symlink at `~/.local/bin/{chan,cs}`
/// whose target ends in this stem is recognized as OURS (so a moved/upgraded
/// install self-heals), while a user's own `cs -> chan` (stem "chan") is left
/// alone.
#[cfg(unix)]
const DESKTOP_BIN_STEM: &str = "chan-desktop";

/// Marker line written into a wrapper script so we only ever rewrite a wrapper
/// WE wrote.
#[cfg(unix)]
const WRAPPER_MARKER: &str = "# chan-desktop bin shim";

/// Ownership substring: any wrapper containing this was written by some version
/// of chan-desktop (the marker text has changed over time), so it is ours to
/// rewrite. A user's own script will not contain it.
#[cfg(unix)]
const WRAPPER_OWNS: &str = "# chan-desktop";

/// The AppImage path from `$APPIMAGE`, or `None` when not running from an
/// AppImage (macOS, `cargo run`, a deb/rpm install). Also consumed by
/// `linux_gui_stack` to gate the bundle-first loader fixups.
pub fn appimage_path() -> Option<PathBuf> {
    std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

/// How this chan-desktop was installed, which decides how the shims are made.
#[cfg(unix)]
#[derive(Debug, PartialEq, Eq)]
enum InstallKind {
    /// Linux AppImage: write wrapper scripts re-execing this `$APPIMAGE`.
    AppImage(PathBuf),
    /// A stable on-disk binary we can symlink to directly — a macOS `.app`
    /// bundle binary or a Linux deb/rpm `/usr/bin/chan-desktop`.
    Symlink(PathBuf),
    /// Dev build / unrecognized layout: do nothing.
    None,
}

/// Classify the install from the AppImage path and the (canonicalized) running
/// exe. Pure, so the precedence is unit-tested without the process env. The
/// `.app` and `/usr` checks are not `cfg`-gated because the two patterns are
/// mutually exclusive in practice (a Linux path never contains
/// `.app/Contents/MacOS/`, a macOS app binary is never under `/usr`), which
/// keeps both branches testable on any host.
#[cfg(unix)]
fn classify_install(appimage: Option<PathBuf>, exe: Option<PathBuf>) -> InstallKind {
    if let Some(appimage) = appimage {
        return InstallKind::AppImage(appimage);
    }
    let Some(exe) = exe else {
        return InstallKind::None;
    };
    let s = exe.to_string_lossy();
    // macOS .app bundle binary: …/Chan.app/Contents/MacOS/chan-desktop.
    if s.contains(".app/Contents/MacOS/") {
        return InstallKind::Symlink(exe);
    }
    // Linux deb/rpm: a system-installed /usr/bin/chan-desktop (or /usr/local).
    if s.starts_with("/usr/") && exe.file_name() == Some(OsStr::new(DESKTOP_BIN_STEM)) {
        return InstallKind::Symlink(exe);
    }
    InstallKind::None
}

/// Gather the env + exe and classify. The exe is canonicalized so the symlink
/// target is the real binary (stable across an in-place self-upgrade) and a
/// moved install re-points cleanly.
#[cfg(unix)]
fn detect_kind() -> InstallKind {
    let appimage = appimage_path();
    let exe = std::env::current_exe()
        .ok()
        .map(|e| e.canonicalize().unwrap_or(e));
    classify_install(appimage, exe)
}

/// The wrapper script that re-execs `target` as `name`. `exec -a` (a bash
/// builtin present on every AppImage-capable desktop) forces argv[0] regardless
/// of how the AppImage `AppRun` shim would otherwise rewrite it.
#[cfg(unix)]
fn wrapper_script(name: &str, target: &Path) -> String {
    format!(
        "#!/usr/bin/env bash\n\
         {WRAPPER_MARKER}\n\
         # Re-exec the chan-desktop AppImage as `{name}` so the right argv[0]\n\
         # dispatch fires (CLI / control client) instead of the GUI. Rewritten\n\
         # on launch if the AppImage path changes; delete this file to opt out.\n\
         exec -a {name} {} \"$@\"\n",
        shell_single_quote(target),
    )
}

/// Single-quote a path for the wrapper so a space or shell metacharacter in the
/// AppImage path is safe. Embedded single quotes are escaped the POSIX way
/// (`'\''`).
#[cfg(unix)]
fn shell_single_quote(p: &Path) -> String {
    format!("'{}'", p.to_string_lossy().replace('\'', "'\\''"))
}

/// What to do with a wrapper-script shim given the current file contents.
#[cfg(unix)]
#[derive(Debug, PartialEq, Eq)]
enum WrapperPlan {
    /// Leave it: a user's own file (no marker), or already current.
    Skip,
    /// Write this script (absent, or ours-but-stale).
    Write(String),
}

#[cfg(unix)]
fn plan_wrapper(name: &str, appimage: &Path, existing: Option<&str>) -> WrapperPlan {
    let desired = wrapper_script(name, appimage);
    match existing {
        // A file we did not write: never touch it.
        Some(content) if !content.contains(WRAPPER_OWNS) => WrapperPlan::Skip,
        // Our wrapper, already current.
        Some(content) if content == desired => WrapperPlan::Skip,
        // Absent, or our wrapper but stale -> (re)write.
        _ => WrapperPlan::Write(desired),
    }
}

/// What we observe at a shim path, enough to plan a symlink without clobbering
/// anything foreign.
#[cfg(unix)]
#[derive(Debug, PartialEq, Eq)]
enum Observed {
    /// Nothing there.
    Absent,
    /// A symlink, with the literal path it points at.
    SymlinkTo(PathBuf),
    /// A regular file (or anything that is not a symlink we can read): foreign.
    NonSymlink,
}

#[cfg(unix)]
fn observe(path: &Path) -> Observed {
    match std::fs::symlink_metadata(path) {
        Err(_) => Observed::Absent,
        Ok(meta) if meta.file_type().is_symlink() => match std::fs::read_link(path) {
            Ok(target) => Observed::SymlinkTo(target),
            Err(_) => Observed::NonSymlink,
        },
        Ok(_) => Observed::NonSymlink,
    }
}

/// What to do with a symlink shim.
#[cfg(unix)]
#[derive(Debug, PartialEq, Eq)]
enum SymAction {
    /// Leave it: already current, or foreign.
    Skip,
    /// Create the symlink (nothing there).
    Link,
    /// Replace our own stale symlink (the binary moved / self-upgraded).
    Relink,
}

#[cfg(unix)]
fn plan_symlink(target: &Path, observed: &Observed) -> SymAction {
    match observed {
        Observed::Absent => SymAction::Link,
        // Never clobber a real file (a user's standalone `chan` binary, etc.).
        Observed::NonSymlink => SymAction::Skip,
        Observed::SymlinkTo(current) => {
            if current == target {
                SymAction::Skip
            } else if current.file_name() == Some(OsStr::new(DESKTOP_BIN_STEM)) {
                // A symlink pointing at a (different) chan-desktop binary is one
                // of ours that went stale -> re-point. A user's `cs -> chan`
                // (stem "chan") falls through to Skip.
                SymAction::Relink
            } else {
                SymAction::Skip
            }
        }
    }
}

/// Install (or refresh) one shim. Returns `Ok(true)` when it wrote/updated the
/// shim, `Ok(false)` when nothing was needed (foreign entry, already current).
#[cfg(unix)]
fn install_one(kind: &InstallKind, bin_dir: &Path, name: &str) -> std::io::Result<bool> {
    use std::os::unix::fs::PermissionsExt;

    let path = bin_dir.join(name);
    match kind {
        InstallKind::None => Ok(false),
        InstallKind::AppImage(appimage) => {
            // Read the existing wrapper as text. An entry that exists but is not
            // readable UTF-8 (a binary, a symlink to one) is foreign -> skip.
            let existing = match std::fs::read_to_string(&path) {
                Ok(content) => Some(content),
                Err(_) if path.symlink_metadata().is_ok() => return Ok(false),
                Err(_) => None,
            };
            match plan_wrapper(name, appimage, existing.as_deref()) {
                WrapperPlan::Skip => Ok(false),
                WrapperPlan::Write(script) => {
                    std::fs::create_dir_all(bin_dir)?;
                    std::fs::write(&path, script)?;
                    let mut perms = std::fs::metadata(&path)?.permissions();
                    perms.set_mode(0o755);
                    std::fs::set_permissions(&path, perms)?;
                    Ok(true)
                }
            }
        }
        InstallKind::Symlink(target) => match plan_symlink(target, &observe(&path)) {
            SymAction::Skip => Ok(false),
            SymAction::Link => {
                std::fs::create_dir_all(bin_dir)?;
                std::os::unix::fs::symlink(target, &path)?;
                Ok(true)
            }
            SymAction::Relink => {
                std::fs::create_dir_all(bin_dir)?;
                std::fs::remove_file(&path)?;
                std::os::unix::fs::symlink(target, &path)?;
                Ok(true)
            }
        },
    }
}

/// Install the `~/.local/bin/{chan,cs}` shims for this install, self-healing any
/// stale shim WE wrote. No-op for a dev build / unrecognized layout or when
/// there is no home dir. Returns the number of shims written/updated.
///
/// Best-effort: a per-shim failure is logged and the other shim is still
/// attempted; nothing here is fatal to boot.
#[cfg(unix)]
pub fn install_bin_shims() -> std::io::Result<u32> {
    let kind = detect_kind();
    if matches!(kind, InstallKind::None) {
        return Ok(0);
    }
    let Some(home) = dirs::home_dir() else {
        return Ok(0);
    };
    let bin_dir = home.join(".local").join("bin");

    let mut changed = 0u32;
    for name in SHIM_NAMES {
        match install_one(&kind, &bin_dir, name) {
            Ok(true) => changed += 1,
            Ok(false) => {}
            Err(e) => tracing::warn!(error = %e, shim = name, "installing bin shim failed"),
        }
    }
    Ok(changed)
}

/// Non-unix: chan-desktop ships macOS/Linux only, and the shim mechanisms
/// (symlinks, `exec -a`) are unix, so there is nothing to install elsewhere.
#[cfg(not(unix))]
pub fn install_bin_shims() -> std::io::Result<u32> {
    Ok(0)
}

/// Legacy entry point. chan-desktop's `main()` still calls this; it delegates to
/// [`install_bin_shims`] so the call site keeps compiling until it migrates to
/// the new name. Returns `Ok(true)` when any shim was written/updated.
pub fn install_appimage_cs_wrapper() -> std::io::Result<bool> {
    install_bin_shims().map(|n| n > 0)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn classify_appimage_takes_precedence() {
        let appimage = PathBuf::from("/home/u/Chan.AppImage");
        // Even with a plausible exe, $APPIMAGE wins.
        assert_eq!(
            classify_install(
                Some(appimage.clone()),
                Some(PathBuf::from("/usr/bin/chan-desktop"))
            ),
            InstallKind::AppImage(appimage)
        );
    }

    #[test]
    fn classify_macos_app_bundle_is_symlink() {
        let exe = PathBuf::from("/Applications/Chan.app/Contents/MacOS/chan-desktop");
        assert_eq!(
            classify_install(None, Some(exe.clone())),
            InstallKind::Symlink(exe)
        );
    }

    #[test]
    fn classify_linux_system_install_is_symlink() {
        let exe = PathBuf::from("/usr/bin/chan-desktop");
        assert_eq!(
            classify_install(None, Some(exe.clone())),
            InstallKind::Symlink(exe)
        );
    }

    #[test]
    fn classify_dev_build_is_none() {
        // A cargo target dir is neither an .app bundle nor a /usr install.
        assert_eq!(
            classify_install(
                None,
                Some(PathBuf::from("/home/u/chan/target/debug/chan-desktop"))
            ),
            InstallKind::None
        );
        assert_eq!(classify_install(None, None), InstallKind::None);
    }

    #[test]
    fn wrapper_script_pins_argv0_and_quotes_path() {
        let s = wrapper_script("chan", Path::new("/home/u/Apps/Chan x86_64.AppImage"));
        assert!(s.starts_with("#!/usr/bin/env bash\n"));
        assert!(s.contains(WRAPPER_MARKER));
        // argv[0] forced to the name, AppImage path single-quoted (space-safe).
        assert!(s.contains("exec -a chan '/home/u/Apps/Chan x86_64.AppImage' \"$@\""));
        // The same target produces a distinct script per name.
        let cs = wrapper_script("cs", Path::new("/home/u/Apps/Chan x86_64.AppImage"));
        assert!(cs.contains("exec -a cs '/home/u/Apps/Chan x86_64.AppImage' \"$@\""));
        assert_ne!(s, cs);
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
    fn wrapper_plan_writes_when_absent_and_skips_foreign() {
        let appimage = Path::new("/home/u/Chan.AppImage");
        assert!(matches!(
            plan_wrapper("cs", appimage, None),
            WrapperPlan::Write(_)
        ));
        // No marker -> someone else's cs. Hands off.
        let foreign = "#!/bin/sh\nexec /usr/local/bin/chan shell \"$@\"\n";
        assert_eq!(
            plan_wrapper("cs", appimage, Some(foreign)),
            WrapperPlan::Skip
        );
    }

    #[test]
    fn wrapper_plan_skips_current_and_rewrites_stale() {
        let appimage = Path::new("/home/u/Chan.AppImage");
        let current = wrapper_script("cs", appimage);
        assert_eq!(
            plan_wrapper("cs", appimage, Some(&current)),
            WrapperPlan::Skip
        );
        // Our wrapper, but the AppImage moved -> rewrite.
        let stale = wrapper_script("cs", Path::new("/old/Chan.AppImage"));
        assert!(matches!(
            plan_wrapper("cs", appimage, Some(&stale)),
            WrapperPlan::Write(_)
        ));
    }

    #[test]
    fn wrapper_plan_recognizes_an_older_marker_as_ours() {
        // A wrapper written by an earlier chan-desktop (different marker text)
        // is still ours to refresh, not orphaned as foreign.
        let appimage = Path::new("/home/u/Chan.AppImage");
        let old = "#!/usr/bin/env bash\n# chan-desktop cs wrapper\nexec -a cs '/old/Chan.AppImage' \"$@\"\n";
        assert!(matches!(
            plan_wrapper("cs", appimage, Some(old)),
            WrapperPlan::Write(_)
        ));
    }

    #[test]
    fn symlink_plan_links_when_absent() {
        let target = Path::new("/Applications/Chan.app/Contents/MacOS/chan-desktop");
        assert_eq!(plan_symlink(target, &Observed::Absent), SymAction::Link);
    }

    #[test]
    fn symlink_plan_skips_current_and_foreign_file() {
        let target = Path::new("/usr/bin/chan-desktop");
        assert_eq!(
            plan_symlink(target, &Observed::SymlinkTo(target.to_path_buf())),
            SymAction::Skip
        );
        // A real file (a user's standalone `chan` binary) is never clobbered.
        assert_eq!(plan_symlink(target, &Observed::NonSymlink), SymAction::Skip);
    }

    #[test]
    fn symlink_plan_relinks_our_stale_link_but_leaves_a_user_link() {
        let target = Path::new("/Applications/Chan.app/Contents/MacOS/chan-desktop");
        // Our own link, now stale (the .app moved) -> re-point.
        let stale = Observed::SymlinkTo(PathBuf::from("/old/Chan.app/Contents/MacOS/chan-desktop"));
        assert_eq!(plan_symlink(target, &stale), SymAction::Relink);
        // A user's `cs -> chan` (a standalone install) is foreign -> leave it.
        let user = Observed::SymlinkTo(PathBuf::from("/home/u/.local/bin/chan"));
        assert_eq!(plan_symlink(target, &user), SymAction::Skip);
    }
}
