# Phase-15 round-3 backlog

Items deliberately deferred from round-2 (v0.21.0): tested-state not reached, or
@@Host-deferred. Do NOT rush these into v0.21.0.

## Survey bubbles 2.3 (@@Host-deferred 2026-05-31)

The @@Host-targeted survey-bubble UI is a REBUILD (backend event-pump + reply
round-trip + F->draft), deleted 2026-05-29 (`55179ad9`). v0.21.0 ships poke
protocol 2.2 only (agent<->agent). Reuse anchors: `round-2-part-2.md` section
2.3, git history (`55179ad9` parents, `75892d7c`, `a8b52a00`, `c69e2fcf`).

## IDX Option B: embeddings as a proper background job

Round-2 shipped Option A (gate preflight on BM25-ready; embed continues on its
existing background thread) + C-CAP. Option B = make the embed a fully separate
background job with its own status, changing the chan-workspace
`reindex_with_aggression` contract. Cleaner long-term shape; deferred for risk.

## IDX bg-embed chip clobber (found by @@LaneC during A)

During background-embed, a concurrent watcher edit sets Reindexing then
`set_idle{embedding:None}`, dropping the embed chip until the next flush (+ a ~ms
preflight blip). COSMETIC, self-healing, search-correct (verified `lanec_probe`
4s during embed). Largely mooted by C-CAP (capped big repos do not bg-embed). A
correct fix needs a SHARED bg-embed signal independent of the reindex status (the
watcher overwrites `embedding` when it sets Reindexing, so a `set_idle`
preserve-param cannot help) - this couples to Option B above.

## IDX embed in-flush chip freeze (QA-confirmed, @@LaneC 2026-05-31)

The chip does not advance DURING a single embed flush (the candle BERT forward
pass blocks the progress thread), so it can sit frozen at e.g. 402/403 for the
flush duration. Observable even on a modest ~400-note drive under core
contention; the C-CAP 2000-file ceiling does not catch it. **Caveat:** the
"minutes" freeze is exaggerated by the multi-agent test env (4 agents + builds
peg the 8 cores); a real single-user sees a much shorter freeze. Fix lever:
CHIP option (y) emit a heartbeat progress tick on a timer during the forward
pass, and/or (x) smaller embed batches (lower `EMBED_BATCH_CHUNKS` - the QA
repro was 403 chunks = a SINGLE batch, ~450-680% CPU under contention, so the
chip cannot advance within the one flush). Couples to the embed-batch tuning +
Option B above. Full evidence in `event-lane-c.md`.

## Desktop verifies (@@Host post-release spot-check, 2026-05-31)

Not round-3 work, recorded here so it is not lost: @@Host will spot-check on
chan-desktop after the release; if any is still buggy, a fresh bug-fix issue:
- BUG-EDITOR (WKWebView conceal-on-tab-switch),
- RELOAD (Ctrl+R reverse-search),
- DESKTOP-OPEN handoff branch (`chan open <known-workspace-path>` from outside a
  terminal -> `maybe_handoff_to_desktop`): the workspace-root match + the CLI
  guidance path are CLI-smoked + @@LaneC-QA'd, but the actual desktop handoff is
  Chrome-untestable (Blink) so it is desktop-only.

## DESKTOP-SHELL: cs-shell extraction to a shared crate (@@Host-deferred 2026-05-31)

chan-desktop should run cs via `argv[0]=="cs"` (part-2 enhancement) so
chan-desktop users get a functional cs shell + MCP without the `chan` binary.
Requires extracting ~400-500 lines of cs-shell CLIENT from
`crates/chan/src/main.rs` into a shared crate (`chan-shell`) that both chan +
chan-desktop depend on: ShellAction+TerminalAction clap enums (~432-487),
cmd_shell / cmd_shell_search / cmd_shell_terminal (~2033-2333),
send_control_request (~2334+), the client `ControlRequest` enum (~1908-1960, a
DUP of chan-server `control_socket.rs`'s server enum), ControlResponse,
open_env / control_socket_env, the render helpers, AGENT_SUBMIT_CHORD.
chan-desktop main detects `argv[0]=="cs"` -> `chan_shell::dispatch`. Bonus
cleanup: unifies the duplicated client/server `ControlRequest` enum.
**RISK:** cross-crate clap derive + the serde tags must stay BYTE-IDENTICAL or
every cs command breaks at runtime (gate-blind wire trap - needs a wire-smoke of
every cs command, not just a green build). ~0.5-1 focused day; a good fresh
round-3 opener. No user blocked (cs works via the chan binary meanwhile).
DESKTOP-OPEN (the double-click path) is independent and SHIPS in v0.21.0.

## Per-agent submit-encoding map (round-3, @@Host: "we absolutely need this")

`cs terminal write --submit` + the SPA team-work submit currently append the
Claude-only chord `\x1b[27;9;13~` (Cmd+Enter, xterm modifyOtherKeys), so codex +
gemini agents do NOT auto-submit (codex ignores the chord and submits on `\r`;
gemini untested). Multi-agent teams need per-agent support. Design
(@@Host-directed 2026-05-31):
- **Default `cs terminal write` writes PURE BYTES** - no implicit submit/chord.
  This is already the default (submit=false is raw passthrough, no `\n` strip);
  keep + enforce it.
- **Submission is opt-in via a per-agent flag** carrying the right encoding:
  - claude -> `\x1b[27;9;13~` (the current `--submit`).
  - codex  -> `\r` (plain CR; codex ignores the Claude chord silently, per the
    submitMode.ts comment).
  - gemini -> TBD: probe live in a chan terminal (as claude was probed 2026-05-20).
  Shape: `--submit=<agent>` (claude|codex|gemini) is cleaner than separate
  per-agent flags; unset = pure bytes (the default).
- **Three consumers must share one map:** `cs terminal write`
  (main.rs `apply_submit_chord`), the server-side
  `terminal_sessions.rs::SubmitMode::submit_chord`, and the SPA team-work submit
  (`submitMode.ts` `encodeForAgentSubmit`).
- **Team Work plumbing:** the bootstrap/dialog needs each member's agent TYPE to
  send the right submit (a per-member or team-wide agent field on the team
  config). Couples to the Team Work surface.
- `submitMode.ts` already flags it: "Per-agent encoding map is deferred."
