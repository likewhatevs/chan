// chan: an AI-native workspace for your Markdown notes and projects.
//
// This library holds the whole `chan` CLI surface so two binaries can
// drive it: the standalone `chan` binary (`src/main.rs`, a thin shim
// calling `run(.., Personality::Standalone)`) and chan-desktop, which
// dispatches `chan` in-process when invoked through a `~/.local/bin/chan`
// shim (`Personality::Desktop`). The only behavioural fork between the two
// is the `Personality` passed to [`run`]: see `cmd_serve` (browser vs
// desktop handoff) and `chan upgrade` (CLI tarball replace vs desktop
// updater).
//
// The top-level surface carries the process-lifecycle and app-level
// commands; the workspace registry and per-workspace content operations
// are grouped under `chan workspace`:
//
//   chan workspace add <path>       register a directory as a chan
//                                   workspace in ~/.chan/config.toml
//   chan workspace ls [--json]      list registered workspaces,
//                                   most-recent first. --json emits
//                                   a stable machine-readable shape.
//   chan workspace rm <path>        drop a workspace from the registry
//                                   (filesystem contents untouched)
//   chan workspace index <path>     rebuild the search index + graph
//   chan workspace search <path> <query>
//                                   query the BM25 index
//   chan workspace graph <path>     inspect semantic or filesystem graph edges
//   chan workspace status [path]    report workspace/index/graph health
//   chan workspace metadata export PATH ARCHIVE.tar.zst
//                                   export a workspace's chan metadata
//   chan workspace contacts import csv FILE --into DIR
//                                   import a Google Contacts CSV as one
//                                   markdown note per contact under DIR
//   chan open {PATH} [-4|-6] [--host H --port N]
//                                   register + serve a workspace. Defaults
//                                   to 127.0.0.1 (loopback only); -6 picks
//                                   ::1 instead. The embedded web editor
//                                   talks to this. With chan-desktop running
//                                   it hands the workspace to a native window.
//   chan open {URL} [--name --script]
//                                   register a devserver (scheme://host) with
//                                   the desktop instead of serving a path.
//   chan close {PATH} [--remove]    tear down a workspace's server; --remove
//                                   also forgets it from the registry.
//   chan config get [KEY]           print a preference value
//   chan config set KEY=VALUE       update a preference
//
// Anything that touches the registry / workspace contents goes through
// `chan_workspace::Library` and `chan_workspace::Workspace` so the library's
// invariants (atomic writes, path sandbox, special-file refusal,
// cross-process writer lock) apply uniformly.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use chan_server::{
    build_fs_graph, EditorPrefs, EditorTheme, FsGraphResponse, FsGraphScope as ServerFsGraphScope,
    LineSpacing, ServeConfig, ServerConfig, ThemeChoice,
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

/// Extended `long_about` for `chan open` (the workspace-serve form). The
/// keybindings list is generated from `web/src/state/shortcuts.ts` (the
/// single source of truth for chan's chords). Resync after any change to that file
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
  Graph                                        Cmd+Shift+M   (or Mod+. M (Hybrid Nav))
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
    /// Manage a chan workspace: register, list, and forget
    /// workspaces, and drive a workspace's content (index, reports,
    /// search, graph, status, metadata, contacts).
    ///
    /// Every registry mutation and content operation routes through
    /// `chan_workspace::Library` / `Workspace` so atomic writes, the
    /// path sandbox, the special-file refusal, and the cross-process
    /// writer lock apply uniformly.
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
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
    /// Tear down a running server for a workspace, releasing its writer
    /// lock — the inverse of `chan open {path}`. "Not currently served" is
    /// treated as success (close is idempotent). With `--remove` it then
    /// also forgets the workspace from the registry, like `chan workspace
    /// rm`, independent of whether anything was serving it.
    Close {
        #[arg(value_hint = clap::ValueHint::AnyPath)]
        path: PathBuf,
        /// After tearing down the server, also forget the workspace from the
        /// registry (filesystem contents untouched). Runs regardless of the
        /// teardown outcome.
        #[arg(long)]
        remove: bool,
    },
    /// Open a workspace, or register a devserver.
    ///
    /// `chan open {PATH}` registers the directory as a workspace (creating
    /// it if needed) and serves it: with chan-desktop running it hands the
    /// workspace to a native window and returns; otherwise it binds a local
    /// loopback server and prints the URL. Serving is load-bearing — a bare
    /// `chan workspace add` only registers; serving mounts the workspace so
    /// the editor, terminal, and devserver can reach it.
    ///
    /// `chan open {URL}` (a `scheme://host[:port]` value) registers a
    /// devserver with the desktop instead: the `{url, name, script}` entry
    /// lands in the same config the launcher reads, and the desktop dials
    /// it. Needs a running chan-desktop; `--name` / `--script` apply only to
    /// this form.
    #[command(long_about = SERVE_LONG_ABOUT)]
    Open {
        /// A local workspace PATH, or a devserver URL (scheme://host[:port]).
        /// A value containing `://` is treated as a devserver URL; anything
        /// else is a path.
        target: Option<String>,
        /// (URL form) Optional label for the devserver's launcher section.
        #[arg(long)]
        name: Option<String>,
        /// (URL form) Optional connect script the desktop runs before it
        /// dials the devserver.
        #[arg(long)]
        script: Option<String>,
        /// (PATH form) Serve the given path verbatim instead of suggesting
        /// the enclosing VCS repository root. Without this flag, `chan
        /// open` refuses to start when the workspace path lives inside
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
        /// proxy multiplex many `chan open` instances under one host
        /// (e.g. `workspace.example.com/{user}/`). Canonicalized to
        /// `/seg[/seg...]` with `[A-Za-z0-9-]+` segments; trailing
        /// slashes and `//` runs are tolerated. Anything else is
        /// rejected.
        #[arg(long)]
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
        /// Force a standalone server: bind this workspace directly and skip
        /// both the chan-desktop handoff and the local devserver
        /// registration, even when one is running on this box. The escape
        /// hatch for automation and for serving a workspace the local
        /// devserver / desktop should not take over.
        #[arg(long)]
        standalone: bool,
    },
    /// Show which registered workspaces are currently being served, and
    /// by what.
    ///
    /// A live writer-lock holder means the workspace is served; the
    /// holder's pid and start time come from its `writer.lock` record.
    /// The serving kind (standalone `serve` / chan-desktop / devserver)
    /// is resolved from the holder's control socket when reachable, and
    /// shown as `served` otherwise.
    Ps {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Run a headless multi-workspace devserver on one address.
    ///
    /// Aggregates many workspaces behind one port: a `chan open <path>`
    /// on this box registers its workspace with the running devserver and
    /// exits instead of binding its own server, so the devserver owns each
    /// workspace's single-writer flock. A desktop client lists, opens, and
    /// forgets workspaces over the management API. What was mounted comes
    /// back on the next start.
    Devserver {
        /// Host address to bind. Default 127.0.0.1 (loopback). Use
        /// 0.0.0.0 / :: to listen on all interfaces; there is no TLS and
        /// only a bearer-token gate, so reach a remote devserver over an
        /// `ssh -L` tunnel rather than binding it on a public interface.
        #[arg(long, default_value = "127.0.0.1")]
        bind: IpAddr,
        /// Port to bind.
        #[arg(long, default_value_t = 8787)]
        port: u16,
        /// Run under a systemd user service (Linux): create and start the
        /// `chan-devserver.service` user unit (re-attaching if it is already
        /// running), so it survives the launching shell and logout, then
        /// follow its journal. Off Linux, runs in the foreground.
        #[arg(long)]
        systemd: bool,
        /// Run under a macOS launchd LaunchAgent (`app.chan.devserver`):
        /// write and load the agent (re-attaching if it is already running),
        /// so it survives the launching shell, then follow its log. Off
        /// macOS, runs in the foreground. Mutually exclusive with --systemd.
        #[arg(long, conflicts_with = "systemd")]
        launchd: bool,
        /// Tunnel endpoint URL. With --tunnel-token, the devserver also dials
        /// this gateway and publishes its tenant content at
        /// `{user}.devserver.chan.app/{workspace}/*`, alongside the local
        /// management server.
        #[arg(long, default_value = "https://devserver.chan.app/v1/tunnel")]
        tunnel_url: String,
        /// Personal access token (chan_pat_*) from id.chan.app. Setting this
        /// enables tunnel mode: the devserver dials the gateway and publishes
        /// every mounted workspace behind one registration. The devserver
        /// identity is resolved backend-side from the token, so there is no
        /// name to pass. Prefer the CHAN_TUNNEL_TOKEN env var so the secret
        /// does not appear in `ps`.
        #[arg(long, env = "CHAN_TUNNEL_TOKEN")]
        tunnel_token: Option<String>,
    },
    /// Read or write settings persisted outside the workspace. Keys use
    /// the same namespaces as the web Settings overlay where possible
    /// (`editor.*`, `server.*`).
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Self-upgrade: read release metadata from chan.app, download
    /// the selected CLI asset, verify SHA256, and atomically replace
    /// the running binary. Set `CHAN_UPDATE_CHECK=0` to silence the
    /// banner that fires on `chan open` startup.
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
    /// by a running `chan open`. Connects to the per-server Unix-
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
}

