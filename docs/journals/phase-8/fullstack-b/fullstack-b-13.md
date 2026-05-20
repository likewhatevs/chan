# fullstack-b-13: Shell/agent submit-mode toggle + survey-reply echo consumer

Owner: @@FullStackB
Date: 2026-05-20

## Goal

Today the rich prompt's Cmd+Enter submit + the survey-reply
echo path both write text to the underlying terminal's PTY
ending with a literal Enter (`\n`). For a shell, Enter
submits — fine. For an agent running inside the terminal
(Claude Code / codex / gemini), Enter inserts a newline into
the agent's input draft; only Cmd+Enter submits the message.
Reply text ends up wedged in the agent's input draft,
unsubmitted. @@Alex's verbatim ask: `poke<cmd+enter>` not
`poke<enter>`.

Three deliverables:

1. **Per-prompt shell/agent submit-mode toggle** — small
   icon button in the rich-prompt header toolbar (matches
   the `fullstack-a-24` floating-pill toolbar pattern).
   States: "Shell" (default; today's behaviour) and "Agent".
   Persists per-prompt-session as a new SerTab field
   (suggest `rpsm?: "shell" | "agent"`; conditional spread
   on serialize; absence = shell default).
2. **Chord-encoding research + decision** — figure out what
   byte sequence agents (specifically Claude Code; cross-
   check codex / gemini) accept as "submit." Likely
   candidates: xterm modifier-other-keys
   `\x1b[27;9;13~`, raw `\x0d` (CR-only, no LF), or some
   bracketed-paste-mode terminator. Pin the choice in the
   task tail with a one-liner reproducer (e.g. echo bytes
   to a Claude Code session via `printf`, see what
   submits).
3. **Two consumer sites consume the toggle**:
   * **Rich-prompt Cmd+Enter submit path**:
     `submitRichPrompt` / `sendUserInput` (in
     `TerminalTab.svelte`). Shell mode → trailing byte is
     `\n` as today (or empty if buffer already ends in
     `\n`). Agent mode → trailing chord per the encoding
     research above.
   * **Survey-reply echo path**: the SPA emits a literal
     "poke" string + Enter into the PTY when the user
     clicks a survey-reply option. Find the call site
     (grep for the poke string OR the place that writes
     after `writeSurveyReply`); apply the same toggle.

## Background

Bug entries in [`../phase-8-bugs.md`](../phase-8-bugs.md):
* "Survey-reply echoes to the terminal as `poke<Enter>`;
  breaks agents that need `poke<Cmd+Enter>`"
* (Same root family as item C in the rich-prompt session
  evolution work — see
  [`../architect/rich-prompt-session-evolution.md`](../architect/rich-prompt-session-evolution.md)).

Today's PTY-write path:
* Rich prompt submit → `submit()` → `onSubmit(buffer)` →
  `submitRichPrompt(source)` → `sendUserInput(source)` →
  WebSocket frame `{type: "input", data}` →
  chan-server `routes/terminal.rs` → PTY.
* No explicit trailing-newline append in JS; the buffer's
  own trailing characters dictate what arrives. Whatever
  produces `poke<Enter>` for survey replies is a separate
  emission path; find it.

Encoding research notes:
* Claude Code's terminal listens for a "submit" chord. The
  exact byte sequence depends on the terminal's keybinding
  interpretation. Test reproducer: `printf 'pwd\x1b[27;9;13~' > /dev/<pty>`
  (or send via the chan-server WS path); see if Claude Code
  submits.
* If `\x1b[27;9;13~` doesn't work, try alternative xterm
  modifier encodings (CSI 13 ; 5 u for CR+Ctrl, etc.) and
  raw `\x0d`.
* Document the chosen encoding inline in the toggle's
  implementation with a comment citing the test result.

## Acceptance criteria

* Rich-prompt header toolbar has a clearly-labelled
  "Shell" / "Agent" toggle (icon + state-reflecting label
  acceptable). Default "Shell."
* Toggle state persists per-prompt-session (SerTab); empty
  / unset case round-trips identically to today's SerTab.
* In shell mode: today's behaviour preserved byte-for-byte
  in both consumer sites (rich-prompt submit + survey-reply
  echo).
* In agent mode: rich-prompt Cmd+Enter sends the buffer +
  the agent-submit chord. Tested against a live Claude
  Code session; the buffer arrives as a single submitted
  message in Claude Code's input.
* In agent mode: clicking a survey-reply option sends
  `poke<agent-chord>` (or whatever the current reply
  string is) instead of `poke<Enter>`. Tested against a
  live Claude Code session.
* Encoding choice documented inline with a one-line
  reproducer and a citation to the source.
* `vitest` green for the toggle-state SerTab round-trip +
  the consumer wiring. End-to-end PTY behaviour can't be
  fully unit-tested; @@WebtestB verifies on lane-B.

## How to start

1. Spin up a test server with a terminal running Claude
   Code (or any agent with a Cmd+Enter submit chord).
2. Empirically nail down the chord encoding — drop bytes
   into the PTY via `sendUserInput` from the browser
   console and see which one triggers a submit in Claude
   Code's input box.
3. Pin the encoding. Then wire the toggle.
4. Find the survey-reply echo call site (the "poke" string
   emission). Most likely lives near the
   `writeSurveyReply` consumer in BubbleOverlay or
   watcherEvents — grep `"poke"` first.
5. Test both consumers end-to-end against a live agent
   session.

## Coordination

* Pairs with [`fullstack-a-28`](../fullstack-a/fullstack-a-28.md)
  (BubbleOverlay regression cluster). -a-28 owns the
  rendering/dismissal side of the bubble; this task owns
  the PTY-write side. The survey-reply call site that
  emits "poke<Enter>" today might live inside the
  BubbleOverlay code path; coordinate at task-cut if the
  two tasks need to touch the same file. Recommended split:
  -a-28 changes WHAT triggers the reply (dismissal); -b-13
  changes WHAT bytes hit the PTY in response.
* @@WebtestB verifies on lane-B against a live Claude Code
  session in a chan terminal.

## 2026-05-20 — boot + scope question for @@Architect (@@FullStackB)

Bootstrapped this task. Initial grounding turned up a finding
that re-shapes the work envelope; flagging before I commit to
an implementation direction. Permission event for the chord-
encoding probe already fired to @@Alex (see
[`../alex/event-fullstack-b-alex.md`](../alex/event-fullstack-b-alex.md)
2026-05-20 entry).

### Finding: the survey-reply echo is server-side, not SPA-side

Task body said:
> "the SPA emits a literal 'poke' string + Enter into the PTY
> when the user clicks a survey-reply option"

and

> "Server side likely unchanged (the PTY-write is the SPA's
> responsibility post-systacean-9)."

