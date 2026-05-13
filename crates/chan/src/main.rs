// chan: notes app with embedded web editor.
//
// This binary is the user-facing entry point. Subcommands:
//
//   chan add <path> [--name NAME]   register a directory as a chan
//                                   drive in ~/.chan/config.toml
//   chan list                       list registered drives,
//                                   most-recent first
//   chan remove <path>              drop a drive from the registry
//                                   (filesystem contents untouched)
//   chan rename <path> <name>       set / clear a drive's display
//                                   name
//   chan serve [-4|-6] [--host H --port N]
//                                   run the HTTP server. Defaults
//                                   to 127.0.0.1 (loopback only);
//                                   -6 picks ::1 instead. The
//                                   embedded web editor talks to
//                                   this.
//   chan index <path>               rebuild the search index +
//                                   graph for the drive
//   chan search <path> <query>      query the BM25 index
//   chan contacts import csv FILE --into DIR
//                                   import a Google Contacts CSV
//                                   as one markdown note per
//                                   contact under DIR (drive-
//                                   relative). Notes carry
//                                   `chan.kind: contact`
//                                   frontmatter for graph + @
//                                   picker classification.
//
// Anything that touches the registry / drive contents goes through
// `chan_drive::Library` and `chan_drive::Drive` so the library's
// invariants (atomic writes, path sandbox, special-file refusal,
// cross-process writer lock) apply uniformly.