/// Subcommands for `chan workspace`. Groups the workspace-registry
/// operations (add / ls / rm) with the per-workspace content
/// operations (index / reports / search / graph / status / metadata /
/// contacts) under one verb, so the top-level surface carries only the
/// process-lifecycle and app-level commands (open, close, devserver,
/// config, ...). Mirrors the `IndexAction` / `ReportsAction`
/// sub-enum pattern.
#[derive(Subcommand, Debug)]
enum WorkspaceAction {
    /// Register a directory as a chan workspace.
    ///
    /// The baseline filesystem walk + markdown read + documentation
    /// graph + BM25 always runs. Semantic search is an optional
    /// layer, off by default to keep workspaces lean. chan-reports
    /// is on by default for new workspaces (`chan workspace reports
    /// disable` turns it off).
    Add {
        path: PathBuf,
        /// Enable per-workspace semantic search (BGE-small
        /// dense vectors). Per-workspace footprint; needs the shared
        /// model (`chan workspace index download-model`). Off by
        /// default.
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
    Ls {
        /// Emit machine-readable JSON:
        /// `{"workspaces":[{path,metadata_key,last_seen_at},...]}`.
        /// `last_seen_at` is RFC3339 UTC. The text format is
        /// unchanged when this flag is omitted.
        #[arg(long)]
        json: bool,
    },
    /// Drop a workspace from the registry. Does not delete the
    /// directory or its content; only forgets it on this machine.
    Rm {
        #[arg(value_hint = clap::ValueHint::AnyPath)]
        path: PathBuf,
    },
    /// Rebuild the search index + graph; manage the embedding
    /// model + per-workspace Hybrid-search opt-in. Subcommand-driven
    /// (rather than a flat `chan workspace index <path>`)
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
        /// Workspace root (required).
        path: Option<PathBuf>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Import and export chan metadata for a registered workspace.
    Metadata {
        #[command(subcommand)]
        action: MetadataAction,
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
        /// Workspace root (required).
        /// Auto-registers the path if not already known, so
        /// `chan workspace contacts import csv ... --workspace /some/dir`
        /// works without a prior `chan workspace add`.
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
        /// Archive path created by `chan workspace metadata export`.
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
        /// Archive path created by `chan workspace metadata export`.
        archive: PathBuf,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

/// Subcommands for `chan workspace index`. Subcommand-driven (rather than a
/// flat `chan workspace index <path>`) so the surface
/// covers rebuild, model download, semantic-search toggle, and
/// state inspection. Older scripts' flat `chan workspace index <path>` is now
/// `chan workspace index rebuild <path>`.
///
/// Symmetric naming matches the `chan workspace reports
/// enable/disable` parallel pair so scripted callers can pattern-
/// match `<feature> enable / disable` across the surface.
#[derive(Subcommand, Debug)]
enum IndexAction {
    /// Rebuild the search index + graph for a workspace. Older
    /// scripts used a flat `chan workspace index <path>`; the explicit verb keeps it
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
        /// Workspace root (required).
        #[arg(long)]
        path: Option<PathBuf>,
        /// Curated HuggingFace model id.
        #[arg(long)]
        model: String,
    },
    /// Flip the workspace's Hybrid-search opt-in. Refuses if the model
    /// isn't downloaded; the error points at `chan workspace index
    /// download-model`. The flag persists in
    /// `<index_dir>/config.toml` so it survives `chan open`
    /// restarts.
    EnableSemantic {
        /// Workspace root (required).
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Flip the workspace back to BM25-only.
    DisableSemantic {
        /// Workspace root (required).
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Print the semantic-search state: current mode, model
    /// presence, model path + size, opt-in flag.
    Status {
        /// Workspace root (required).
        #[arg(long)]
        path: Option<PathBuf>,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

/// Subcommands for `chan workspace reports`. Mirrors
/// `IndexAction::{EnableSemantic,DisableSemantic}`'s shape so
/// scripted callers can pattern-match `<feature> enable / disable`
/// uniformly across the surface (`chan workspace index enable-semantic` /
/// `chan workspace reports enable`).
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
        /// Workspace root (required).
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
///   the `cs -> chan` symlink). `chan open` always runs its own server and
///   opens the browser; it never hands off to a running chan-desktop.
///   `chan upgrade` replaces the CLI tarball in place.
/// - [`Personality::Desktop`] — chan-desktop invoked as `chan` (via the
///   `~/.local/bin/chan` shim). `chan open` integrates with the desktop:
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
        Command::Workspace { action } => match action {
            WorkspaceAction::Add {
                path,
                semantic_search,
                reports,
            } => cmd_add(path, semantic_search, reports),
            WorkspaceAction::Ls { json } => cmd_list(json),
            WorkspaceAction::Rm { path } => cmd_remove(path).await,
            WorkspaceAction::Index { action } => cmd_index(action),
            WorkspaceAction::Reports { action } => cmd_reports(action),
            WorkspaceAction::Search { path, query, limit } => cmd_search(path, query, limit),
            WorkspaceAction::Graph {
                path,
                scope,
                target,
                depth,
                limit,
                json,
            } => cmd_graph(path, scope, target, depth, limit, json),
            WorkspaceAction::Status { path, json } => cmd_status(path, json),
            WorkspaceAction::Metadata { action } => cmd_metadata(action),
            WorkspaceAction::Contacts { action } => match action {
                ContactsAction::Import { source } => match source {
                    ImportSource::Csv {
                        file,
                        into,
                        provider,
                        dry_run,
                        overwrite,
                        workspace,
                    } => {
                        cmd_contacts_import_csv(file, into, provider, dry_run, overwrite, workspace)
                    }
                },
            },
        },
        Command::Shell { action } => chan_shell::dispatch(action).await,
        Command::Completions { shell } => cmd_completions(shell),
        Command::Close { path, remove } => cmd_close(path, remove).await,
        Command::Open {
            target,
            name,
            script,
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
            standalone,
        } => {
            // Polymorphic dispatch: a `scheme://host` value registers a
            // devserver via the desktop handoff; anything else is a local
            // workspace path that gets registered + served.
            match target {
                Some(t) if looks_like_devserver_url(&t) => {
                    cmd_open_devserver(t, name, script).await
                }
                _ => {
                    let addr = resolve_listen_addr(host, ipv4, ipv6, port)?;
                    let prefix = chan_server::sanitize_prefix(prefix.as_deref().unwrap_or(""))
                        .map_err(|e| anyhow::anyhow!("invalid --prefix: {e}"))?;
                    cmd_serve(
                        ServeArgs {
                            addr,
                            prefix,
                            idle_timeout: timeout,
                            path: target.map(PathBuf::from),
                            here,
                            no_token,
                            no_browser,
                            search_aggression,
                            no_settings,
                            standalone,
                            verbose,
                        },
                        personality,
                    )
                    .await
                }
            }
        }
        Command::Ps { json } => cmd_ps(json).await,
        Command::Devserver {
            bind,
            port,
            systemd,
            launchd,
            tunnel_url,
            tunnel_token,
        } => cmd_devserver(bind, port, systemd, launchd, tunnel_url, tunnel_token).await,
        Command::Config { action } => cmd_config(action),
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
/// is default-off (`DashboardConfig::reports_enabled = false`), so on a source
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

fn ensure_workspace_registered(
    lib: &Library,
    root: &Path,
) -> Result<chan_workspace::KnownWorkspace> {
    lib.register_workspace(root)
        .with_context(|| format!("registering {}", root.display()))
}

fn cmd_add(path: PathBuf, semantic_search: bool, reports: bool) -> Result<()> {
    // Mirror `chan open`'s behavior: create the directory if it
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
    // boot-time activation so a `chan workspace add --reports` lands the
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

/// The process serving a workspace, behind its writer-lock holder.
/// Produced by `serving_kind`'s `Identify` round-trip; serializes to
/// `standalone` / `desktop` / `devserver` for `chan ps --json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ServedBy {
    /// A dedicated `chan open` bound to this one workspace.
    Standalone,
    /// chan-desktop's embedded server.
    Desktop,
    /// A multi-workspace `chan devserver`.
    Devserver,
}

impl ServedBy {
    fn label(self) -> &'static str {
        match self {
            ServedBy::Standalone => "standalone",
            ServedBy::Desktop => "desktop",
            ServedBy::Devserver => "devserver",
        }
    }
}

/// One `chan ps` row: a registered workspace and its serving state.
#[derive(Serialize)]
struct PsRow {
    path: String,
    served: bool,
    /// `None` when free, or served but the kind is not yet resolved.
    served_by: Option<ServedBy>,
    pid: Option<u32>,
    /// RFC3339 lock-acquisition time of the holder.
    since: Option<String>,
}

#[derive(Serialize)]
struct PsOutput {
    workspaces: Vec<PsRow>,
}

/// `chan ps`: report each registered workspace's serving state. Serving
/// is decided by a live writer-lock holder (`lock::is_free` is false);
/// the holder's pid + start time come from the `writer.lock` record.
async fn cmd_ps(json: bool) -> Result<()> {
    let lib = library()?;
    let mut rows = Vec::new();
    for ws in lib.list_workspaces() {
        let lock_dir = lib.workspace_paths_for(&ws.root_path).map(|p| p.lock);
        let served = lock_dir
            .as_deref()
            .map(|d| !chan_workspace::lock::is_free(d))
            .unwrap_or(false);
        let record = if served {
            lock_dir
                .as_deref()
                .and_then(chan_workspace::lock::read_lock_record)
        } else {
            None
        };
        let pid = record.as_ref().map(|r| r.pid);
        let since = record.map(|r| r.started_at);
        let served_by = match (served, pid) {
            (true, Some(p)) => serving_kind(p).await,
            _ => None,
        };
        rows.push(PsRow {
            path: ws.root_path.display().to_string(),
            served,
            served_by,
            pid,
            since,
        });
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&PsOutput { workspaces: rows })?
        );
        return Ok(());
    }
    if rows.is_empty() {
        println!("(no workspaces registered)");
        return Ok(());
    }
    println!("{:<7}  {:<11}  {:>8}  WORKSPACE", "STATE", "BY", "PID");
    for r in &rows {
        let state = if r.served { "served" } else { "free" };
        let by = match r.served_by {
            Some(k) => k.label(),
            None if r.served => "served",
            None => "-",
        };
        let pid = r.pid.map_or_else(|| "-".to_string(), |p| p.to_string());
        println!("{:<7}  {:<11}  {:>8}  {}", state, by, pid, r.path);
    }
    Ok(())
}

/// Resolve the serving kind behind `holder_pid` with an `Identify`
/// round-trip to its control socket. Returns `None` when the holder has
/// no reachable control socket or does not answer; `chan ps` then shows
/// `served` without a kind.
async fn serving_kind(holder_pid: u32) -> Option<ServedBy> {
    let socket = control_socket_for_pid(holder_pid)?;
    let message = chan_shell::send_control_request(&socket, chan_shell::ControlRequest::Identify)
        .await
        .ok()?;
    let identity: chan_shell::Identity = serde_json::from_str(&message).ok()?;
    Some(match identity.kind {
        chan_shell::ServeKind::Standalone => ServedBy::Standalone,
        chan_shell::ServeKind::Desktop => ServedBy::Desktop,
        chan_shell::ServeKind::Devserver => ServedBy::Devserver,
    })
}

async fn cmd_remove(path: PathBuf) -> Result<()> {
    let lib = library()?;
    // Tear down a running serve first: `reset_workspace` takes the writer
    // flock and would otherwise fail `WorkspaceLocked` on a live serve.
    // Best-effort — if we can't reach the holder, fall through and let the
    // reset surface the real error.
    // `remove: true` so a devserver/desktop host also unregisters the
    // workspace from its own library + overlay (not just the local config.toml).
    let _ = unserve_running(&lib, &path, true).await;
    remove_from_registry(&lib, &path)
}

/// Forget `path` from the registry: drop the registry key and the whole
/// `~/.chan/workspaces/<key>/` metadata dir (trash included), leaving the
/// filesystem contents untouched. Shared by `chan workspace rm` and `chan
/// close --remove`. The caller is responsible for tearing down any running
/// serve first (`unregister_workspace` does not).
fn remove_from_registry(lib: &Library, path: &Path) -> Result<()> {
    // Capture the metadata root before `unregister_workspace` drops the
    // registry key (after which the path no longer resolves to it).
    let metadata_root = lib.workspace_paths_for(path).map(|p| p.root);
    let removed = lib
        .unregister_workspace(path)
        .with_context(|| format!("unregistering {}", path.display()))?;
    if removed {
        // `reset_workspace(Everything)` deliberately preserves the trash +
        // lock dirs (other callers rely on that). Forgetting a workspace means
        // "forget everything", so drop the whole metadata dir — trash
        // included — leaving no `~/.chan/workspaces/<key>/` behind.
        if let Some(root) = metadata_root {
            let _ = std::fs::remove_dir_all(&root);
        }
        println!("unregistered: {}", path.display());
    } else {
        println!("(not registered: {})", path.display());
    }
    Ok(())
}

/// `chan close {path}`: tear down a running server holding `path`, releasing
/// its writer lock. Best-effort — "not currently served" (and an unreachable
/// holder) is treated as success, since the goal is "this workspace is not
/// served". With `remove`, it then also forgets the workspace from the
/// registry (`chan workspace rm`), INDEPENDENT of the teardown outcome.
async fn cmd_close(path: PathBuf, remove: bool) -> Result<()> {
    let lib = library()?;
    // Pass `remove` through so a host (devserver/desktop) that serves this
    // workspace also unregisters it from its own library + overlay; the local
    // `remove_from_registry` below then handles the caller's config.toml +
    // metadata (and the not-served / standalone cases the host can't).
    match unserve_running(&lib, &path, remove).await {
        Ok(UnserveOutcome::Unserved) => println!("closed: {}", path.display()),
        Ok(UnserveOutcome::NotServed) => println!("(not served: {})", path.display()),
        // A reachable-but-failed teardown is still "best effort": report it,
        // then (with --remove) forget the workspace anyway.
        Err(e) => eprintln!(
            "chan: could not reach the server for {} ({e}); treating as closed.",
            path.display()
        ),
    }
    if remove {
        remove_from_registry(&lib, &path)?;
    }
    Ok(())
}

enum UnserveOutcome {
    /// A live holder was reached and told to unserve; its flock released.
    Unserved,
    /// No live process holds the workspace (unregistered, no lock record,
    /// or the recorded holder is gone).
    NotServed,
}

/// Shared by `chan close` and `chan workspace rm`. Discovers the process
/// serving `path` from its `writer.lock` record, reaches it over its
/// control socket, asks it to tear down (the server decides scope: a
/// dedicated serve exits, a devserver/desktop unmounts just that tenant),
/// and waits for the flock to release.
///
/// With `remove`, a HOST (devserver / desktop) also UNREGISTERS the workspace
/// from its library + overlay, so the removal is reflected in the host's own
/// registry — not just the caller's local `config.toml`. This is what keeps a
/// devserver-served workspace from lingering in the launcher (and surviving a
/// restart) after `chan close --remove` / `chan workspace rm`.
async fn unserve_running(lib: &Library, path: &Path, remove: bool) -> Result<UnserveOutcome> {
    let Some(paths) = lib.workspace_paths_for(path) else {
        return Ok(UnserveOutcome::NotServed); // not registered => nothing serving
    };
    let Some(record) = chan_workspace::lock::read_lock_record(&paths.lock) else {
        return Ok(UnserveOutcome::NotServed); // no holder record on disk
    };
    let Some(socket) = control_socket_for_pid(record.pid) else {
        // A record but no reachable control socket: the holder is gone
        // (stale record — the lock is free / steal-able) or runs no control
        // socket. Nothing to tear down over the wire.
        return Ok(UnserveOutcome::NotServed);
    };
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    chan_shell::send_control_request(
        &socket,
        chan_shell::ControlRequest::Close {
            path: canonical,
            remove,
        },
    )
    .await
    .with_context(|| format!("asking the server (pid {}) to tear down", record.pid))?;
    wait_for_lock_release(&paths.lock);
    Ok(UnserveOutcome::Unserved)
}

/// Find a control socket for `pid` by its well-known name
/// (`$TMPDIR/chan-control-<pid>-<rand>.sock`). A dedicated `chan open` serve
/// has exactly one; a multi-tenant devserver has one per tenant under the
/// same pid. Either way every socket routes the `Close { path }` verb to the
/// server, which acts by path — so the first match is sufficient and we
/// must NOT broadcast (once the first tenant unmounts, the rest 404).
/// Returns `None` where the socket isn't a temp-dir file (Windows named
/// pipes aren't enumerable here — teardown over the wire is unix-first this
/// round).
fn control_socket_for_pid(pid: u32) -> Option<PathBuf> {
    control_socket_for_pid_in(&std::env::temp_dir(), pid)
}

fn control_socket_for_pid_in(dir: &Path, pid: u32) -> Option<PathBuf> {
    let prefix = format!("chan-control-{pid}-");
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(&prefix) && name.ends_with(".sock") {
            return Some(entry.path());
        }
    }
    None
}

/// Block (bounded) until the writer lock for `lock_dir` is free after a
/// serve was asked to unserve. The server drops the flock asynchronously
/// during graceful shutdown, so a `chan open` racing right behind would
/// otherwise see a transient `WorkspaceLocked`.
fn wait_for_lock_release(lock_dir: &Path) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !chan_workspace::lock::is_free(lock_dir) {
        if Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
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
    eprintln!("hint: open repo root:    chan open {}", repo_abs.display());
    eprintln!(
        "hint: open only subdir:  chan open --here {}",
        root_abs.display(),
    );
}

/// Resolved `chan open` invocation: every CLI input after listen-addr
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
    standalone: bool,
    verbose: bool,
}

/// Make a serve root absolute against the process cwd. `canonicalize`
/// resolves symlinks for an existing dir; `std::path::absolute` makes a
/// not-yet-created path absolute lexically (so `chan open new-dir` still
/// lands under the cwd); the final fallback returns the input unchanged
/// (only reachable if both fail, e.g. an unreadable cwd). The result must
/// be absolute so the desktop handoff — which runs with cwd "/" — and the
/// canonical-path-keyed registry both see the directory the user ran in.
fn absolutize_serve_root(root: PathBuf) -> PathBuf {
    std::fs::canonicalize(&root)
        .or_else(|_| std::path::absolute(&root))
        .unwrap_or(root)
}

/// Error for a command invoked without its required workspace path. Every
/// command names the workspace root explicitly; `hint` is a complete,
/// valid example invocation to suggest.
fn missing_workspace_path(cmd: &str, hint: &str) -> anyhow::Error {
    anyhow::anyhow!("chan {cmd} requires a workspace path; e.g. `{hint}`")
}

/// Discriminate `chan open`'s polymorphic argument: a value shaped like
/// `scheme://host…` is a devserver URL; everything else is a local workspace
/// path. We don't pull a URL crate for the discriminator — the desktop parses
/// and validates the full URL when it dials. Requiring `://` with a non-empty
/// scheme and authority keeps a Windows path (`C:\…`) or a bare `host:port`
/// (no `//`) from misfiring as a URL — mirroring §3's "reject bare host:port"
/// so the path/URL split is unambiguous.
fn looks_like_devserver_url(target: &str) -> bool {
    match target.split_once("://") {
        Some((scheme, rest)) => {
            !scheme.is_empty()
                && scheme
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
                && !rest.is_empty()
        }
        None => false,
    }
}

/// `chan open {url}`: REGISTER a devserver by URL via the CLI→desktop handoff,
/// then return. It does NOT dial/connect — connecting is the launcher's
/// Connect button. The devserver entry lives in the desktop's config (the same
/// registry the launcher reads), so this needs a running chan-desktop to land
/// into; without one there is nowhere to persist it (no standalone fallback —
/// a URL is never served locally).
async fn cmd_open_devserver(
    url: String,
    name: Option<String>,
    script: Option<String>,
) -> Result<()> {
    // Refuse a devserver-in-a-devserver: this CLI running inside a devserver
    // session has no path to the desktop's registry, and nesting one headless
    // multi-tenant server inside another is not a shape the registry models.
    if in_devserver_context().await {
        anyhow::bail!(
            "cannot register a devserver from inside a devserver: `chan open {url}` writes \
             into the desktop's devserver registry, which a devserver session cannot reach. \
             Run it from chan-desktop (or a plain shell on the box running chan-desktop)."
        );
    }
    use chan_server::handoff::Outcome;
    match chan_server::handoff::try_open_devserver(&url, name.as_deref(), script.as_deref()).await {
        Outcome::HandedOff => {
            // Registered, not connected: point the user at the launcher's
            // Connect button. Labelled by --name when given, else the URL.
            let label = name.as_deref().unwrap_or(&url);
            println!("registered \"{label}\". Open it from the launcher.");
            Ok(())
        }
        Outcome::VersionSkew {
            desktop_version, ..
        } => anyhow::bail!(
            "chan-desktop is version {desktop_version}, CLI is {}; cannot register the \
             devserver. Restart chan-desktop to pick up the new version.",
            chan_server::handoff::CHAN_VERSION,
        ),
        Outcome::DesktopError { message } => {
            anyhow::bail!("chan-desktop could not register the devserver: {message}")
        }
        // No desktop = nowhere to register. Unlike the path form, a URL never
        // falls back to a standalone serve (mirrors the window-op "needs the
        // desktop" refusal).
        Outcome::NoDesktop => {
            anyhow::bail!("chan open {url} needs the chan desktop app running.")
        }
    }
}

/// True when this CLI runs inside a chan terminal that a `chan devserver`
/// serves — `chan open {url}` would otherwise register a devserver into a
/// devserver, which the registry (a desktop-config concept) does not nest.
/// Resolved by an `Identify` round-trip on `$CHAN_CONTROL_SOCKET`; an absent
/// socket / unreachable holder / any other serving kind ⇒ not a devserver
/// context (so a plain shell or a desktop terminal proceeds to the handoff).
async fn in_devserver_context() -> bool {
    let Some(socket) = std::env::var("CHAN_CONTROL_SOCKET")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    else {
        return false;
    };
    let Ok(message) = chan_shell::send_control_request(
        &PathBuf::from(socket),
        chan_shell::ControlRequest::Identify,
    )
    .await
    else {
        return false;
    };
    matches!(
        serde_json::from_str::<chan_shell::Identity>(&message),
        Ok(chan_shell::Identity {
            kind: chan_shell::ServeKind::Devserver,
            ..
        })
    )
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
        standalone,
        verbose,
    } = args;
    let lib = library()?;
    // `chan open {path}` requires an explicit workspace root; with no path it
    // is a clear error. An explicit path auto-registers, so `chan open
    // /some/dir` works without a prior `chan workspace add`.
    let root = path.ok_or_else(|| missing_workspace_path("open", "chan open ."))?;
    // Resolve to an absolute path against the CLI's cwd before anything
    // downstream consumes it. The macOS desktop handoff opens the
    // workspace in a process whose cwd is "/", and the workspace registry
    // is keyed by the canonical path, so a bare `chan open .` must not
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
    // session, ask it to open this workspace in a native window and EXIT. The
    // desktop then owns the workspace's flock; the CLI must NOT also open the
    // workspace (the single-writer invariant). This runs BEFORE
    // `open_workspace` so a successful handoff never double-opens. Every
    // fallback (no desktop, refused, stale socket, bad handshake, version
    // skew, GUI-absent) drops through to the standalone server path below.
    if personality == Personality::Desktop && !standalone {
        if let Some(outcome) = maybe_handoff_to_desktop(&root).await {
            return outcome;
        }
    }