Grepped `"poke"` across `web/src/` + `crates/`. The only PTY
write of that literal lives at:

```
crates/chan-server/src/terminal_sessions.rs:502
  fn dispatch_agent_event(&self, event: AgentEvent) {
      ...
      // TODO: wire /clear, /effort, and /fast automation here once
      // @@Alex's richer control commands are cut for a later task.
      session.send_input(b"poke\n");
  }
```

`dispatch_agent_event` is invoked from `event_watcher.rs` when
fsnotify ingests a `survey-reply` (or `poke` / `pre-flight`)
file. The receiving session is matched by `tab_name`. The SPA
never writes "poke" bytes — it writes the reply file via
`api.writeTerminalEventReply` → `routes/terminal.rs`, and the
fsnotify ingest path on the same chan-server process is what
emits the PTY bytes.

The bug is therefore server-side: the per-prompt shell/agent
toggle has to reach the server somehow so that
`dispatch_agent_event` picks `b"poke\n"` (shell) vs
`b"poke<agent-chord>"` (agent) per receiving session.

### Three implementation options

I want @@Architect's call before I commit, because the choice
affects whether the patch stays small (Option 1) or grows
significantly (Options 2 / 3).

**Option 1 — per-session config field on the server (small,
incremental)**

* Add `submit_mode: SubmitMode { Shell, Agent }` to chan-server's
  `Session` struct (defaults to `Shell` so today's behaviour is
  byte-for-byte preserved).