use std::io::{IsTerminal, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use chan_drive::{Library, SearchOpts};
use chan_server::ServeConfig;
use clap::{Parser, Subcommand};

mod update;

/// Extended description for `chan serve`. The keybindings list is
/// the source of truth users land on via `chan serve --help`; the
/// in-app handler (web/src/App.svelte) must stay in sync. Chords
/// here are the BROWSER set; the native shell (chan-desktop) lays
/// VS Code-shaped chords on top. On Linux / Windows substitute
/// Ctrl for Cmd everywhere except the entries marked browser's
/// own (those are handled by the browser regardless of platform).
const SERVE_LONG_ABOUT: &str = "\
Run the HTTP server. Defaults to 127.0.0.1 (loopback only).

In-app keybindings (Cmd = Ctrl on Linux / Windows):

  Settings                Cmd+,
  Files                   Cmd+P
  Assistant               Cmd+I
  Search across files     Cmd+Shift+F
  Graph                   Cmd+Shift+M
  Save                    Cmd+S
  New file                Ctrl+Alt+N
  Next tab                Alt+Shift+]
  Previous tab            Alt+Shift+[
  Jump to tab N           Ctrl+Alt+1..9
  Pop top overlay         Esc

Handled by the browser:

  Find on page            Cmd+F
  Find next               Cmd+G
  Find previous           Cmd+Shift+G
  Close tab               Cmd+W
  Zoom in / out / reset   Cmd+= / Cmd+- / Cmd+0
";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Increase logging. -v = info, -vv = debug, -vvv = trace.
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Register a directory as a chan drive.
    Add {
        path: PathBuf,
        /// Display name shown in the recents list and window title.
        /// Defaults to the directory's basename on first registration.
        #[arg(long)]
        name: Option<String>,
    },
    /// List registered drives, most-recent first.
    List,
    /// Drop a drive from the registry. Does not delete the
    /// directory or its content; only forgets it on this machine.
    Remove { path: PathBuf },
    /// Set or clear a drive's display name.
    ///
    /// Both PATH and NAME are required: defaulting PATH to the
    /// default drive would silently rename the wrong drive when
    /// multiple are registered. Explicit beats friendly here. To
    /// rename the drive you're standing in: `chan rename . NEWNAME`.
    Rename {
        path: PathBuf,
        /// Pass `""` to clear the name.
        name: String,
    },
    /// Run the HTTP server. Defaults to 127.0.0.1 (loopback only).
    #[command(long_about = SERVE_LONG_ABOUT)]
    Serve {
        path: Option<PathBuf>,
        /// Serve the given path verbatim instead of suggesting the
        /// enclosing VCS repository root. Without this flag, `chan
        /// serve` refuses to start when the drive path lives inside
        /// a Git / Mercurial / Subversion working tree (exit 70 +
        /// `chan-error: vcs-parent` marker on stderr) because the
        /// repo root is almost always a better drive root: it
        /// keeps cross-file links, the graph, and search aligned
        /// with the project boundary. Pass `--here` when you
        /// genuinely want to scope the drive to a subdir.
        #[arg(long)]
        here: bool,
        /// Host address to bind. Default 127.0.0.1 (or ::1 with -6).
        /// Use 0.0.0.0 / :: to listen on all interfaces. chan has no
        /// TLS and only a bearer-token gate, so any non-loopback host
        /// exposes your drive in plaintext on that network.
        #[arg(long)]
        host: Option<IpAddr>,
        /// Force IPv4-only listening. With no --host, binds 127.0.0.1.
        /// Mutually exclusive with -6.
        #[arg(short = '4', long = "ipv4", conflicts_with = "ipv6")]
        ipv4: bool,
        /// Force IPv6-only listening. With no --host, binds ::1.
        /// Mutually exclusive with -4.
        #[arg(short = '6', long = "ipv6")]
        ipv6: bool,
        #[arg(long, default_value_t = 8787)]
        port: u16,
        /// URL path prefix to mount the server under. Lets a reverse
        /// proxy multiplex many `chan serve` instances under one host
        /// (e.g. `drive.example.com/{user}/`). Canonicalized to
        /// `/seg[/seg...]` with `[A-Za-z0-9-]+` segments; trailing
        /// slashes and `//` runs are tolerated. Anything else is
        /// rejected. Mutually exclusive with --tunnel-token (the
        /// public gateway already strips /{user}/{drive}).
        #[arg(long, conflicts_with = "tunnel_token")]
        prefix: Option<String>,
        /// Idle timeout before the server triggers a graceful
        /// shutdown. Accepts `30s`, `5m`, `1h`. Useful for systemd
        /// socket-activated deployments where many idle instances
        /// stack on one host. Without this flag the server stays
        /// resident indefinitely.
        #[arg(long, value_parser = parse_idle_timeout)]
        timeout: Option<Duration>,
        /// Skip the per-launch bearer-token gate. Local dev only;
        /// never expose a no-token server on a shared machine.
        #[arg(long)]
        no_token: bool,
        /// Do not open the system default browser when the server is
        /// ready. The URL is still printed; useful for shells that
        /// host the UI in their own window (chan-desktop) or for
        /// headless / scripted invocations.
        #[arg(long)]
        no_browser: bool,
        /// Lock down the Settings panel: the SPA greys the cog and
        /// the server refuses every settings-write route with 403
        /// (PATCH /api/drive, /api/config, /api/server/config,
        /// PUT/DELETE /api/llm/keys/*, POST /api/storage/reset,
        /// POST /api/index/rebuild). Tunnel mode already implies
        /// this; the flag lets a local serve opt in for kiosk-style
        /// deployments (shared workstation, demo box) where the
        /// drive owner is not the operator at the keyboard.
        #[arg(long)]
        no_settings: bool,
        /// Tunnel endpoint URL. With --tunnel-token, chan serve
        /// dials this instead of binding a local listener.
        #[arg(long, default_value = "https://drive.chan.app/v1/tunnel")]
        tunnel_url: String,
        /// Personal access token (chan_pat_*) from id.chan.app.
        /// Setting this enables tunnel mode: chan serve does not
        /// bind a local TCP listener and instead publishes the
        /// drive at {user}.drive.chan.app/{drive}/*. Prefer the
        /// CHAN_TUNNEL_TOKEN env var so the secret does not appear
        /// in `ps`.
        #[arg(long, env = "CHAN_TUNNEL_TOKEN")]
        tunnel_token: Option<String>,
        /// Drive name to publish at /{user}/<name>. Must be
        /// lowercase [a-z0-9-], 1-32 chars, no leading/trailing
        /// hyphen. Defaults to a sanitized form of the drive's
        /// stored display name (e.g. "My Notes" -> "my-notes");
        /// chan emits a NOTE when it had to sanitize.
        #[arg(long)]
        tunnel_drive: Option<String>,
        /// Expose the tunneled drive without an OAuth gate. By
        /// default, `{user}.drive.chan.app/{drive}/` 404s anonymous
        /// visitors; the drive owner opens it from id.chan.app's
        /// dashboard via a short-lived drive-gate handoff. With
        /// --tunnel-public, anyone with the URL can reach the drive
        /// over the same tunnel. Requires --tunnel-token (or
        /// `CHAN_TUNNEL_TOKEN`); clap rejects the flag otherwise so
        /// it can't silently no-op on a non-tunnel run.
        #[arg(long, requires = "tunnel_token")]
        tunnel_public: bool,
    },
    /// Rebuild the search index + graph for a drive.
    Index { path: PathBuf },
    /// Query the BM25 search index.
    Search {
        path: PathBuf,
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Self-upgrade: download the latest release from chan.app/dl,
    /// verify SHA256, and atomically replace the running binary.
    /// URLs are hardcoded; the only knobs are `-y` (skip prompt),
    /// `--check` (report only), and `--version` (pin a release).
    /// Set `CHAN_UPDATE_CHECK=0` to silence the banner that fires
    /// on `chan serve` startup.
    Upgrade {
        /// Skip the confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
        /// Only check + report; do not download or replace the
        /// binary. Returns success in both directions.
        #[arg(long)]
        check: bool,
        /// Pin a specific version instead of querying chan.app/dl.
        /// Useful for downgrading or pinning to a tested release.
        #[arg(long)]
        version: Option<String>,
    },
    /// Internal: run the chan-llm MCP server on stdio against a
    /// drive. Spawned as a subprocess by the ClaudeCli backend
    /// (chan-llm v2 path, chan-llm issue #1) so claude routes its
    /// file edits through chan-drive's gates instead of touching the
    /// drive directly. Not for end-user invocation.
    #[command(name = "__mcp", hide = true)]
    Mcp {
        /// Drive root to expose. Must already be registered.
        path: PathBuf,
        /// Apply write_file calls without producing a "deferred"
        /// error. Off by default; chan-llm flips it on per session
        /// when the user has enabled auto-apply in settings.
        #[arg(long)]
        auto_apply: bool,
    },
    /// Internal: stdio bridge to the MCP server hosted in-process
    /// by a running `chan serve`. Connects to the per-server Unix-
    /// domain socket and pipes stdin/stdout through it. Used by the
    /// ClaudeCli / GeminiCli backends so the agent's MCP child can
    /// reach the live drive without trying to reopen it (which would
    /// deadlock against chan-drive's per-drive flock). Not for
    /// end-user invocation.
    #[command(name = "__mcp-proxy", hide = true)]
    McpProxy {
        /// Unix-domain socket path the running chan-server listens
        /// on. Resolved at request time by chan-server, embedded in
        /// the gemini settings.json / claude --mcp-config payload.
        socket: PathBuf,
    },
    /// Manage contacts inside a drive. Today: import contacts from
    /// an external source as one markdown note per contact, with
    /// `chan.kind: contact` frontmatter so the editor and graph
    /// classify them automatically.
    Contacts {
        #[command(subcommand)]
        action: ContactsAction,
    },
}

#[derive(Subcommand, Debug)]
enum ContactsAction {
    /// Import contacts from an external source as markdown notes.
    /// Pick the source format with a sub-subcommand.
    Import {
        #[command(subcommand)]
        source: ImportSource,
    },
}

#[derive(Subcommand, Debug)]
enum ImportSource {
    /// Import from a CSV file. Currently only Google Contacts
    /// CSV is supported (export at contacts.google.com -> Export
    /// -> "Google CSV"). Other CSV dialects can be added later
    /// behind --provider.
    Csv {
        /// Path to the CSV file.
        file: PathBuf,
        /// Drive-relative folder where notes will land. Created
        /// if it does not exist. Use `""` to write at the drive
        /// root.
        #[arg(long)]
        into: String,
        /// Source provider's CSV format. Currently only "google".
        #[arg(long, default_value = "google")]
        provider: String,
        /// Parse and report what would be written; do not touch
        /// the drive.
        #[arg(long)]
        dry_run: bool,
        /// Replace existing files instead of skipping them. The
        /// per-contact line in the report changes from SKIPPED to
        /// OVERWROTE so it's clear which files moved.
        #[arg(long)]
        overwrite: bool,
        /// Drive root. Defaults to the registered default drive.
        /// Auto-registers the path if not already known, so
        /// `chan contacts import csv ... --drive /some/dir`
        /// works without a prior `chan add`.
        #[arg(long)]
        drive: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    match cli.command {
        Command::Add { path, name } => cmd_add(path, name),
        Command::List => cmd_list(),
        Command::Remove { path } => cmd_remove(path),
        Command::Rename { path, name } => cmd_rename(path, name),
        Command::Serve {
            path,
            here,
            host,
            ipv4,
            ipv6,
            port,
            prefix,
            timeout,
            no_token,
            no_browser,
            no_settings,
            tunnel_url,
            tunnel_token,
            tunnel_drive,
            tunnel_public,
        } => {
            let addr = resolve_listen_addr(host, ipv4, ipv6, port)?;
            let prefix = chan_server::sanitize_prefix(prefix.as_deref().unwrap_or(""))
                .map_err(|e| anyhow::anyhow!("invalid --prefix: {e}"))?;
            // serve is the only async subcommand; everything else
            // stays sync so the CLI starts up without a runtime.
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .context("building tokio runtime")?;
            let res = rt.block_on(cmd_serve(
                addr,
                prefix,
                timeout,
                path,
                here,
                no_token,
                no_browser,
                no_settings,
                tunnel_url,
                tunnel_token,
                tunnel_drive,
                tunnel_public,
            ));
            // Don't block on blocking-pool tasks (e.g. an in-flight
            // initial reindex on a large drive): chan-drive's reindex
            // is uncancellable today, so a normal Runtime drop would
            // wait for it after Ctrl-C. shutdown_background detaches
            // the pool so the process can exit; the index may be left
            // partially populated until the next rebuild.
            rt.shutdown_background();
            res
        }
        Command::Index { path } => cmd_index(path),
        Command::Search { path, query, limit } => cmd_search(path, query, limit),
        Command::Upgrade {
            yes,
            check,
            version,
        } => {
            // The upgrader uses reqwest + tokio internally; reuse the
            // same runtime shape as cmd_serve so we stay async without
            // forcing a sync HTTP dep.
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .context("building tokio runtime")?;
            rt.block_on(update::run_upgrade(update::UpgradeOptions {
                assume_yes: yes,
                check_only: check,
                version_override: version,
                verbose: cli.verbose > 0,
            }))
        }
        Command::Mcp { path, auto_apply } => {
            // Same shape as serve: stdio MCP needs a tokio runtime
            // for the async server, but everything outside it stays
            // sync.
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .context("building tokio runtime")?;
            rt.block_on(cmd_mcp(path, auto_apply))
        }
        Command::McpProxy { socket } => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .context("building tokio runtime")?;
            rt.block_on(cmd_mcp_proxy(socket))
        }
        Command::Contacts { action } => match action {
            ContactsAction::Import { source } => match source {
                ImportSource::Csv {
                    file,
                    into,
                    provider,
                    dry_run,
                    overwrite,
                    drive,
                } => cmd_contacts_import_csv(file, into, provider, dry_run, overwrite, drive),
            },
        },
    }
}

fn init_tracing(verbosity: u8) {
    let level = match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(level)),
        )
        .with_writer(std::io::stderr)
        .init();
}

