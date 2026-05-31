//! The `cs` client surface: the clap subcommand tree (`ShellAction` /
//! `TerminalAction`) and the dispatch that turns each action into a
//! control-socket round-trip. Lifted verbatim out of the `chan` binary so
//! `chan-desktop` can drive the same `cs` commands without the `chan`
//! binary on PATH.
//!
//! RISK: the clap derive here is wire-load-bearing. Every flag name,
//! `infer_subcommands`, and arg shape is part of the `cs` contract; a
//! drift breaks commands at runtime with a green build. Wire-smoke every
//! `cs` command after touching this file, not just `cargo build`.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Subcommand;

use crate::control::{absolutize, control_socket_env, open_env, send_control_request};
use crate::submit::{apply_submit_chord, SubmitAgent};
use crate::wire::ControlRequest;

#[derive(Subcommand, Debug)]
pub enum ShellAction {
    /// Open a path in the current window (same as `chan open`). Without
    /// a path, opens the terminal's current directory in the browser.
    Open {
        #[arg(value_hint = clap::ValueHint::AnyPath)]
        path: Option<PathBuf>,
    },
    /// Open the documentation graph in the current window. With a path,
    /// focuses the graph on that file or directory.
    Graph {
        #[arg(value_hint = clap::ValueHint::AnyPath)]
        path: Option<PathBuf>,
    },
    /// Open a Dashboard tab in the current window.
    Dashboard {
        /// Initial carousel slide index (0-based). Out-of-range values
        /// land on the default slide.
        #[arg(long = "carousel-index")]
        carousel_index: Option<u32>,
        /// Open with carousel auto-rotation OFF (the new tab's
        /// `autoRotate` is false). Default leaves rotation on. Spelled
        /// one-r to match `--carousel-index`.
        #[arg(long = "carousel-off")]
        carousel_off: bool,
    },
    /// Terminal operations against the current window's live sessions.
    ///
    /// Prefix matching applies here too: `cs t n` / `cs t w` / `cs t l`
    /// resolve to terminal new / write / list.
    #[command(infer_subcommands = true)]
    Terminal {
        #[command(subcommand)]
        action: TerminalAction,
    },
    /// Run the same content search the UI does, against the running
    /// window's workspace. Prints a markdown table by default; `--json`
    /// emits compact machine output and `--json --pretty` indents it.
    Search {
        /// Query string. Multiple words are joined with spaces.
        #[arg(required = true, num_args = 1..)]
        query: Vec<String>,
        /// Maximum number of result rows (one per file). Default 20.
        #[arg(long)]
        limit: Option<u32>,
        /// Emit JSON instead of the markdown table. Compact by default.
        #[arg(long)]
        json: bool,
        /// With --json, pretty-print (indent) the JSON. Ignored without
        /// --json.
        #[arg(long)]
        pretty: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum TerminalAction {
    /// Open a new terminal tab in the current window.
    New {
        /// Working directory for the new terminal (workspace-relative or
        /// absolute under the workspace root). Defaults to the workspace
        /// root.
        #[arg(value_hint = clap::ValueHint::AnyPath)]
        path: Option<PathBuf>,
        /// Tab name ($CHAN_TAB_NAME inside the new terminal).
        #[arg(long = "tab-name")]
        tab_name: Option<String>,
        /// Broadcast group ($CHAN_TAB_GROUP). Defaults to "default".
        #[arg(long = "tab-group")]
        tab_group: Option<String>,
    },
    /// Write raw bytes to live terminal session(s), selected by name
    /// and/or group. No newline is appended: `cs terminal write $'ls\n'`
    /// runs; `cs terminal write ls` only types. At least one selector is
    /// required.
    Write {
        /// Literal bytes to write. Omit with --stdin to stream instead.
        cmd: Option<String>,
        /// Read the bytes from this process's stdin instead of `cmd`.
        #[arg(long)]
        stdin: bool,
        /// After the bytes, append the named agent's submit chord so a
        /// running agent submits the input hands-free (the completion-poke
        /// path). Trailing newlines are stripped first. Values:
        /// `claude` (Cmd+Enter chord), `codex` / `gemini` (plain CR).
        /// Omit it to write pure bytes: the input parks in the agent's
        /// compose box unsubmitted (a bare newline is a newline to an
        /// agent, not a submit).
        #[arg(long, value_name = "AGENT")]
        submit: Option<SubmitAgent>,
        /// Target every session with this tab name.
        #[arg(long = "tab-name")]
        tab_name: Option<String>,
        /// Target every session in this group (broadcast).
        #[arg(long = "tab-group")]
        tab_group: Option<String>,
    },
    /// List live terminal sessions, grouped by group. Markdown by
    /// default; `--json` for compact machine output, `--json --pretty`
    /// for indented JSON.
    List {
        /// Emit machine-readable JSON instead of the markdown table.
        #[arg(long)]
        json: bool,
        /// Indent the JSON output. Only meaningful with `--json`.
        #[arg(long)]
        pretty: bool,
    },
    /// Restart live terminal session(s) selected by name and/or group,
    /// preserving each session's spawn command and env so an agent
    /// relaunches. At least one selector is required. Used by the Team
    /// Work bootstrap to restart its own terminal (a shell cannot restart
    /// the shell running its own script; the server does it out of band).
    Restart {
        /// Restart every session with this tab name.
        #[arg(long = "tab-name")]
        tab_name: Option<String>,
        /// Restart every session in this group.
        #[arg(long = "tab-group")]
        tab_group: Option<String>,
    },
}

/// Dispatch a `cs <action>` against the current window's chan-server.
/// Was `cmd_shell` in the `chan` binary.
pub async fn dispatch(action: ShellAction) -> Result<()> {
    match action {
        ShellAction::Open { path } => {
            let env = open_env()?;
            // No path -> open the terminal's cwd in the browser.
            let abs = absolutize(path.unwrap_or(PathBuf::from(".")))?;
            let message = send_control_request(
                &env.control_socket,
                ControlRequest::OpenPath {
                    window_id: env.window_id,
                    path: abs,
                },
            )
            .await?;
            eprintln!("{message}");
            Ok(())
        }
        ShellAction::Graph { path } => {
            let env = open_env()?;
            let abs = path.map(absolutize).transpose()?;
            let message = send_control_request(
                &env.control_socket,
                ControlRequest::OpenGraph {
                    window_id: env.window_id,
                    path: abs,
                },
            )
            .await?;
            eprintln!("{message}");
            Ok(())
        }
        ShellAction::Dashboard {
            carousel_index,
            carousel_off,
        } => {
            let env = open_env()?;
            let message = send_control_request(
                &env.control_socket,
                ControlRequest::OpenDashboard {
                    window_id: env.window_id,
                    carousel_index,
                    carousel_off,
                },
            )
            .await?;
            eprintln!("{message}");
            Ok(())
        }
        ShellAction::Terminal { action } => cmd_shell_terminal(action).await,
        ShellAction::Search {
            query,
            limit,
            json,
            pretty,
        } => cmd_shell_search(query.join(" "), limit, json, pretty).await,
    }
}

/// `cs search <query>`: run the workspace content search on the running
/// server (the same `Workspace::search` the UI's `/api/search/content`
/// uses) and print the results. Markdown table by default; `--json`
/// compact, `--json --pretty` indented. Mirrors the `cs terminal list`
/// output convention.
async fn cmd_shell_search(
    query: String,
    limit: Option<u32>,
    json: bool,
    pretty: bool,
) -> Result<()> {
    let socket = control_socket_env()?;
    let raw = send_control_request(&socket, ControlRequest::Search { query, limit }).await?;
    if json {
        // Compact by default; --pretty re-indents. Both go to stdout so
        // the output pipes cleanly.
        if pretty {
            let value: serde_json::Value =
                serde_json::from_str(&raw).context("parsing search JSON")?;
            println!(
                "{}",
                serde_json::to_string_pretty(&value).context("formatting search JSON")?
            );
        } else {
            println!("{raw}");
        }
    } else {
        print!("{}", render_search_markdown(&raw)?);
    }
    Ok(())
}

/// Render the `cs search` result JSON
/// (`{ready, mode, query, hits: [{path, heading, start_line, snippet,
/// score}]}`) as a markdown list. This is the default human output;
/// `--json` emits the raw payload instead. No hits yields a short line
/// rather than an empty list.
fn render_search_markdown(raw: &str) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(raw).context("parsing search JSON")?;
    let hits = value
        .get("hits")
        .and_then(|h| h.as_array())
        .ok_or_else(|| anyhow::anyhow!("search JSON missing `hits`"))?;
    if hits.is_empty() {
        return Ok("No matches.\n".to_string());
    }
    let str_field = |h: &serde_json::Value, key: &str| {
        h.get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let mut out = String::new();
    for h in hits {
        let path = str_field(h, "path");
        let heading = str_field(h, "heading");
        let line = h.get("start_line").and_then(|v| v.as_u64()).unwrap_or(0);
        // `path:line` locator, then the best heading, then the snippet on
        // an indented continuation so the list stays scannable.
        if heading.is_empty() {
            out.push_str(&format!("- {path}:{line}\n"));
        } else {
            out.push_str(&format!("- {path}:{line} - {heading}\n"));
        }
        let snippet = str_field(h, "snippet");
        if !snippet.is_empty() {
            // The BM25 snippet highlights matches with `<b>...</b>` (the
            // markup the frontend renders); convert to markdown `**bold**`
            // for this markdown output. Collapse newlines so one hit stays
            // on one logical block.
            let flat = snippet
                .replace('\n', " ")
                .replace("<b>", "**")
                .replace("</b>", "**");
            out.push_str(&format!("  {}\n", flat.trim()));
        }
    }
    Ok(out)
}

async fn cmd_shell_terminal(action: TerminalAction) -> Result<()> {
    match action {
        TerminalAction::New {
            path,
            tab_name,
            tab_group,
        } => {
            let env = open_env()?;
            let abs = path.map(absolutize).transpose()?;
            let message = send_control_request(
                &env.control_socket,
                ControlRequest::OpenTermNew {
                    window_id: env.window_id,
                    path: abs,
                    tab_name,
                    tab_group,
                },
            )
            .await?;
            eprintln!("{message}");
            Ok(())
        }
        TerminalAction::Write {
            cmd,
            stdin,
            submit,
            tab_name,
            tab_group,
        } => {
            if tab_name.is_none() && tab_group.is_none() {
                anyhow::bail!("cs terminal write needs --tab-name and/or --tab-group");
            }
            // Raw bytes, no implicit newline (@@Alex decision). --stdin
            // reads this process's stdin to EOF; otherwise the literal
            // `cmd`. Terminal input is UTF-8 text.
            let data = if stdin {
                use std::io::Read;
                let mut buf = Vec::new();
                std::io::stdin()
                    .read_to_end(&mut buf)
                    .context("reading stdin")?;
                String::from_utf8(buf).context("stdin must be UTF-8 for cs terminal write")?
            } else {
                cmd.ok_or_else(|| anyhow::anyhow!("cs terminal write needs a command or --stdin"))?
            };
            // --submit=<agent>: strip trailing newlines then append that
            // agent's submit chord so a running agent submits the input
            // hands-free (the completion poke). Mirrors
            // `encodeForAgentSubmit` in web/src/terminal/submitMode.ts.
            let data = apply_submit_chord(data, submit);
            let socket = control_socket_env()?;
            let message = send_control_request(
                &socket,
                ControlRequest::TermWrite {
                    tab_name,
                    tab_group,
                    data,
                },
            )
            .await?;
            eprintln!("{message}");
            Ok(())
        }
        TerminalAction::List { json, pretty } => {
            let socket = control_socket_env()?;
            let raw = send_control_request(&socket, ControlRequest::TermList).await?;
            if json {
                // Compact by default; --pretty re-indents. Both go to
                // stdout so the output pipes cleanly.
                if pretty {
                    let value: serde_json::Value =
                        serde_json::from_str(&raw).context("parsing terminal list JSON")?;
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&value)
                            .context("formatting terminal list JSON")?
                    );
                } else {
                    println!("{raw}");
                }
            } else {
                print!("{}", render_terminal_list_markdown(&raw)?);
            }
            Ok(())
        }
        TerminalAction::Restart {
            tab_name,
            tab_group,
        } => {
            if tab_name.is_none() && tab_group.is_none() {
                anyhow::bail!("cs terminal restart needs --tab-name and/or --tab-group");
            }
            let socket = control_socket_env()?;
            let message = send_control_request(
                &socket,
                ControlRequest::TermRestart {
                    tab_name,
                    tab_group,
                },
            )
            .await?;
            eprintln!("{message}");
            Ok(())
        }
    }
}