* Add a thin HTTP route (`PUT /api/terminal/sessions/{id}/submit-mode`
  body `{mode: "shell"|"agent"}`) that the SPA hits whenever the
  rich-prompt toggle flips. Mirrors the existing
  `setTerminalWatcher` shape.
* `dispatch_agent_event` reads `session.submit_mode` and picks
  the trailing bytes from a single `const AGENT_SUBMIT_CHORD:
  &[u8]` (the chord we pin from @@Alex's probe).
* SPA: `rpsm?: "s" | "a"` SerTab field + the `TerminalRichPromptState.submitMode` field + an icon button in the rich-prompt header toolbar + a `submit()` call site update that also routes the chord into `sendUserInput` for the Cmd+Enter path.
* Footprint: ~6 files, ~150 LOC, two tests (server route + SPA toggle round-trip).

**Option 2 — SPA intercepts via a server frame (medium)**

* Server stops emitting bytes directly. Instead it sends a new
  WS frame `{type: "agent-event", from, event_type, ...}` to the
  matching session's WS clients.
* SPA receives the frame and emits the encoded bytes via
  `sendUserInput`, picking the encoding from the per-prompt
  toggle locally.
* Pros: all encoding decisions on the SPA; future per-agent
  encoding maps are trivial.
* Cons: changes the dispatch contract (the cross-process /
  cross-tab pokes that today rely on the server-side echo for
  the case where the SPA isn't open get broken — or need a
  fallback for the no-WS-attached case). Bigger blast radius.

**Option 3 — move emission entirely to the SPA (large)**

* SPA polls or subscribes to event files via systacean-9's read
  endpoint and emits `poke<chord>` itself when new replies
  arrive.
* Server-side `dispatch_agent_event` deleted.
* Pros: cleanest separation; per-prompt toggle is purely a SPA
  concern.
* Cons: significant refactor; reintroduces the polling vs push
  question the server's fsnotify ingest already solved; risk of
  regression on the case where two chan tabs are open to the
  same drive (which gets the echo?).

### My recommendation: Option 1

It's the smallest delta, preserves the existing dispatch
architecture, and lands in the same envelope as the original
task scope. The new HTTP route is a thin shell that mirrors
`setTerminalWatcher` (same auth, same session resolution). The
per-session model is a natural fit because the toggle's whole
purpose is "what does THIS agent terminal accept as submit" —
a session-level property.

### Coordination touchpoints worth noting

* **Cross-lane with @@FullStackA's -a-28**: both add a
  conditional SerTab field to `web/src/state/tabs.svelte.ts`
  (-a-28 adds `dbi?: string[]`; this task adds `rpsm?: "s" | "a"`).
  Independent additions — both are extra optional fields with
  conditional-spread on serialize — but the SerTab type
  definition lives in one place so we should both stage
  explicit files and check `git diff --staged --stat` before
  each commit. I'll add `rpsm?` near the existing rich-prompt
  `rpb` / `rph` / `rpo` / `rpm` / `rpc` cluster so it stays
  visually grouped; -a-28 can drop `dbi?` near the bubble-
  overlay state (none today; new line / region).
* **Cross-lane with @@Systacean's -10**: their in-flight diff
  on `event_watcher.rs` adds `is_watcher_event_filename` to the
  fsnotify ingest path. Adjacent to but not overlapping with
  my server-side touch in `terminal_sessions.rs::dispatch_agent_event`.
  I'll stay clear of `event_watcher.rs` entirely; the
  `AgentEvent` struct + `dispatch_agent_event`'s receiver-
  session matching are unchanged in their diff so my option-1
  edit doesn't collide.

### What I'm doing while you decide

Auto Mode: I won't sit idle. Source-side prerequisites that
hold under all three options:

1. SerTab `rpsm?: "s" | "a"` field add + `TerminalRichPromptState`
   `submitMode?: "shell" | "agent"` field add + the
   `toSerTab` / `fromSerTab` round-trip + tests.
2. Icon toggle button in `TerminalRichPrompt.svelte` header
   (matches the existing `Bot` / `FolderSearch` / `Send` /
   `Chevron` / `X` cluster); per-prompt state binding.
3. Wire the toggle's "agent" branch through `submit()` →
   `onSubmit(buffer)` → `submitRichPrompt(source)` →
   `sendUserInput` with a placeholder
   `AGENT_SUBMIT_CHORD = "\x1b[27;9;13~"` constant (final
   value pinned post-@@Alex's probe; the constant is one-line
   to swap).

This SPA scaffolding is required by all three options. I'll
hold off on the server-side / WS-frame / emission-relocation
changes until you've picked the architecture.

Will fire a poke event after this append.

## 2026-05-20 — chord-encoding probe results

@@Alex approved Option 2 (throwaway chan test server + WS-frame
byte injection). Ran the probe with @@Alex live-watching.

### Probe setup

* Throwaway drive at `/tmp/chan-test-phase8-rpsm` (canonical
  `/private/tmp/chan-test-phase8-rpsm`).
* `./target/debug/chan serve /tmp/chan-test-phase8-rpsm` (the
  May 20 11:19 debug binary; no rebuild — chord probe is
  WS-level, not SPA-UI level).
* Opened in Chrome MCP at the launch URL.
* Installed a `WebSocket.prototype.send` interceptor to capture
  the live WS handle into `window.__capturedWs__`, and a
  `window.__chordProbe__(bytes)` helper that fires
  `{type:"input", data:bytes}` frames through it. Captured the
  WS after typing into the spawned terminal.
* Spawned a terminal tab via Cmd+Alt+T (web-Mac chord from
  `-b-9`).

### Claude Code v2.1.145

| Bytes                | Effect on Claude Code prompt input        |
|----------------------|-------------------------------------------|
| `probe1`             | Lands as draft text "probe1".             |
| `\x1b[27;9;13~`      | **SUBMITS** the draft. probe1 acknowledged. Sautéed for 2s. |
| `probe2_with_nl`     | Lands as draft text on top of cleared input. |
| `\n` (LF)            | Inserts newline into draft (cursor → line 2). Does NOT submit. Status row flips to "ctrl+g to edit in Vim" indicating multi-line mode. |
| `\x1b[27;9;13~` (2nd)| Submits `/exit` cleanly — Claude Code exits, returns to bash prompt. |

**Verdict for Claude Code**: the chord is
`\x1b[27;9;13~` (xterm "modifyOtherKeys" CSI sequence for
Cmd+Enter). `\n` is treated as multi-line newline.
Today's `b"poke\n"` is exactly the failure mode @@Alex hit
("poke" sits in the agent's draft, never submitted).

### Codex v0.130.0

| Bytes              | Effect on codex prompt input                    |
|--------------------|-------------------------------------------------|
| `\n` (LF)          | Silent. No effect on the trust-prompt. No effect at the main prompt either (silent ignore). |
| `\r` (CR)          | At the trust-prompt: confirms "Yes, continue." At the main prompt: **submits** the draft.  |
| `probeC1`          | Lands as draft text "probeC1".                  |
| `\x1b[27;9;13~`    | No effect. probeC1 still in the draft.          |
| `\r` (post-chord)  | Submits probeC1 → codex called pwd + ran a profile read tool + responded "Ready. Need a concrete task beyond probeC1." |

**Verdict for codex**: chord = `\r` (CR). Does NOT
recognize Claude Code's `\x1b[27;9;13~` chord. `\n` is
silently dropped (not even newline-into-draft, just gone).

### Divergence summary

| Agent          | Submit chord     | `\n` effect            |
|----------------|------------------|------------------------|
| Claude Code    | `\x1b[27;9;13~`  | Newline in multi-line draft |
| Codex          | `\r`             | Silent / ignored       |

**Per @@Alex's directive ("if codex fails it's fine, i just
want the signal")**: ship single-chord with Claude Code's
encoding. Codex's `\r` chord is documented here as future
work for a per-agent encoding map (Round-3 polish or later
mini-wave). Single-chord ship is justified because:

* Claude Code is the primary target user (chan's own
  development is Claude Code).
* The bug (`poke<Enter>` not submitted) is exactly the
  Claude Code case; codex's draft is unaffected by `\n` (it
  silently drops, so no wedge state).
* Per-agent encoding adds a new SerTab dim (which agent is in
  the terminal) we don't have a clean signal for yet
  (`$CHAN_TAB_NAME` is set by the spawner, not by the agent
  itself).

Gemini probe skipped (per @@Alex "skip if your bandwidth is
tight, no signal is also signal"; the codex divergence alone
already settles the universality question).

### Chord constant to ship

```
// fullstack-b-13: Claude Code v2.1.145 accepts this byte
// sequence as the "submit" chord (xterm modifyOtherKeys CSI
// for Cmd+Enter). codex v0.130.0 accepts `\r` instead; the
// divergence is documented in fullstack-b-13.md tail. Single-
// chord ship per @@Alex 2026-05-20.
const AGENT_SUBMIT_CHORD = "\x1b[27;9;13~";
```

### Teardown footprint (per feedback-test-server-workflow)

* `kill 60435` on `./target/debug/chan serve` (process exited
  with 144 / SIGTERM as expected).
* `rm -rf /tmp/chan-test-phase8-rpsm` — throwaway drive removed.
* `./target/debug/chan remove /private/tmp/chan-test-phase8-rpsm`
  — registry entry unregistered.
* Chrome MCP tab closed via `tabs_close_mcp`.

No persistent side effects.

### Status: chord pinned

Holding for @@Architect's architecture call (Options 1 / 2 /
3 at the prior task-tail append). With the chord constant in
hand, the implementation is unblocked once @@Architect chooses
the propagation shape.

