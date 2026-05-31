# Round-2 @@LaneD — terminal bugs + cs CLI + desktop + Team Work

## You are @@LaneD

Domain this round: terminal / cs CLI / keyboard protocol / chan-desktop /
Team Work (round-1's Lane-C domain - this is a continuation of your own round-1
code: the incomplete Shift+Enter fix, the `cs term` surface, the tab groups).
Read `bootstrap.md`, then this file, then the terminal bugs + the whole "cs
command line" + "Team work" sections of `round-2-part-2.md` and the `chan open`
note under "Common functionality", then `coordination.md`. Coordinate through
**@@Architect (@@LaneA)**, not @@Host. Confirm understanding, then start wave-1.

This is the heaviest lane; it runs in three waves and you should **spawn
subagents** within scope (a terminal/keyboard subagent, a cs-CLI subagent, a
Team-Work subagent). If wave-3 (Team Work) cannot reach a tested state by round
close, it goes to the round-3 backlog rather than into the release - do not rush
it in.

## You own

`web/src/components/TerminalTab.svelte`, `web/src/terminal/keymap.ts` +
`session.ts`, `web/src/state/shortcuts.ts`, `web/src/App.svelte` (the
`onWindowKey` matcher region), `crates/chan/src/main.rs` (cs clap + `cmd_open`),
`crates/chan-server/src/control_socket.rs`, `terminal_sessions.rs`,
`routes/terminal.rs`, `desktop/src-tauri/src/serve.rs` (the KEY_BRIDGE_JS),
`web/src/state/teamOrchestrator.svelte.ts`, `BubbleOverlay.svelte`,
`bubbleStub.svelte.ts`, the **`TerminalTab` + `TeamWorkState` region** of
`web/src/state/tabs.svelte.ts`, and the **`handleWindowCommand` region** of
`web/src/state/store.svelte.ts`.

Shared-file note: `tabs.svelte.ts` (@@LaneB owns the `DashboardTab` region) and
`store.svelte.ts` (@@LaneC owns the index/status region). Stay in your regions;
chained staged-diff commit discipline on every shared-file commit.

## Tasks

### Wave 1 — gating bugs + the rename (start now; SUBMIT first)

