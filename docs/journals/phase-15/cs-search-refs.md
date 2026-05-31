# cs search - sha-verified source refs (from @@Architect, for @@LaneC during the read outage)

All quotes below are from `crates/chan/src/main.rs` at HEAD `cf2c8b2c`,
sha256 `713bc4de8f15dba12eb4cd321d9d60a1fef0a7357ec11c9db524bf0028a89130`
(disk == git blob, verified). Trust these over a confabulating read; re-`shasum`
the file before you edit and confirm it still matches.

`cs search` is a TOP-LEVEL `ShellAction` (like open/graph/dashboard) but needs a
READ-BACK, so it mirrors the `TerminalAction::List` pattern, NOT the
fire-and-forget OpenPath arms.

## 1. ShellAction enum (:432) - add a `Search` variant here

The Dashboard variant shows the flag style; copy `{ json, pretty }` from List:

```
enum ShellAction {
    Open { path: Option<PathBuf> },
    Graph { path: Option<PathBuf> },
    Dashboard { #[arg(long = "carousel-index")] carousel_index: Option<u32> },
    #[command(infer_subcommands = true)]
    Terminal { #[command(subcommand)] action: TerminalAction },
    // ADD:
    // /// Search workspace content (same as the UI search). Markdown by
    // /// default; --json for machine output, --json --pretty to indent.
    // Search { query: String, #[arg(long)] json: bool, #[arg(long)] pretty: bool },
}
```

## 2. ControlRequest (:1877) - add `Search { query }` in the Category-2 block

`TermList` is the precedent (a read-back with no window_id):

```
enum ControlRequest {
    // Category 1 (window_id, fire-and-forget): OpenPath / OpenGraph /
    //   OpenTermNew / OpenDashboard
    // Category 2 (no window_id, server resolves): TermWrite / TermList /
    //   TermRestart
    TermList,
    // ADD: Search { query: String },
}

// ControlResponse (:1924) - the read-back returns its payload as `message`:
enum ControlResponse { Ok { message: String }, Error { message: String } }
```

So your SERVER handler (control_socket.rs) runs Workspace::search, serializes
the results to a JSON string, and returns `ControlResponse::Ok { message: <that
JSON> }`. The client renders markdown/json from that string (below).

## 3. THE TEMPLATE - TerminalAction::List read-back arm (cmd_shell_terminal :2101)

Your `ShellAction::Search` arm in `cmd_shell` (:1996) is this, verbatim shape:

```
TerminalAction::List { json, pretty } => {
    let socket = control_socket_env()?;
    let raw = send_control_request(&socket, ControlRequest::TermList).await?;
    if json {
        if pretty {
            let value: serde_json::Value =
                serde_json::from_str(&raw).context("parsing terminal list JSON")?;
            println!("{}", serde_json::to_string_pretty(&value)
                .context("formatting terminal list JSON")?);
        } else {
            println!("{raw}");
        }
    } else {
        print!("{}", render_terminal_list_markdown(&raw)?);
    }
    Ok(())
}
```

For Search: swap `ControlRequest::TermList` -> `ControlRequest::Search { query }`
and `render_terminal_list_markdown` -> a new `render_search_markdown(&raw)`
(mirror that helper). Put the arm in `cmd_shell` (ShellAction dispatch), not
cmd_shell_terminal.

## 4. send_control_request (:2187) - unchanged, use as-is

Returns the `ControlResponse::Ok { message }` string (or bails on Error). Same
call site shape as List. No edit needed here.

## Net change set for cs search (3 files, additive)
- main.rs: ShellAction::Search variant + cmd_shell arm + render_search_markdown.
- control_socket.rs (you have this): ControlRequest::Search + handler ->
  Workspace::search -> JSON -> Ok{message}.
- chain `git add` the search hunks only; gate; chained-commit on cf2c8b2c.