Will fire a poke event with the chord result + teardown
confirmation.

## 2026-05-20 — server-side implemented (Option 1)

@@Architect approved Option 1 (per-session config field + thin
HTTP route). Server slice landed.

### Files changed (server only — SPA side parked)

* `crates/chan-server/src/terminal_sessions.rs`
  * New `pub enum SubmitMode { Shell, Agent }` with a
    `submit_chord(self) -> &'static [u8]` method. `Shell ⇒
    "\n"`, `Agent ⇒ "\x1b[27;9;13~"`. The chord byte string
    is documented inline with a citation to the live probe
    against Claude Code v2.1.145.
  * New `agent_mode: AtomicBool` field on `Session` (default
    `false`, encoding Shell). `Session::submit_mode()` +
    `Session::set_submit_mode(mode)` accessors using
    `Ordering::Relaxed` (matches the existing pattern across
    `Session`).
  * New `Registry::set_submit_mode(session_id, mode) -> bool`.
    Returns `true` on success, `false` when the session id is
    unknown — same shape as `Registry::set_watcher`.
  * `Registry::dispatch_agent_event` now reads
    `session.submit_mode()` and writes `b"poke"` + the
    appropriate chord bytes. Shell mode is byte-for-byte
    identical to the previous behaviour (`b"poke\n"`); agent
    mode swaps the trailing byte to `\x1b[27;9;13~`.
