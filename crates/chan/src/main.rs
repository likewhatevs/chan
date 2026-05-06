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
//   chan serve [--port N]           run the HTTP server bound to
//                                   127.0.0.1; the embedded web
//                                   editor talks to this. NOT
//                                   IMPLEMENTED YET; routes port
//                                   in follow-up commits.
//   chan index <path>               rebuild the search index +
//                                   graph for the drive
//   chan search <path> <query>      query the BM25 index
//
// Anything that touches the registry / drive contents goes through
// `chan_core::Library` and `chan_core::Drive` so the library's
// invariants (atomic writes, path sandbox, special-file refusal,
// cross-process writer lock) apply uniformly.

use std::net::SocketAddr;
use std::path::PathBuf;

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
    Rename {
        path: PathBuf,
        /// Pass `""` to clear the name.
        name: String,
    },
    /// Run the HTTP server bound to 127.0.0.1.
    ///
    /// NOT IMPLEMENTED YET. Routes are being ported from the old
    /// chan-core in follow-up commits.
    Serve {
        path: Option<PathBuf>,
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
            port,
            no_token,
        } => {
            // serve is the only async subcommand; everything else
            // stays sync so the CLI starts up without a runtime.
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .context("building tokio runtime")?;
            rt.block_on(cmd_serve(path, port, no_token))
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

fn cmd_add(path: PathBuf, name: Option<String>) -> Result<()> {
    let entry = library()?
        .register_drive(&path, name)
        .with_context(|| format!("registering {}", path.display()))?;
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

async fn cmd_serve(path: Option<PathBuf>, port: u16, no_token: bool) -> Result<()> {
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
    lib.register_drive(&root, None)
        .with_context(|| format!("registering {}", root.display()))?;
    let drive = lib.open_drive(&root)?;

    let addr: SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .context("parsing bind address")?;
    let config = ServeConfig { addr, no_token };
    chan_server::serve(drive, config)
        .await
        .context("running server")
}

fn cmd_index(path: PathBuf) -> Result<()> {
    let lib = library()?;
    // Idempotent: registering an already-known drive only touches
    // last_opened. CLI users expect `chan index /some/path` to work
    // without a prior `chan add`.
    lib.register_drive(&path, None)
        .with_context(|| format!("registering {}", path.display()))?;
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
    lib.register_drive(&path, None)
        .with_context(|| format!("registering {}", path.display()))?;
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
