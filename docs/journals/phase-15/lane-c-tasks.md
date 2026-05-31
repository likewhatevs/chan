# Lane C — Terminal + chan shell

## Bootstrap prompt

You are **@@LaneC**. Read `bootstrap.md`, then this file, then
`plan-round-1.md`, then `coordination.md` (do not read `roadmap-round-1.md` —
it is @@Alex's and already decomposed here). Mission: fix the Shift+Enter
regression,
add terminal tab-groups with group-scoped broadcast, and build the new
`chan shell` / `cs` CLI on the existing control-socket push path. Two
subagents: **C-term** (terminal behavior) and **C-shell** (the CLI).
Coordinate directly with @@Alex; cut tasks to @@LaneA (for `tabs.svelte.ts`
DashboardTab/flip region) only if truly needed. Confirm understanding, then
wait for the go.

**You own:** `keymap.ts`, `TerminalTab.svelte`, `session.ts`,
`routes/terminal.rs`, `terminal_sessions.rs`, `control_socket.rs`,
`main.rs`, the `TerminalTab` group field + broadcast/group fns of
`tabs.svelte.ts`, and the `handleWindowCommand` region of `store.svelte.ts`.
@@LaneA owns the `DashboardTab`/`flipHybrid` region of `tabs.svelte.ts`;
@@LaneB owns the scope region of `store.svelte.ts` — stay in your regions.

## Tasks (see plan-round-1.md for root causes + file:line refs)

- **C1 (C-term).** Shift+Enter regression (BUG-3). Fix the stale-zero
  keyboard-protocol state after reconnect. Validate with a real agent
  (claude/codex) in the terminal; if no agent CLI in the test env, flag
  empirically-unverified to @@Alex.
- **C2 (C-term).** Tab-groups + group broadcast. Group is a plain string on
  `TerminalTab` (default `"default"`), **no lifecycle/pool** (see
  plan-round-1). "Group" context-menu field, `$CHAN_TAB_GROUP` env, thread
  through WS query / restart / `CreateOptions`, scope the Cmd+Shift+I client
  broadcast to same-`group` terminals. Also store `tab_name`/`tab_group` per
  live session in the `TerminalRegistry` (so `cs term list`/`write` can
  resolve groups server-side). **Reaches `CK-GROUP`** -> tell C-shell.
  Validate with 4 terminals (2 `default`, 2 `foobar`).
- **SHELL-1 (C-shell).** `Command::Shell` + `cmd_shell` + `argv[0]=="cs"`
  dispatch + symlink-trigger integration test (no symlink in the build).
  `term` is a nested subcommand group: `new` / `write` / `list`.
- **SHELL-2 (C-shell).** Extend `control_socket.rs`. Category 1 (open a UI
  tab: `open`/`graph`/`term new`/`dashboard`): push a `window_command`
  (`open` reuses `OpenPath`). Category 2 (`term write`/`term list`): take a
  **read handle** to the `TerminalRegistry` (plumb it at the control-socket
  construction site; do **not** edit `terminal_sessions.rs`). `term write`
  resolves sessions by `--tab-name` / `--tab-group` and calls
  `session.send_input()`; `term list` returns JSON grouped by group on the
  same connection. The group resolution is **gated on `CK-GROUP`**. Clear
  error outside chan's terminal.
- **SHELL-3 (C-shell).** Extend `handleWindowCommand` + `WindowCommandFrame`
  with `open_graph` / `open_term_new` / `open_dashboard` arms (existing
  open-tab fns). No SPA arm for `term write`/`term list` (both server-side).

## Coordination checkpoints
- `CK-GROUP`: C-term -> C-shell (gates the `term_write` group fan-out arm).
- `terminal_sessions.rs` is C-term only now; C-shell stays out of it.

## Open decisions to raise with @@Alex
- Exact Shift+Enter fix (decide empirically).
- (Resolved) Group model: plain string label, no pool/lifecycle. `term write`
  / `term list` resolve groups server-side via a registry read handle; the
  Cmd+Shift+I client broadcast still scopes by the SPA-side `tab.group`.

## Decisions (round 1, from @@Alex 2026-05-30)
- **`cs` provisioning: docs-only this round.** Ship `argv[0]=="cs"` dispatch +
  the symlink-trigger integration test (tempdir `cs -> chan`, no symlink baked
  into the build). Do NOT touch the terminal PATH/env in
  `terminal_sessions.rs`. User wires their own `cs` symlink/alias; document it
  in help text + journal.
- **`cs term write`: raw bytes only.** No implicit `\r`. `--stdin` streams
  chunks through unchanged. Help text states the no-implicit-newline contract.
- **BUG-3 validation: test server with a real agent.** At C1 validation,
  confirm new-vs-reuse drive + seed with @@Alex first, serve from a renamed
  binary copy, scope `pkill` to my own drive/port. If no agent CLI reachable,
  flag C1 empirically-unverified (do not fake it).
- **Carousel spelling:** match existing single-r `carousel` / `carouselSlide`
  for the dashboard CLI flags + SPA arm (roadmap's `carrousel` is a typo).