* `crates/chan-server/src/routes/terminal.rs`
  * New `pub struct SubmitModeBody { mode: String }`. Accepts
    `"shell"` or `"agent"`; anything else is a 400.
  * New `pub async fn api_set_terminal_submit_mode(...)` —
    mirrors `api_set_terminal_watcher` (tunnel-public gate,
    path-bound session id, JSON body, 204 on success, 404
    when the session id is unknown, 400 on a bad mode value).
* `crates/chan-server/src/routes/mod.rs`
  * Added `api_set_terminal_submit_mode` to the
    `pub use terminal::{...}` re-export block.
* `crates/chan-server/src/lib.rs`
  * Added `api_set_terminal_submit_mode` to the
    `use routes::{...}` import.
  * Added `put` to `use axum::routing::{...}`.
  * Wired `PUT /api/terminal/:session/submit-mode` onto the
    router alongside the existing `:session/watcher` +
    `:session/event-reply` routes.

### Tests added

* `terminal_sessions::tests::submit_mode_chord_constants_match_probe_findings`
  — pins `SubmitMode::{Shell, Agent}::submit_chord()` and the
  default (`Shell`). Catches a chord-byte regression at the
  smallest possible surface.
* `terminal_sessions::tests::set_submit_mode_flips_field_and_handles_missing_session`
  — registry-level: create session, flip Agent, verify field,
  flip back, verify field, unknown id returns false. Pins the
  whole setter contract.
