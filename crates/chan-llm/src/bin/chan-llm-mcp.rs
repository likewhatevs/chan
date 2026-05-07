//! `chan-llm-mcp`: standalone MCP server binary.
//!
//! Any MCP client (Claude Desktop, Claude Code, Cursor, Continue,
//! ...) can spawn this process to gain chan-core-sandboxed access
//! to a chan drive's read/write/list/search tools.
//!
//! Usage:
//!
//!     chan-llm-mcp --drive /path/to/drive [--config /path/to/llm.toml]
//!
//! `auto_apply_writes` is forced to true here: the standalone path
//! has no host UI to defer to, and confirmation is the MCP client's
//! responsibility (Claude Code's permission prompt, Cursor's tool
//! gating, etc.). The embedded ClaudeCli backend (issue #1) follows
//! a different code path where chan-llm itself owns the gate.

use std::path::PathBuf;
use std::process::ExitCode;

use chan_core::Library;
use chan_llm::mcp::Server;

fn main() -> ExitCode {
    // Note: rmcp uses `tracing` for protocol logging but we don't
    // install a subscriber here to keep the binary dep-light. When
    // an MCP client integration breaks, the eprintln paths below
    // surface the actual error; deeper protocol traces are a
    // follow-up (add `tracing-subscriber` under the `mcp` feature).

    let args: Args = match Args::parse(std::env::args().skip(1)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("chan-llm-mcp: {e}");
            eprintln!();
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };

    if args.help {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }

    let Some(drive_root) = args.drive else {
        eprintln!("chan-llm-mcp: --drive is required");
        eprintln!();
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };

    let lib = match args.config {
        Some(path) => Library::open_at(path),
        None => Library::open(),
    };
    let lib = match lib {
        Ok(l) => l,
        Err(e) => {
            eprintln!("chan-llm-mcp: open library: {e}");
            return ExitCode::FAILURE;
        }
    };

    let drive = match lib.open_drive(&drive_root) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("chan-llm-mcp: open drive {}: {e}", drive_root.display());
            eprintln!(
                "(if the path isn't registered yet, run `chan drive add {}` first.)",
                drive_root.display()
            );
            return ExitCode::FAILURE;
        }
    };

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("chan-llm-mcp: build runtime: {e}");
            return ExitCode::FAILURE;
        }
    };

    let server = Server::new(drive, /* auto_apply_writes = */ true);
    if let Err(e) = runtime.block_on(server.serve_stdio()) {
        eprintln!("chan-llm-mcp: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

const USAGE: &str = "\
chan-llm-mcp - MCP server exposing chan drive tools over stdio

USAGE:
    chan-llm-mcp --drive <path> [--config <path>]

OPTIONS:
    --drive <path>     Absolute path of the chan drive to expose.
                       Must already be registered (use `chan drive add`).
    --config <path>    Override for the chan-core registry config
                       (defaults to ~/.chan/config.toml).
    -h, --help         Print this help.
";

#[derive(Default)]
struct Args {
    drive: Option<PathBuf>,
    config: Option<PathBuf>,
    help: bool,
}

impl Args {
    fn parse<I: Iterator<Item = String>>(mut it: I) -> Result<Self, String> {
        let mut out = Args::default();
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--drive" => {
                    let v = it.next().ok_or("--drive needs a value")?;
                    out.drive = Some(PathBuf::from(v));
                }
                "--config" => {
                    let v = it.next().ok_or("--config needs a value")?;
                    out.config = Some(PathBuf::from(v));
                }
                "-h" | "--help" => out.help = true,
                other => return Err(format!("unknown argument: {other}")),
            }
        }
        Ok(out)
    }
}