fn library() -> Result<Library> {
    Library::open().context("opening chan registry")
}

/// Resolve the display name to register for `root`. Behavior:
///
///   - When the caller passed an explicit name (non-empty), use it
///     verbatim. The user's choice always wins.
///   - When the drive is already registered with a non-empty name,
///     keep it. Re-registration is a no-op for the name field.
///   - Otherwise default to the directory's basename. If that
///     basename collides with another already-registered drive's
///     name, prompt the user on a TTY for an alternative; on a
///     non-TTY (chan-app embedding, scripts, CI) fall back to a
///     `<basename> (<parent-dir>)` disambiguator so the registry
///     stays unambiguous without blocking startup.
fn resolve_drive_name(lib: &Library, root: &Path, requested: Option<String>) -> Result<String> {
    if let Some(n) = requested
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        return Ok(n.to_string());
    }
    let drives = lib.list_drives();
    if let Some(existing) = drives
        .iter()
        .find(|d| same_path(&d.path, root))
        .and_then(|d| d.name.as_deref())
        .filter(|s| !s.is_empty())
    {
        return Ok(existing.to_string());
    }
    let basename = root
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "drive".to_string());
    let conflict = drives
        .iter()
        .any(|d| !same_path(&d.path, root) && d.name.as_deref() == Some(basename.as_str()));
    if !conflict {
        return Ok(basename);
    }
    let auto = disambiguate_name(&basename, root);
    if std::io::stdin().is_terminal() && std::io::stderr().is_terminal() {
        Ok(prompt_drive_name(&basename, root, &auto))
    } else {
        eprintln!(
            "chan: drive name '{basename}' already in use; auto-naming as '{auto}'. \
             Rename later with `chan rename {} <name>`.",
            root.display(),
        );
        Ok(auto)
    }
}

/// "Notes (Documents)" style disambiguator: append the immediate
/// parent directory name in parens. Stable per path so re-running
/// chan against the same drive lands the same name.
fn disambiguate_name(basename: &str, root: &Path) -> String {
    let parent = root
        .parent()
        .and_then(|p| p.file_name())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if parent.is_empty() {
        basename.to_string()
    } else {
        format!("{basename} ({parent})")
    }
}

