# C3b + S1 wire-shape draft + the $CHAN_WINDOW_ID self-hosting finding (@@LaneA)

Drafted while C3b/S1 are HELD for @@Lead's go (post team-server rebuild).
Design only; no implementation lands until @@Lead sequences it.

## FINDING: agent terminals can't target a window (the $CHAN_WINDOW_ID gap)

@@Lead's question answered honestly:

- My C3a e2e used a REAL BROWSER window (Chrome, `?w=smoke`) for the SPA, so
  the Svelte `handleWindowCommand` responder + the layout it returned were
  genuinely exercised. BUT I ran `cs pane` from my Bash tool with
  `CHAN_WINDOW_ID=smoke` set MANUALLY to match the browser. I did NOT run it
  from an agent terminal.
- Confirmed empirically: my agent terminal (@@LaneA) has an EMPTY
  `$CHAN_WINDOW_ID`. So `cs pane` from here fails at `open_env()`
  ("not running inside a chan session; this needs $CHAN_WINDOW_ID").
- This is PRE-EXISTING and structural, not a C3a regression: every
  category-1 window-targeting command (`cs open`, `cs graph`,
  `cs dashboard`, `cs terminal new`) needs `$CHAN_WINDOW_ID` the same way.
  C3a's `cs pane` correctly mirrors that contract.

Root cause (code): `spawn_team` (control_socket.rs) creates agent sessions
with `window_id: None` (no client attached at spawn). The PTY env is fixed
at spawn, so `$CHAN_WINDOW_ID` is never set. A later SPA WS-attach to an
existing session uses `attach_for_ws` (attach, not create), so it does NOT
backfill the registry `window_id` either. Net: a team-spawned agent session
has NO window_id anywhere -> `window_ids_matching` returns nothing for it,
so a `--tab-name` selector (the survey pattern) would ALSO fail to resolve
an agent's own window.

Why surveys still work this round: `cs terminal survey` targets @@Host's
tab, and @@Host is in a real browser SPA window (window_id present), so
`window_ids_matching(@@Host-tab)` finds it. Agent-to-agent or
agent-self-inspection has no window to resolve.

### Fix options (an architecture call for @@Lead)

1. **Bind agent sessions to the displaying window (the real enabler).**
   When the orchestrator brings a team up IN an SPA window, pass that
   window's id down to `spawn_team` so each agent session carries it ->
   `$CHAN_WINDOW_ID` is set in the agent shell, and `cs pane` (and
   `cs open`, etc.) "just work" from any agent terminal, targeting the
   window the agent is displayed in. Needs the spawn path to learn the
   window id (the SPA Team Work path has it; the `cs terminal team` CLI
   path is category-2 with no window_id today -- it would need one).
2. **`cs pane --tab-name <X>` selector (survey pattern), $CHAN_WINDOW_ID as
   fallback.** Resolve the target window via `window_ids_matching` like
   survey does. Works for any tab whose session has a window_id (e.g. an
   agent naming a tab in @@Host's browser window). Does NOT help an agent
   inspect a window that owns only window_id-less sessions. Cheap, additive,
   and useful regardless of (1); I'd fold it into C3a's `cs pane`.
3. Both: (1) makes self-hosting transparent; (2) is a general selector.

Recommendation: do (2) now as a small `cs pane` enhancement (it's the same
selector survey already proved), and treat (1) as an S1-adjacent decision
(it also affects whether `cs terminal team new/load` should carry a
window_id so spawned agents are window-bound). Your call on sequencing.

## C3b wire shapes (exec ops on the C3a channel)

Extends the same window bus + `/api/window/reply` round-trip. New request:

```
ControlRequest::PaneExec {
    window_id: String,           // or resolved via --tab-name (see fix 2)
    op: PaneOp,
}
PaneOp =
  | Focus  { pane_id: String }
  | Split  { pane_id: Option<String>, dir: SplitDir }   // dir = left | bottom
  | Resize { pane_id: Option<String>, dir: SplitDir, amount: f64 }  // ratio delta
  | CloseTab  { pane_id: Option<String>, tab_id: Option<String>, force: bool }
  | ClosePane { pane_id: Option<String>, force: bool }
  | CloseAll  { force: bool }
```

Server pushes `WindowCommand::PaneExec { request_id, op }`; the SPA executes
via the tabs API (`setActivePane`/`splitPane`/`paneModeResize`/`closeTab`/
`closePane`) and replies `{ ok, applied, blocked: [...] }`. A close that
hits a dirty FileTab or live TerminalTab WITHOUT `force` is reported in
`blocked` (PARTIAL FAILURE, non-zero exit); `--force` closes anyway. The CLI
prints the result. `dir` maps `left`->split row before, `bottom`->split
column after (the tabs `splitPane(paneId, "row"|"column", placement)`).

CLI: `cs pane focus <pane> | split left|bottom [pane] | resize ... |
close [--tab <id>] [--pane <id>] [--all] [--force]`. (Bare `cs pane`
stays the C3a query.)

## S1 wire shape (SPA-visible CLI team spawn)

Goal: `cs terminal team new` surfaces in the running SPA (today the spawn is
server-side via the registry; the SPA only learns about the sessions when it
next attaches). Push a server->SPA window_command after spawn so the SPA
opens/attaches the new team's tabs in the live window:

```
WindowCommand::TeamSpawned {
    group: String,
    members: Vec<{ tab_name: String, session_id: String }>,
}
```

The SPA's `handleWindowCommand` opens a terminal tab per member, attaching
to the existing `session_id` (the server already spawned the PTY), grouped
under `group`. This reuses the C3a channel direction (server->SPA push); no
reply needed (fire-and-forget), so it does NOT need the window bus -- just
`send_window_command`. Couples with fix (1): if the spawn is window-bound,
S1's push goes to that window naturally.

Open question for S1: which window receives the TeamSpawned push? Same
$CHAN_WINDOW_ID question -- the `cs terminal team` CLI path has no window_id
today. Resolving fix (1) answers this too.