- **SUBMIT** (`round-2-part-2.md`, the "Shift+Enter still submits" carryover).
  Your round-1 fix added reload persistence but no fallback when an agent never
  announced its keyboard protocol, so `terminalMetaKeyBytes()`
  (`keymap.ts:48-75`) returns null and xterm sends a plain `\r`. Land round-1
  candidate (c): a sane default sequence when Shift+Enter is held and protocol
  state is unknown, and/or re-query negotiation on reconnect
  (`TerminalTab.svelte` ~596-604). **Needs a real-agent smoke** (a running
  claude/codex in the terminal, not a shell); `keymap.test.ts` only covers
  serialize/restore and misses this. **This is CK-SUBMIT** and it **gates the
  poke protocol for the whole team** (a bare `\n` written into a running agent's
  stdin won't submit until this lands). Do it first; poke @@Architect + all
  lanes when it lands.
- **RELOAD** (the "Ctrl+R steals bash reverse-search" remap). `app.window.reload`
  must be **Ctrl+Shift+R on Linux/Windows, Cmd+R on macOS, NEVER plain Ctrl+R**.
  Branch the keymap per-OS (mirror the metaKey-gated Cmd+Shift+I precedent at
  `shortcuts.ts:110`), not a new descriptor field. Touch points listed in the
  part doc: `shortcuts.ts:177-184`, the `App.svelte` `onWindowKey` matcher
  (pinned by `cmdRWindowReload.test.ts`), the desktop bridge `case 'KeyR'`
  (`serve.rs:613` + tests `:1014-1024`), and regenerate the help table
  (`node web/scripts/shortcuts-table.mjs` -> `main.rs` SERVE_LONG_ABOUT).
  Verify plain Ctrl+R falls through to the PTY on every platform. Rust +
  frontend -> full repo gate. Real-terminal smoke (Ctrl+R reaches the shell's
  reverse-search).
- **LINKS** (terminal links not clickable). `WebLinksAddon` is already loaded
  (`TerminalTab.svelte:613`) with the default ctor; pass a custom handler that
  reuses `openExternalUrl()` (`web/src/editor/external_links.ts:73`) - web
  `window.open(_blank)`, Tauri external browser, matching the editor. Browser-
  smoke (and ideally a chan-desktop smoke).
- **CS-RENAME** (`cs term` -> `cs terminal`). Rename the clap group
  `Term`/`TermAction` in `main.rs`; update the `argv[0]=="cs"` symlink
  integration test + help text. Pre-release: **drop `term`, no alias.** **This
  is CK-RENAME** - @@LaneC's `cs search` and the poke/bootstrap docs build on
  `cs terminal`. Poke @@Architect + @@LaneC when it lands.

### Wave 2 — the rest of the cs surface + desktop

- **CS-PREFIX** (prefix match, iproute2-style). It's a clap built-in:
  `infer_subcommands(true)` on the root + the nested `terminal` enum. Current
  names disambiguate (o/g/d/t/s top-level; n/w/l/r under terminal). No
  hand-rolled prefix logic. Add a test that `cs t r` -> `terminal restart`.
- **CS-RESTART** (`cs terminal restart [--tab-name --tab-group]`). Add a
  control-socket `ControlRequest::TermRestart` that resolves by name/group and
  calls `Registry::restart()` server-side (category-2, like `term write`). This
  out-of-band server path is what a shell needs to restart its own shell.
  Confirm `restart_options()` re-applies the per-terminal startup command/env so
  a relaunched agent comes back. **This is CK-RESTART** - the Team Work
  self-restart (wave-3) depends on it.
- **CS-LIST** (`cs terminal list` markdown/table by default, `--json` for
  machine output) - align with the SEARCH output convention (markdown default,
  `--json`, `--json --pretty`); not a bespoke "always pretty JSON".
- **CS-CAROUSEL** (`--carousel-off` for `cs dashboard`, default on). Spell it
  one-r `--carousel-off` (matches the existing `--carousel-index`). Sets a field
  on the newly created `DashboardTab` - **coordinate the field shape with
  @@LaneB at CK-CAROUSEL** (their `DashboardTab` region); do not invent your own.
- **DESKTOP-SHELL** (`chan shell` in chan-desktop). Make chan-desktop recognise
  `argv[0]=="cs"` and run that code path, so chan-desktop users get a functional
  MCP + Hybrid Terminal shell without the `chan` binary.
- **DESKTOP-OPEN** (`chan open <path>` - @@Host APPROVED for round-2). The OS
  file-association entry, distinct from `cs open`: assess whether the path
  belongs to a known workspace, then offer to turn it on / open it, or reject
  with guidance to create one. Today `cmd_open` (`main.rs`) just pushes
  `OpenPath` assuming `$CHAN_CONTROL_SOCKET`/`$CHAN_WINDOW_ID` - i.e. it behaves
  like `cs open` and only works inside a chan terminal. Build the real
  double-click path.

### Wave 3 — Team Work (backlog-able if untested at close)

- **TEAM-GROUP**: add "Terminal tab group name" to the team setup dialog,
  default derived from the filename (`/tmp/new-team-1/chan-team.toml` ->
  `chan-team`); on conflict with a live terminal group at creation time, append
  `-N`. Enumerate live groups from the registry (the same source
  `cs terminal list` reads) so dialog + CLI agree.
- **TEAM-CONSOLIDATE**: team setup creates terminals via the same code path as
  `cs terminal new --tab-name=x --tab-group=y`; consolidate all term-creation
  onto one path.
- **TEAM-SELFSTART** (the reported bug: the bootstrap's own terminal never
  launches its agent). Fix via **CS-RESTART**: the bootstrap calls
  `cs terminal restart` against its own tab-name/group; the server restarts that
  session so the new shell launches the agent from its startup command. Depends
  on **CK-RESTART**.
- **POKE-2.2**: the completion poke
  (`cs terminal write --tab-name=<target> 'poke ...'`). No new CLI - it reuses
  `cs terminal write` + the registry-by-name path. **Depends on CK-SUBMIT** (a
  bare `\n` won't submit into a running agent until SUBMIT lands).
- **Survey bubbles (2.3) are DEFERRED to round-3** by @@Host. Do NOT build the
  BubbleOverlay event-pump / reply round-trip this round. v0.21.0 ships poke
  protocol **2.2** only (agent<->agent). Leave `BubbleOverlay`/`TeamWorkState`
  hooks as-is.

## Cross-lane coordination (you produce three checkpoints)

- **CK-SUBMIT (you -> all):** SUBMIT landed -> agent poke delivery works.
- **CK-RENAME (you -> @@LaneC + @@Architect):** `cs terminal` landed -> @@LaneC's
  `cs search` and the poke/bootstrap docs rebase.
- **CK-RESTART (you -> your own wave-3):** `cs terminal restart` server path
  landed -> TEAM-SELFSTART.
- **CK-CAROUSEL (you <- @@LaneB):** agree the `DashboardTab` carousel-off field
  shape before CS-CAROUSEL.
- **CK-INDEX-IDLE (you <- @@LaneC):** their indexing fix makes your RELOAD reload
  smoke reliable; coordinate the reload smoke after it lands.

## Gate + smoke

Full repo pre-push gate (Rust heavily touched): fmt, clippy -D warnings, test,
`build --no-default-features`; `web/`: svelte-check + vitest + build. The
release gate also builds the **gateway** workspace - @@Architect runs that at
release, but keep your changes from breaking it. **Real-agent smoke required**
for SUBMIT and POKE-2.2 (a running claude/codex, not a shell). Append progress
to `round-2-lane-d-journal.md` + `event-lane-d.md`; poke @@Architect on each
checkpoint/completion.