fn prompt_drive_name(basename: &str, root: &Path, default: &str) -> String {
    eprintln!("Drive name '{basename}' is already used by another drive in the registry.");
    eprintln!("Path: {}", root.display());
    eprint!("Pick a different name (or press Enter for '{default}'): ");
    let _ = std::io::stderr().flush();
    let mut buf = String::new();
    let _ = std::io::stdin().read_line(&mut buf);
    let trimmed = buf.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    let ca = a.canonicalize().unwrap_or_else(|_| a.to_path_buf());
    let cb = b.canonicalize().unwrap_or_else(|_| b.to_path_buf());
    ca == cb
}

/// Register the drive AND make sure it ends up with a non-empty
/// display name. `register_drive` only sets the name on first
/// insert (chan-drive's "never clobber a user-set name" policy),
/// so a previously-unnamed entry stays unnamed on subsequent
/// `chan serve` calls. We backfill via `rename_drive` so users
/// who already had a drive registered before the auto-name change
/// still see a real name in the file browser without typing
/// `chan rename` first.
/// Pick the URL-safe drive name to publish under
/// `{user}.drive.chan.app/<name>`. The registry display name
/// (used in the file browser, logs, etc.) and the wire name
/// are decoupled: the display name can be "My Notes", but the
/// tunnel name has to satisfy `is_valid_drive_name`.
///
/// - With `--tunnel-drive`: validate it; bail with a clear
///   message + a suggested sanitized form if rejected.
/// - Without: take the registry name (or basename), sanitize.
///   Warn when sanitize altered it. Bail when sanitize yields
///   `None` (the path collapses to all punctuation).
fn resolve_tunnel_drive_name(
    flag: Option<String>,
    registry_name: Option<&str>,
    root: &Path,
) -> Result<String> {
    if let Some(name) = flag {
        if chan_server::tunnel::is_valid_drive_name(&name) {
            return Ok(name);
        }
        let suggestion = chan_server::tunnel::sanitize_drive_name(&name);
        let max = chan_server::tunnel::MAX_DRIVE_NAME_LEN;
        let hint = match suggestion {
            Some(s) => format!(" Try --tunnel-drive={s}."),
            None => String::new(),
        };
        anyhow::bail!(
            "--tunnel-drive {name:?} is not URL-safe (need [a-z0-9-], 1-{max} chars, no leading/trailing hyphen).{hint}"
        );
    }
    let source = registry_name
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            root.file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
    if chan_server::tunnel::is_valid_drive_name(&source) {
        return Ok(source);
    }
    match chan_server::tunnel::sanitize_drive_name(&source) {
        Some(sanitized) => {
            eprintln!(
                "NOTE: drive name {source:?} sanitized to {sanitized:?} for the tunnel URL. \
                 Pass --tunnel-drive to override."
            );
            Ok(sanitized)
        }
        None => {
            let max = chan_server::tunnel::MAX_DRIVE_NAME_LEN;
            anyhow::bail!(
                "cannot derive a URL-safe tunnel drive name from {source:?}. \
                 Pass --tunnel-drive=<name> ([a-z0-9-], 1-{max} chars, no leading/trailing hyphen)."
            );
        }
    }
}

fn ensure_drive_named(
    lib: &Library,
    root: &Path,
    requested: Option<String>,
) -> Result<chan_drive::KnownDrive> {
    let resolved = resolve_drive_name(lib, root, requested)?;
    let entry = lib
        .register_drive(root, Some(resolved.clone()))
        .with_context(|| format!("registering {}", root.display()))?;
    if entry.name.as_deref().unwrap_or("").is_empty() {
        lib.rename_drive(root, Some(resolved.clone()))
            .with_context(|| format!("renaming {}", root.display()))?;
        // KnownDrive carries private fields (canonical_path) so we
        // can't struct-update the prior entry. register_drive is
        // idempotent: re-call it to pick up the new name plus
        // whatever else the registry recomputed.
        return lib
            .register_drive(root, Some(resolved))
            .with_context(|| format!("re-registering {}", root.display()));
    }
    Ok(entry)
}

fn cmd_add(path: PathBuf, name: Option<String>) -> Result<()> {
    // Mirror `chan serve`'s behavior: create the directory if it
    // doesn't exist yet. Single verb covers both "register an
    // existing dir" and "make a fresh drive here". A separate
    // `chan init` would be a synonym; not worth the mental
    // overhead.
    if !path.exists() {
        std::fs::create_dir_all(&path)
            .with_context(|| format!("creating drive root {}", path.display()))?;
    }
    let lib = library()?;
    let entry = ensure_drive_named(&lib, &path, name)?;
    println!(
        "registered: {} ({})",
        entry.path.display(),
        entry.name.as_deref().unwrap_or("unnamed"),
    );
    Ok(())
}

fn cmd_list() -> Result<()> {
    let drives = library()?.list_drives();
    if drives.is_empty() {
        println!("(no drives registered)");
        return Ok(());
    }
    for d in drives {
        let name = d.name.as_deref().unwrap_or("unnamed");
        println!(
            "{:<24} {}  (last opened {})",
            name,
            d.path.display(),
            d.last_opened.format("%Y-%m-%d %H:%M"),
        );
    }
    Ok(())
}

fn cmd_remove(path: PathBuf) -> Result<()> {
    let removed = library()?
        .unregister_drive(&path)
        .with_context(|| format!("unregistering {}", path.display()))?;
    if removed {
        println!("unregistered: {}", path.display());
    } else {
        println!("(not registered: {})", path.display());
    }
    Ok(())
}

