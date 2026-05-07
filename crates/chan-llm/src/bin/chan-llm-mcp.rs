//! `chan-llm-mcp`: standalone MCP server binary.
//!
//! Any MCP client (Claude Desktop, Claude Code, Cursor, Continue,
//! ...) can spawn this process to gain chan-core-sandboxed access
//! to a chan drive's read/write/list/search tools.
//!
//! Usage:
//!
//!     chan-llm-mcp --drive /path/to/drive [--config /path/to/llm.toml]
//!                  [--auto-apply]
//!
//! `--auto-apply` is the explicit, opt-in knob for letting the
//! server's `write_file` tool hit disk without producing a
//! "deferred" error. The default is OFF: the standalone binary
//! still works for clients (Claude Desktop, Cursor, ...) that
//! surface their own confirmation UI, but they have to flip the
//! flag themselves once their user has consented. The embedded
//! ClaudeCli path in chan-llm (issue #1) flips it from
//! `LlmConfig.auto_apply_writes` so the user's preference in the
//! chan UI carries through to the MCP subprocess.

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

    let server = Server::new(drive, args.auto_apply);
    if let Err(e) = runtime.block_on(server.serve_stdio()) {
        eprintln!("chan-llm-mcp: {e}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

const USAGE: &str = "\
chan-llm-mcp - MCP server exposing chan drive tools over stdio

USAGE:
    chan-llm-mcp --drive <path> [--config <path>] [--auto-apply]

OPTIONS:
    --drive <path>     Absolute path of the chan drive to expose.
                       Must already be registered (use `chan drive add`).
    --config <path>    Override for the chan-core registry config
                       (defaults to ~/.chan/config.toml).
    --auto-apply       Apply write_file tool calls without producing
                       a 'deferred' error. Off by default: the MCP
                       client is expected to surface a confirmation
                       UI before flipping this on.
    -h, --help         Print this help.
";

#[derive(Default)]
struct Args {
    drive: Option<PathBuf>,
    config: Option<PathBuf>,
    auto_apply: bool,
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
                "--auto-apply" => out.auto_apply = true,
                "-h" | "--help" => out.help = true,
                other => return Err(format!("unknown argument: {other}")),
            }
        }
        Ok(out)
    }
}
