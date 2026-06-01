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
use clap::{Parser, Subcommand};

use crate::control::{absolutize, control_socket_env, open_env, send_control_request};
use crate::submit::{apply_submit_chord, SubmitAgent};
use crate::wire::{ControlRequest, SurveyFollowup, SurveySpec};

/// Top-level `cs` parser. The `chan` binary reaches `cs` through its own
/// `Cli` (rewriting `cs ...` into `chan shell ...` in `parse_cli`), but
/// `chan-desktop` has no `chan` binary, so it parses `cs` argv directly
/// through this. `infer_subcommands` mirrors the `chan shell` command so
/// `cs t l` / `cs o` resolve the same way under both front ends.
#[derive(Parser, Debug)]
#[command(
    name = "cs",
    about = "Drive the current chan window from its terminal."
)]
#[command(infer_subcommands = true)]
struct CsCli {
    #[command(subcommand)]
    action: ShellAction,
}

/// Parse a full `cs` argv (argv[0] included) and dispatch it. The entry
/// `chan-desktop` calls when invoked through a `cs` name, so desktop users
/// get the `cs` client without a `chan` binary on PATH. clap prints help /
/// usage and exits on a parse error or `--help`, exactly like the `chan`
/// binary's `Cli::parse_from`.
pub async fn run_cs<I>(args: I) -> Result<()>
where
    I: IntoIterator,
    I::Item: Into<std::ffi::OsString> + Clone,
{
    dispatch(CsCli::parse_from(args).action).await
}

#[derive(Subcommand, Debug)]
pub enum ShellAction {
    /// Open a path in the current window. Without a path, opens the
    /// terminal's current directory in the browser.
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
    /// Raise a survey over the SPA window(s) that own the matching
    /// terminal tab(s) and BLOCK until the user answers. Prints the chosen
    /// option label to stdout, or (on `[F]`) the path of the followup file
    /// the UI created. At least one selector is required. Used by an agent
    /// to ask @@Host a question and wait for the decision.
    Survey {
        /// Raise the survey on the window owning this tab name.
        #[arg(long = "tab-name")]
        tab_name: Option<String>,
        /// Raise the survey on every window owning a tab in this group.
        #[arg(long = "tab-group")]
        tab_group: Option<String>,
        /// Optional heading shown above the body.
        #[arg(long)]
        title: Option<String>,
        /// An answer option (1..=4). Repeat for each: `--option A
        /// --option B`. The UI numbers them [1]..[4].
        #[arg(long = "option", value_name = "LABEL")]
        option: Vec<String>,
        /// Also offer an `[F]` follow-up affordance (the UI writes a
        /// followup file and returns its path instead of an option).
        /// Requires `--followup-dir` so the followup is always team-scoped.
        #[arg(long, requires = "followup_dir")]
        followup: bool,
        /// Team directory (workspace-relative) the `[F]` followup file is
        /// created under, at `{dir}/followups/followup-{from}-{to}-{n}.md`.
        /// Required with `--followup`.
        #[arg(long = "followup-dir", value_name = "TEAM_DIR")]
        followup_dir: Option<String>,
        /// Override the followup author (`from`). Defaults to
        /// `$CHAN_TAB_NAME` (the surveying agent's tab). Only used with
        /// `--followup`.
        #[arg(long)]
        from: Option<String>,
        /// Override the followup target (`to`). Defaults to the survey
        /// target (`--tab-name`, or `--tab-group` for a group). Only used
        /// with `--followup`.
        #[arg(long)]
        to: Option<String>,
        /// Read the markdown problem body from this process's stdin
        /// instead of the positional `body` (handy for multi-line bodies).
        #[arg(long)]
        stdin: bool,
        /// The markdown problem body. Multiple words join with spaces.
        /// Omit only with `--stdin`.
        #[arg(num_args = 0..)]
        body: Vec<String>,
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
        TerminalAction::Survey {
            tab_name,
            tab_group,
            title,
            option,
            followup,
            followup_dir,
            from,
            to,
            stdin,
            body,
        } => {
            cmd_shell_survey(SurveyArgs {
                tab_name,
                tab_group,
                title,
                option,
                followup,
                followup_dir,
                from,
                to,
                stdin,
                body,
            })
            .await
        }
    }
}

/// The parsed `cs terminal survey` arguments, grouped so the dispatch does
/// not pass ten positional parameters around.
struct SurveyArgs {
    tab_name: Option<String>,
    tab_group: Option<String>,
    title: Option<String>,
    option: Vec<String>,
    followup: bool,
    followup_dir: Option<String>,
    from: Option<String>,
    to: Option<String>,
    stdin: bool,
    body: Vec<String>,
}