fn cmd_rename(path: PathBuf, name: String) -> Result<()> {
    let new_name = if name.is_empty() { None } else { Some(name) };
    let ok = library()?
        .rename_drive(&path, new_name.clone())
        .with_context(|| format!("renaming {}", path.display()))?;
    if ok {
        println!(
            "renamed: {} ({})",
            path.display(),
            new_name.as_deref().unwrap_or("unnamed"),
        );
    } else {
        println!("(not registered: {})", path.display());
    }
    Ok(())
}

/// Parse a `--timeout` value: an unsigned integer plus a `s` / `m`
/// / `h` suffix. Reject zero so a typo doesn't get the server killed
/// on the first activity check. We deliberately don't pull the
/// `humantime` crate for this; the accepted shapes are the only ones
/// that matter for systemd service files (`OnInactiveSec=` style).
fn parse_idle_timeout(s: &str) -> Result<Duration, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty timeout".into());
    }
    let (num, unit) = match s.as_bytes().last() {
        Some(b's' | b'm' | b'h') => s.split_at(s.len() - 1),
        _ => return Err(format!("expected suffix s|m|h, got {s:?}")),
    };
    let n: u64 = num
        .parse()
        .map_err(|e| format!("invalid timeout number {num:?}: {e}"))?;
    if n == 0 {
        return Err("timeout must be > 0".into());
    }
    Ok(match unit {
        "s" => Duration::from_secs(n),
        "m" => Duration::from_secs(n * 60),
        "h" => Duration::from_secs(n * 60 * 60),
        _ => unreachable!("suffix already validated"),
    })
}

/// Resolve final listen address from the user's flags.
///
/// `--host` is authoritative when given; `-4` / `-6` only validate
/// its family. With no `--host`, `-4` selects 127.0.0.1, `-6` selects
/// ::1, and neither selects 127.0.0.1 (the historical default).
fn resolve_listen_addr(
    host: Option<IpAddr>,
    ipv4: bool,
    ipv6: bool,
    port: u16,
) -> Result<SocketAddr> {
    let ip = match host {
        Some(ip) => {
            if ipv4 && !ip.is_ipv4() {
                anyhow::bail!("-4 requires an IPv4 --host, got {ip}");
            }
            if ipv6 && !ip.is_ipv6() {
                anyhow::bail!("-6 requires an IPv6 --host, got {ip}");
            }
            ip
        }
        None if ipv6 => IpAddr::V6(Ipv6Addr::LOCALHOST),
        None => IpAddr::V4(Ipv4Addr::LOCALHOST),
    };
    Ok(SocketAddr::new(ip, port))
}

/// Emit the structured `vcs-parent` refusal to stderr. The shape is
/// a contract consumed by chan-desktop (and any other wrapping
/// shell):
///
///   - Exit code `70` (set by the caller after this returns).
///   - One stderr line begins with `chan-error: vcs-parent ` and
///     carries `kind=<git|hg|svn> repo_root=<abs path> path=<abs
///     path>` in that order, single-line, space-separated. Values
///     run to end-of-line so paths with spaces don't break the
///     parse; wrappers split on `key=` boundaries, not on spaces.
///   - The surrounding human-readable lines are advisory and may
///     change wording; the marker is the stable bit.
///
/// Documented in the desktop hand-off; do NOT reshape without
/// bumping the marker prefix (e.g. `chan-error-v2: ...`) so old
/// shells fail closed instead of silently misparsing.
fn print_vcs_parent_error(root: &Path, parent: &chan_drive::VcsParent) {
    // Canonicalize both paths for the marker so wrappers get
    // absolute, symlink-resolved forms. Fall back to the input
    // when canonicalize fails (root may not yet exist on disk).
    let root_abs = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let repo_abs = std::fs::canonicalize(&parent.repo_root)
        .unwrap_or_else(|_| parent.repo_root.clone());
    let kind_human = match parent.kind {
        chan_drive::VcsKind::Git => "Git",
        chan_drive::VcsKind::Mercurial => "Mercurial",
        chan_drive::VcsKind::Subversion => "Subversion",
    };
    eprintln!(
        "error: drive '{}' is inside a {} repository at '{}'.",
        root_abs.display(),
        kind_human,
        repo_abs.display(),
    );
    eprintln!("       Serving the repository root keeps cross-file links, the graph,");
    eprintln!("       and search aligned with the project boundary.");
    eprintln!(
        "chan-error: vcs-parent kind={} repo_root={} path={}",
        parent.kind.as_str(),
        repo_abs.display(),
        root_abs.display(),
    );
    eprintln!(
        "hint: serve repo root:    chan serve {}",
        repo_abs.display()
    );
    eprintln!(
        "hint: serve only subdir:  chan serve --here {}",
        root_abs.display(),
    );
}