    // CLI-to-devserver registration. Unlike the desktop handoff this runs
    // for the standalone binary too and does NOT require a GUI session: a
    // devserver is exactly where SSH-only boxes live. A running same-user
    // devserver mounts this workspace and owns its flock, so the CLI prints
    // a note and exits WITHOUT opening it (the single-writer invariant).
    // Runs BEFORE `open_workspace` so a successful registration never
    // double-opens. --standalone and CHAN_NO_DEVSERVER_HANDOFF opt out; every
    // non-registered outcome drops through to the standalone server path below.
    if !standalone && !chan_server::devserver_handoff::devserver_handoff_opt_out() {
        use chan_server::devserver_handoff::Outcome;
        match chan_server::devserver_handoff::try_register_devserver(&root).await {
            Outcome::Registered { prefix: _ } => {
                println!(
                    "chan: registered {} with the local devserver",
                    root.display()
                );
                return Ok(());
            }
            Outcome::VersionSkew => {
                eprintln!(
                    "chan: a local devserver is running a different version; \
                     cannot register. Starting a standalone server."
                );
            }
            Outcome::Error(message) => {
                eprintln!(
                    "chan: the local devserver could not mount this workspace \
                     ({message}); starting a standalone server."
                );
            }
            // No devserver discovered: the load-bearing default path.
            Outcome::NoDevserver => {}
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
    };
    chan_server::serve(lib, workspace, config)
        .await
        .with_context(|| format!("running server on {addr}"))
}

/// Run a headless multi-workspace devserver bound to `bind:port`. By default
/// it runs in the foreground. `--systemd` (Linux) supervises it under the
/// `chan-devserver.service` user unit; `--launchd` (macOS) supervises it under
/// the `app.chan.devserver` LaunchAgent. Either re-attaches when its service is
/// already running; off its own OS each prints a note and runs in the
/// foreground.
async fn cmd_devserver(
    bind: IpAddr,
    port: u16,
    systemd: bool,
    launchd: bool,
    tunnel_url: String,
    tunnel_token: Option<String>,
) -> Result<()> {
    let addr = SocketAddr::new(bind, port);
    if !addr.ip().is_loopback() {
        eprintln!(
            "WARNING: binding to {} exposes the devserver on a non-loopback \
             interface. There is no TLS and only a bearer-token gate; reach a \
             remote devserver over `ssh -L` instead of binding it publicly.",
            addr.ip()
        );
    }
    // Resolve tunnel mode from --tunnel-token. When set, the devserver also
    // dials the gateway and publishes every mounted workspace behind one
    // registration; the local management server still binds. The tunnel runs
    // only in the FOREGROUND devserver: the token is a secret, and the
    // supervised backends would have to persist it in the unit file / launchd
    // plist (0644) to re-exec with it, so the combination is refused.
    let tunnel = match tunnel_token {
        Some(token) => {
            // Warn when the token came in via the flag rather than the env var
            // (clap doesn't expose the source, so compare to env directly). The
            // flag value is in `ps` output until the process exits; the env var
            // is not.
            if std::env::var("CHAN_TUNNEL_TOKEN").ok().as_deref() != Some(token.as_str()) {
                eprintln!(
                    "WARNING: --tunnel-token is visible in `ps` output. \
                     Prefer CHAN_TUNNEL_TOKEN env var instead."
                );
            }
            if systemd || launchd {
                anyhow::bail!(
                    "chan devserver: tunnel mode (--tunnel-token) is not supported under \
                     --systemd/--launchd; the supervised backend would persist the token in \
                     the unit file. Run the devserver in the foreground (or under your own \
                     supervisor) to enable the tunnel."
                );
            }
            Some(chan_server::DevserverTunnel { tunnel_url, token })
        }
        None => None,
    };
    // `--systemd` / `--launchd` supervise the foreground devserver under the
    // platform service manager so it survives the launching terminal. Each is a
    // no-op off its own OS (the other backend is not wired there), so fall back
    // to the foreground.
    if systemd {
        if cfg!(target_os = "linux") {
            return run_devserver_under_systemd(addr).await;
        }
        eprintln!("chan devserver: NOTE: --systemd is Linux-only; running in the foreground.");
    }
    if launchd {
        if cfg!(target_os = "macos") {
            return run_devserver_under_launchd(addr).await;
        }
        eprintln!("chan devserver: NOTE: --launchd is macOS-only; running in the foreground.");
    }
    // Resolve the local-listener decision for the foreground path only. Tunnel
    // mode defaults to NOT binding the loopback port (the gateway is the
    // surface, and it 404s the management API anyway); `CHAN_DEVSERVER_LISTEN`
    // overrides either way. The supervised backends always bind locally (they
    // re-exec without the env var, and tunnel mode is refused there), so the
    // resolution lives below the systemd/launchd branches.
    let listen = resolve_devserver_listen(tunnel.is_some(), devserver_listen_override())?;
    run_devserver_foreground(addr, tunnel, listen).await
}

/// Whether the foreground devserver binds a local TCP listener. Tunnel mode
/// defaults to no-bind (the gateway is the surface); `CHAN_DEVSERVER_LISTEN`
/// forces either way. Tunnel-off + LISTEN=0 leaves nothing reachable (no local
/// listener, no tunnel — only the `chan open` discovery socket), so it is a
/// hard error rather than a silently-unreachable devserver.
fn resolve_devserver_listen(tunnel_mode: bool, listen_override: Option<bool>) -> Result<bool> {
    let listen = listen_override.unwrap_or(!tunnel_mode);
    if !listen && !tunnel_mode {
        anyhow::bail!(
            "chan devserver: CHAN_DEVSERVER_LISTEN=0 with no tunnel leaves nothing reachable \
             (no local listener and no tunnel). Set CHAN_TUNNEL_TOKEN to publish through the \
             gateway, or unset CHAN_DEVSERVER_LISTEN to bind the local listener."
        );
    }
    Ok(listen)
}

/// Read `CHAN_DEVSERVER_LISTEN` as a tri-state: unset or empty ⇒ `None` (use the
/// tunnel-mode default), `"0"` ⇒ `Some(false)`, any other non-empty value ⇒
/// `Some(true)` (mirrors `CHAN_NO_DESKTOP_HANDOFF`'s truthiness).
fn devserver_listen_override() -> Option<bool> {
    std::env::var("CHAN_DEVSERVER_LISTEN")
        .ok()
        .and_then(|v| parse_listen_override(&v))
}

/// Pure parse for [`devserver_listen_override`] so the tri-state is unit-tested
/// without touching the process environment.
fn parse_listen_override(raw: &str) -> Option<bool> {
    if raw.is_empty() {
        None
    } else {
        Some(raw != "0")
    }
}

/// Run the devserver in the foreground. The no-supervisor default and the
/// systemd unit's `ExecStart` / launchd agent's `ProgramArguments` all land
/// here. `tunnel` carries the gateway registration when `--tunnel-token` is
/// set; the supervised backends never pass it (tunnel mode is foreground-only).
async fn run_devserver_foreground(
    addr: SocketAddr,
    tunnel: Option<chan_server::DevserverTunnel>,
    listen: bool,
) -> Result<()> {
    let lib = library()?;
    chan_server::run_devserver(
        lib,
        chan_server::DevserverConfig {
            addr,
            host_label: devserver_host_label(),
            tunnel,
            listen,
        },
    )
    .await
    .context("running devserver")
}

/// Human label for the box, shown in the management API. Falls back to a
/// generic label when the hostname is empty.
fn devserver_host_label() -> String {
    let host = gethostname::gethostname().to_string_lossy().into_owned();
    if host.trim().is_empty() {
        "devserver".to_string()
    } else {
        host
    }
}

/// The systemd user unit name for the devserver.
const DEVSERVER_SYSTEMD_UNIT: &str = "chan-devserver.service";

/// Supervise the devserver under a systemd user service: ensure linger,
/// create + start the unit (or re-attach to a running one), then stream its
/// journal until the unit stops. The controlling terminal sees the
/// devserver's output and notices when it dies, and a unit that cannot
/// start exits non-zero loudly so a watching desktop catches it.
async fn run_devserver_under_systemd(addr: SocketAddr) -> Result<()> {
    ensure_systemd_linger().await?;

    if unit_is_active().await {
        // Re-attaching to a unit that is already running. A journal follow
        // won't re-emit the unit's original start line, so the supervisor
        // re-provides the token contract itself (see emit_devserver_token_marker).
        emit_devserver_token_marker(DEVSERVER_TOKEN_WAIT).await?;
        eprintln!(
            "chan devserver: re-attaching to the running systemd user service \
             {DEVSERVER_SYSTEMD_UNIT}"
        );
    } else {
        let unit_path = write_devserver_unit(addr)?;
        eprintln!("chan devserver: wrote {}", unit_path.display());
        systemctl_user(&["daemon-reload"]).await?;
        systemctl_user(&["enable", "--now", DEVSERVER_SYSTEMD_UNIT]).await?;
        if !wait_until_active(Duration::from_secs(10)).await {
            anyhow::bail!(
                "chan devserver: the systemd user service {DEVSERVER_SYSTEMD_UNIT} \
                 failed to start:\n{}",
                recent_unit_journal().await
            );
        }
        // The freshly started service prints the token marker to its own stdout,
        // which under the unit lands in the journal — invisible to this terminal
        // on a host with no readable journal. Emit it directly from the persisted
        // config so the desktop reconnects regardless; fail loud if it never
        // lands rather than claim "started" on a token we cannot surface.
        emit_devserver_token_marker(DEVSERVER_TOKEN_WAIT).await?;
        eprintln!(
            "chan devserver: started the systemd user service \
             {DEVSERVER_SYSTEMD_UNIT} (bind={addr})"
        );
    }

    follow_unit_until_stopped().await
}

/// How long the supervisor waits for the service's bearer token to land in the
/// persisted config before giving up. A fresh `Type=simple` unit reports active
/// before its first persist, so a brief poll covers that race; every later start
/// finds the token on the first read.
const DEVSERVER_TOKEN_WAIT: Duration = Duration::from_secs(5);

/// Resolve the persisted devserver bearer token, polling `read` until it yields
/// a token or `timeout` elapses. Injecting the reader keeps the poll/timeout
/// contract testable without a real config on disk.
async fn resolve_devserver_token(
    read: impl Fn() -> Option<String>,
    timeout: Duration,
) -> Option<String> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(token) = read() {
            return Some(token);
        }
        if Instant::now() >= deadline {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Print the locked `CHAN_DEVSERVER_TOKEN=` marker to stdout — the same contract
/// the foreground server emits — directly from the supervisor, read from the
/// persisted 0600 config. Token delivery must not depend on this user being able
/// to read the unit journal (a uid below `SYS_UID_MAX`, or a user outside the
/// `systemd-journal`/`adm` groups, cannot): the desktop control terminal scrapes
/// this marker to reconnect, and the journal follow is only human-facing log
/// streaming. A duplicate marker re-surfaced by the journal on readable hosts is
/// harmless — the scraper takes the last one.
///
/// Errors when the token never lands within `timeout`. The point of `--systemd`
/// supervision is to hand a client a token to reconnect with; a unit that is
/// active but whose token cannot be surfaced is unreachable, so fail loud rather
/// than babysit it. The unit stays running, so a later re-attach can recover it.
async fn emit_devserver_token_marker(timeout: Duration) -> Result<()> {
    match resolve_devserver_token(chan_server::persisted_devserver_token, timeout).await {
        Some(token) => {
            println!("{}{token}", chan_server::DEVSERVER_TOKEN_MARKER);
            Ok(())
        }
        None => anyhow::bail!(
            "chan devserver: the supervised service is active but its bearer \
             token could not be read from ~/.chan/devserver/config.json; the \
             control terminal cannot authenticate to it"
        ),
    }
}

/// Ensure lingering is enabled so the user service survives logout. Fails
/// loudly with a manual hint when it cannot be ensured.
async fn ensure_systemd_linger() -> Result<()> {
    let user = std::env::var("USER").ok().filter(|u| !u.is_empty());
    // Already lingering? Then it is ensured. `loginctl enable-linger` does a
    // polkit check on every call that a non-root user without an interactive
    // authority is denied EVEN when linger is already on, so only call it
    // when linger is actually off.
    if let Some(user) = user.as_deref() {
        if user_linger_enabled(user).await {
            return Ok(());
        }
    }
    let mut args: Vec<&str> = vec!["enable-linger"];
    if let Some(user) = user.as_deref() {
        args.push(user);
    }
    let output = run_tool("loginctl", &args).await?;
    if !output.status.success() {
        anyhow::bail!(
            "chan devserver --systemd: linger is off (so the service would not \
             survive logout) and `loginctl enable-linger` was denied:\n{}\n\
             enable it once, as root: sudo loginctl enable-linger {}",
            String::from_utf8_lossy(&output.stderr).trim(),
            user.as_deref().unwrap_or("$USER"),
        );
    }
    Ok(())
}

/// Whether `loginctl` reports `Linger=yes` for `user`.
async fn user_linger_enabled(user: &str) -> bool {
    matches!(
        run_tool("loginctl", &["show-user", user, "-p", "Linger"]).await,
        Ok(output) if String::from_utf8_lossy(&output.stdout).trim() == "Linger=yes"
    )
}

/// Write `~/.config/systemd/user/chan-devserver.service` whose `ExecStart`
/// runs THIS binary's foreground devserver on `addr`. Returns the unit path.
fn write_devserver_unit(addr: SocketAddr) -> Result<PathBuf> {
    let exe = std::env::current_exe()
        .context("resolving the chan binary path for the systemd unit ExecStart")?;
    let dir = systemd_user_unit_dir()?;
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let unit_path = dir.join(DEVSERVER_SYSTEMD_UNIT);
    let unit = format!(
        "[Unit]\n\
         Description=chan devserver\n\
         After=network.target\n\
         \n\
         [Service]\n\
         ExecStart={exe} devserver --bind={ip} --port={port}\n\
         Restart=on-failure\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        exe = exe.display(),
        ip = addr.ip(),
        port = addr.port(),
    );
    std::fs::write(&unit_path, unit).with_context(|| format!("writing {}", unit_path.display()))?;
    Ok(unit_path)
}

/// `$XDG_CONFIG_HOME/systemd/user`, else `$HOME/.config/systemd/user`.
fn systemd_user_unit_dir() -> Result<PathBuf> {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(xdg).join("systemd").join("user"));
    }
    let home = std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .context("no HOME for the systemd user unit directory")?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("systemd")
        .join("user"))
}

