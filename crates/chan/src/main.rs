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
//   chan serve [--host H --port N]  run the HTTP server. Defaults
//                                   to 127.0.0.1 (loopback only);
//                                   the embedded web editor talks
//                                   to this. NOT IMPLEMENTED YET;
//                                   routes port in follow-up
//                                   commits.
//   chan index <path>               rebuild the search index +
//                                   graph for the drive
//   chan search <path> <query>      query the BM25 index
//
// Anything that touches the registry / drive contents goes through
// `chan_core::Library` and `chan_core::Drive` so the library's
// invariants (atomic writes, path sandbox, special-file refusal,
// cross-process writer lock) apply uniformly.

use std::io::{IsTerminal, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chan_core::{Library, SearchOpts};
use chan_server::ServeConfig;
use clap::{Parser, Subcommand};

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
    ///
    /// NOT IMPLEMENTED YET. Routes are being ported from the old
    /// chan-core in follow-up commits.
    Serve {
        path: Option<PathBuf>,
        /// Host address to bind. Default 127.0.0.1 (loopback). Use
        /// 0.0.0.0 to listen on all interfaces. chan has no TLS and
        /// only a bearer-token gate, so any non-loopback host
        /// exposes your drive in plaintext on that network.
        #[arg(long, default_value_t = IpAddr::V4(Ipv4Addr::LOCALHOST))]
        host: IpAddr,
        #[arg(long, default_value_t = 8787)]
        port: u16,
        /// Skip the per-launch bearer-token gate. For tests and the
        /// desktop shell only; never expose a no-token server on a
        /// shared machine.
        #[arg(long)]
        no_token: bool,
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
            host,
            port,
            no_token,
        } => {
            // serve is the only async subcommand; everything else
            // stays sync so the CLI starts up without a runtime.
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .context("building tokio runtime")?;
            rt.block_on(cmd_serve(host, port, path, no_token))
        }
        Command::Index { path } => cmd_index(path),
        Command::Search { path, query, limit } => cmd_search(path, query, limit),
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
/// insert (chan-core's "never clobber a user-set name" policy),
/// so a previously-unnamed entry stays unnamed on subsequent
/// `chan serve` calls. We backfill via `rename_drive` so users
/// who already had a drive registered before the auto-name change
/// still see a real name in the file browser without typing
/// `chan rename` first.
fn ensure_drive_named(
    lib: &Library,
    root: &Path,
    requested: Option<String>,
) -> Result<chan_core::KnownDrive> {
    let resolved = resolve_drive_name(lib, root, requested)?;
    let entry = lib
        .register_drive(root, Some(resolved.clone()))
        .with_context(|| format!("registering {}", root.display()))?;
    if entry.name.as_deref().unwrap_or("").is_empty() {
        lib.rename_drive(root, Some(resolved.clone()))
            .with_context(|| format!("renaming {}", root.display()))?;
        // rename_drive returned ok; reflect the new name in the
        // returned struct without a re-fetch round-trip.
        return Ok(chan_core::KnownDrive {
            name: Some(resolved),
            ..entry
        });
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

async fn cmd_serve(host: IpAddr, port: u16, path: Option<PathBuf>, no_token: bool) -> Result<()> {
    let lib = library()?;
    // Resolve the drive root: explicit arg first, then the registry
    // default, then the platform default. Auto-register so users
    // can `chan serve /some/dir` without a prior `chan add`.
    let root = path
        .or_else(|| lib.default_drive_root())
        .unwrap_or_else(|| lib.effective_default_drive_root());
    if !root.exists() {
        std::fs::create_dir_all(&root)
            .with_context(|| format!("creating drive root {}", root.display()))?;
    }
    ensure_drive_named(&lib, &root, None)?;
    let drive = lib.open_drive(&root)?;

    // Loud warning: the auth model assumes loopback. No TLS, only a
    // bearer token. Binding off-loopback exposes the drive in the
    // clear to anyone on that network, including unauthenticated
    // probes if --no-token is also set.
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

    let addr = SocketAddr::new(host, port);
    let config = ServeConfig { addr, no_token };
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
    let stats = drive.reindex().context("reindex")?;
    println!(
        "indexed {} files (skipped {}) in {} ms",
        stats.files_indexed, stats.files_skipped, stats.elapsed_ms,
    );
    Ok(())
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
        println!("{:<6.3}  {}", hit.score, hit.path);
        if let Some(snippet) = hit.snippets.first() {
            println!("        {}", snippet.text.lines().next().unwrap_or(""));
        }
    }
    Ok(())
}