/// Render the `cs terminal list` registry JSON
/// (`{groups: {group: [{name, session_id, cwd}]}}`) as a markdown table
/// grouped by terminal group. This is the default human output; `--json`
/// emits the raw payload instead. An empty registry yields a short line
/// rather than a blank table.
fn render_terminal_list_markdown(raw: &str) -> Result<String> {
    let value: serde_json::Value =
        serde_json::from_str(raw).context("parsing terminal list JSON")?;
    let groups = value
        .get("groups")
        .and_then(|g| g.as_object())
        .ok_or_else(|| anyhow::anyhow!("terminal list JSON missing `groups`"))?;
    if groups.is_empty() {
        return Ok("No live terminal sessions.\n".to_string());
    }
    let str_field = |s: &serde_json::Value, key: &str| {
        s.get(key)
            .and_then(|v| v.as_str())
            .unwrap_or("-")
            .to_string()
    };
    let mut out = String::new();
    for (group, sessions) in groups {
        out.push_str(&format!("## {group}\n\n"));
        out.push_str("| name | session | cwd |\n");
        out.push_str("| --- | --- | --- |\n");
        if let Some(arr) = sessions.as_array() {
            for s in arr {
                out.push_str(&format!(
                    "| {} | {} | {} |\n",
                    str_field(s, "name"),
                    str_field(s, "session_id"),
                    str_field(s, "cwd"),
                ));
            }
        }
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_markdown_converts_bold_highlight_and_locator() {
        let raw = r#"{"hits":[{"path":"a.md","start_line":3,"heading":"H","snippet":"the <b>fox</b> ran"}]}"#;
        let out = render_search_markdown(raw).expect("render");
        assert!(out.contains("- a.md:3 - H"), "locator: {out}");
        // <b>...</b> highlight -> markdown **bold**.
        assert!(out.contains("the **fox** ran"), "bold: {out}");
        assert!(!out.contains("<b>"), "no raw html: {out}");
    }

    #[test]
    fn terminal_list_markdown_empty_is_short_line() {
        let out = render_terminal_list_markdown(r#"{"groups":{}}"#).expect("render");
        assert_eq!(out, "No live terminal sessions.\n");
    }
}
