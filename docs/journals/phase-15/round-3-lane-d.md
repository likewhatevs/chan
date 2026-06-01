# Phase-15 round-3 - @@LaneD (Desktop + CLI)

You are @@LaneD. Read `round-3-bootstrap.md` (process) and `round-3-status.md`
(active wave) first; the technical source is `round-3-plan.md` (Theme 2). You
own the CLI / control-socket / desktop surface and are the SINGLE owner of those
shared files. Spawn subagents as needed.

## Your files (no other lane edits these)

- crates/chan/src/main.rs (CLI surface) + new crates/chan-shell/**
- crates/chan-server/src/control_socket.rs (ControlRequest, incl. TermSurvey)
- crates/chan-server/src/terminal_sessions.rs (SubmitMode + survey dispatch)
- web/src/.../submitMode.ts
- desktop/** (Tauri shell, tauri.conf.json)

@@LaneC routes its survey-transport needs THROUGH you; you add the TermSurvey
frame + command per the survey contract @@Architect holds. Do not let @@LaneC
edit these files.

## Your work scope, by wave

### Wave 1 - cs-shell extraction + per-agent submit map (FOUNDATION)

- Extract the cs CLIENT into a new `crates/chan-shell` crate that both `chan`
  and `chan-desktop` depend on: `ShellAction` / `TerminalAction`
  (main.rs:435-554), `cmd_shell*` / `send_control_request` (~2082-2389), the
  client `ControlRequest` / `ControlResponse` (~1911-1967, a DUP of
  control_socket.rs:33-95 - UNIFY it), `open_env` / `control_socket_env`, the
  render helpers, `AGENT_SUBMIT_CHORD`, and the `argv[0]=="cs"` rewrite
  (`parse_cli` ~769-786). RISK: cross-crate clap derive + serde tags must stay
  BYTE-IDENTICAL or every cs command breaks at runtime (gate-blind) - WIRE-SMOKE
  every cs command (new + the `cs` alias), not just a green build.
- Per-agent submit-encoding map: one shared map across main.rs
  `apply_submit_chord`, terminal_sessions `SubmitMode::submit_chord`, and
  submitMode.ts `encodeForAgentSubmit`. Shape `--submit=<agent>`
  (claude=`\x1b[27;9;13~`, codex=`\r`, gemini=probe live in a chan terminal);
  unset = pure bytes (already the default). Add REAL codex + gemini auto-submit
  smoke tests (@@Host: "we absolutely need this").
- This wave UNBLOCKS @@LaneC's Wave-2 survey command, so land it clean + early.

### Wave 2 - desktop shell + argv0 + remove chan open + survey transport

- chan-desktop `shell` + `argv[0]=="cs"` so desktop users get cs + MCP without
  the `chan` binary (chan-desktop depends on chan-shell).
- Remove `chan open` from `chan`; move the OS-file-association + handoff entry
  into chan-desktop (cmd_open ~2021-2063, `maybe_handoff_to_desktop` ~1379-1425).
  The inside-terminal "open in current window" stays as `cs open`
  (`chan shell open`).
- Linux AppImage `cs` story: implement the chosen option (recommended:
  chan-desktop installs a `cs` wrapper into `~/.local/bin` on first run with
  argv[0] detection). Confirm the choice with @@Architect.
- Survey transport for @@LaneC: add the `control_socket` TermSurvey frame, the
  `cs terminal survey` command, the WindowCommand that shows the SPA overlay,
  and carry the chosen-option/followup-path reply back to the BLOCKED CLI, per
  the survey contract.

### Wave 3 - smoke + polish

- The multi-agent submit / team-work plumbing smoke tests (with @@LaneC).
  Desktop polish, AppImage `cs` verify, carryover buffer.

## Touch points (@@Architect arbitrates)

- D->C (Wave 1->2): chan-shell must land in your Wave 1 before @@LaneC's survey
  command; you build the survey TRANSPORT in Wave 2 per the shared payload/reply
  shape.
- You provide the per-agent submit map that @@LaneC's team-config agent field
  consumes.

## Completion (each wave)

Gated-green + local merge + journal entry + poke @@Architect "wave N done".