#[allow(clippy::too_many_arguments)]
async fn cmd_serve(
    addr: SocketAddr,
    prefix: String,
    idle_timeout: Option<Duration>,
    path: Option<PathBuf>,
    here: bool,
    no_token: bool,
    no_browser: bool,
    no_settings: bool,
    tunnel_url: String,
    tunnel_token: Option<String>,
    tunnel_drive: Option<String>,
    tunnel_public: bool,
) -> Result<()> {
    let lib = library()?;
    // Resolve the drive root: explicit arg first, then the registry
    // default, then the platform default. Auto-register so users
    // can `chan serve /some/dir` without a prior `chan add`.
    let root = path
        .or_else(|| lib.default_drive_root())
        .unwrap_or_else(|| lib.effective_default_drive_root());
    // VCS-parent gate. If `root` is inside a Git / Mercurial /
    // Subversion working tree, refuse with a structured error so a
    // wrapping shell (chan-desktop) can parse the marker line and
    // offer the user a choice between repo root and the subdir.
    // Runs before any state mutation: no directory creation, no
    // registry write. `--here` opts the caller out for the case
    // where serving the subdir is the genuine intent.
    if !here {
        if let Some(parent) = chan_drive::detect_parent_vcs(&root) {
            print_vcs_parent_error(&root, &parent);
            std::process::exit(70);
        }
    }
    if !root.exists() {
        std::fs::create_dir_all(&root)
            .with_context(|| format!("creating drive root {}", root.display()))?;
    }
    let known = ensure_drive_named(&lib, &root, None)?;
    let drive = lib.open_drive(&root)?;

    // Best-effort update notice. The banner reads cached state
    // (no network) so an air-gapped host pays zero startup cost.
    // The probe runs as a detached tokio task with short timeouts;
    // its failures are swallowed at `debug` level. Honors
    // CHAN_UPDATE_CHECK=0 and the standard *_PROXY env vars
    // (reqwest reads them automatically).
    update::maybe_print_banner();
    tokio::spawn(update::run_probe());

    if let Some(token) = tunnel_token {
        // Warn when the token came in via the flag rather than the
        // env var (clap doesn't expose the source, so compare to env
        // directly). The flag value is in `ps` output until the
        // process exits; the env var is not.
        if std::env::var("CHAN_TUNNEL_TOKEN").ok().as_deref() != Some(token.as_str()) {
            eprintln!(
                "WARNING: --tunnel-token is visible in `ps` output. \
                 Prefer CHAN_TUNNEL_TOKEN env var instead."
            );
        }
        let drive_name = resolve_tunnel_drive_name(tunnel_drive, known.name.as_deref(), &root)?;
        if tunnel_public {
            eprintln!(
                "WARNING: --public exposes this drive at \
                 drive.chan.app/<user>/{drive_name} with no auth gate. \
                 Anyone with the URL has read/write access."
            );
        }
        return chan_server::serve_via_tunnel(
            lib,
            drive,
            &tunnel_url,
            token,
            drive_name,
            tunnel_public,
            !no_browser,
        )
        .await
        .context("running tunnel client");
    }

    // Loud warning: the auth model assumes loopback. No TLS, only a
    // bearer token. Binding off-loopback exposes the drive in the
    // clear to anyone on that network, including unauthenticated
    // probes if --no-token is also set.
    let host = addr.ip();
    if !host.is_loopback() {
        eprintln!(
            "WARNING: binding to {host} exposes chan on a non-loopback \
             interface. There is no TLS; the bearer token is sent in \
             plaintext. Do not use this on an untrusted network."
        );
        if no_token {
            eprintln!(
                "WARNING: --no-token + non-loopback host = open read/write \
                 access to your drive for anyone who can reach this port."
            );
        }
    }

    if no_settings {
        eprintln!("chan: --no-settings is set; the SPA will grey the cog and all settings-write routes will refuse with 403.");
    }
    let config = ServeConfig {
        addr,
        no_token,
        prefix,
        idle_timeout,
        // Default: open the browser on bind. --no-browser opts out
        // (desktop shells that host the UI in their own window,
        // headless / scripted invocations). Honored in both local
        // and tunnel mode.
        open_browser: !no_browser,
        // Local serve trusts the operator by default; --no-settings
        // opts into the same UI grey + server 403 that tunnel mode
        // gets, for kiosk / shared-workstation deployments. The
        // public-tunnel redactions on GETs are kept tunnel-only:
        // a local operator on the same machine has nothing to hide
        // from themselves.
        settings_disabled: no_settings,
        tunnel_public: false,
    };
    chan_server::serve(lib, drive, config)
        .await
        .context("running server")
}

fn cmd_index(path: PathBuf) -> Result<()> {
    let lib = library()?;
    // Idempotent: registering an already-known drive only touches
    // last_opened. CLI users expect `chan index /some/path` to work
    // without a prior `chan add`. First-touch defaults the name to
    // the directory's basename (or prompts on conflict) so the
    // file browser doesn't show "(unnamed)" later.
    ensure_drive_named(&lib, &path, None)?;
    let drive = lib.open_drive(&path)?;

    // Live progress on stderr so the user can see the embed pass
    // is making progress; on a big drive it can run for tens of
    // minutes. Use a TTY-friendly carriage return rewrite when
    // stderr is interactive; fall back to plain lines (one per
    // file) when redirected so logs stay readable.
    use std::io::{IsTerminal, Write};
    let tty = std::io::stderr().is_terminal();
    // chan-drive 0.7 reshaped progress: a single `ProgressEvent` with
    // a `stage` enum (IndexFile / EmbedBatch / GraphRebuild / ...),
    // current/total counters, and an optional label. We surface the
    // two stages the reindex CLI cared about; everything else folds
    // into a generic "still working" line so nothing escapes the user
    // silently on large drives.
    let callback = chan_drive::progress::progress_fn(move |p| {
        let line = match p.stage {
            chan_drive::progress::ProgressStage::IndexFile => format!(
                "[{}/{}] {}",
                p.current.saturating_add(1),
                p.total,
                p.label.as_deref().unwrap_or("")
            ),
            chan_drive::progress::ProgressStage::EmbedBatch => format!(
                "[{}/{}] embedding {} chunks...",
                p.current.saturating_add(1),
                p.total,
                p.current
            ),
            other => format!("{other:?} {}", p.label.as_deref().unwrap_or("")),
        };
        if tty {
            let mut err = std::io::stderr().lock();
            let _ = write!(err, "\r\x1b[2K{line}");
            let _ = err.flush();
        } else {
            eprintln!("{line}");
        }
    });
    let summary = drive
        .reindex_with(None, callback.as_ref())
        .context("reindex")?;
    if tty {
        eprintln!();
    }

    println!(
        "indexed {}/{} files, {} chunks ({} errors)",
        summary.indexed,
        summary.files,
        summary.chunks,
        summary.errors.len(),
    );
    for (path, e) in &summary.errors {
        eprintln!("  error: {path}: {e}");
    }
    Ok(())
}

