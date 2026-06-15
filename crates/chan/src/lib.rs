// chan: notes app with embedded web editor.
//
// This library holds the whole `chan` CLI surface so two binaries can
// drive it: the standalone `chan` binary (`src/main.rs`, a thin shim
// calling `run(.., Personality::Standalone)`) and chan-desktop, which
// dispatches `chan` in-process when invoked through a `~/.local/bin/chan`
// shim (`Personality::Desktop`). The only behavioural fork between the two
// is the `Personality` passed to [`run`] — see `cmd_serve` (browser vs
// desktop handoff) and `chan upgrade` (CLI tarball replace vs desktop
// updater). Subcommands:
//
//   chan add <path>                 register a directory as a chan
//                                   workspace in ~/.chan/config.toml
//   chan list [--json]              list registered workspaces,
//                                   most-recent first. --json emits
//                                   a stable machine-readable shape.
//   chan remove <path>              drop a workspace from the registry
//                                   (filesystem contents untouched)
//   chan serve [-4|-6] [--host H --port N]
//                                   run the HTTP server. Defaults
//                                   to 127.0.0.1 (loopback only);
//                                   -6 picks ::1 instead. The
//                                   embedded web editor talks to
//                                   this.
//   chan index <path>               rebuild the search index +
//                                   graph for the workspace
//   chan search <path> <query>      query the BM25 index
//   chan graph <path>               inspect semantic or filesystem graph edges
//   chan status [path]              report workspace/index/graph health
//   chan config get [KEY]           print a preference value
//   chan config set KEY=VALUE       update a preference
//   chan metadata export PATH ARCHIVE.tar.zst
//                                   export chan metadata for a workspace
//   chan metadata import PATH ARCHIVE.tar.zst [--rescan]
//                                   import metadata with SCM guard
//   chan contacts import csv FILE --into DIR
//                                   import a Google Contacts CSV
//                                   as one markdown note per
//                                   contact under DIR (workspace-
//                                   relative). Notes carry
//                                   `chan.kind: contact`
//                                   frontmatter for graph + @
//                                   picker classification.
//
// Anything that touches the registry / workspace contents goes through
// `chan_workspace::Library` and `chan_workspace::Workspace` so the library's
// invariants (atomic writes, path sandbox, special-file refusal,
// cross-process writer lock) apply uniformly.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use chan_server::{
    build_fs_graph, EditorPrefs, EditorTheme, FsGraphResponse, FsGraphScope as ServerFsGraphScope,
    LineSpacing, ServeConfig, ServerConfig, ThemeChoice, TunnelServeConfig,
};
use chan_shell::ShellAction;
use chan_workspace::{
    EdgeKind, KnownWorkspace, Library, MetadataExportOptions, MetadataImportOptions,
    SearchAggression, SearchMode, SearchOpts,
};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use serde::Serialize;

mod update;

/// Extended description for `chan serve`. The keybindings list is
/// generated from `web/src/state/shortcuts.ts` (the single source of
/// truth for chan's chords). Resync after any change to that file
/// with `node web/scripts/shortcuts-table.mjs --serve-long-about`
/// and paste the output between the BEGIN/END markers below. The
/// native shell (chan-desktop) layers VS Code-shaped chords on top
/// of the browser set; those are documented in the same TS source.
const SERVE_LONG_ABOUT: &str = "\
Run the HTTP server. Defaults to 127.0.0.1 (loopback only).

