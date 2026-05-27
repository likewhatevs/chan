//! `chan-llm-mcp`: standalone MCP server binary.
//!
//! Any MCP client (Claude Desktop, Claude Code, Cursor, Continue,
//! ...) can spawn this process to gain chan-drive-sandboxed access
//! to a chan drive's read/write/list/search tools.
//!
//! Usage:
//!
//!     chan-llm-mcp --drive /path/to/drive [--config /path/to/config.toml]
//!                  [--max-media-bytes N]
//!
//! `write_file` writes apply immediately through chan-drive's
//! sandbox; the MCP client is responsible for any user
//! confirmation before invoking the tool.
//!
//! `--max-media-bytes N` overrides the per-response cap on
//! `read_media` (default 10 MiB). The standalone binary keeps this
//! as a CLI flag so it can stay independent of app settings.

use std::path::PathBuf;
use std::process::ExitCode;

use chan_llm::mcp::Server;
use chan_workspace::Library;

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

    let drive = match lib.open_workspace(&drive_root) {
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

    let mut server = Server::new(drive);
    if let Some(cap) = args.max_media_bytes {
        server = server.with_max_media_bytes(cap);
    }
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
                 [--max-media-bytes <N>]

OPTIONS:
    --drive <path>           Absolute path of the chan drive to expose.
                             Must already be registered
                             (use `chan drive add`).
    --config <path>          Override for the chan-drive registry config
                             (defaults to ~/.chan/config.toml).
    --max-media-bytes <N>    Hard cap on a single read_media response,
                             in bytes. Default 10 MiB. Oversized files
                             error with `media too large` instead of
                             being silently downscaled.
    -h, --help               Print this help.
";

#[derive(Default)]
struct Args {
    drive: Option<PathBuf>,
    config: Option<PathBuf>,
    max_media_bytes: Option<u64>,
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
                "--max-media-bytes" => {
                    let v = it.next().ok_or("--max-media-bytes needs a value")?;
                    let n: u64 = v
                        .parse()
                        .map_err(|_| format!("--max-media-bytes: not a u64: {v}"))?;
                    out.max_media_bytes = Some(n);
                }
                "-h" | "--help" => out.help = true,
                other => return Err(format!("unknown argument: {other}")),
            }
        }
        Ok(out)
    }
}