/// Stream the unit's journal to stdout, returning when the unit is no
/// longer active. A unit left in a failed state returns an error so the
/// caller exits non-zero.
async fn follow_unit_until_stopped() -> Result<()> {
    // The unit's `is-active` state — NOT journalctl's lifetime — is the
    // authoritative stop signal. journalctl can exit early for reasons that
    // have nothing to do with the unit: most commonly a host where this user
    // has no readable journal (a uid below SYS_UID_MAX, which journald treats
    // as a system user and never gives a per-user journal, or a user outside
    // the `systemd-journal`/`adm` groups). Conflating that with "the unit
    // stopped" would declare a healthy devserver dead.
    let mut follow = tokio::process::Command::new("journalctl")
        .args(["--user", "-u", DEVSERVER_SYSTEMD_UNIT, "-f", "-n", "20"])
        .spawn()
        .context("spawning journalctl to follow the devserver service")?;
    let mut streaming = true;
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        if !unit_is_active().await {
            break;
        }
        if streaming && matches!(follow.try_wait(), Ok(Some(_))) {
            // Lost the log stream while the unit is still running: keep
            // supervising via is-active, just without journal output.
            eprintln!(
                "chan devserver: journal streaming for {DEVSERVER_SYSTEMD_UNIT} \
                 stopped (is this user in the `systemd-journal` group?); still \
                 supervising the service"
            );
            streaming = false;
        }
    }
    let _ = follow.start_kill();
    let _ = follow.wait().await;
    if unit_is_failed().await {
        anyhow::bail!(
            "chan devserver: the systemd user service {DEVSERVER_SYSTEMD_UNIT} \
             entered a failed state:\n{}",
            recent_unit_journal().await
        );
    }
    eprintln!(
        "chan devserver: the systemd user service {DEVSERVER_SYSTEMD_UNIT} is no \
         longer active."
    );
    Ok(())
}