/// Run chan-llm's MCP server on stdio against `path`. Spawned by
/// the ClaudeCli backend through `--mcp-config`; not user-facing.
///
/// We deliberately do NOT auto-register the drive here: the host
/// (chan-server) has already gone through `ensure_drive_named` for
/// this drive when the session started, and the MCP subprocess
/// inherits that registry. If the drive isn't registered when the
/// agent invokes the subcommand, that's a wiring bug worth
/// surfacing rather than silently fixing.
async fn cmd_mcp(path: PathBuf, auto_apply: bool) -> Result<()> {
    let drive = library()?
        .open_drive(&path)
        .with_context(|| format!("opening drive {}", path.display()))?;
    chan_llm::mcp::Server::new(drive, auto_apply)
        .serve_stdio()
        .await
        .context("running MCP server")
}

/// Bridge between the agent subprocess and the MCP server hosted in
/// chan-server. Connects to the Unix-domain socket and pipes
/// stdin -> socket and socket -> stdout concurrently. Returns when
/// either direction closes, which is the normal end of a session.
#[cfg(unix)]
async fn cmd_mcp_proxy(socket: PathBuf) -> Result<()> {
    use tokio::io::{stdin, stdout};
    use tokio::net::UnixStream;
    let stream = UnixStream::connect(&socket)
        .await
        .with_context(|| format!("connecting to mcp socket {}", socket.display()))?;
    let (mut read_sock, mut write_sock) = stream.into_split();
    let mut stdin = stdin();
    let mut stdout = stdout();
    // Two simultaneous copies; the first to finish ends the session.
    // tokio::io::copy_bidirectional doesn't fit here because stdin /
    // stdout aren't a single duplex stream.
    let to_socket = tokio::io::copy(&mut stdin, &mut write_sock);
    let from_socket = tokio::io::copy(&mut read_sock, &mut stdout);
    tokio::select! {
        r = to_socket => { r.context("piping stdin to mcp socket")?; }
        r = from_socket => { r.context("piping mcp socket to stdout")?; }
    }
    Ok(())
}

/// Windows stub: chan's MCP bridge runs over Unix-domain sockets; the
/// proxy subcommand has no counterpart on Windows. The CLI still
/// accepts `__mcp-proxy` so flag-parsing stays target-agnostic, but
/// invoking it fails fast instead of half-working.
#[cfg(not(unix))]
async fn cmd_mcp_proxy(_socket: PathBuf) -> Result<()> {
    anyhow::bail!("__mcp-proxy is unix-only");
}

fn cmd_search(path: PathBuf, query: String, limit: u32) -> Result<()> {
    let lib = library()?;
    ensure_drive_named(&lib, &path, None)?;
    let drive = lib.open_drive(&path)?;
    let opts = SearchOpts {
        limit,
        ..Default::default()
    };
    let res = drive.search(&query, &opts).context("search")?;
    if res.hits.is_empty() {
        println!("(no hits)");
        return Ok(());
    }
    for hit in res.hits {
        let where_ = if hit.heading.is_empty() {
            hit.path.clone()
        } else {
            format!("{}#{}", hit.path, hit.heading)
        };
        println!("{:<6.3}  {}", hit.score, where_);
        let first = hit.snippet.lines().next().unwrap_or("");
        if !first.is_empty() {
            println!("        {first}");
        }
    }
    Ok(())
}

fn cmd_contacts_import_csv(
    file: PathBuf,
    into: String,
    provider: String,
    dry_run: bool,
    overwrite: bool,
    drive: Option<PathBuf>,
) -> Result<()> {
    use chan_drive::contacts::{
        google::parse_google_csv, slug::slug_for, ImportOpts, ProviderKind,
    };
    use std::collections::HashSet;

    // Provider gate. Only Google CSV today; the flag exists so the
    // help text and the wire shape are stable when more land.
    let prov =
        ProviderKind::parse(&provider).with_context(|| format!("unknown provider: {provider}"))?;
    if prov != ProviderKind::Google {
        anyhow::bail!("only --provider google is supported today");
    }

    // Parse the CSV up front. A bad file should fail before we
    // touch the drive, so the user doesn't end up with a half-
    // created Contacts/ dir on a typo.
    let csv_bytes = std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
    let contacts = parse_google_csv(csv_bytes.as_slice())
        .with_context(|| format!("parsing {}", file.display()))?;
    if contacts.is_empty() {
        println!("(no contacts in {})", file.display());
        return Ok(());
    }

    let lib = library()?;
    let root = drive
        .or_else(|| lib.default_drive_root())
        .unwrap_or_else(|| lib.effective_default_drive_root());
    if !root.exists() {
        std::fs::create_dir_all(&root)
            .with_context(|| format!("creating drive root {}", root.display()))?;
    }
    ensure_drive_named(&lib, &root, None)?;
    let drive = lib.open_drive(&root)?;

    if dry_run {
        // Mirror the orchestrator's slug + existence check loop
        // without writing. Existence checks against the live drive
        // so SKIPPED / OVERWROTE labels are accurate.
        let mut taken: HashSet<String> = HashSet::new();
        let mut unnamed = 0usize;
        let dir_norm = into.trim_matches('/').to_string();
        let mut wrote = 0usize;
        let mut overwrote = 0usize;
        let mut skipped = 0usize;
        let on_disk = |p: &str| drive.exists(p);
        for c in &contacts {
            let path = slug_for(c, &dir_norm, &mut taken, &mut unnamed, &on_disk);
            let exists = drive.exists(&path);
            if exists && !overwrite {
                println!("WOULD SKIP      {path}  (exists)");
                skipped += 1;
            } else if exists {
                println!("WOULD OVERWRITE {path}");
                overwrote += 1;
            } else {
                println!("WOULD WRITE     {path}");
                wrote += 1;
            }
        }
        println!();
        println!(
            "{wrote} would write, {overwrote} would overwrite, \
             {skipped} would skip (dry-run; no files changed)"
        );
        return Ok(());
    }

    let summary = drive
        .import_contacts(&into, contacts, ImportOpts { overwrite })
        .context("importing contacts")?;
    print_import_summary(&summary);
    Ok(())
}