/// `cs terminal survey`: build a [`SurveySpec`] and round-trip a BLOCKING
/// [`ControlRequest::TermSurvey`]. The server holds the connection open
/// until the user answers, so this call blocks; the reply (the chosen
/// option label, or the followup-file path on `[F]`) goes to stdout so it
/// pipes cleanly, matching the "the tool returns that option" contract.
async fn cmd_shell_survey(args: SurveyArgs) -> Result<()> {
    let SurveyArgs {
        tab_name,
        tab_group,
        title,
        option,
        followup,
        followup_dir,
        from,
        to,
        stdin,
        body,
    } = args;

    if tab_name.is_none() && tab_group.is_none() {
        anyhow::bail!("cs terminal survey needs --tab-name and/or --tab-group");
    }
    // The contract caps options at 1..=4 (the UI numbers them [1]..[4]).
    if option.is_empty() || option.len() > 4 {
        anyhow::bail!(
            "cs terminal survey needs 1..=4 --option values (got {})",
            option.len()
        );
    }
    // Body comes from stdin (multi-line bodies) or the positional words.
    let body_markdown = if stdin {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("reading survey body from stdin")?;
        buf
    } else {
        body.join(" ")
    };
    if body_markdown.trim().is_empty() {
        anyhow::bail!("cs terminal survey needs a markdown body (positional words or --stdin)");
    }
    // The [F] team context (2026-06-01 amendment): only populated when
    // --followup is set, so a survey without a followup carries `null`.
    // clap already guarantees --followup-dir is present when --followup is.
    let followup_ctx = if followup {
        Some(build_followup(
            followup_dir,
            from,
            to,
            &tab_name,
            &tab_group,
        )?)
    } else {
        None
    };
    let spec = SurveySpec {
        // Server-minted; left empty here (see SurveySpec docs).
        survey_id: String::new(),
        title,
        body_markdown,
        options: option,
        allow_followup: followup,
        followup: followup_ctx,
    };
    let socket = control_socket_env()?;
    let message = send_control_request(
        &socket,
        ControlRequest::TermSurvey {
            tab_name,
            tab_group,
            spec,
        },
    )
    .await?;
    // The reply is the result the caller wants captured, so it goes to
    // stdout (unlike the queued-request acks the other commands eprintln).
    println!("{message}");
    Ok(())
}

/// Resolve the `[F]` followup team context, reading `$CHAN_TAB_NAME` from the
/// process env and delegating the pure precedence to [`resolve_followup`] (so
/// the derivation is unit-testable without touching the environment, the same
/// split as `open_env` / `open_env_from`).
fn build_followup(
    followup_dir: Option<String>,
    from: Option<String>,
    to: Option<String>,
    tab_name: &Option<String>,
    tab_group: &Option<String>,
) -> Result<SurveyFollowup> {
    resolve_followup(
        followup_dir,
        std::env::var("CHAN_TAB_NAME").ok(),
        from,
        to,
        tab_name.clone(),
        tab_group.clone(),
    )
}

/// The pure followup-context precedence per the 2026-06-01 amendment:
/// `from` <- `$CHAN_TAB_NAME` (fallback `--from`); `to` <- the survey target
/// (`--tab-name`, then `--tab-group`; fallback `--to`). `dir` comes straight
/// from `--followup-dir` (clap-required with `--followup`). Bails with a clear
/// message if `dir`/`from`/`to` cannot be resolved, so a followup is always
/// well-named and team-scoped.
fn resolve_followup(
    followup_dir: Option<String>,
    env_tab_name: Option<String>,
    from: Option<String>,
    to: Option<String>,
    tab_name: Option<String>,
    tab_group: Option<String>,
) -> Result<SurveyFollowup> {
    let trimmed = |s: String| {
        let s = s.trim().to_string();
        (!s.is_empty()).then_some(s)
    };
    let dir = followup_dir
        .and_then(trimmed)
        .ok_or_else(|| anyhow::anyhow!("--followup-dir is required with --followup"))?;
    // from: the surveying agent's own tab, overridable with --from.
    let from = env_tab_name
        .and_then(trimmed)
        .or_else(|| from.and_then(trimmed))
        .ok_or_else(|| {
            anyhow::anyhow!("--followup needs a `from`: set $CHAN_TAB_NAME or pass --from")
        })?;
    // to: the survey target. A selector is always present (checked by the
    // caller), so the tab name / group resolves; --to is the final fallback.
    let to = tab_name
        .and_then(trimmed)
        .or_else(|| tab_group.and_then(trimmed))
        .or_else(|| to.and_then(trimmed))
        .ok_or_else(|| {
            anyhow::anyhow!("--followup needs a `to` target (--tab-name/--tab-group)")
        })?;
    Ok(SurveyFollowup { dir, from, to })
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

    #[test]
    fn resolve_followup_prefers_env_tab_name_and_tab_name_target() {
        // from <- $CHAN_TAB_NAME; to <- --tab-name; --from/--to ignored when
        // the higher-priority sources are present.
        let f = resolve_followup(
            Some("team-a".into()),
            Some("LaneD".into()),
            Some("ignored-from".into()),
            Some("ignored-to".into()),
            Some("Architect".into()),
            Some("group-x".into()),
        )
        .expect("resolve");
        assert_eq!(f.dir, "team-a");
        assert_eq!(f.from, "LaneD");
        assert_eq!(f.to, "Architect");
    }

    #[test]
    fn resolve_followup_falls_back_to_from_flag_and_group_then_to_flag() {
        // No env tab name -> --from. No tab name -> --tab-group for `to`.
        let f = resolve_followup(
            Some("team-a".into()),
            None,
            Some("flag-from".into()),
            None,
            None,
            Some("group-x".into()),
        )
        .expect("resolve");
        assert_eq!(f.from, "flag-from");
        assert_eq!(f.to, "group-x");

        // No tab name and no group -> --to fallback.
        let f = resolve_followup(
            Some("team-a".into()),
            Some("LaneD".into()),
            None,
            Some("flag-to".into()),
            None,
            None,
        )
        .expect("resolve");
        assert_eq!(f.to, "flag-to");
    }

    #[test]
    fn resolve_followup_requires_dir_from_and_to() {
        // Missing / blank dir.
        assert!(resolve_followup(
            Some("  ".into()),
            Some("LaneD".into()),
            None,
            None,
            Some("Architect".into()),
            None,
        )
        .is_err());
        // No from anywhere.
        assert!(resolve_followup(
            Some("team-a".into()),
            None,
            None,
            None,
            Some("Architect".into()),
            None,
        )
        .is_err());
        // No to anywhere.
        assert!(resolve_followup(
            Some("team-a".into()),
            Some("LaneD".into()),
            None,
            None,
            None,
            None,
        )
        .is_err());
    }
}