In-app keybindings (Cmd = Ctrl on Linux / Windows):

  App
  ---
  Flip focused Hybrid                          Cmd+,
  Team Work                                    Cmd+Alt+P   (macOS web + native everywhere; all platforms via Mod+. p (Hybrid Nav))
  File browser                                 Cmd+Alt+O   (macOS web + native everywhere; all platforms via Mod+. o (Hybrid Nav))
  Graph                                        Cmd+Shift+M   (or Mod+. v (Hybrid Nav))
  New terminal                                 Cmd+Alt+T   (macOS web + native everywhere; all platforms via Mod+. t (Hybrid Nav))
  Reload window                                Cmd+R   (Ctrl+Shift+R on Linux / Windows)
  New draft                                    Cmd+N
  Lock screen                                  Cmd+. L
  Dismiss overlay                              Esc
  Search                                       Cmd+S
  Dashboard                                    Alt+Shift+D   (or Mod+. i (Hybrid Nav))
  
  File
  ----
  Delete file or directory                     Backspace
  
  Panes
  -----
  Enter Hybrid Nav                             Cmd+.
  Flip Hybrid                                  Cmd+. Tab
  Previous pane                                Alt+[
  Next pane                                    Alt+]
  Close all tabs in pane                       Cmd+. x
  Kill pane                                    Cmd+. Backspace
  Close empty pane                             Cmd+W   (empty panes only; otherwise the browser / window close fires)
  
  Tabs
  ----
  Close tab                                    Ctrl+D   (Cmd+W also closes the tab on native)
  Reopen closed tab                            Ctrl+Alt+T
  Next tab                                     Alt+Shift+]
  Previous tab                                 Alt+Shift+[
  Jump to tab N                                Ctrl+Alt+1..9
  
  Editor
  ------
  Show Source Code (toggle rendered/source)    Cmd+E
  Bold                                         Cmd+B
  Italic                                       Cmd+I
  
  Terminal
  --------
  Copy selection                               Cmd+C   (Ctrl+Shift+C on Linux / Windows)
  Paste                                        Cmd+V   (Ctrl+Shift+V on Linux / Windows)
  Show/Hide Rich Prompt                        Cmd+Shift+P   (Ctrl+Shift+P (desktop) / Alt+Shift+P (web) on Linux / Windows)
  Find in terminal                             Cmd+F
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
    /// Register a directory as a chan workspace.
    ///
    /// The baseline filesystem walk + markdown read + documentation
    /// graph + BM25 always runs. Semantic search is an optional
    /// layer, off by default to keep workspaces lean. chan-reports
    /// is on by default for new workspaces (`chan reports disable`
    /// turns it off).
    Add {
        path: PathBuf,
        /// Enable per-workspace semantic search (BGE-small
        /// dense vectors). Per-workspace footprint; needs the shared
        /// model (`chan index download-model`). Off by default.
        #[arg(long = "semantic-search")]
        semantic_search: bool,
        /// Force-enable per-workspace chan-reports (language
        /// detection + SLOC + COCOMO). Per-workspace footprint;
        /// maintained incrementally from filesystem events. Reports
        /// are already on by default for new workspaces; the flag
        /// persists the setting explicitly and runs the kickoff
        /// scan at add time.
        #[arg(long = "reports")]
        reports: bool,
    },
    /// List registered workspaces, most-recent first.
    List {
        /// Emit machine-readable JSON:
        /// `{"workspaces":[{path,metadata_key,last_seen_at},...]}`.
        /// `last_seen_at` is RFC3339 UTC. The text format is
        /// unchanged when this flag is omitted.
        #[arg(long)]
        json: bool,
    },
    /// Drive the current chan window from its terminal (the `cs` alias).
    ///
    /// Reached as `chan shell <action>` or, via a `cs -> chan` symlink
    /// the user puts on PATH, as `cs <action>`. Every action targets the
    /// chan window that spawned this terminal ($CHAN_WINDOW_ID +
    /// $CHAN_CONTROL_SOCKET); outside a chan terminal they error clearly.
    ///
    /// To enable the short `cs` name, symlink it onto your PATH once:
    ///   ln -s "$(command -v chan)" ~/.local/bin/cs
    /// chan ships no symlink; this is the only setup it needs.
    ///
    /// iproute2-style prefix matching: the cs actions disambiguate on
    /// their first letter, so `cs o` / `cs g` / `cs d` / `cs t` resolve
    /// to open / graph / dashboard / terminal.
    #[command(infer_subcommands = true)]
    Shell {
        #[command(subcommand)]
        action: ShellAction,
    },
    /// Generate shell completion scripts.
    Completions {
        /// Shell to generate completions for.
        shell: Shell,
    },
    /// Drop a workspace from the registry. Does not delete the
    /// directory or its content; only forgets it on this machine.
    ///
    Remove {
        #[arg(value_hint = clap::ValueHint::AnyPath)]
        path: PathBuf,
    },
    /// Run the HTTP server. Defaults to 127.0.0.1 (loopback only).
    #[command(long_about = SERVE_LONG_ABOUT)]
    Serve {
        path: Option<PathBuf>,
        /// Serve the given path verbatim instead of suggesting the
        /// enclosing VCS repository root. Without this flag, `chan
        /// serve` refuses to start when the workspace path lives inside
        /// a Git / Mercurial / Subversion working tree (exit 70 +
        /// `chan-error: vcs-parent` marker on stderr) because the
        /// repo root is almost always a better workspace root: it
        /// keeps cross-file links, the graph, and search aligned
        /// with the project boundary. Pass `--here` when you
        /// genuinely want to scope the workspace to a subdir.
        #[arg(long)]
        here: bool,
        /// Host address to bind. Default 127.0.0.1 (or ::1 with -6).
        /// Use 0.0.0.0 / :: to listen on all interfaces. chan has no
        /// TLS and only a bearer-token gate, so any non-loopback host
        /// exposes your workspace in plaintext on that network.
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
        /// (e.g. `workspace.example.com/{user}/`). Canonicalized to
        /// `/seg[/seg...]` with `[A-Za-z0-9-]+` segments; trailing
        /// slashes and `//` runs are tolerated. Anything else is
        /// rejected. Mutually exclusive with --tunnel-token (the
        /// public gateway already strips /{user}/{workspace}).
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
        /// Search indexer resource profile. Overrides
        /// `server.search.aggression` for this run.
        #[arg(long, value_parser = parse_search_aggression)]
        search_aggression: Option<SearchAggression>,
        /// Lock down the Settings panel: the SPA greys the cog and
        /// the server refuses every settings-write route with 403
        /// (PATCH /api/workspace, /api/config, /api/server/config,
        /// POST /api/storage/reset, POST /api/index/rebuild).
        /// Tunnel mode already implies
        /// this; the flag lets a local serve opt in for kiosk-style
        /// deployments (shared workstation, demo box) where the
        /// workspace owner is not the operator at the keyboard.
        #[arg(long)]
        no_settings: bool,
        /// Tunnel endpoint URL. With --tunnel-token, chan serve
        /// dials this instead of binding a local listener.
        #[arg(long, default_value = "https://workspace.chan.app/v1/tunnel")]
        tunnel_url: String,
        /// Personal access token (chan_pat_*) from id.chan.app.
        /// Setting this enables tunnel mode: chan serve does not
        /// bind a local TCP listener and instead publishes the
        /// workspace at {user}.workspace.chan.app/{workspace}/*. Prefer the
        /// CHAN_TUNNEL_TOKEN env var so the secret does not appear
        /// in `ps`.
        #[arg(long, env = "CHAN_TUNNEL_TOKEN")]
        tunnel_token: Option<String>,
        /// Workspace URL slug to publish at /{user}/<name>. Must be
        /// lowercase [a-z0-9-], 1-32 chars, no leading/trailing
        /// hyphen. Defaults to a sanitized form of the workspace path
        /// basename; chan emits a NOTE when it had to sanitize.
        #[arg(long)]
        tunnel_workspace_name: Option<String>,
        /// Expose the tunneled workspace without an OAuth gate. By
        /// default, `{user}.workspace.chan.app/{workspace}/` 404s anonymous
        /// visitors; the workspace owner opens it from id.chan.app's
        /// dashboard via a short-lived workspace-gate handoff. With
        /// --tunnel-public, anyone with the URL can reach the workspace
        /// over the same tunnel. Requires --tunnel-token (or
        /// `CHAN_TUNNEL_TOKEN`); clap rejects the flag otherwise so
        /// it can't silently no-op on a non-tunnel run.
        #[arg(long, requires = "tunnel_token")]
        tunnel_public: bool,
    },
    /// Rebuild the search index + graph; manage the embedding
    /// model + per-workspace Hybrid-search opt-in. Subcommand-driven
    /// (rather than a flat `chan index <path>`)
    /// so the model + semantic-toggle controls live alongside
    /// the rebuild action; mirrors `chan config <action>`.
    Index {
        #[command(subcommand)]
        action: IndexAction,
    },
    /// Enable/disable per-workspace chan-reports
    /// (language detection + SLOC + COCOMO). On by default for
    /// new workspaces; toggle here or via the pre-flight UI /
    /// Settings.
    Reports {
        #[command(subcommand)]
        action: ReportsAction,
    },
    /// Query the BM25 search index.
    Search {
        path: PathBuf,
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Query graph/index data for a workspace.
    ///
    /// --scope all reads the semantic markdown graph. --scope file/directory reads
    /// the filesystem graph used by the File Browser's "Graph this" action.
    Graph {
        path: PathBuf,
        /// Scope the graph query to the whole workspace, one file, or a directory subtree.
        #[arg(long, value_enum, default_value_t = GraphScope::All)]
        scope: GraphScope,
        /// Workspace-relative file or directory path for --scope file/directory.
        #[arg(long)]
        target: Option<String>,
        /// Directory depth for --scope directory. 1 means direct children only.
        #[arg(long, default_value_t = 1)]
        depth: usize,
        /// Maximum number of edges printed in text mode.
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Report workspace, index, graph, and code-report status.
    Status {
        /// Workspace root. Defaults to the registered default workspace.
        path: Option<PathBuf>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Read or write settings persisted outside the workspace. Keys use
    /// the same namespaces as the web Settings overlay where possible
    /// (`editor.*`, `server.*`).
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Import and export chan metadata for a registered workspace.
    Metadata {
        #[command(subcommand)]
        action: MetadataAction,
    },
    /// Self-upgrade: read release metadata from chan.app, download
    /// the selected CLI asset, verify SHA256, and atomically replace
    /// the running binary. Set `CHAN_UPDATE_CHECK=0` to silence the
    /// banner that fires on `chan serve` startup.
    Upgrade {
        /// Skip the confirmation prompt.
        #[arg(short = 'y', long)]
        yes: bool,
        /// Only check + report; do not download or replace the
        /// binary. Returns success in both directions.
        #[arg(long)]
        check: bool,
        /// Pin a specific version instead of querying latest metadata.
        /// Pass a bare version, for example `0.14.0`.
        #[arg(long)]
        version: Option<String>,
    },
    /// Internal: run the chan-llm MCP server on stdio against a
    /// workspace. Spawned by MCP clients so file edits route through
    /// chan-workspace's gates instead of touching the workspace directly.
    /// Not for end-user invocation.
    #[command(name = "__mcp", hide = true)]
    Mcp {
        /// Workspace root to expose. Must already be registered.
        path: PathBuf,
    },
    /// Internal: stdio bridge to the MCP server hosted in-process
    /// by a running `chan serve`. Connects to the per-server Unix-
    /// domain socket and pipes stdin/stdout through it. Used by the
    /// external MCP clients so agent child processes can reach the
    /// live workspace without trying to reopen it (which would deadlock
    /// against chan-workspace's per-workspace flock). Not for end-user
    /// invocation.
    #[command(name = "__mcp-proxy", hide = true)]
    McpProxy {
        /// Unix-domain socket path the running chan-server listens
        /// on. Resolved at request time by chan-server, embedded in
        /// the gemini settings.json / claude --mcp-config payload.
        socket: PathBuf,
    },
    /// Manage contacts inside a workspace. Today: import contacts from
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
        /// Workspace-relative directory where notes will land. Created
        /// if it does not exist. Use `""` to write at the workspace
        /// root.
        #[arg(long)]
        into: String,
        /// Source provider's CSV format. Currently only "google".
        #[arg(long, default_value = "google")]
        provider: String,
        /// Parse and report what would be written; do not touch
        /// the workspace.
        #[arg(long)]
        dry_run: bool,
        /// Replace existing files instead of skipping them. The
        /// per-contact line in the report changes from SKIPPED to
        /// OVERWROTE so it's clear which files moved.
        #[arg(long)]
        overwrite: bool,
        /// Workspace root. Defaults to the registered default workspace.
        /// Auto-registers the path if not already known, so
        /// `chan contacts import csv ... --workspace /some/dir`
        /// works without a prior `chan add`.
        #[arg(long)]
        workspace: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum GraphScope {
    All,
    File,
    Directory,
}

#[derive(Subcommand, Debug)]
enum ConfigAction {
    /// Print one setting value, or all supported settings when no
    /// key is given.
    Get {
        /// Dotted key, e.g. `editor.theme` or
        /// `server.attachments_dir`. Empty prints the full TOML.
        key: Option<String>,
        /// Emit JSON instead of a scalar / TOML body.
        #[arg(long)]
        json: bool,
    },
    /// Update a setting. Accepts `key=value` or `key value`.
    Set {
        /// Dotted key, with or without `=value` appended.
        key: String,
        /// Value to assign. Omit when `key` already contains `=value`.
        value: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum MetadataAction {
    /// Export metadata for a registered workspace to a .tar.zst archive.
    Export {
        /// Workspace root.
        path: PathBuf,
        /// Output archive path. Must end in .tar.zst and not exist.
        archive: PathBuf,
    },
    /// Import metadata into a registered workspace from a .tar.zst archive.
    Import {
        /// Workspace root.
        path: PathBuf,
        /// Archive path created by `chan metadata export`.
        archive: PathBuf,
        /// Rebuild the workspace index and graph after import.
        #[arg(long)]
        rescan: bool,
        /// Import even when archive SCM identity does not match.
        #[arg(long = "force-scm")]
        force_scm: bool,
    },
    /// Print the archive manifest without importing it.
    Inspect {
        /// Archive path created by `chan metadata export`.
        archive: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

/// Subcommands for `chan index`. Subcommand-driven (rather than a
/// flat `chan index <path>`) so the surface
/// covers rebuild, model download, semantic-search toggle, and
/// state inspection. Older scripts' flat `chan index <path>` is now
/// `chan index rebuild <path>`.
///
/// Symmetric naming matches the `chan reports
/// enable/disable` parallel pair so scripted callers can pattern-
/// match `<feature> enable / disable` across the surface.
#[derive(Subcommand, Debug)]
enum IndexAction {
    /// Rebuild the search index + graph for a workspace. Older
    /// scripts used a flat `chan index <path>`; the explicit verb keeps it
    /// alongside the model/semantic actions. Accepts either the
    /// positional `<PATH>` (backwards-compat) OR `--path <PATH>`
    /// (uniform with the other four subcommands so wrappers can
    /// pass `--path` to all of them). At least one
    /// must be supplied.
    Rebuild {
        /// Workspace root, positional form.
        path: Option<PathBuf>,
        /// Workspace root, flag form (synonym for the positional).
        #[arg(long = "path", value_name = "PATH")]
        path_flag: Option<PathBuf>,
    },
    /// Download the embedding model into
    /// `<user-config>/chan/models/<model-name>/`. Idempotent: a
    /// re-run with the model already present is a fast no-op.
    /// Default model is `BAAI/bge-small-en-v1.5`; the
    /// `--model` flag forward-compats a future multi-model
    /// picker.
    DownloadModel {
        /// HuggingFace model id, e.g. `BAAI/bge-small-en-v1.5`.
        #[arg(long, default_value = "BAAI/bge-small-en-v1.5")]
        model: String,
    },
    /// List curated embedding models accepted by chan.
    ListModels {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Set the embedding model configured for a workspace.
    SetModel {
        /// Workspace root. Defaults to the registered default workspace.
        #[arg(long)]
        path: Option<PathBuf>,
        /// Curated HuggingFace model id.
        #[arg(long)]
        model: String,
    },
    /// Flip the workspace's Hybrid-search opt-in. Refuses if the model
    /// isn't downloaded; the error points at `chan index
    /// download-model`. The flag persists in
    /// `<index_dir>/config.toml` so it survives `chan serve`
    /// restarts.
    EnableSemantic {
        /// Workspace root. Defaults to the registered default workspace.
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Flip the workspace back to BM25-only.
    DisableSemantic {
        /// Workspace root. Defaults to the registered default workspace.
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Print the semantic-search state: current mode, model
    /// presence, model path + size, opt-in flag.
    Status {
        /// Workspace root. Defaults to the registered default workspace.
        #[arg(long)]
        path: Option<PathBuf>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

/// Subcommands for `chan reports`. Mirrors
/// `IndexAction::{EnableSemantic,DisableSemantic}`'s shape so
/// scripted callers can pattern-match `<feature> enable / disable`
/// uniformly across the surface (`chan index enable-semantic` /
/// `chan reports enable`).
///
/// Default state for both features is OFF (lean-workspace
/// baseline); explicit opt-in via this CLI / the
/// pre-flight UI / Settings flips them on.
#[derive(Subcommand, Debug)]
enum ReportsAction {
    /// Enable per-workspace chan-report (language detection, SLOC
    /// counts, COCOMO estimate) and trigger an initial scan if
    /// no persisted report exists. Idempotent: re-enable is a
    /// no-op.
    Enable {
        /// Workspace root. Defaults to the registry's current workspace
        /// when omitted.
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,
    },
    /// Disable per-workspace chan-report. Destructive: drops the
    /// persisted `report.jsonl` so re-enabling later triggers a
    /// fresh scan. Pass `-y` to skip the confirmation prompt.
    Disable {
        /// Workspace root.
        #[arg(long, value_name = "PATH")]
        path: Option<PathBuf>,
        /// Skip the destructive-action confirmation prompt.
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
}

/// Parse argv with the `cs` alias rewrite. When the binary is invoked
/// through a `cs` symlink (argv[0] basename == "cs"), the remaining args
/// parse as `chan shell <args>`, so `cs terminal list` == `chan shell
/// terminal list`. The symlink is the user's to create (documented in
/// `chan shell --help`); the build never ships one.
/// Parse `args` (typically `std::env::args_os()`) into the clap [`Cli`],
/// applying the `cs` alias rewrite. The `cs -> chan` symlink (or a
/// chan-desktop launched as `cs`) makes `arg0`'s file_stem `cs`; in that
/// case we splice `shell` in as the subcommand so `cs <action>` resolves to
/// `chan shell <action>`. Invoked as `chan` (the standalone shim or
/// chan-desktop's `chan` dispatch) there is no rewrite. Takes `args` rather
/// than reading the environment so chan-desktop can hand us its own argv.
fn parse_cli<I, T>(args: I) -> Cli
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let argv: Vec<std::ffi::OsString> = args.into_iter().map(Into::into).collect();
    let Some(arg0) = argv.first() else {
        // No arg0 is unreachable for a real process; fall back to the
        // environment so a degenerate caller still parses something.
        return Cli::parse();
    };
    if !chan_shell::invoked_as_cs(arg0) {
        return Cli::parse_from(argv);
    }
    let mut rewritten: Vec<std::ffi::OsString> = Vec::with_capacity(argv.len() + 1);
    rewritten.push(arg0.clone());
    rewritten.push("shell".into());
    rewritten.extend(argv.into_iter().skip(1));
    Cli::parse_from(rewritten)
}

/// Which binary is driving the `chan` CLI, and therefore how the
/// desktop-aware subcommands behave.
///
/// - [`Personality::Standalone`] — the `chan` binary from install.sh (and
///   the `cs -> chan` symlink). `chan serve` always runs its own server and
///   opens the browser; it never hands off to a running chan-desktop.
///   `chan upgrade` replaces the CLI tarball in place.
/// - [`Personality::Desktop`] — chan-desktop invoked as `chan` (via the
///   `~/.local/bin/chan` shim). `chan serve` integrates with the desktop:
///   it hands the workspace to the running desktop, or launches the GUI.
///   `chan upgrade` drives the desktop's `tauri-plugin-updater`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Personality {
    Standalone,
    Desktop,
}

/// Parse `args` and run the selected subcommand to completion.
///
/// This is the single entry point for the whole `chan` CLI. The caller owns
/// the tokio runtime (so it can pick the multi-threaded flavour `serve`
/// needs and `shutdown_background()` to detach chan-workspace's uncancellable
/// reindex pool on exit); everything here runs inside it. Sync subcommands
/// execute inline on the runtime thread, which is fine for a
/// run-one-thing-and-exit CLI.
pub async fn run<I, T>(args: I, personality: Personality) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = parse_cli(args);
    init_tracing(cli.verbose);
    let verbose = cli.verbose > 0;

    match cli.command {
        Command::Add {
            path,
            semantic_search,
            reports,
        } => cmd_add(path, semantic_search, reports),
        Command::List { json } => cmd_list(json),
        Command::Shell { action } => chan_shell::dispatch(action).await,
        Command::Completions { shell } => cmd_completions(shell),
        Command::Remove { path } => cmd_remove(path),
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
            search_aggression,
            no_settings,
            tunnel_url,
            tunnel_token,
            tunnel_workspace_name,
            tunnel_public,
        } => {
            let addr = resolve_listen_addr(host, ipv4, ipv6, port)?;
            let prefix = chan_server::sanitize_prefix(prefix.as_deref().unwrap_or(""))
                .map_err(|e| anyhow::anyhow!("invalid --prefix: {e}"))?;
            cmd_serve(
                ServeArgs {
                    addr,
                    prefix,
                    idle_timeout: timeout,
                    path,
                    here,
                    no_token,
                    no_browser,
                    search_aggression,
                    no_settings,
                    tunnel_url,
                    tunnel_token,
                    tunnel_workspace_name,
                    tunnel_public,
                    verbose,
                },
                personality,
            )
            .await
        }
        Command::Index { action } => cmd_index(action),
        Command::Reports { action } => cmd_reports(action),
        Command::Search { path, query, limit } => cmd_search(path, query, limit),
        Command::Graph {
            path,
            scope,
            target,
            depth,
            limit,
            json,
        } => cmd_graph(path, scope, target, depth, limit, json),
        Command::Status { path, json } => cmd_status(path, json),
        Command::Config { action } => cmd_config(action),
        Command::Metadata { action } => cmd_metadata(action),
        Command::Upgrade {
            yes,
            check,
            version,
        } => match personality {
            // Standalone (install.sh) replaces the CLI tarball in place.
            Personality::Standalone => {
                update::run_upgrade(update::UpgradeOptions {
                    assume_yes: yes,
                    check_only: check,
                    version_override: version,
                    verbose,
                })
                .await
            }
            // Desktop drives the running desktop's tauri-plugin-updater
            // instead (no tarball). `yes` is moot — the fire-and-return flow
            // has no prompt.
            Personality::Desktop => cmd_upgrade_desktop(check, version).await,
        },
        Command::Mcp { path } => cmd_mcp(path).await,
        Command::McpProxy { socket } => cmd_mcp_proxy(socket).await,
        Command::Contacts { action } => match action {
            ContactsAction::Import { source } => match source {
                ImportSource::Csv {
                    file,
                    into,
                    provider,
                    dry_run,
                    overwrite,
                    workspace,
                } => cmd_contacts_import_csv(file, into, provider, dry_run, overwrite, workspace),
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
                .unwrap_or_else(|_| fallback_filter(level)),
        )
        .with_writer(std::io::stderr)
        .init();
}

/// tokei (pulled in transitively by chan-report for the language-count
/// lens) logs `Unknown extension: <ext>` at WARN through tokei's own
/// `LanguageType::from_path` for every file it can't classify. chan-report
/// is default-off (`IndexConfig::reports_enabled = false`), so on a source
/// tree with reports enabled this is pure console noise with no downstream
/// effect (the graph language lens already degrades when a bucket is
/// absent). Cap tokei at ERROR so the spam disappears but genuine tokei
/// errors still surface.
///
/// Applied to the FALLBACK filter only (`RUST_LOG` parses first via
/// `try_from_default_env`), so anyone who explicitly wants tokei detail
/// keeps full control by setting `RUST_LOG`.
const TOKEI_LOG_DIRECTIVE: &str = "tokei=error";

fn fallback_filter(level: &str) -> tracing_subscriber::EnvFilter {
    tracing_subscriber::EnvFilter::new(level).add_directive(
        TOKEI_LOG_DIRECTIVE
            .parse()
            .expect("static tokei log directive parses"),
    )
}

fn library() -> Result<Library> {
    Library::open().context("opening chan registry")
}

fn same_path(a: &Path, b: &Path) -> bool {
    let ca = a.canonicalize().unwrap_or_else(|_| a.to_path_buf());
    let cb = b.canonicalize().unwrap_or_else(|_| b.to_path_buf());
    ca == cb
}

/// Pick the URL-safe workspace name to publish under
/// `{user}.workspace.chan.app/<name>`. This is a tunnel URL concern,
/// separate from local path-keyed workspace metadata.
///
/// - With `--tunnel-workspace-name`: validate it; bail with a clear
///   message + a suggested sanitized form if rejected.
/// - Without: take the path basename and sanitize. Warn when
///   sanitize altered it. Bail when sanitize yields `None` (the
///   basename collapses to all punctuation).
fn resolve_tunnel_workspace_name(flag: Option<String>, root: &Path) -> Result<String> {
    if let Some(name) = flag {
        if chan_server::tunnel::is_valid_workspace_name(&name) {
            return Ok(name);
        }
        let suggestion = chan_server::tunnel::sanitize_workspace_name(&name);
        let max = chan_server::tunnel::MAX_WORKSPACE_NAME_LEN;
        let hint = match suggestion {
            Some(s) => format!(" Try --tunnel-workspace-name={s}."),
            None => String::new(),
        };
        anyhow::bail!(
            "--tunnel-workspace-name {name:?} is not URL-safe (need [a-z0-9-], 1-{max} chars, no leading/trailing hyphen).{hint}"
        );
    }
    let source = root
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if chan_server::tunnel::is_valid_workspace_name(&source) {
        return Ok(source);
    }
    match chan_server::tunnel::sanitize_workspace_name(&source) {
        Some(sanitized) => {
            eprintln!(
                "NOTE: workspace path basename {source:?} sanitized to {sanitized:?} for the tunnel URL. \
                 Pass --tunnel-workspace-name to override."
            );
            Ok(sanitized)
        }
        None => {
            let max = chan_server::tunnel::MAX_WORKSPACE_NAME_LEN;
            anyhow::bail!(
                "cannot derive a URL-safe tunnel workspace name from {source:?}. \
                 Pass --tunnel-workspace-name=<name> ([a-z0-9-], 1-{max} chars, no leading/trailing hyphen)."
            );
        }
    }
}

fn ensure_workspace_registered(
    lib: &Library,
    root: &Path,
) -> Result<chan_workspace::KnownWorkspace> {
    lib.register_workspace(root)
        .with_context(|| format!("registering {}", root.display()))
}

fn cmd_add(path: PathBuf, semantic_search: bool, reports: bool) -> Result<()> {
    // Mirror `chan serve`'s behavior: create the directory if it
    // doesn't exist yet. Single verb covers both "register an
    // existing dir" and "make a fresh workspace here". A separate
    // `chan init` would be a synonym; not worth the mental
    // overhead.
    if !path.exists() {
        std::fs::create_dir_all(&path)
            .with_context(|| format!("creating workspace root {}", path.display()))?;
    }
    let lib = library()?;
    let entry = ensure_workspace_registered(&lib, &path)?;
    // Opt-in feature flags. Persist before
    // boot-time activation so a `chan add --reports` lands the
    // flag immediately + the kickoff scan runs once.
    if semantic_search || reports {
        let workspace = lib
            .open_workspace(&entry.root_path)
            .with_context(|| format!("opening workspace at {}", entry.root_path.display()))?;
        if semantic_search {
            workspace
                .set_semantic_enabled(true)
                .context("persisting semantic_enabled flag")?;
        }
        if reports {
            workspace
                .set_reports_enabled(true)
                .context("persisting reports_enabled flag")?;
        }
        workspace
            .boot()
            .context("BOOT after enabling optional features")?;
    }
    println!("registered: {}", entry.root_path.display());
    if semantic_search {
        println!("semantic search enabled");
    }
    if reports {
        println!("chan-reports enabled");
    }
    Ok(())
}

fn cmd_list(json: bool) -> Result<()> {
    let workspaces = library()?.list_workspaces();
    if json {
        let out = WorkspaceListOutput {
            workspaces: workspaces.iter().map(WorkspaceListEntry::from).collect(),
        };
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }
    if workspaces.is_empty() {
        println!("(no workspaces registered)");
        return Ok(());
    }
    for d in workspaces {
        println!(
            "{}  (last seen {}, metadata {})",
            d.root_path.display(),
            d.last_seen_at.format("%Y-%m-%d %H:%M"),
            d.metadata_key,
        );
    }
    Ok(())
}

fn cmd_completions(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
    Ok(())
}

fn cmd_remove(path: PathBuf) -> Result<()> {
    let lib = library()?;
    let removed = lib
        .unregister_workspace(&path)
        .with_context(|| format!("unregistering {}", path.display()))?;
    if removed {
        println!("unregistered: {}", path.display());
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

fn parse_search_aggression(s: &str) -> Result<SearchAggression, String> {
    s.parse()
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
fn print_vcs_parent_error(root: &Path, parent: &chan_workspace::VcsParent) {
    // Canonicalize both paths for the marker so wrappers get
    // absolute, symlink-resolved forms. Fall back to the input
    // when canonicalize fails (root may not yet exist on disk).
    let root_abs = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let repo_abs =
        std::fs::canonicalize(&parent.repo_root).unwrap_or_else(|_| parent.repo_root.clone());
    let kind_human = match parent.kind {
        chan_workspace::VcsKind::Git => "Git",
        chan_workspace::VcsKind::Mercurial => "Mercurial",
        chan_workspace::VcsKind::Subversion => "Subversion",
    };
    eprintln!(
        "error: workspace '{}' is inside a {} repository at '{}'.",
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

/// Resolved `chan serve` invocation: every CLI input after listen-addr
/// and prefix resolution, grouped so the handler takes one argument
/// instead of a 15-parameter tail.
struct ServeArgs {
    addr: SocketAddr,
    prefix: String,
    idle_timeout: Option<Duration>,
    path: Option<PathBuf>,
    here: bool,
    no_token: bool,
    no_browser: bool,
    search_aggression: Option<SearchAggression>,
    no_settings: bool,
    tunnel_url: String,
    tunnel_token: Option<String>,
    tunnel_workspace_name: Option<String>,
    tunnel_public: bool,
    verbose: bool,
}

/// Make a serve root absolute against the process cwd. `canonicalize`
/// resolves symlinks for an existing dir; `std::path::absolute` makes a
/// not-yet-created path absolute lexically (so `chan serve new-dir` still
/// lands under the cwd); the final fallback returns the input unchanged
/// (only reachable if both fail, e.g. an unreadable cwd). The result must
/// be absolute so the desktop handoff — which runs with cwd "/" — and the
/// canonical-path-keyed registry both see the directory the user ran in.
fn absolutize_serve_root(root: PathBuf) -> PathBuf {
    std::fs::canonicalize(&root)
        .or_else(|_| std::path::absolute(&root))
        .unwrap_or(root)
}

async fn cmd_serve(args: ServeArgs, personality: Personality) -> Result<()> {
    let ServeArgs {
        addr,
        prefix,
        idle_timeout,
        path,
        here,
        no_token,
        no_browser,
        search_aggression,
        no_settings,
        tunnel_url,
        tunnel_token,
        tunnel_workspace_name,
        tunnel_public,
        verbose,
    } = args;
    let lib = library()?;
    // Resolve the workspace root: explicit arg first, then the registry
    // default, then the platform default. Auto-register so users
    // can `chan serve /some/dir` without a prior `chan add`.
    let root = path
        .or_else(|| lib.default_workspace_root())
        .unwrap_or_else(|| lib.effective_default_workspace_root());
    // Resolve to an absolute path against the CLI's cwd before anything
    // downstream consumes it. The macOS desktop handoff opens the
    // workspace in a process whose cwd is "/", and the workspace registry
    // is keyed by the canonical path, so a bare `chan serve .` must not
    // leak a relative root (the desktop would resolve it against "/" and
    // open the filesystem root).
    let root = absolutize_serve_root(root);
    // VCS-parent gate. If `root` is inside a Git / Mercurial /
    // Subversion working tree, refuse with a structured error so a
    // wrapping shell (chan-desktop) can parse the marker line and
    // offer the user a choice between repo root and the subdir.
    // Runs before any state mutation: no directory creation, no
    // registry write. `--here` opts the caller out for the case
    // where serving the subdir is the genuine intent.
    if !here {
        if let Some(parent) = chan_workspace::detect_parent_vcs(&root) {
            print_vcs_parent_error(&root, &parent);
            std::process::exit(70);
        }
    }
    if !root.exists() {
        std::fs::create_dir_all(&root)
            .with_context(|| format!("creating workspace root {}", root.display()))?;
    }

    // CLI-to-desktop handoff. Only the Desktop personality (chan-desktop
    // dispatched as `chan`) integrates with a running desktop; the
    // standalone `chan` binary always owns its own server (browser) and
    // never hands off. When a same-user chan-desktop is running in a GUI
    // session and this isn't a tunnel run, ask it to open this workspace in
    // a native window and EXIT. The desktop then owns the workspace's flock;
    // the CLI must NOT also open the workspace (the single-writer
    // invariant). This runs BEFORE `open_workspace` so a successful handoff
    // never double-opens. Every fallback (no desktop, refused, stale socket,
    // bad handshake, version skew, GUI-absent, tunnel) drops through to the
    // standalone server path below.
    if personality == Personality::Desktop && tunnel_token.is_none() {
        if let Some(outcome) = maybe_handoff_to_desktop(&root).await {
            return outcome;
        }
    }

    ensure_workspace_registered(&lib, &root)?;
    let workspace = lib.open_workspace(&root)?;

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
        let workspace_name = resolve_tunnel_workspace_name(tunnel_workspace_name, &root)?;
        if tunnel_public {
            eprintln!(
                "WARNING: --public exposes this workspace at \
                 workspace.chan.app/<user>/{workspace_name} with no auth gate. \
                 Anyone with the URL has read/write access."
            );
        }
        return chan_server::serve_via_tunnel(
            lib,
            workspace,
            TunnelServeConfig {
                tunnel_url: &tunnel_url,
                token,
                workspace_name,
                public: tunnel_public,
                open_browser: !no_browser,
                search_aggression,
            },
        )
        .await
        .context("running tunnel client");
    }

    // Loud warning: the auth model assumes loopback. No TLS, only a
    // bearer token. Binding off-loopback exposes the workspace in the
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
                 access to your workspace for anyone who can reach this port."
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
        search_aggression,
        verbose,
        // Local serve trusts the operator by default; --no-settings
        // opts into the same UI grey + server 403 that --tunnel-public
        // forces, for kiosk / shared-workstation deployments. The
        // public-tunnel redactions on GETs are kept tunnel-only:
        // a local operator on the same machine has nothing to hide
        // from themselves.
        settings_disabled: no_settings,
        tunnel_public: false,
    };
    chan_server::serve(lib, workspace, config)
        .await
        .with_context(|| format!("running server on {addr}"))
}

/// Integrate a Desktop-personality `chan serve` with the desktop app.
///
/// Returns:
/// - `Some(Ok(()))` when the desktop opened the workspace window (either a
///   running desktop took the handoff, or we launched the GUI and it did):
///   the CLI exits WITHOUT opening the workspace (the desktop owns the flock).
/// - `Some(Err(..))` when desktop integration was attempted but failed hard
///   (GUI launch failed / timed out). The caller propagates the error; a
///   Desktop invocation does NOT silently fall back to the browser.
/// - `None` only when desktop integration does not apply (opted out via
///   `CHAN_NO_DESKTOP_HANDOFF`, no GUI session such as SSH, a running desktop
///   of a skewed version, or a non-unix build): the caller falls back to the
///   standalone server path. These are the cases where a browser/URL is the
///   only sensible outcome.
///
/// The caller already restricted this to the Desktop personality and excluded
/// tunnel mode. Here we add the GUI-session + explicit-opt-out gates, then
/// hand off to a running desktop or launch one.
async fn maybe_handoff_to_desktop(root: &Path) -> Option<Result<()>> {
    // Explicit opt-out for automation, and the headless auto-skip: over SSH
    // (no GUI session) there's no window to show, so a printed URL is the
    // only useful outcome. Both keep the load-bearing standalone path.
    if chan_server::handoff::handoff_opt_out() {
        return None;
    }
    if !chan_server::handoff::gui_session_present() {
        return None;
    }

    match chan_server::handoff::try_handoff(root).await {
        chan_server::handoff::Outcome::HandedOff => {
            // The desktop owns the workspace from here; the CLI is just a
            // launcher. Print a short note to stdout (where the URL
            // would otherwise go) and exit 0.
            println!("chan: opened {} in chan-desktop.", root.display());
            Some(Ok(()))
        }
        chan_server::handoff::Outcome::VersionSkew {
            desktop_version,
            desktop_protocol: _,
        } => {
            // A running desktop of a DIFFERENT version (e.g. the binary was
            // upgraded but the old desktop is still running). Launching our
            // version would fight the old one for the singleton socket, so
            // name the skew and fall back to a standalone server rather than
            // risk two desktops.
            eprintln!(
                "chan: chan-desktop is version {desktop_version}, CLI is {}; \
                 cannot hand off. Restart chan-desktop to pick up the new \
                 version. Starting a standalone server for now.",
                chan_server::handoff::CHAN_VERSION,
            );
            None
        }
        chan_server::handoff::Outcome::DesktopError { message } => {
            eprintln!(
                "chan: chan-desktop could not open the workspace ({message}); \
                 starting a standalone server."
            );
            None
        }
        // No running desktop: launch the GUI and open the workspace in it.
        // A Desktop invocation never falls back to the browser here.
        chan_server::handoff::Outcome::NoDesktop => maybe_launch_desktop(root).await,
    }
}

/// Launch the desktop GUI for a `chan serve` that found no running desktop,
/// then hand it the workspace. Unix-only (the desktop + handoff socket are
/// unix); off unix there's no GUI to launch, so fall back to standalone.
#[cfg(unix)]
async fn maybe_launch_desktop(root: &Path) -> Option<Result<()>> {
    Some(launch_desktop_and_handoff(root).await)
}

#[cfg(not(unix))]
async fn maybe_launch_desktop(_root: &Path) -> Option<Result<()>> {
    None
}

/// Spawn the chan-desktop GUI and hand `root` to it once it's up.
///
/// Only reached from the Desktop personality, so `current_exe()` IS the
/// chan-desktop binary. Spawns the GUI detached, then polls the well-known
/// handoff socket — the GUI binds it during setup — re-attempting
/// `try_handoff` until it opens the workspace or a generous deadline passes
/// (a cold GUI boot starts the embedded server and a window, which takes a
/// few seconds).
#[cfg(unix)]
async fn launch_desktop_and_handoff(root: &Path) -> Result<()> {
    spawn_desktop_gui().context("launching chan-desktop")?;

    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        tokio::time::sleep(Duration::from_millis(400)).await;
        match chan_server::handoff::try_handoff(root).await {
            chan_server::handoff::Outcome::HandedOff => {
                println!("chan: launched chan-desktop and opened {}.", root.display());
                return Ok(());
            }
            // Not up yet (socket absent / connect refused): keep waiting.
            chan_server::handoff::Outcome::NoDesktop => {}
            // The desktop we just launched is up but won't take the handoff.
            // Surface and stop retrying rather than spin to the deadline.
            chan_server::handoff::Outcome::VersionSkew {
                desktop_version, ..
            } => {
                anyhow::bail!(
                    "launched chan-desktop is version {desktop_version}, CLI is {}; \
                     cannot hand off",
                    chan_server::handoff::CHAN_VERSION,
                );
            }
            chan_server::handoff::Outcome::DesktopError { message } => {
                anyhow::bail!("chan-desktop could not open the workspace: {message}");
            }
        }
        if std::time::Instant::now() >= deadline {
            anyhow::bail!(
                "timed out waiting for chan-desktop to start; run `chan serve` again \
                 once it is up, or set CHAN_NO_DESKTOP_HANDOFF=1 for a standalone server"
            );
        }
    }
}

/// Launch the chan-desktop GUI as a detached process.
///
/// `current_exe()` is the chan-desktop binary (this only runs for the Desktop
/// personality). We start it with a clean argv0 (NOT `chan`/`cs`) so the
/// pre-GUI argv probe falls through to a normal GUI launch instead of
/// re-dispatching as the CLI.
#[cfg(unix)]
fn spawn_desktop_gui() -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    let exe = std::env::current_exe()?;

    // macOS: launching the bare Mach-O inside the `.app` can start the process
    // without LaunchServices activating/foregrounding it. Prefer
    // `open <Name>.app`, which hands launch to LaunchServices (proper
    // activation + single-instance). Derive the bundle by climbing
    // `…/<Name>.app/Contents/MacOS/<bin>`.
    #[cfg(target_os = "macos")]
    {
        if let Some(bundle) = macos_app_bundle(&exe) {
            return Command::new("/usr/bin/open")
                .arg(bundle)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map(|_| ());
        }
        // Not in a bundle (dev build): fall through to the direct exec below.
    }

    // Linux AppImage: `$APPIMAGE` is the real, relaunchable image, while
    // `current_exe()` is the ephemeral `/tmp/.mount_*` path. Prefer
    // `$APPIMAGE`; off an AppImage (deb/rpm) `current_exe()` is
    // `/usr/bin/chan-desktop`, which relaunches fine.
    let target = std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .unwrap_or(exe);
    Command::new(&target)
        // Clean argv0 so the spawned process boots the GUI, not the alias.
        .arg0(&target)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        // New process group so Ctrl-C in the launching terminal doesn't also
        // kill the desktop we just started.
        .process_group(0)
        .spawn()
        .map(|_| ())
}

/// Climb `…/<Name>.app/Contents/MacOS/<bin>` to the `.app` bundle dir, if
/// `exe` is laid out that way. Returns None for a loose dev binary.
#[cfg(target_os = "macos")]
fn macos_app_bundle(exe: &Path) -> Option<PathBuf> {
    let macos_dir = exe.parent()?; // …/Contents/MacOS
    let contents = macos_dir.parent()?; // …/Contents
    let bundle = contents.parent()?; // …/<Name>.app
    let is_bundle = bundle.extension().map(|e| e == "app").unwrap_or(false)
        && macos_dir.file_name().map(|n| n == "MacOS").unwrap_or(false)
        && contents
            .file_name()
            .map(|n| n == "Contents")
            .unwrap_or(false);
    is_bundle.then(|| bundle.to_path_buf())
}

/// `chan upgrade` for the Desktop personality: drive the running desktop's
/// `tauri-plugin-updater` instead of replacing a CLI tarball.
///
/// With `check_only` we query a running desktop and report — we do NOT launch
/// one just to check (that would pop a window). Otherwise we find or launch
/// the desktop and trigger the install (fire-and-return: the desktop owns the
/// download/install/relaunch). `--version` pinning is unsupported (the desktop
/// updater always installs the latest published release).
#[cfg(unix)]
async fn cmd_upgrade_desktop(check_only: bool, version_override: Option<String>) -> Result<()> {
    use chan_server::handoff::UpgradeOutcome;

    if version_override.is_some() {
        eprintln!(
            "chan: --version is not supported for a desktop install; the desktop \
             updater always installs the latest published release. Ignoring it."
        );
    }

    match chan_server::handoff::try_upgrade(check_only).await {
        UpgradeOutcome::Checked { available, .. } => {
            match available {
                Some(v) => {
                    println!(
                        "chan: chan-desktop {v} is available. Run `chan upgrade` to install it."
                    )
                }
                None => println!("chan: chan-desktop is up to date."),
            }
            Ok(())
        }
        UpgradeOutcome::Started { .. } => {
            println!(
                "chan: chan-desktop is updating in the background; it will relaunch when done."
            );
            Ok(())
        }
        UpgradeOutcome::VersionSkew {
            desktop_version, ..
        } => anyhow::bail!(
            "chan-desktop is version {desktop_version}, CLI is {}; restart chan-desktop, \
             then run `chan upgrade` again",
            chan_server::handoff::CHAN_VERSION,
        ),
        UpgradeOutcome::DesktopError { message } => {
            anyhow::bail!("chan-desktop could not upgrade: {message}")
        }
        UpgradeOutcome::NoDesktop => {
            if check_only {
                // No running desktop to ask; launching one just to check would
                // pop a window. Point the user at the install path instead.
                anyhow::bail!(
                    "no running chan-desktop to check. Open chan-desktop, or run \
                     `chan upgrade` (without --check) to launch and update it"
                );
            }
            launch_desktop_then_upgrade().await
        }
    }
}

/// Launch the desktop GUI (none was running) and trigger its updater once it
/// is up. Mirrors `launch_desktop_and_handoff` but for the upgrade trigger.
#[cfg(unix)]
async fn launch_desktop_then_upgrade() -> Result<()> {
    use chan_server::handoff::UpgradeOutcome;

    spawn_desktop_gui().context("launching chan-desktop")?;

    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        tokio::time::sleep(Duration::from_millis(400)).await;
        match chan_server::handoff::try_upgrade(false).await {
            UpgradeOutcome::Started { .. } => {
                println!("chan: launched chan-desktop; it is updating in the background.");
                return Ok(());
            }
            // Not up yet (socket absent / connect refused): keep waiting.
            UpgradeOutcome::NoDesktop => {}
            // check_only=false never returns Checked, but be exhaustive.
            UpgradeOutcome::Checked { .. } => return Ok(()),
            UpgradeOutcome::VersionSkew {
                desktop_version, ..
            } => anyhow::bail!(
                "launched chan-desktop is version {desktop_version}, CLI is {}; cannot upgrade",
                chan_server::handoff::CHAN_VERSION,
            ),
            UpgradeOutcome::DesktopError { message } => {
                anyhow::bail!("chan-desktop could not upgrade: {message}")
            }
        }
        if std::time::Instant::now() >= deadline {
            anyhow::bail!("timed out waiting for chan-desktop to start");
        }
    }
}

#[cfg(not(unix))]
async fn cmd_upgrade_desktop(_check_only: bool, _version_override: Option<String>) -> Result<()> {
    anyhow::bail!("desktop `chan upgrade` is only supported on unix")
}

/// Dispatch the `chan reports {enable,disable}`
/// subcommands. Parallels `cmd_index_set_semantic`'s shape: open
/// the workspace (with the path-resolution fallback to the registry's
/// default), flip the per-workspace `reports_enabled` flag, surface
/// the verb on stdout. `disable` is destructive — drops the
/// persisted `report.jsonl` so re-enable triggers a fresh scan;
/// gated on `--yes` or an interactive prompt (explicit
/// confirmation for a destructive action).
fn cmd_reports(action: ReportsAction) -> Result<()> {
    match action {
        ReportsAction::Enable { path } => cmd_reports_set(path, true, false),
        ReportsAction::Disable { path, yes } => cmd_reports_set(path, false, yes),
    }
}

fn cmd_reports_set(path: Option<PathBuf>, enabled: bool, skip_confirm: bool) -> Result<()> {
    let lib = library()?;
    let root = path
        .or_else(|| lib.default_workspace_root())
        .unwrap_or_else(|| lib.effective_default_workspace_root());
    let workspace = lib
        .open_workspace(&root)
        .with_context(|| format!("opening workspace at {}", root.display()))?;
    // Destructive-action confirmation for disable. The non-
    // interactive `-y` flag skips the prompt; an interactive TTY
    // without `-y` blocks until the user confirms.
    if !enabled && !skip_confirm {
        eprintln!(
            "About to disable chan-reports for workspace at {}",
            workspace.root().display(),
        );
        eprintln!(
            "This drops the persisted report.jsonl. Re-enabling later \
             triggers a fresh scan."
        );
        eprint!("Continue? [y/N] ");
        use std::io::Write;
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        let answer = line.trim().to_ascii_lowercase();
        if !(answer == "y" || answer == "yes") {
            eprintln!("Aborted.");
            return Ok(());
        }
    }
    workspace
        .set_reports_enabled(enabled)
        .context("persisting reports_enabled flag")?;
    if enabled {
        // Kick off the initial scan via `boot` so the flag flip
        // produces visible data without waiting for the next
        // `Workspace::report()` consumer.
        workspace.boot().context("BOOT after enabling reports")?;
    }
    let verb = if enabled { "enabled" } else { "disabled" };
    println!(
        "chan-reports {verb} for workspace at {}",
        workspace.root().display()
    );
    Ok(())
}

fn cmd_index(action: IndexAction) -> Result<()> {
    match action {
        IndexAction::Rebuild { path, path_flag } => {
            // Either form works. Both supplied → the
            // flag wins; users have to be explicit anyway and the
            // flag is the canonical shape going forward. Neither
            // supplied → clean error, not a clap-default panic.
            let resolved = path_flag.or(path).ok_or_else(|| {
                anyhow::anyhow!(
                    "`chan index rebuild` requires a workspace path (positional or `--path`)"
                )
            })?;
            cmd_index_rebuild(resolved)
        }
        IndexAction::DownloadModel { model } => cmd_index_download_model(&model),
        IndexAction::ListModels { json } => cmd_index_list_models(json),
        IndexAction::SetModel { path, model } => cmd_index_set_model(path, &model),
        IndexAction::EnableSemantic { path } => cmd_index_set_semantic(path, true),
        IndexAction::DisableSemantic { path } => cmd_index_set_semantic(path, false),
        IndexAction::Status { path, json } => cmd_index_status(path, json),
    }
}

fn cmd_index_rebuild(path: PathBuf) -> Result<()> {
    let lib = library()?;
    // Idempotent: registering an already-known workspace only touches
    // last_seen_at. CLI users expect `chan index rebuild /some/path`
    // to work without a prior `chan add`.
    ensure_workspace_registered(&lib, &path)?;
    let workspace = lib.open_workspace(&path)?;

    // Live progress on stderr so the user can see the embed pass
    // is making progress; on a big workspace it can run for tens of
    // minutes. Use a TTY-friendly carriage return rewrite when
    // stderr is interactive; fall back to plain lines (one per
    // file) when redirected so logs stay readable.
    use std::io::{IsTerminal, Write};
    let tty = std::io::stderr().is_terminal();
    // chan-workspace 0.7 reshaped progress: a single `ProgressEvent` with
    // a `stage` enum (IndexFile / EmbedBatch / GraphRebuild / ...),
    // current/total counters, and an optional label. We surface the
    // two stages the reindex CLI cared about; everything else folds
    // into a generic "still working" line so nothing escapes the user
    // silently on large workspaces.
    let callback = chan_workspace::progress::progress_fn(move |p| {
        let line = match p.stage {
            chan_workspace::progress::ProgressStage::IndexFile => format!(
                "[{}/{}] {}",
                p.current.saturating_add(1),
                p.total,
                p.label.as_deref().unwrap_or("")
            ),
            chan_workspace::progress::ProgressStage::EmbedBatch => format!(
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
    let summary = workspace
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
    // Surface embed-phase resumption when it fired. Skipped on full
    // first-time builds (count is 0) so the success path stays terse.
    if summary.embeds_reused > 0 {
        println!(
            "reused {} embedding shard{} from prior run",
            summary.embeds_reused,
            if summary.embeds_reused == 1 { "" } else { "s" },
        );
    }
    for (path, e) in &summary.errors {
        eprintln!("  error: {path}: {e}");
    }
    Ok(())
}

fn cmd_index_list_models(json: bool) -> Result<()> {
    let models = chan_workspace::index::config::embedding_models();
    if json {
        println!("{}", serde_json::to_string_pretty(models)?);
    } else {
        for model in models {
            let marker = if model.is_default { "default" } else { "" };
            println!(
                "{:<28} {:<19} dim={:<4} {:<8} {:<7} {}",
                model.id, model.label, model.dim, model.size_label, marker, model.note
            );
        }
    }
    Ok(())
}

/// Stub when the binary is built without
/// `--features embeddings`. The candle + hf-hub stack is gated
/// behind that feature; without it there's nothing to download.
/// Bail with a clear message instead of a missing-symbol error.
#[cfg(not(feature = "embeddings"))]
fn cmd_index_download_model(_model: &str) -> Result<()> {
    anyhow::bail!("chan was built without `--features embeddings`; semantic search is unavailable")
}

#[cfg(not(feature = "embeddings"))]
fn cmd_index_set_semantic(_path: Option<PathBuf>, _enabled: bool) -> Result<()> {
    anyhow::bail!("chan was built without `--features embeddings`; semantic search is unavailable")
}

#[cfg(not(feature = "embeddings"))]
fn cmd_index_set_model(_path: Option<PathBuf>, _model: &str) -> Result<()> {
    anyhow::bail!("chan was built without `--features embeddings`; semantic search is unavailable")
}

#[cfg(not(feature = "embeddings"))]
fn cmd_index_status(_path: Option<PathBuf>, _json: bool) -> Result<()> {
    anyhow::bail!("chan was built without `--features embeddings`; semantic search is unavailable")
}

/// Download the embedding model into the per-machine
/// cache. Blocking; the hf-hub backend prints its own progress to
/// stderr when stderr is a TTY. Idempotent — if the model is
/// already laid out in the cache the call returns immediately.
#[cfg(feature = "embeddings")]
fn cmd_index_download_model(model: &str) -> Result<()> {
    use chan_workspace::index::embeddings::{
        global_models_dir, repo_dir_name, resolve_model, Embedder,
    };
    if chan_workspace::index::config::embedding_model(model).is_none() {
        anyhow::bail!(
            "unknown embedding model: {model} (run `chan index list-models` to list supported models)"
        );
    }
    let cache_dir = global_models_dir();
    let expected_dir = cache_dir.join(repo_dir_name(model));
    if resolve_model(model).is_ok() {
        println!(
            "model {} already present at {}",
            model,
            expected_dir.display()
        );
        return Ok(());
    }
    std::fs::create_dir_all(&cache_dir)
        .with_context(|| format!("create model cache {}", cache_dir.display()))?;
    eprintln!(
        "downloading {} into {} (this may take a few minutes)",
        model,
        cache_dir.display()
    );
    Embedder::open(model, &cache_dir).with_context(|| format!("download model {model}"))?;
    println!("downloaded {} into {}", model, expected_dir.display());
    Ok(())
}

#[cfg(feature = "embeddings")]
fn cmd_index_set_model(path: Option<PathBuf>, model: &str) -> Result<()> {
    if chan_workspace::index::config::embedding_model(model).is_none() {
        anyhow::bail!(
            "unknown embedding model: {model} (run `chan index list-models` to list supported models)"
        );
    }
    let lib = library()?;
    let root = path
        .or_else(|| lib.default_workspace_root())
        .unwrap_or_else(|| lib.effective_default_workspace_root());
    let workspace = lib
        .open_workspace(&root)
        .with_context(|| not_a_chan_workspace_hint(&root))?;
    workspace
        .set_semantic_model(model)
        .context("persisting semantic model")?;
    println!(
        "semantic model set to {model} for workspace at {}",
        workspace.root().display()
    );
    Ok(())
}

/// Flip the per-workspace Hybrid-search opt-in. On enable,
/// refuses if the model isn't downloaded; the user is pointed at
/// `chan index download-model`. On disable, always succeeds (the
/// underlying `set_semantic_enabled` is idempotent).
///
/// Deliberately does NOT auto-register an unregistered path.
/// Refusing here surfaces a clean "not a chan workspace at <path>"
/// instead of a registration side-effect that leaks the
/// implementation detail.
#[cfg(feature = "embeddings")]
fn cmd_index_set_semantic(path: Option<PathBuf>, enabled: bool) -> Result<()> {
    use chan_workspace::index::embeddings::resolve_model;
    let lib = library()?;
    let root = path
        .or_else(|| lib.default_workspace_root())
        .unwrap_or_else(|| lib.effective_default_workspace_root());
    let workspace = lib
        .open_workspace(&root)
        .with_context(|| not_a_chan_workspace_hint(&root))?;
    if enabled {
        let model = workspace
            .semantic_model()
            .context("reading workspace's model id")?;
        if let Err(err) = resolve_model(&model) {
            return Err(anyhow::anyhow!(
                "{err}\nrun `chan index download-model` to fetch it"
            ));
        }
    }
    workspace
        .set_semantic_enabled(enabled)
        .context("persisting semantic_enabled flag")?;
    let verb = if enabled { "enabled" } else { "disabled" };
    println!(
        "semantic search {verb} for workspace at {}",
        workspace.root().display()
    );
    Ok(())
}

/// Print the per-workspace semantic-search state. Text by
/// default; `--json` emits a `{workspaces:[{...}]}`-style object for
/// scripting (single workspace in the response; the shape is plural so
/// a future multi-workspace variant lands as a pure extension).
///
/// Read-only access, lock-free + no auto-register.
/// Taking the writer lock via `Workspace::open` (and
/// auto-registering missing paths) would surface against a
/// live-served workspace as "workspace is locked by another
/// process", and against an
/// unregistered path leak "Error: registering <path>". So the
/// helper looks up the registered workspace's index dir directly and
/// loads `IndexConfig` from disk — no Workspace handle, no flock, no
/// side-effects. Missing-from-registry → clean
/// "not a chan workspace at <path>".
#[cfg(feature = "embeddings")]
fn cmd_index_status(path: Option<PathBuf>, json: bool) -> Result<()> {
    use chan_workspace::index::embeddings::{global_models_dir, repo_dir_name, resolve_model};
    let lib = library()?;
    let root = path
        .or_else(|| lib.default_workspace_root())
        .unwrap_or_else(|| lib.effective_default_workspace_root());
    let workspace_paths = lib
        .workspace_paths_for(&root)
        .ok_or_else(|| anyhow::anyhow!(not_a_chan_workspace_hint(&root)))?;
    // Canonical path comes back from the registry entry; falls back
    // to the user-supplied root if the registry lookup somehow
    // races (impossible while we hold a Library handle, but the
    // ladder keeps the display correct without panicking).
    let canonical_root = lib
        .list_workspaces()
        .into_iter()
        .find(|d| same_path(&d.root_path, &root))
        .map(|d| d.root_path)
        .unwrap_or(root);
    let cfg = chan_workspace::index::config::load(&workspace_paths.index).with_context(|| {
        format!(
            "reading index config at {}",
            workspace_paths.index.display()
        )
    })?;
    let model = cfg.model;
    let semantic_enabled = cfg.semantic_enabled;
    let expected_dir = global_models_dir().join(repo_dir_name(&model));
    let model_present = resolve_model(&model).is_ok();
    let model_size_bytes = if model_present {
        Some(dir_total_size(&expected_dir))
    } else {
        None
    };
    let mode = if semantic_enabled && model_present {
        "hybrid"
    } else {
        "bm25"
    };
    if json {
        // Emit `reports_enabled`
        // alongside `semantic_enabled` so chan-desktop's
        // `get_workspace_features` IPC can read both flags from one
        // CLI round-trip. `chan_workspace::index::config::load`
        // already populated both fields; this is a strict
        // additive extension (existing JSON consumers ignore
        // unknown fields).
        let body = serde_json::json!({
            "workspace": canonical_root.display().to_string(),
            "mode": mode,
            "model_present": model_present,
            "model_name": model,
            "model_path": expected_dir.display().to_string(),
            "model_size_bytes": model_size_bytes,
            "semantic_enabled": semantic_enabled,
            "reports_enabled": cfg.reports_enabled,
        });
        println!("{}", serde_json::to_string_pretty(&body)?);
    } else {
        println!("workspace:            {}", canonical_root.display());
        println!("mode:             {mode}");
        println!("model:            {model}");
        println!("model path:       {}", expected_dir.display());
        println!(
            "model present:    {}",
            if model_present {
                "yes"
            } else {
                "no (run `chan index download-model`)"
            }
        );
        if let Some(bytes) = model_size_bytes {
            println!("model size:       {}", humanize_bytes(bytes));
        }
        println!(
            "semantic enabled: {}",
            if semantic_enabled { "yes" } else { "no" }
        );
    }
    Ok(())
}

/// User-facing message when a CLI subcommand is
/// pointed at a path the registry doesn't know. Surfaces a clear
/// "not a chan workspace at <path>" hint with a `chan add` next-step
/// instead of leaking the implementation detail (auto-register
/// side-effect, `WorkspaceNotRegistered(<path>)`, etc.).
///
/// Gated on `embeddings` to match both
/// call sites (`cmd_index_set_semantic`, `cmd_index_status`).
/// Without the gate `--no-default-features` builds with
/// `RUSTFLAGS=-D warnings` fail on dead_code.
#[cfg(feature = "embeddings")]
fn not_a_chan_workspace_hint(root: &std::path::Path) -> String {
    format!(
        "not a chan workspace at {}; run `chan add {}` first",
        root.display(),
        root.display()
    )
}

/// Recursive size of every regular file under `dir`. Mirrors the
/// helper in `chan-server::routes::index` so the CLI status output
/// agrees with the API's `model_size_bytes` field.
#[cfg(feature = "embeddings")]
fn dir_total_size(dir: &std::path::Path) -> u64 {
    fn walk(dir: &std::path::Path, total: &mut u64) {
        let Ok(it) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in it.flatten() {
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_dir() {
                walk(&entry.path(), total);
            } else if ft.is_file() {
                if let Ok(meta) = entry.metadata() {
                    *total += meta.len();
                }
            }
        }
    }
    let mut total = 0;
    walk(dir, &mut total);
    total
}

#[cfg(feature = "embeddings")]
fn humanize_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    let b = bytes as f64;
    if b >= MB {
        format!("{:.1} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/// Run chan-llm's MCP server on stdio against `path`. Spawned by
/// external MCP clients through config files; not user-facing.
///
/// We deliberately do NOT auto-register the workspace here: the host
/// (chan-server) has already registered the workspace for
/// this workspace when the session started, and the MCP subprocess
/// inherits that registry. If the workspace isn't registered when the
/// agent invokes the subcommand, that's a wiring bug worth
/// surfacing rather than silently fixing.
async fn cmd_mcp(path: PathBuf) -> Result<()> {
    let workspace = library()?
        .open_workspace(&path)
        .with_context(|| format!("opening workspace {}", path.display()))?;
    chan_llm::mcp::Server::new(workspace)
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
    chan_server::run_mcp_stdio_proxy(socket)
        .await
        .context("running MCP proxy")
}

/// Windows stub: chan's MCP bridge runs over Unix-domain sockets; the
/// proxy subcommand has no counterpart on Windows. The CLI still
/// accepts `__mcp-proxy` so flag-parsing stays target-agnostic, but
/// invoking it fails fast instead of half-working.
#[cfg(not(unix))]
async fn cmd_mcp_proxy(_socket: PathBuf) -> Result<()> {
    anyhow::bail!("__mcp-proxy is unix-only");
}

/// Pick the CLI content-search mode, mirroring the `/api/search/content`
/// route: Hybrid (BM25 + dense, RRF-fused) only when the workspace opted
/// in via `semantic_enabled` AND the embedding model is on disk;
/// otherwise BM25. Keeping the CLI and the route on the same rule means
/// `chan search` and the editor's search panel agree on what ran.
#[cfg(feature = "embeddings")]
fn resolve_search_mode(workspace: &chan_workspace::Workspace) -> SearchMode {
    use chan_workspace::index::embeddings::resolve_model;
    let enabled = workspace.semantic_enabled().unwrap_or(false);
    let model_present = workspace
        .semantic_model()
        .map(|m| resolve_model(&m).is_ok())
        .unwrap_or(false);
    if enabled && model_present {
        SearchMode::Hybrid
    } else {
        SearchMode::Bm25
    }
}

/// Without the `embeddings` feature the dense stack is compiled out, so
/// the facade collapses Hybrid to BM25 anyway; request BM25 directly.
#[cfg(not(feature = "embeddings"))]
fn resolve_search_mode(_workspace: &chan_workspace::Workspace) -> SearchMode {
    SearchMode::Bm25
}

fn cmd_search(path: PathBuf, query: String, limit: u32) -> Result<()> {
    let lib = library()?;
    ensure_workspace_registered(&lib, &path)?;
    let workspace = lib.open_workspace(&path)?;
    let opts = SearchOpts {
        mode: resolve_search_mode(&workspace),
        limit,
        ..Default::default()
    };
    let res = workspace.search(&query, &opts).context("search")?;
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

#[derive(Serialize)]
struct WorkspaceListOutput {
    workspaces: Vec<WorkspaceListEntry>,
}

#[derive(Serialize)]
struct WorkspaceListEntry {
    path: String,
    /// Stable per-workspace metadata storage key under ~/.chan/workspaces/.
    metadata_key: String,
    /// RFC3339 UTC timestamp.
    last_seen_at: String,
}

impl From<&KnownWorkspace> for WorkspaceListEntry {
    fn from(d: &KnownWorkspace) -> Self {
        Self {
            path: d.root_path.display().to_string(),
            metadata_key: d.metadata_key.clone(),
            last_seen_at: d.last_seen_at.to_rfc3339(),
        }
    }
}

#[derive(Serialize)]
struct GraphQueryOutput {
    root: String,
    scope: &'static str,
    target: Option<String>,
    nodes: Vec<String>,
    edges: Vec<GraphEdgeOutput>,
}

#[derive(Serialize)]
struct GraphEdgeOutput {
    source: String,
    target: String,
    kind: &'static str,
    anchor: Option<String>,
}

#[derive(Serialize)]
struct StatusOutput {
    root: String,
    metadata_key: Option<String>,
    index: StatusIndex,
    graph: StatusGraph,
    report: StatusReport,
}

#[derive(Serialize)]
struct StatusIndex {
    ready: bool,
    indexed_docs: u64,
    indexed_vectors: u64,
    model: String,
}

#[derive(Serialize)]
struct StatusGraph {
    files: usize,
    edges: usize,
    tags: usize,
}

#[derive(Serialize)]
struct StatusReport {
    files: u64,
    code: u64,
    comments: u64,
    blanks: u64,
    complexity: u64,
    by_language: Vec<StatusLanguage>,
    cocomo_model: String,
    estimated_cost_usd: f64,
}

#[derive(Serialize)]
struct StatusLanguage {
    name: String,
    files: u64,
    code: u64,
}

#[derive(Serialize)]
struct ConfigOutput {
    editor: EditorPrefs,
    server: ServerConfig,
}

fn cmd_graph(
    path: PathBuf,
    scope: GraphScope,
    target: Option<String>,
    depth: usize,
    limit: usize,
    json: bool,
) -> Result<()> {
    let lib = library()?;
    ensure_workspace_registered(&lib, &path)?;
    let workspace = lib.open_workspace(&path)?;
    if scope != GraphScope::All {
        return cmd_filesystem_graph(&workspace, scope, target, depth, limit, json);
    }
    let graph = workspace.graph().context("opening graph")?;
    let nodes = graph_scope_nodes(&workspace, graph, scope, target.as_deref(), depth)?;
    let node_set: std::collections::BTreeSet<&str> = nodes.iter().map(String::as_str).collect();
    let mut edges = Vec::new();
    for src in &nodes {
        for edge in graph
            .neighbors(src)
            .with_context(|| format!("querying graph neighbors for {src}"))?
        {
            if scope == GraphScope::All || node_set.contains(edge.dst.as_str()) {
                edges.push(GraphEdgeOutput {
                    source: edge.src,
                    target: edge.dst,
                    kind: edge_kind_label(edge.kind),
                    anchor: edge.anchor,
                });
            }
        }
    }
    let out = GraphQueryOutput {
        root: workspace.root().display().to_string(),
        scope: graph_scope_label(scope),
        target,
        nodes,
        edges,
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }
    println!(
        "{} graph: {} nodes, {} edges",
        out.root,
        out.nodes.len(),
        out.edges.len()
    );
    for edge in out.edges.iter().take(limit) {
        let anchor = edge
            .anchor
            .as_deref()
            .map(|a| format!("#{a}"))
            .unwrap_or_default();
        println!(
            "{:<8} {} -> {}{}",
            edge.kind, edge.source, edge.target, anchor
        );
    }
    if out.edges.len() > limit {
        println!("... {} more edges", out.edges.len() - limit);
    }
    Ok(())
}

fn cmd_filesystem_graph(
    workspace: &chan_workspace::Workspace,
    scope: GraphScope,
    target: Option<String>,
    depth: usize,
    limit: usize,
    json: bool,
) -> Result<()> {
    let fs_scope = match scope {
        GraphScope::All => unreachable!("all scope is handled by cmd_graph"),
        GraphScope::File => ServerFsGraphScope::File,
        GraphScope::Directory => ServerFsGraphScope::Directory,
    };
    if scope == GraphScope::File && target.as_deref().unwrap_or("").is_empty() {
        anyhow::bail!("--target is required for --scope file");
    }
    let path = target.as_deref().unwrap_or("");
    let out =
        build_fs_graph(workspace, fs_scope, path, depth).context("building filesystem graph")?;
    if json {
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }
    print_filesystem_graph(&out, limit);
    Ok(())
}

fn print_filesystem_graph(out: &FsGraphResponse, limit: usize) {
    println!(
        "{} filesystem graph: {} nodes, {} edges, scope={}, depth={}, truncated={}",
        out.root,
        out.nodes.len(),
        out.edges.len(),
        out.scope,
        out.depth,
        out.truncated
    );
    for edge in out.edges.iter().take(limit) {
        println!("{:<8} {} -> {}", edge.kind, edge.source, edge.target);
    }
    if out.edges.len() > limit {
        println!("... {} more edges", out.edges.len() - limit);
    }
}

fn cmd_status(path: Option<PathBuf>, json: bool) -> Result<()> {
    let lib = library()?;
    let root = path
        .or_else(|| lib.default_workspace_root())
        .unwrap_or_else(|| lib.effective_default_workspace_root());
    ensure_workspace_registered(&lib, &root)?;
    let workspace = lib.open_workspace(&root)?;
    let known = lib
        .list_workspaces()
        .into_iter()
        .find(|d| same_path(&d.root_path, workspace.root()));
    let index = workspace.index_stats().context("reading index stats")?;
    let graph = workspace.graph().context("opening graph")?;
    let graph_files = graph.files().context("reading graph files")?;
    let mut graph_edges = 0usize;
    for file in &graph_files {
        graph_edges += graph
            .neighbors(file)
            .with_context(|| format!("querying graph neighbors for {file}"))?
            .len();
    }
    let tags = graph.tags().context("reading graph tags")?.len();
    let report = workspace.report().context("reading code report")?;
    let by_language = report
        .by_language
        .into_iter()
        .take(12)
        .map(|l| StatusLanguage {
            name: l.name,
            files: l.files,
            code: l.code,
        })
        .collect();
    let out = StatusOutput {
        root: workspace.root().display().to_string(),
        metadata_key: known.map(|d| d.metadata_key),
        index: StatusIndex {
            ready: index.ready,
            indexed_docs: index.indexed_docs,
            indexed_vectors: index.indexed_vectors,
            model: index.model,
        },
        graph: StatusGraph {
            files: graph_files.len(),
            edges: graph_edges,
            tags,
        },
        report: StatusReport {
            files: report.totals.files,
            code: report.totals.code,
            comments: report.totals.comments,
            blanks: report.totals.blanks,
            complexity: report.totals.complexity,
            by_language,
            cocomo_model: report.cocomo.model,
            estimated_cost_usd: report.cocomo.estimated_cost_usd,
        },
    };
    if json {
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }
    println!("workspace: {}", out.root);
    if let Some(metadata_key) = &out.metadata_key {
        println!("metadata: {metadata_key}");
    }
    println!(
        "index: ready={} docs={} vectors={} model={}",
        out.index.ready, out.index.indexed_docs, out.index.indexed_vectors, out.index.model
    );
    println!(
        "graph: files={} edges={} tags={}",
        out.graph.files, out.graph.edges, out.graph.tags
    );
    println!(
        "report: files={} code={} comments={} blanks={} complexity={} cocomo={} cost=${:.2}",
        out.report.files,
        out.report.code,
        out.report.comments,
        out.report.blanks,
        out.report.complexity,
        out.report.cocomo_model,
        out.report.estimated_cost_usd
    );
    if !out.report.by_language.is_empty() {
        println!("languages:");
        for lang in &out.report.by_language {
            println!(
                "  {:<18} files={:<5} code={}",
                lang.name, lang.files, lang.code
            );
        }
    }
    Ok(())
}

fn graph_scope_nodes(
    workspace: &chan_workspace::Workspace,
    graph: &chan_workspace::GraphView,
    scope: GraphScope,
    target: Option<&str>,
    depth: usize,
) -> Result<Vec<String>> {
    match scope {
        GraphScope::All => graph.files().context("reading graph files"),
        GraphScope::File => {
            let target = target.context("--target is required for --scope file")?;
            let target = target.trim_matches('/').to_string();
            let stat = workspace
                .stat(&target)
                .with_context(|| format!("stat graph file target `{target}`"))?;
            if stat.is_dir {
                anyhow::bail!("--scope file requires a file; `{target}` is a directory");
            }
            Ok(vec![target])
        }
        GraphScope::Directory => {
            let target = target.unwrap_or("").trim_matches('/');
            if !target.is_empty() {
                let stat = workspace
                    .stat(target)
                    .with_context(|| format!("stat graph directory target `{target}`"))?;
                if !stat.is_dir {
                    anyhow::bail!("--scope directory requires a directory; `{target}` is not");
                }
            }
            let entries = if target.is_empty() {
                workspace.list_tree().context("listing workspace tree")?
            } else {
                workspace
                    .list_tree_prefix(target)
                    .context("listing directory tree")?
            };
            let files: std::collections::BTreeSet<String> = graph
                .files()
                .context("reading graph files")?
                .into_iter()
                .collect();
            Ok(entries
                .into_iter()
                .filter(|e| !e.is_dir)
                .filter(|e| directory_depth_in_scope(&e.path, target, depth))
                .map(|e| e.path)
                .filter(|p| files.contains(p))
                .collect())
        }
    }
}

fn directory_depth_in_scope(path: &str, directory: &str, depth: usize) -> bool {
    if depth == 0 {
        return false;
    }
    let rel = if directory.is_empty() {
        path
    } else if path == directory {
        ""
    } else if let Some(rest) = path
        .strip_prefix(directory)
        .and_then(|s| s.strip_prefix('/'))
    {
        rest
    } else {
        return false;
    };
    !rel.is_empty() && rel.split('/').count() <= depth
}

fn graph_scope_label(scope: GraphScope) -> &'static str {
    match scope {
        GraphScope::All => "all",
        GraphScope::File => "file",
        GraphScope::Directory => "directory",
    }
}

fn edge_kind_label(kind: EdgeKind) -> &'static str {
    match kind {
        EdgeKind::Link => "link",
        EdgeKind::Mention => "mention",
        EdgeKind::Tag => "tag",
    }
}

fn cmd_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Get { key, json } => {
            let editor = EditorPrefs::load().context("loading editor preferences")?;
            let server = ServerConfig::load().context("loading server config")?;
            match key.as_deref() {
                None | Some("") => {
                    let output = ConfigOutput { editor, server };
                    if json {
                        println!("{}", serde_json::to_string_pretty(&output)?);
                    } else {
                        print!("{}", toml::to_string_pretty(&output)?);
                    }
                }
                Some(k) => {
                    let value = read_config_key(&editor, &server, k)?;
                    if json {
                        println!("{}", serde_json::to_string(&value)?);
                    } else {
                        println!("{}", scalar_to_string(&value));
                    }
                }
            }
            Ok(())
        }
        ConfigAction::Set { key, value } => {
            let (key, raw_value) = split_assignment(&key, value.as_deref())?;
            if key.starts_with("server.") {
                let mut cfg = ServerConfig::load().context("loading server config")?;
                write_server_config_key(&mut cfg, &key, &raw_value)?;
                cfg.save().context("saving server config")?;
            } else {
                let mut prefs = EditorPrefs::load().context("loading editor preferences")?;
                write_pref_key(&mut prefs, &key, &raw_value)?;
                prefs.save().context("saving editor preferences")?;
            }
            println!("{key} = {raw_value}");
            Ok(())
        }
    }
}

fn cmd_metadata(action: MetadataAction) -> Result<()> {
    match action {
        MetadataAction::Export { path, archive } => {
            let lib = library()?;
            let report = lib
                .export_metadata_archive(
                    &path,
                    &archive,
                    MetadataExportOptions {
                        chan_version: env!("CARGO_PKG_VERSION").to_string(),
                    },
                )
                .context("exporting metadata archive")?;
            println!(
                "exported {} files ({} bytes) to {}",
                report.files,
                report.bytes,
                report.archive_path.display()
            );
            println!("source metadata: {}", report.manifest.source_metadata_key);
            Ok(())
        }
        MetadataAction::Import {
            path,
            archive,
            rescan,
            force_scm,
        } => {
            let lib = library()?;
            ensure_workspace_registered(&lib, &path)?;
            let report = lib
                .import_metadata_archive(
                    &path,
                    &archive,
                    MetadataImportOptions { rescan, force_scm },
                )
                .context("importing metadata archive")?;
            println!(
                "imported {} files ({} bytes) from {}",
                report.files,
                report.bytes,
                archive.display()
            );
            println!("subtrees: {}", report.imported_subtrees.join(", "));
            if report.rescanned {
                println!("rescan: completed");
            }
            Ok(())
        }
        MetadataAction::Inspect { archive, json } => {
            let lib = library()?;
            let manifest = lib
                .inspect_metadata_archive(&archive)
                .context("inspecting metadata archive")?;
            if json {
                println!("{}", serde_json::to_string_pretty(&manifest)?);
            } else {
                println!("format: {}", manifest.archive_format_version);
                println!("chan: {}", manifest.chan_version);
                println!("created: {}", manifest.created_at);
                println!("source root: {}", manifest.source_root);
                println!("source metadata: {}", manifest.source_metadata_key);
                println!("subtrees: {}", manifest.included_subtrees.join(", "));
                if let Some(scm) = manifest.scm {
                    if !scm.remotes.is_empty() {
                        println!("scm remotes: {}", scm.remotes.join(", "));
                    }
                    if let Some(head) = scm.head {
                        println!("scm head: {head}");
                    }
                }
            }
            Ok(())
        }
    }
}

/// Accept both `chan config set k=v` and `chan config set k v`.
/// Returns `(key, value)`. Bails with a clear message on empty values
/// so a typo doesn't silently wipe a preference.
fn split_assignment(key: &str, value: Option<&str>) -> Result<(String, String)> {
    if let Some(v) = value {
        if v.is_empty() {
            anyhow::bail!("value must not be empty (got `{key}=`)");
        }
        return Ok((key.to_owned(), v.to_owned()));
    }
    if let Some((k, v)) = key.split_once('=') {
        let k = k.trim();
        let v = v.trim();
        if k.is_empty() {
            anyhow::bail!("key must not be empty");
        }
        if v.is_empty() {
            anyhow::bail!("value must not be empty (got `{key}`)");
        }
        return Ok((k.to_owned(), v.to_owned()));
    }
    anyhow::bail!("missing value: use `{key}=VALUE` or `{key} VALUE`")
}

fn read_config_key(
    editor: &EditorPrefs,
    server: &ServerConfig,
    key: &str,
) -> Result<serde_json::Value> {
    match key {
        "editor.theme" => Ok(serde_json::json!(theme_choice_label(editor.theme))),
        "editor.editor_theme" => Ok(serde_json::json!(editor_theme_label(editor.editor_theme))),
        "editor.line_spacing" => Ok(serde_json::json!(line_spacing_label(editor.line_spacing))),
        "editor.date_format" => Ok(serde_json::json!(editor.date_format.clone())),
        "editor.pane_widths.inspector" => Ok(serde_json::json!(editor.pane_widths.inspector)),
        "editor.pane_widths.graph" => Ok(serde_json::json!(editor.pane_widths.graph)),
        "editor.pane_widths.browser" => Ok(serde_json::json!(editor.pane_widths.browser)),
        "editor.pane_widths.search" => Ok(serde_json::json!(editor.pane_widths.search)),
        "editor.pane_widths.outline" => Ok(serde_json::json!(editor.pane_widths.outline)),
        "server.attachments_dir" => Ok(serde_json::json!(server.attachments_dir.clone())),
        "server.search.aggression" => Ok(serde_json::json!(server.search.aggression.as_str())),
        "server.terminal.idle_timeout_secs" => {
            Ok(serde_json::json!(server.terminal.idle_timeout_secs))
        }
        "server.terminal.session_cap" => Ok(serde_json::json!(server.terminal.session_cap)),
        "server.terminal.ring_bytes" => Ok(serde_json::json!(server.terminal.ring_bytes)),
        _ => Err(anyhow::anyhow!(
            "unknown key `{key}`; try `chan config get` to list current values"
        )),
    }
}

fn write_pref_key(prefs: &mut EditorPrefs, key: &str, value: &str) -> Result<()> {
    match key {
        "editor.theme" => {
            prefs.theme = parse_theme_choice(value)?;
        }
        "editor.editor_theme" => {
            prefs.editor_theme = parse_editor_theme(value)?;
        }
        "editor.line_spacing" => {
            prefs.line_spacing = parse_line_spacing(value)?;
        }
        "editor.date_format" => {
            prefs.date_format = value.to_owned();
        }
        "editor.pane_widths.inspector" => {
            prefs.pane_widths.inspector = parse_u32(key, value)?;
        }
        "editor.pane_widths.graph" => {
            prefs.pane_widths.graph = parse_u32(key, value)?;
        }
        "editor.pane_widths.browser" => {
            prefs.pane_widths.browser = parse_u32(key, value)?;
        }
        "editor.pane_widths.search" => {
            prefs.pane_widths.search = parse_u32(key, value)?;
        }
        "editor.pane_widths.outline" => {
            prefs.pane_widths.outline = parse_u32(key, value)?;
        }
        _ => {
            anyhow::bail!("unknown key `{key}`; try `chan config get` to list current values");
        }
    }
    Ok(())
}

fn write_server_config_key(cfg: &mut ServerConfig, key: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        anyhow::bail!("{key} must be non-empty");
    }
    match key {
        "server.attachments_dir" => {
            cfg.attachments_dir = value.to_owned();
        }
        "server.search.aggression" => {
            cfg.search.aggression = value
                .parse()
                .map_err(|e: String| anyhow::anyhow!("{key}: {e}"))?;
        }
        "server.terminal.idle_timeout_secs" => {
            cfg.terminal.idle_timeout_secs = parse_nonzero_u64(key, value)?;
        }
        "server.terminal.session_cap" => {
            cfg.terminal.session_cap = parse_nonzero_usize(key, value)?;
        }
        "server.terminal.ring_bytes" => {
            cfg.terminal.ring_bytes = parse_nonzero_usize(key, value)?;
        }
        _ => {
            anyhow::bail!("unknown key `{key}`; try `chan config get` to list current values");
        }
    }
    Ok(())
}

fn parse_theme_choice(value: &str) -> Result<ThemeChoice> {
    match value {
        "system" => Ok(ThemeChoice::System),
        "light" => Ok(ThemeChoice::Light),
        "dark" => Ok(ThemeChoice::Dark),
        _ => anyhow::bail!("expected system|light|dark, got `{value}`"),
    }
}

fn parse_editor_theme(value: &str) -> Result<EditorTheme> {
    match value {
        "github" => Ok(EditorTheme::Github),
        "google_docs" => Ok(EditorTheme::GoogleDocs),
        "word" => Ok(EditorTheme::Word),
        _ => anyhow::bail!("expected github|google_docs|word, got `{value}`"),
    }
}

fn parse_line_spacing(value: &str) -> Result<LineSpacing> {
    match value {
        "standard" => Ok(LineSpacing::Standard),
        "compact" => Ok(LineSpacing::Compact),
        // `tight` is an accepted legacy alias for `compact` (same
        // density target), so muscle memory and existing
        // scripts keep working; the canonical reader (`config get`)
        // echoes back `compact` so the user is nudged toward the new
        // spelling without losing their preference.
        "tight" => Ok(LineSpacing::Compact),
        _ => anyhow::bail!("expected standard|compact, got `{value}`"),
    }
}

fn parse_u32(key: &str, value: &str) -> Result<u32> {
    value
        .parse::<u32>()
        .with_context(|| format!("{key}: expected non-negative integer, got `{value}`"))
}

fn parse_nonzero_u64(key: &str, value: &str) -> Result<u64> {
    let parsed = value
        .parse::<u64>()
        .with_context(|| format!("{key} must be a positive integer"))?;
    if parsed == 0 {
        anyhow::bail!("{key} must be greater than 0");
    }
    Ok(parsed)
}

fn parse_nonzero_usize(key: &str, value: &str) -> Result<usize> {
    let parsed = value
        .parse::<usize>()
        .with_context(|| format!("{key} must be a positive integer"))?;
    if parsed == 0 {
        anyhow::bail!("{key} must be greater than 0");
    }
    Ok(parsed)
}

fn theme_choice_label(t: ThemeChoice) -> &'static str {
    match t {
        ThemeChoice::System => "system",
        ThemeChoice::Light => "light",
        ThemeChoice::Dark => "dark",
    }
}

fn editor_theme_label(t: EditorTheme) -> &'static str {
    match t {
        EditorTheme::Github => "github",
        EditorTheme::GoogleDocs => "google_docs",
        EditorTheme::Word => "word",
    }
}

fn line_spacing_label(s: LineSpacing) -> &'static str {
    match s {
        LineSpacing::Standard => "standard",
        LineSpacing::Compact => "compact",
    }
}

/// Render a single-value response without the JSON quotes / braces.
/// Strings unquote, numbers stringify, everything else falls back to
/// the JSON shape.
fn scalar_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

fn cmd_contacts_import_csv(
    file: PathBuf,
    into: String,
    provider: String,
    dry_run: bool,
    overwrite: bool,
    workspace: Option<PathBuf>,
) -> Result<()> {
    use chan_workspace::contacts::{
        google::parse_google_csv, slug::SlugAllocator, ImportOpts, ProviderKind,
    };

    // Provider gate. Only Google CSV today; the flag exists so the
    // help text and the wire shape are stable when more land.
    let prov =
        ProviderKind::parse(&provider).with_context(|| format!("unknown provider: {provider}"))?;
    if prov != ProviderKind::Google {
        anyhow::bail!("only --provider google is supported today");
    }

    // Parse the CSV up front. A bad file should fail before we
    // touch the workspace, so the user doesn't end up with a half-
    // created Contacts/ dir on a typo.
    let csv_bytes = std::fs::read(&file).with_context(|| format!("reading {}", file.display()))?;
    let contacts = parse_google_csv(csv_bytes.as_slice())
        .with_context(|| format!("parsing {}", file.display()))?;
    if contacts.is_empty() {
        println!("(no contacts in {})", file.display());
        return Ok(());
    }

    let lib = library()?;
    let root = workspace
        .or_else(|| lib.default_workspace_root())
        .unwrap_or_else(|| lib.effective_default_workspace_root());
    if !root.exists() {
        std::fs::create_dir_all(&root)
            .with_context(|| format!("creating workspace root {}", root.display()))?;
    }
    ensure_workspace_registered(&lib, &root)?;
    let workspace = lib.open_workspace(&root)?;

    if dry_run {
        // Mirror the orchestrator's slug + existence check loop
        // without writing. Existence checks against the live workspace
        // so SKIPPED / OVERWROTE labels are accurate.
        let dir_norm = into.trim_matches('/').to_string();
        let mut wrote = 0usize;
        let mut overwrote = 0usize;
        let mut skipped = 0usize;
        let on_disk = |p: &str| workspace.exists(p);
        let mut slugs = SlugAllocator::new(&dir_norm, &on_disk);
        for c in &contacts {
            let path = slugs.slug_for(c);
            let exists = workspace.exists(&path);
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

    let summary = workspace
        .import_contacts(&into, contacts, ImportOpts { overwrite })
        .context("importing contacts")?;
    print_import_summary(&summary);
    Ok(())
}

fn print_import_summary(summary: &chan_workspace::ImportSummary) {
    use chan_workspace::ImportOutcome;
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

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_app_bundle_climbs_to_dot_app() {
        // The real .app layout resolves to the bundle dir.
        let exe = PathBuf::from("/Applications/Chan.app/Contents/MacOS/chan-desktop");
        assert_eq!(
            macos_app_bundle(&exe),
            Some(PathBuf::from("/Applications/Chan.app"))
        );
        // A loose dev binary (cargo target dir) is not a bundle.
        assert_eq!(
            macos_app_bundle(&PathBuf::from("/Users/x/chan/target/debug/chan-desktop")),
            None
        );
        // A path shaped like a bundle but without the .app extension is not
        // a bundle either.
        assert_eq!(
            macos_app_bundle(&PathBuf::from("/x/Chan/Contents/MacOS/chan-desktop")),
            None
        );
    }

    #[test]
    fn absolutize_serve_root_is_always_absolute() {
        // The bug: a relative root (`.`) handed to the desktop made it
        // open "/". The invariant that fixes it is simply that the serve
        // root is always absolute before the handoff — regardless of
        // whether the dir exists yet.
        assert!(absolutize_serve_root(PathBuf::from(".")).is_absolute());
        assert!(absolutize_serve_root(PathBuf::from("does/not/exist/yet")).is_absolute());
        assert!(absolutize_serve_root(PathBuf::from("/tmp")).is_absolute());
        // A relative path lands under the cwd, not the filesystem root.
        let cwd = std::env::current_dir().unwrap();
        assert!(absolutize_serve_root(PathBuf::from("sub/dir")).starts_with(&cwd));
    }

    fn ipv4(s: &str) -> IpAddr {
        s.parse().unwrap()
    }
    fn ipv6(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    /// The fallback filter must (1) parse the static tokei directive
    /// without panicking at startup for every verbosity level and (2)
    /// actually carry the tokei cap. A malformed directive would panic
    /// the binary on launch; a dropped directive would let the spam back.
    #[test]
    fn fallback_filter_caps_tokei_for_every_level() {
        for level in ["warn", "info", "debug", "trace"] {
            let rendered = fallback_filter(level).to_string();
            assert!(
                rendered.contains("tokei"),
                "level {level} filter dropped the tokei directive: {rendered}"
            );
        }
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
    fn parse_search_aggression_accepts_known_levels() {
        assert_eq!(
            parse_search_aggression("conservative").unwrap(),
            SearchAggression::Conservative
        );
        assert_eq!(
            parse_search_aggression("balanced").unwrap(),
            SearchAggression::Balanced
        );
        assert_eq!(
            parse_search_aggression("aggressive").unwrap(),
            SearchAggression::Aggressive
        );
        assert!(parse_search_aggression("turbo").is_err());
    }

    #[test]
    fn index_model_subcommands_parse() {
        let cli = Cli::try_parse_from(["chan", "index", "list-models", "--json"]).unwrap();
        match cli.command {
            Command::Index {
                action: IndexAction::ListModels { json },
            } => assert!(json),
            other => panic!("unexpected command: {other:?}"),
        }

        let cli = Cli::try_parse_from([
            "chan",
            "index",
            "set-model",
            "--path",
            "/tmp/workspace",
            "--model",
            "BAAI/bge-base-en-v1.5",
        ])
        .unwrap();
        match cli.command {
            Command::Index {
                action: IndexAction::SetModel { path, model },
            } => {
                assert_eq!(path, Some(PathBuf::from("/tmp/workspace")));
                assert_eq!(model, "BAAI/bge-base-en-v1.5");
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn metadata_subcommands_parse() {
        let cli = Cli::try_parse_from([
            "chan",
            "metadata",
            "export",
            "/tmp/workspace",
            "/tmp/meta.tar.zst",
        ])
        .unwrap();
        match cli.command {
            Command::Metadata {
                action: MetadataAction::Export { path, archive },
            } => {
                assert_eq!(path, PathBuf::from("/tmp/workspace"));
                assert_eq!(archive, PathBuf::from("/tmp/meta.tar.zst"));
            }
            other => panic!("unexpected command: {other:?}"),
        }

        let cli = Cli::try_parse_from([
            "chan",
            "metadata",
            "import",
            "/tmp/workspace",
            "/tmp/meta.tar.zst",
            "--rescan",
            "--force-scm",
        ])
        .unwrap();
        match cli.command {
            Command::Metadata {
                action:
                    MetadataAction::Import {
                        path,
                        archive,
                        rescan,
                        force_scm,
                    },
            } => {
                assert_eq!(path, PathBuf::from("/tmp/workspace"));
                assert_eq!(archive, PathBuf::from("/tmp/meta.tar.zst"));
                assert!(rescan);
                assert!(force_scm);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn embedding_model_registry_json_uses_default_key() {
        let body = serde_json::to_value(chan_workspace::index::config::embedding_models()).unwrap();
        let first = &body.as_array().unwrap()[0];
        assert_eq!(first["id"], "BAAI/bge-small-en-v1.5");
        assert_eq!(first["default"], true);
        assert_eq!(first["dim"], 384);
        assert!(first.get("is_default").is_none());
    }

    #[test]
    fn directory_graph_scope_depth_matches_direct_children() {
        assert!(directory_depth_in_scope("notes/a.md", "notes", 1));
        assert!(!directory_depth_in_scope("notes/archive/a.md", "notes", 1));
        assert!(directory_depth_in_scope("notes/archive/a.md", "notes", 2));
        assert!(!directory_depth_in_scope("other/a.md", "notes", 2));
    }

    #[test]
    fn root_graph_scope_depth_matches_top_level_files() {
        assert!(directory_depth_in_scope("a.md", "", 1));
        assert!(!directory_depth_in_scope("notes/a.md", "", 1));
        assert!(directory_depth_in_scope("notes/a.md", "", 2));
        assert!(!directory_depth_in_scope("a.md", "", 0));
    }

    #[test]
    fn tunnel_workspace_flag_passes_through_when_valid() {
        let root = PathBuf::from("/tmp/whatever");
        let out = resolve_tunnel_workspace_name(Some("notes".into()), &root).unwrap();
        assert_eq!(out, "notes");
    }

    #[test]
    fn tunnel_workspace_flag_rejected_with_suggestion() {
        let root = PathBuf::from("/tmp/whatever");
        let err = resolve_tunnel_workspace_name(Some("My Workspace!".into()), &root).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("not URL-safe"), "{msg}");
        assert!(
            msg.contains("--tunnel-workspace-name=my-workspace"),
            "{msg}"
        );
    }

    #[test]
    fn tunnel_workspace_flag_rejected_when_unsanitizable() {
        let root = PathBuf::from("/tmp/whatever");
        let err = resolve_tunnel_workspace_name(Some("---".into()), &root).unwrap_err();
        assert!(err.to_string().contains("not URL-safe"));
    }

    #[test]
    fn tunnel_workspace_default_uses_path_basename() {
        let root = PathBuf::from("/tmp/Daily Journal");
        let out = resolve_tunnel_workspace_name(None, &root).unwrap();
        assert_eq!(out, "daily-journal");
    }

    #[test]
    fn tunnel_workspace_default_bails_when_basename_collapses() {
        let root = PathBuf::from("/tmp/---");
        let err = resolve_tunnel_workspace_name(None, &root).unwrap_err();
        assert!(err.to_string().contains("cannot derive"));
    }

    #[test]
    fn config_split_assignment_accepts_equals_form() {
        let (k, v) = split_assignment("editor.theme=dark", None).unwrap();
        assert_eq!(k, "editor.theme");
        assert_eq!(v, "dark");
    }

    #[test]
    fn config_split_assignment_accepts_two_args() {
        let (k, v) = split_assignment("editor.theme", Some("dark")).unwrap();
        assert_eq!(k, "editor.theme");
        assert_eq!(v, "dark");
    }

    #[test]
    fn config_split_assignment_rejects_empty_value() {
        // `chan config set editor.theme=` is the typo-with-trailing-`=`
        // form. We must refuse it so a bad invocation never wipes a
        // preference to "".
        let err = split_assignment("editor.theme=", None).unwrap_err();
        assert!(err.to_string().contains("must not be empty"));

        let err = split_assignment("editor.theme", Some("")).unwrap_err();
        assert!(err.to_string().contains("must not be empty"));
    }

    #[test]
    fn config_split_assignment_demands_a_value() {
        let err = split_assignment("editor.theme", None).unwrap_err();
        assert!(err.to_string().contains("missing value"));
    }

    #[test]
    fn config_read_then_write_round_trips_theme() {
        let mut prefs = EditorPrefs::default();
        write_pref_key(&mut prefs, "editor.theme", "dark").unwrap();
        assert_eq!(prefs.theme, ThemeChoice::Dark);
        let server = ServerConfig::default();
        let v = read_config_key(&prefs, &server, "editor.theme").unwrap();
        assert_eq!(v, serde_json::json!("dark"));
    }

    #[test]
    fn config_pane_width_round_trips_u32() {
        let mut prefs = EditorPrefs::default();
        write_pref_key(&mut prefs, "editor.pane_widths.search", "320").unwrap();
        assert_eq!(prefs.pane_widths.search, 320);
        let server = ServerConfig::default();
        let v = read_config_key(&prefs, &server, "editor.pane_widths.search").unwrap();
        assert_eq!(v, serde_json::json!(320));
    }

    #[test]
    fn config_server_paths_round_trip() {
        let editor = EditorPrefs::default();
        let mut server = ServerConfig::default();
        write_server_config_key(&mut server, "server.attachments_dir", "media/2026").unwrap();
        assert_eq!(server.attachments_dir, "media/2026");
        assert_eq!(
            read_config_key(&editor, &server, "server.attachments_dir").unwrap(),
            serde_json::json!("media/2026")
        );
    }

    #[test]
    fn config_search_aggression_round_trips() {
        let editor = EditorPrefs::default();
        let mut server = ServerConfig::default();
        write_server_config_key(&mut server, "server.search.aggression", "aggressive").unwrap();
        assert_eq!(server.search.aggression, SearchAggression::Aggressive);
        assert_eq!(
            read_config_key(&editor, &server, "server.search.aggression").unwrap(),
            serde_json::json!("aggressive")
        );
        let err =
            write_server_config_key(&mut server, "server.search.aggression", "turbo").unwrap_err();
        assert!(err
            .to_string()
            .contains("expected conservative|balanced|aggressive"));
    }

    #[test]
    fn config_server_paths_reject_empty_values() {
        let mut server = ServerConfig::default();
        let err = write_server_config_key(&mut server, "server.attachments_dir", "").unwrap_err();
        assert!(err.to_string().contains("non-empty"));
    }

    #[test]
    fn config_write_rejects_bad_theme_value() {
        let mut prefs = EditorPrefs::default();
        let err = write_pref_key(&mut prefs, "editor.theme", "neon").unwrap_err();
        assert!(err.to_string().contains("system|light|dark"));
    }

    #[test]
    fn config_line_spacing_accepts_canonical_tokens() {
        let mut prefs = EditorPrefs::default();
        write_pref_key(&mut prefs, "editor.line_spacing", "standard").unwrap();
        assert_eq!(prefs.line_spacing, LineSpacing::Standard);
        write_pref_key(&mut prefs, "editor.line_spacing", "compact").unwrap();
        assert_eq!(prefs.line_spacing, LineSpacing::Compact);
    }

    #[test]
    fn config_line_spacing_accepts_legacy_tight_alias() {
        // Older CLI scripts and muscle memory may still pass
        // `tight`; treat it as `compact` rather than erroring so
        // `chan config set` doesn't break those callers. The read
        // path normalizes the value back to `compact` (see
        // `line_spacing_label`).
        let mut prefs = EditorPrefs::default();
        write_pref_key(&mut prefs, "editor.line_spacing", "tight").unwrap();
        assert_eq!(prefs.line_spacing, LineSpacing::Compact);
        assert_eq!(line_spacing_label(prefs.line_spacing), "compact");
    }

    #[test]
    fn config_line_spacing_rejects_unknown_value() {
        let mut prefs = EditorPrefs::default();
        let err = write_pref_key(&mut prefs, "editor.line_spacing", "sparse").unwrap_err();
        assert!(err.to_string().contains("standard|compact"));
    }

    #[test]
    fn config_line_spacing_label_round_trips() {
        // Read path: `chan config get editor.line_spacing` echoes
        // the canonical lowercase token, not the legacy `tight`.
        assert_eq!(line_spacing_label(LineSpacing::Standard), "standard");
        assert_eq!(line_spacing_label(LineSpacing::Compact), "compact");
    }

    #[test]
    fn config_write_rejects_bad_pane_width_value() {
        let mut prefs = EditorPrefs::default();
        let err = write_pref_key(&mut prefs, "editor.pane_widths.search", "-1").unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("non-negative integer"),
            "expected validation error, got: {msg}"
        );
    }

    #[test]
    fn config_unknown_key_is_rejected() {
        let prefs = EditorPrefs::default();
        let server = ServerConfig::default();
        let err = read_config_key(&prefs, &server, "editor.nope").unwrap_err();
        assert!(err.to_string().contains("unknown key"));

        let mut prefs = EditorPrefs::default();
        let err = write_pref_key(&mut prefs, "editor.nope", "x").unwrap_err();
        assert!(err.to_string().contains("unknown key"));

        let mut server = ServerConfig::default();
        let err = write_server_config_key(&mut server, "server.nope", "x").unwrap_err();
        assert!(err.to_string().contains("unknown key"));
    }

    // --- graph_scope_nodes rejection coverage (syseng-1 residuals 1+2) ---
    //
    // syseng's hardening pass observed `chan graph --target ../etc/hosts`
    // and `chan graph --target notes/no-such-file.md` returning
    // `1 nodes, 0 edges` with exit 0 instead of a clear rejection.
    // `graph_scope_nodes` now stats the target through chan-workspace and
    // bails on escape / missing / wrong-type; these tests pin that.

    fn open_graph_test_workspace() -> (
        tempfile::TempDir,
        tempfile::TempDir,
        std::sync::Arc<chan_workspace::Workspace>,
    ) {
        let cfg = tempfile::TempDir::new().unwrap();
        let workspace_root = tempfile::TempDir::new().unwrap();
        let lib = chan_workspace::Library::open_at(cfg.path().join("config.toml")).unwrap();
        lib.register_workspace(workspace_root.path()).unwrap();
        let workspace = lib.open_workspace(workspace_root.path()).unwrap();
        // Lay down a couple of files so the graph view has something
        // to read.
        workspace.write_text("notes/a.md", "# A\n").unwrap();
        workspace.write_text("notes/sub/b.md", "# B\n").unwrap();
        workspace.reindex(None).unwrap();
        (cfg, workspace_root, workspace)
    }

    #[test]
    fn graph_scope_file_rejects_escape_target() {
        let (_cfg, _root, workspace) = open_graph_test_workspace();
        let graph = workspace.graph().unwrap();
        let err = graph_scope_nodes(&workspace, graph, GraphScope::File, Some("../etc/hosts"), 1)
            .unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("escapes workspace root") || msg.contains("PathEscape"),
            "expected escape rejection, got: {msg}"
        );
    }

    #[test]
    fn graph_scope_file_rejects_missing_target() {
        let (_cfg, _root, workspace) = open_graph_test_workspace();
        let graph = workspace.graph().unwrap();
        let err = graph_scope_nodes(
            &workspace,
            graph,
            GraphScope::File,
            Some("notes/no-such-file.md"),
            1,
        )
        .unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("No such file")
                || msg.contains("not found")
                || msg.contains("cannot find"),
            "expected missing-file rejection, got: {msg}"
        );
    }

    #[test]
    fn graph_scope_file_rejects_directory_target() {
        // --scope file with a directory must surface a clear error,
        // not silently succeed with an empty graph.
        let (_cfg, _root, workspace) = open_graph_test_workspace();
        let graph = workspace.graph().unwrap();
        let err =
            graph_scope_nodes(&workspace, graph, GraphScope::File, Some("notes"), 1).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("requires a file"),
            "expected directory rejection, got: {msg}"
        );
    }

    #[test]
    fn graph_scope_directory_rejects_escape_target() {
        let (_cfg, _root, workspace) = open_graph_test_workspace();
        let graph = workspace.graph().unwrap();
        let err = graph_scope_nodes(&workspace, graph, GraphScope::Directory, Some("../etc"), 1)
            .unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("escapes workspace root") || msg.contains("PathEscape"),
            "expected escape rejection, got: {msg}"
        );
    }
}