fn print_import_summary(summary: &chan_drive::ImportSummary) {
    use chan_drive::ImportOutcome;
    for o in &summary.outcomes {
        match o {
            ImportOutcome::Wrote { path } => println!("WROTE     {path}"),
            ImportOutcome::Overwrote { path } => println!("OVERWROTE {path}"),
            ImportOutcome::Skipped { path, reason } => {
                println!("SKIPPED   {path}  ({reason})")
            }
            ImportOutcome::Failed { name, reason } => {
                println!("FAILED    {name}  ({reason})")
            }
        }
    }
    let c = summary.counts();
    println!();
    println!(
        "{} wrote, {} overwrote, {} skipped, {} failed",
        c.wrote, c.overwrote, c.skipped, c.failed
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ipv4(s: &str) -> IpAddr {
        s.parse().unwrap()
    }
    fn ipv6(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn default_is_v4_loopback() {
        let addr = resolve_listen_addr(None, false, false, 8787).unwrap();
        assert_eq!(addr, SocketAddr::new(ipv4("127.0.0.1"), 8787));
    }

    #[test]
    fn ipv4_flag_with_no_host_gives_v4_loopback() {
        let addr = resolve_listen_addr(None, true, false, 8787).unwrap();
        assert_eq!(addr, SocketAddr::new(ipv4("127.0.0.1"), 8787));
    }

    #[test]
    fn ipv6_flag_with_no_host_gives_v6_loopback() {
        let addr = resolve_listen_addr(None, false, true, 8787).unwrap();
        assert_eq!(addr, SocketAddr::new(ipv6("::1"), 8787));
    }

    #[test]
    fn explicit_host_overrides_default() {
        let addr = resolve_listen_addr(Some(ipv4("0.0.0.0")), false, false, 9000).unwrap();
        assert_eq!(addr, SocketAddr::new(ipv4("0.0.0.0"), 9000));
    }

    #[test]
    fn ipv4_flag_rejects_v6_host() {
        let err = resolve_listen_addr(Some(ipv6("::1")), true, false, 8787).unwrap_err();
        assert!(err.to_string().contains("-4"));
    }

    #[test]
    fn ipv6_flag_rejects_v4_host() {
        let err = resolve_listen_addr(Some(ipv4("127.0.0.1")), false, true, 8787).unwrap_err();
        assert!(err.to_string().contains("-6"));
    }

    #[test]
    fn ipv4_flag_accepts_matching_v4_host() {
        let addr = resolve_listen_addr(Some(ipv4("0.0.0.0")), true, false, 8787).unwrap();
        assert_eq!(addr, SocketAddr::new(ipv4("0.0.0.0"), 8787));
    }

    #[test]
    fn ipv6_flag_accepts_matching_v6_host() {
        let addr = resolve_listen_addr(Some(ipv6("::")), false, true, 8787).unwrap();
        assert_eq!(addr, SocketAddr::new(ipv6("::"), 8787));
    }

    #[test]
    fn parse_idle_timeout_units() {
        assert_eq!(parse_idle_timeout("30s").unwrap(), Duration::from_secs(30));
        assert_eq!(parse_idle_timeout("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_idle_timeout("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(
            parse_idle_timeout("  10s  ").unwrap(),
            Duration::from_secs(10)
        );
    }

    #[test]
    fn parse_idle_timeout_rejects_bad_inputs() {
        assert!(parse_idle_timeout("").is_err());
        assert!(parse_idle_timeout("0s").is_err());
        assert!(parse_idle_timeout("0m").is_err());
        assert!(parse_idle_timeout("10").is_err()); // no unit
        assert!(parse_idle_timeout("10x").is_err()); // bad unit
        assert!(parse_idle_timeout("-5s").is_err()); // negative
        assert!(parse_idle_timeout("five s").is_err());
        assert!(parse_idle_timeout("1.5m").is_err()); // no fractional
    }

    #[test]
    fn tunnel_drive_flag_passes_through_when_valid() {
        let root = PathBuf::from("/tmp/whatever");
        let out = resolve_tunnel_drive_name(Some("notes".into()), Some("My Notes"), &root).unwrap();
        assert_eq!(out, "notes");
    }

    #[test]
    fn tunnel_drive_flag_rejected_with_suggestion() {
        let root = PathBuf::from("/tmp/whatever");
        let err = resolve_tunnel_drive_name(Some("My Drive!".into()), Some("My Notes"), &root)
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not URL-safe"), "{msg}");
        assert!(msg.contains("--tunnel-drive=my-drive"), "{msg}");
    }

    #[test]
    fn tunnel_drive_flag_rejected_when_unsanitizable() {
        let root = PathBuf::from("/tmp/whatever");
        let err = resolve_tunnel_drive_name(Some("---".into()), Some("notes"), &root).unwrap_err();
        assert!(err.to_string().contains("not URL-safe"));
    }

    #[test]
    fn tunnel_drive_default_uses_registry_name_as_is_when_valid() {
        let root = PathBuf::from("/tmp/whatever");
        let out = resolve_tunnel_drive_name(None, Some("notes"), &root).unwrap();
        assert_eq!(out, "notes");
    }

    #[test]
    fn tunnel_drive_default_sanitizes_registry_name() {
        let root = PathBuf::from("/tmp/whatever");
        let out = resolve_tunnel_drive_name(None, Some("My Notes"), &root).unwrap();
        assert_eq!(out, "my-notes");
    }

    #[test]
    fn tunnel_drive_default_falls_back_to_basename_when_no_registry_name() {
        let root = PathBuf::from("/tmp/Daily Journal");
        let out = resolve_tunnel_drive_name(None, None, &root).unwrap();
        assert_eq!(out, "daily-journal");
    }

    #[test]
    fn tunnel_drive_default_bails_when_basename_collapses() {
        let root = PathBuf::from("/tmp/---");
        let err = resolve_tunnel_drive_name(None, None, &root).unwrap_err();
        assert!(err.to_string().contains("cannot derive"));
    }
}