* `terminal_sessions::tests::dispatch_agent_event_uses_chord_in_agent_mode`
  — end-to-end against a real PTY: flip to Agent, dispatch a
  Poke event, attach to the session, observe the chord bytes
  arrive in the output ring (rendered as `^[[27;9;13~` by the
  shell's line-discipline echo) AND assert the legacy
  `"poke\n"` shape is absent. Cousin of the existing
  `dispatch_agent_event_writes_poke_to_matching_tab` test
  which pins the Shell-mode behaviour.
* `routes::terminal::tests::api_set_terminal_submit_mode_flips_session_field`
  — route-level: create a session, PUT mode=agent (expect
  204), PUT mode=shell (expect 204), PUT mode=bogus (expect
  400), PUT against an unknown session id (expect 404). Pins
  every branch of the response logic.

### Pre-push gate

* `cargo fmt --check` — clean.
* `cargo clippy --workspace --all-targets -- -D warnings` —
  clean.
* `cargo test --workspace` — green. chan-server suite went
  from 198 (before this slice) → 202 (after; +4 new). All
  other crates unchanged.
* `cargo build --no-default-features` — clean.

SPA-side scaffolding (svelte-check + vitest + npm build) NOT
run for this slice — no SPA changes, and `tabs.svelte.ts`
still carries unstaged @@FullStackA work on -a-28/-29/-30
which would muddy the verdict.

### What's intentionally NOT in this slice

* **The SPA side**: SerTab `rpsm?: "s" | "a"` field add,
  `TerminalRichPromptState.submitMode` field add, header
  toolbar toggle button, `submit()` call-site chord append,
  and the API client call that hits the new
  `PUT /api/terminal/:session/submit-mode` route. All parked
  behind @@FullStackA's tabs.svelte.ts settling per the
  user's coordination directive. The API surface is in place
  and reachable, ready for the SPA-side commit to consume.
* **A per-agent encoding map**: codex's `\r` divergence is
  deferred to Round-3 Track 5 per @@Alex 2026-05-20. Single-
  chord ship with Claude Code's encoding.

### Suggested commit subject

```
chan-server: per-session shell/agent submit-mode toggle + dispatch_agent_event chord branch (fullstack-b-13 server-side)
```

### Coordination footprint

* `terminal_sessions.rs` — no other lane has uncommitted
  edits here per `git status`.
* `routes/terminal.rs` — no other lane has uncommitted edits
  here per `git status`.
* `routes/mod.rs` — no other lane has uncommitted edits here
  per `git status`.
* `lib.rs` — no other lane has uncommitted edits here per
  `git status`.

`tabs.svelte.ts` is untouched by this slice (which is the
whole point of landing the server side first while
@@FullStackA's UI work settles).

### Status

Server-side commit-ready. Holding for @@Architect clearance.
SPA side stays parked until @@FullStackA's tabs.svelte.ts
work commits (then I add the SerTab + toolbar toggle + API
client call as a second commit).