/// Poll until the unit is active, a failure is reported, or the deadline
/// passes. Tolerates the brief `activating` window after `enable --now`.
async fn wait_until_active(timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if unit_is_active().await {
            return true;
        }
        if unit_is_failed().await || Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

async fn unit_is_active() -> bool {
    matches!(
        run_tool("systemctl", &["--user", "is-active", DEVSERVER_SYSTEMD_UNIT]).await,
        Ok(output) if output.status.success()
    )
}

async fn unit_is_failed() -> bool {
    matches!(
        run_tool("systemctl", &["--user", "is-failed", DEVSERVER_SYSTEMD_UNIT]).await,
        Ok(output) if output.status.success()
    )
}

/// Run `systemctl --user <args>`, erroring with stderr on a non-zero exit.
async fn systemctl_user(args: &[&str]) -> Result<()> {
    let mut full: Vec<&str> = vec!["--user"];
    full.extend_from_slice(args);
    let output = run_tool("systemctl", &full).await?;
    if !output.status.success() {
        anyhow::bail!(
            "`systemctl --user {}` failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// The last lines of the unit's journal, for a failure message.
async fn recent_unit_journal() -> String {
    match run_tool(
        "journalctl",
        &[
            "--user",
            "-u",
            DEVSERVER_SYSTEMD_UNIT,
            "--no-pager",
            "-n",
            "30",
        ],
    )
    .await
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_string(),
        Err(e) => format!("(could not read the journal: {e})"),
    }
}

/// Run a tool to completion, capturing its output. Errors only when the
/// tool cannot be spawned (e.g. missing binary), not on a non-zero exit.
async fn run_tool(program: &str, args: &[&str]) -> Result<std::process::Output> {
    tokio::process::Command::new(program)
        .args(args)
        .output()
        .await
        .with_context(|| format!("running `{program} {}`", args.join(" ")))
}

// ---------------------------------------------------------------------------
// macOS launchd backend — mirrors the systemd backend above. The functions are
// always compiled (they only shell out to `launchctl`) and called only under
// `cfg!(target_os = "macos")`; the pure helpers stay unit-testable on any host.
// ---------------------------------------------------------------------------

/// The launchd LaunchAgent label for the devserver. Reverse-DNS off the app
/// bundle id (`app.chan.desktop`).
const DEVSERVER_LAUNCHD_LABEL: &str = "app.chan.devserver";

/// Supervise the devserver under a per-user launchd LaunchAgent: write and load
/// the agent (or re-attach to a running one), emit the token contract, then
/// follow its log until it stops. Unlike systemd there is no linger to ensure —
/// a LaunchAgent in the `gui/<uid>` domain already outlives the launching shell
/// and the GUI login session (it does NOT survive a full logout; that would
/// need a root LaunchDaemon).
async fn run_devserver_under_launchd(addr: SocketAddr) -> Result<()> {
    let uid = current_uid().await?;
    let service = launchd_service_target(uid);

    if launchd_is_active(uid).await {
        // Re-attaching to a running agent. Its stdout (with the token marker)
        // goes to the log file, not this terminal, so the supervisor re-provides
        // the token contract itself (see emit_devserver_token_marker).
        emit_devserver_token_marker(DEVSERVER_TOKEN_WAIT).await?;
        eprintln!(
            "chan devserver: re-attaching to the running launchd agent \
             {DEVSERVER_LAUNCHD_LABEL}"
        );
    } else {
        let plist = write_devserver_launch_agent(addr)?;
        eprintln!("chan devserver: wrote {}", plist.display());
        // Clear any stale (loaded-but-dead) registration so the freshly written
        // plist takes effect; best-effort, it errors when nothing is loaded.
        let _ = run_tool("launchctl", &["bootout", service.as_str()]).await;
        launchctl(&["enable", service.as_str()]).await?;
        let plist_arg = plist.to_string_lossy();
        launchctl(&["bootstrap", &launchd_domain_target(uid), plist_arg.as_ref()]).await?;
        if !wait_until_launchd_active(uid, Duration::from_secs(10)).await {
            anyhow::bail!(
                "chan devserver: the launchd agent {DEVSERVER_LAUNCHD_LABEL} \
                 failed to start:\n{}",
                recent_launchd_log().await
            );
        }
        // Same direct-emit contract as the systemd path: the service logs its
        // own marker to the log file, invisible to this terminal, so surface it
        // from the persisted config and fail loud if it never lands.
        emit_devserver_token_marker(DEVSERVER_TOKEN_WAIT).await?;
        eprintln!(
            "chan devserver: started the launchd agent {DEVSERVER_LAUNCHD_LABEL} \
             (bind={addr})"
        );
    }

    follow_launchd_until_stopped(uid).await
}

/// The current user's numeric uid for the `gui/<uid>` domain target. Shells out
/// to `id -u` rather than adding a libc dependency, mirroring the systemd
/// backend's `$USER` discovery.
async fn current_uid() -> Result<u32> {
    let output = run_tool("id", &["-u"]).await?;
    if !output.status.success() {
        anyhow::bail!(
            "`id -u` failed:\n{}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .context("parsing the current uid from `id -u`")
}

/// `gui/<uid>` — the launchd domain target for the user's GUI login session.
fn launchd_domain_target(uid: u32) -> String {
    format!("gui/{uid}")
}

/// `gui/<uid>/<label>` — the launchd service target for the devserver agent.
fn launchd_service_target(uid: u32) -> String {
    format!("gui/{uid}/{DEVSERVER_LAUNCHD_LABEL}")
}

/// The user's home directory from `$HOME`, for the macOS launchd paths. Mirrors
/// the `$HOME` resolution the systemd unit-dir helper uses (no `dirs` dep).
fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .context("no HOME for the launchd agent paths")
}

/// `~/Library/LaunchAgents/app.chan.devserver.plist`.
fn launch_agent_path() -> Result<PathBuf> {
    Ok(home_dir()?
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{DEVSERVER_LAUNCHD_LABEL}.plist")))
}

/// `~/.chan/devserver/devserver.log` — where the agent's stdout/stderr land
/// (launchd has no journal). Co-located with the 0600 devserver config.
fn devserver_log_path() -> Result<PathBuf> {
    Ok(home_dir()?
        .join(".chan")
        .join("devserver")
        .join("devserver.log"))
}

/// Write the LaunchAgent plist whose `ProgramArguments` run THIS binary's
/// foreground devserver on `addr`. Returns the plist path.
fn write_devserver_launch_agent(addr: SocketAddr) -> Result<PathBuf> {
    let exe = std::env::current_exe()
        .context("resolving the chan binary path for the launchd ProgramArguments")?;
    let log = devserver_log_path()?;
    if let Some(parent) = log.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let plist_path = launch_agent_path()?;
    if let Some(parent) = plist_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let plist = devserver_launch_agent_plist(&exe, addr, &log);
    std::fs::write(&plist_path, plist)
        .with_context(|| format!("writing {}", plist_path.display()))?;
    Ok(plist_path)
}

/// Build the LaunchAgent plist XML. `RunAtLoad` starts it on bootstrap;
/// `KeepAlive`/`SuccessfulExit=false` restarts it only on a crash (the launchd
/// analogue of systemd `Restart=on-failure`); stdout/stderr go to `log`.
fn devserver_launch_agent_plist(exe: &Path, addr: SocketAddr, log: &Path) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{exe}</string>
    <string>devserver</string>
    <string>--bind={ip}</string>
    <string>--port={port}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <dict>
    <key>SuccessfulExit</key>
    <false/>
  </dict>
  <key>StandardOutPath</key>
  <string>{log}</string>
  <key>StandardErrorPath</key>
  <string>{log}</string>
</dict>
</plist>
"#,
        label = DEVSERVER_LAUNCHD_LABEL,
        exe = xml_escape(&exe.to_string_lossy()),
        ip = addr.ip(),
        port = addr.port(),
        log = xml_escape(&log.to_string_lossy()),
    )
}

/// Minimal XML text escaping for plist `<string>` values (paths).
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Run `launchctl <args>`, erroring with stderr on a non-zero exit. For the
/// must-succeed calls (`enable`, `bootstrap`); `bootout` runs best-effort.
async fn launchctl(args: &[&str]) -> Result<()> {
    let output = run_tool("launchctl", args).await?;
    if !output.status.success() {
        anyhow::bail!(
            "`launchctl {}` failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// Whether the agent is loaded AND running.
async fn launchd_is_active(uid: u32) -> bool {
    let service = launchd_service_target(uid);
    matches!(
        run_tool("launchctl", &["print", service.as_str()]).await,
        Ok(output)
            if output.status.success()
                && launchd_print_running(&String::from_utf8_lossy(&output.stdout))
    )
}

/// Whether the agent is loaded, not running, and last exited non-zero.
async fn launchd_is_failed(uid: u32) -> bool {
    let service = launchd_service_target(uid);
    matches!(
        run_tool("launchctl", &["print", service.as_str()]).await,
        Ok(output)
            if output.status.success()
                && launchd_print_failed(&String::from_utf8_lossy(&output.stdout))
    )
}

/// Parse `launchctl print` output for a running service (`state = running`).
fn launchd_print_running(out: &str) -> bool {
    out.lines().any(|l| l.trim() == "state = running")
}

/// Parse `launchctl print` output for a failed service: not running with a
/// non-zero `last exit code`. `(never exited)` and `= 0` are not failures.
fn launchd_print_failed(out: &str) -> bool {
    let not_running = out.lines().any(|l| l.trim() == "state = not running");
    let bad_exit = out.lines().find_map(|l| {
        l.trim()
            .strip_prefix("last exit code = ")
            .and_then(|v| v.parse::<i32>().ok())
    });
    not_running && matches!(bad_exit, Some(code) if code != 0)
}

/// Poll until the agent is active, a failure is reported, or the deadline
/// passes. Tolerates the brief window between bootstrap and first run.
async fn wait_until_launchd_active(uid: u32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if launchd_is_active(uid).await {
            return true;
        }
        if launchd_is_failed(uid).await || Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

/// The last lines of the agent's log file, for a failure message.
async fn recent_launchd_log() -> String {
    let path = match devserver_log_path() {
        Ok(p) => p,
        Err(e) => return format!("(could not resolve the log path: {e})"),
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => {
            let mut tail: Vec<&str> = text.lines().rev().take(30).collect();
            tail.reverse();
            tail.join("\n")
        }
        Err(e) => format!("(could not read {}: {e})", path.display()),
    }
}

/// Stream the agent's log to stdout, returning when the agent is no longer
/// active. An agent left in a failed state returns an error so the caller exits
/// non-zero. Mirrors `follow_unit_until_stopped`.
async fn follow_launchd_until_stopped(uid: u32) -> Result<()> {
    let log = devserver_log_path()?;
    let mut follow = tokio::process::Command::new("tail")
        .args(["-f", "-n", "20"])
        .arg(&log)
        .spawn()
        .context("spawning tail to follow the devserver log")?;
    let mut streaming = true;
    loop {
        tokio::time::sleep(Duration::from_secs(2)).await;
        if !launchd_is_active(uid).await {
            break;
        }
        if streaming && matches!(follow.try_wait(), Ok(Some(_))) {
            eprintln!(
                "chan devserver: log streaming for {DEVSERVER_LAUNCHD_LABEL} \
                 stopped; still supervising the agent"
            );
            streaming = false;
        }
    }
    let _ = follow.start_kill();
    let _ = follow.wait().await;
    if launchd_is_failed(uid).await {
        anyhow::bail!(
            "chan devserver: the launchd agent {DEVSERVER_LAUNCHD_LABEL} entered \
             a failed state:\n{}",
            recent_launchd_log().await
        );
    }
    eprintln!(
        "chan devserver: the launchd agent {DEVSERVER_LAUNCHD_LABEL} is no \
         longer active."
    );
    Ok(())
}

/// Integrate a Desktop-personality `chan open` with the desktop app.
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

/// Launch the desktop GUI for a `chan open` that found no running desktop,
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
                "timed out waiting for chan-desktop to start; run `chan open` again \
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

/// Dispatch the `chan workspace reports {enable,disable}`
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
    let root = path.ok_or_else(|| {
        let (cmd, hint) = if enabled {
            ("reports enable", "chan workspace reports enable --path .")
        } else {
            ("reports disable", "chan workspace reports disable --path .")
        };
        missing_workspace_path(cmd, hint)
    })?;
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
                    "`chan workspace index rebuild` requires a workspace path (positional or `--path`)"
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
    // last_seen_at. CLI users expect `chan workspace index rebuild /some/path`
    // to work without a prior `chan workspace add`.
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
            "unknown embedding model: {model} (run `chan workspace index list-models` to list supported models)"
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
            "unknown embedding model: {model} (run `chan workspace index list-models` to list supported models)"
        );
    }
    let lib = library()?;
    let root = path.ok_or_else(|| {
        missing_workspace_path(
            "index set-model",
            "chan workspace index set-model --path . --model BAAI/bge-small-en-v1.5",
        )
    })?;
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
/// `chan workspace index download-model`. On disable, always succeeds (the
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
    let root = path.ok_or_else(|| {
        let (cmd, hint) = if enabled {
            (
                "index enable-semantic",
                "chan workspace index enable-semantic --path .",
            )
        } else {
            (
                "index disable-semantic",
                "chan workspace index disable-semantic --path .",
            )
        };
        missing_workspace_path(cmd, hint)
    })?;
    let workspace = lib
        .open_workspace(&root)
        .with_context(|| not_a_chan_workspace_hint(&root))?;
    if enabled {
        let model = workspace
            .semantic_model()
            .context("reading workspace's model id")?;
        if let Err(err) = resolve_model(&model) {
            return Err(anyhow::anyhow!(
                "{err}\nrun `chan workspace index download-model` to fetch it"
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
    let root = path.ok_or_else(|| {
        missing_workspace_path("index status", "chan workspace index status --path .")
    })?;
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
    // The screensaver + report/semantic toggles re-homed out of IndexConfig
    // into the dedicated per-workspace dashboard config; read them from there.
    let dashboard = chan_workspace::dashboard::load(&workspace_paths.root).with_context(|| {
        format!(
            "reading dashboard config at {}",
            workspace_paths.root.display()
        )
    })?;
    let model = cfg.model;
    let semantic_enabled = dashboard.semantic_enabled;
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
        // Emit `reports_enabled` alongside `semantic_enabled` so chan-desktop's
        // `get_workspace_features` IPC can read both flags from one CLI
        // round-trip. Both come from the per-workspace dashboard config; this
        // is a strict additive extension (existing JSON consumers ignore
        // unknown fields).
        let body = serde_json::json!({
            "workspace": canonical_root.display().to_string(),
            "mode": mode,
            "model_present": model_present,
            "model_name": model,
            "model_path": expected_dir.display().to_string(),
            "model_size_bytes": model_size_bytes,
            "semantic_enabled": semantic_enabled,
            "reports_enabled": dashboard.reports_enabled,
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
                "no (run `chan workspace index download-model`)"
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
/// "not a chan workspace at <path>" hint with a `chan workspace add` next-step
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
        "not a chan workspace at {}; run `chan workspace add {}` first",
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
/// chan-server. Connects to the server's MCP transport (a Unix-domain
/// socket on unix, a named pipe on Windows) and pipes stdin -> socket and
/// socket -> stdout concurrently. Returns when either direction closes,
/// which is the normal end of a session.
async fn cmd_mcp_proxy(socket: PathBuf) -> Result<()> {
    chan_server::run_mcp_stdio_proxy(socket)
        .await
        .context("running MCP proxy")
}

/// Pick the CLI content-search mode, mirroring the `/api/search/content`
/// route: Hybrid (BM25 + dense, RRF-fused) only when the workspace opted
/// in via `semantic_enabled` AND the embedding model is on disk;
/// otherwise BM25. Keeping the CLI and the route on the same rule means
/// `chan workspace search` and the editor's search panel agree on what ran.
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
    let root = path.ok_or_else(|| missing_workspace_path("status", "chan workspace status ."))?;
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
    let root = workspace.ok_or_else(|| {
        missing_workspace_path(
            "contacts import csv",
            "chan workspace contacts import csv contacts.csv --workspace .",
        )
    })?;
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

    #[test]
    fn devserver_url_discriminator() {
        // scheme://host shapes are devserver URLs.
        assert!(looks_like_devserver_url("https://box.example.com:8787"));
        assert!(looks_like_devserver_url("http://127.0.0.1:8787"));
        assert!(looks_like_devserver_url("https://alice.devserver.chan.app"));
        // Everything else is a local path: bare host:port (no `//`), a
        // relative or absolute path, `.`, a Windows drive path, and an empty
        // authority.
        assert!(!looks_like_devserver_url("box.example.com:8787"));
        assert!(!looks_like_devserver_url("."));
        assert!(!looks_like_devserver_url("./notes"));
        assert!(!looks_like_devserver_url("/home/u/notes"));
        assert!(!looks_like_devserver_url("notes"));
        assert!(!looks_like_devserver_url(r"C:\Users\u\notes"));
        assert!(!looks_like_devserver_url("://nohost"));
        assert!(!looks_like_devserver_url("https://"));
    }

    #[test]
    fn control_socket_for_pid_matches_only_that_pid() {
        let dir = tempfile::TempDir::new().unwrap();
        // A different pid's control socket and an unrelated chan socket are
        // both ignored.
        std::fs::write(dir.path().join("chan-control-999-abcd.sock"), b"").unwrap();
        std::fs::write(dir.path().join("chan-mcp-4242-abcd.sock"), b"").unwrap();
        assert_eq!(control_socket_for_pid_in(dir.path(), 4242), None);
        // The matching pid's socket is found.
        let want = dir.path().join("chan-control-4242-ef01.sock");
        std::fs::write(&want, b"").unwrap();
        assert_eq!(control_socket_for_pid_in(dir.path(), 4242), Some(want));
    }

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

    /// The baked prod tunnel endpoint is a wire string the gateway answers
    /// to: `chan devserver --tunnel-token=…` must resolve prod with no
    /// `--tunnel-url`. Pin it so the default can't silently drift off
    /// `devserver.chan.app` (a green build wouldn't catch a typo).
    #[test]
    fn devserver_tunnel_url_defaults_to_prod_endpoint() {
        let cli = Cli::parse_from(["chan", "devserver"]);
        match cli.command {
            Command::Devserver {
                tunnel_url,
                tunnel_token,
                ..
            } => {
                assert_eq!(tunnel_url, "https://devserver.chan.app/v1/tunnel");
                // No token by default → tunnel mode stays off until opted in.
                assert_eq!(tunnel_token, None);
            }
            other => panic!("expected Command::Devserver, got {other:?}"),
        }
    }

    /// The `listen` resolution matrix (plan.md Theme 1): tunnel mode flips the
    /// default to no-bind; `CHAN_DEVSERVER_LISTEN` overrides; tunnel-off +
    /// LISTEN=0 is the unreachable-devserver hard error.
    #[test]
    fn devserver_listen_matrix() {
        // Tunnel off: default binds; explicit 1 binds; explicit 0 errors
        // (nothing reachable).
        assert!(resolve_devserver_listen(false, None).unwrap());
        assert!(resolve_devserver_listen(false, Some(true)).unwrap());
        assert!(resolve_devserver_listen(false, Some(false)).is_err());
        // Tunnel on: default does NOT bind locally; explicit 0 also doesn't;
        // explicit 1 binds the local listener alongside the tunnel.
        assert!(!resolve_devserver_listen(true, None).unwrap());
        assert!(!resolve_devserver_listen(true, Some(false)).unwrap());
        assert!(resolve_devserver_listen(true, Some(true)).unwrap());
    }

    /// `CHAN_DEVSERVER_LISTEN` is a tri-state: unset/empty ⇒ default, `"0"` ⇒
    /// off, any other non-empty value ⇒ on.
    #[test]
    fn devserver_listen_override_parse() {
        assert_eq!(parse_listen_override(""), None);
        assert_eq!(parse_listen_override("0"), Some(false));
        assert_eq!(parse_listen_override("1"), Some(true));
        // Any non-empty, non-"0" value is truthy (mirrors CHAN_NO_DESKTOP_HANDOFF).
        assert_eq!(parse_listen_override("yes"), Some(true));
    }

    /// An explicit `--tunnel-url` still overrides the baked default.
    #[test]
    fn devserver_tunnel_url_override_wins() {
        let cli = Cli::parse_from([
            "chan",
            "devserver",
            "--tunnel-url",
            "http://127.0.0.1:7777/v1/tunnel",
        ]);
        match cli.command {
            Command::Devserver { tunnel_url, .. } => {
                assert_eq!(tunnel_url, "http://127.0.0.1:7777/v1/tunnel");
            }
            other => panic!("expected Command::Devserver, got {other:?}"),
        }
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
        let cli =
            Cli::try_parse_from(["chan", "workspace", "index", "list-models", "--json"]).unwrap();
        match cli.command {
            Command::Workspace {
                action:
                    WorkspaceAction::Index {
                        action: IndexAction::ListModels { json },
                    },
            } => assert!(json),
            other => panic!("unexpected command: {other:?}"),
        }

        let cli = Cli::try_parse_from([
            "chan",
            "workspace",
            "index",
            "set-model",
            "--path",
            "/tmp/workspace",
            "--model",
            "BAAI/bge-base-en-v1.5",
        ])
        .unwrap();
        match cli.command {
            Command::Workspace {
                action:
                    WorkspaceAction::Index {
                        action: IndexAction::SetModel { path, model },
                    },
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
            "workspace",
            "metadata",
            "export",
            "/tmp/workspace",
            "/tmp/meta.tar.zst",
        ])
        .unwrap();
        match cli.command {
            Command::Workspace {
                action:
                    WorkspaceAction::Metadata {
                        action: MetadataAction::Export { path, archive },
                    },
            } => {
                assert_eq!(path, PathBuf::from("/tmp/workspace"));
                assert_eq!(archive, PathBuf::from("/tmp/meta.tar.zst"));
            }
            other => panic!("unexpected command: {other:?}"),
        }

        let cli = Cli::try_parse_from([
            "chan",
            "workspace",
            "metadata",
            "import",
            "/tmp/workspace",
            "/tmp/meta.tar.zst",
            "--rescan",
            "--force-scm",
        ])
        .unwrap();
        match cli.command {
            Command::Workspace {
                action:
                    WorkspaceAction::Metadata {
                        action:
                            MetadataAction::Import {
                                path,
                                archive,
                                rescan,
                                force_scm,
                            },
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
    fn workspace_group_renames_list_and_remove() {
        // The registry verbs live under `chan workspace` now, with
        // `list` renamed to `ls` and `remove` to `rm`.
        let cli = Cli::try_parse_from(["chan", "workspace", "ls", "--json"]).unwrap();
        match cli.command {
            Command::Workspace {
                action: WorkspaceAction::Ls { json },
            } => assert!(json),
            other => panic!("unexpected command: {other:?}"),
        }

        let cli = Cli::try_parse_from(["chan", "workspace", "rm", "/tmp/workspace"]).unwrap();
        match cli.command {
            Command::Workspace {
                action: WorkspaceAction::Rm { path },
            } => assert_eq!(path, PathBuf::from("/tmp/workspace")),
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn flat_workspace_subcommands_are_rejected() {
        // Pre-release reorg: no back-compat aliases. The old flat forms
        // (`chan add`, `chan list`, `chan index`, ...) must no longer parse
        // as top-level commands now that they moved under `chan workspace`.
        for argv in [
            ["chan", "add"].as_slice(),
            ["chan", "list"].as_slice(),
            ["chan", "remove"].as_slice(),
            ["chan", "index"].as_slice(),
            ["chan", "search"].as_slice(),
            ["chan", "metadata"].as_slice(),
            ["chan", "contacts"].as_slice(),
        ] {
            assert!(
                Cli::try_parse_from(argv).is_err(),
                "flat `{}` should no longer parse as a top-level command",
                argv[1],
            );
        }
    }

    #[test]
    fn ps_command_parses() {
        let cli = Cli::try_parse_from(["chan", "ps", "--json"]).unwrap();
        match cli.command {
            Command::Ps { json } => assert!(json),
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn served_by_json_labels_are_stable() {
        // The `chan ps --json` `served_by` strings are a machine contract.
        assert_eq!(
            serde_json::to_value(ServedBy::Standalone).unwrap(),
            "standalone"
        );
        assert_eq!(serde_json::to_value(ServedBy::Desktop).unwrap(), "desktop");
        assert_eq!(
            serde_json::to_value(ServedBy::Devserver).unwrap(),
            "devserver"
        );
        assert_eq!(ServedBy::Devserver.label(), "devserver");
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
    // syseng's hardening pass observed `chan workspace graph --target ../etc/hosts`
    // and `chan workspace graph --target notes/no-such-file.md` returning
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

    #[tokio::test]
    async fn resolve_devserver_token_returns_first_available() {
        // The common case: the token is already on disk, so the first read wins
        // and no polling happens.
        let token =
            resolve_devserver_token(|| Some("tok_abc".to_string()), Duration::from_secs(5)).await;
        assert_eq!(token.as_deref(), Some("tok_abc"));
    }

    #[tokio::test]
    async fn resolve_devserver_token_polls_until_the_token_lands() {
        // The fresh `Type=simple` race: the unit is active but the service has
        // not persisted yet, so the first reads miss and a later one succeeds.
        let calls = std::cell::Cell::new(0u32);
        let token = resolve_devserver_token(
            || {
                let n = calls.get() + 1;
                calls.set(n);
                (n >= 3).then(|| "tok_late".to_string())
            },
            Duration::from_secs(5),
        )
        .await;
        assert_eq!(token.as_deref(), Some("tok_late"));
        assert!(
            calls.get() >= 3,
            "expected polling, saw {} reads",
            calls.get()
        );
    }

    #[tokio::test]
    async fn resolve_devserver_token_gives_up_after_timeout() {
        // A token that never lands resolves to None at the deadline, which the
        // caller turns into a loud failure rather than supervising blind.
        let token = resolve_devserver_token(|| None, Duration::from_millis(150)).await;
        assert_eq!(token, None);
    }

    #[test]
    fn launch_agent_plist_carries_program_and_keys() {
        let plist = devserver_launch_agent_plist(
            Path::new("/usr/local/bin/chan"),
            "127.0.0.1:8799".parse().unwrap(),
            Path::new("/Users/x/.chan/devserver/devserver.log"),
        );
        assert!(plist.contains("<string>app.chan.devserver</string>"));
        assert!(plist.contains("<string>/usr/local/bin/chan</string>"));
        assert!(plist.contains("<string>devserver</string>"));
        assert!(plist.contains("<string>--bind=127.0.0.1</string>"));
        assert!(plist.contains("<string>--port=8799</string>"));
        assert!(plist.contains("<key>RunAtLoad</key>"));
        assert!(plist.contains("<key>SuccessfulExit</key>"));
        assert!(plist.contains("<string>/Users/x/.chan/devserver/devserver.log</string>"));
    }

    #[test]
    fn launch_agent_plist_escapes_xml_in_paths() {
        let plist = devserver_launch_agent_plist(
            Path::new("/opt/a & b/chan"),
            "127.0.0.1:1".parse().unwrap(),
            Path::new("/tmp/log"),
        );
        assert!(plist.contains("/opt/a &amp; b/chan"));
        assert!(!plist.contains("a & b/chan"));
    }

    #[test]
    fn launchd_print_running_reads_state() {
        // Tab-indented like real `launchctl print` output.
        assert!(launchd_print_running(
            "\tstate = running\n\tpid = 4321\n\tlast exit code = (never exited)\n"
        ));
        assert!(!launchd_print_running(
            "\tstate = not running\n\tlast exit code = (never exited)\n"
        ));
    }

    #[test]
    fn launchd_print_failed_only_on_nonzero_exit() {
        assert!(launchd_print_failed(
            "\tstate = not running\n\tlast exit code = 1\n"
        ));
        // A clean exit, a never-run service, and a running service are not failures.
        assert!(!launchd_print_failed(
            "\tstate = not running\n\tlast exit code = 0\n"
        ));
        assert!(!launchd_print_failed(
            "\tstate = not running\n\tlast exit code = (never exited)\n"
        ));
        assert!(!launchd_print_failed("\tstate = running\n\tpid = 5\n"));
    }
}
