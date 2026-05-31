# Round-2 @@LaneD journal — terminal / cs / desktop / Team Work

Append-only. Curated status goes to @@Architect; full context lives here.
Continuation of round-1's Lane-C domain.

## Wave plan

- Wave 1: SUBMIT (CK-SUBMIT) -> RELOAD -> LINKS -> CS-RENAME (CK-RENAME).
- Wave 2: CS-PREFIX, CS-RESTART (CK-RESTART), CS-LIST, CS-CAROUSEL
  (CK-CAROUSEL <- @@LaneB), DESKTOP-SHELL, DESKTOP-OPEN.
- Wave 3: Team Work (TEAM-GROUP, TEAM-CONSOLIDATE, TEAM-SELFSTART via
  CS-RESTART, POKE-2.2 <- CK-SUBMIT). Backlog-able if untested at close.
- Survey bubbles 2.3: DEFERRED to round-3 by @@Host. NOT building them.

## Log

### SUBMIT (CK-SUBMIT) — code-complete + statically gated; real-agent smoke pending

Root cause (confirmed in source): `terminalMetaKeyBytes` (`keymap.ts:48-75`)
emits a modified-Enter sequence only when the SPA observed the agent's protocol
negotiation (modifyOtherKeys or kitty REPORT_ALL_KEYS). Round-1 added two
coverage layers (in-memory relocation for heap-intact remounts; kp
serialization for page-reload-past-replay). The uncovered case is "agent
already running, never observed negotiating": the agent enabled its protocol
before this tab attached, so the negotiation is neither in the reattach replay
ring nor in the serialized snapshot -> protocol stays pristine-zero ->
`terminalMetaKeyBytes` returns null -> xterm sends plain `\r` -> Shift+Enter
SUBMITS instead of inserting a newline.

Round-1 rejected candidate (c) ("sane default") as unsafe: a blind escape
sequence would pollute a plain shell's command line. The SAFE form of (c):
for Shift+Enter (modifier 2) only, when no enhanced protocol is active, return
a bare **LF (`\n`)**. Rationale (and why it is safe for BOTH foreground
programs):
- Plain shell: the line discipline accepts `\n` exactly like Enter (submits the
  line, no stray bytes on the prompt). No regression.
- Claude Code: reads `\n` as a newline inside its multi-line draft. This is the
  INVERSE of the existing `AGENT_SUBMIT_CHORD` (`submitMode.ts:15`,
  `\x1b[27;9;13~`), whose comment records a live probe (2026-05-20):
  "bare `\n` lands as a newline in its multi-line draft and never submits."
  That probe is the empirical backing for this fallback being correct for
  Claude.
- Scoped to Shift+Enter only. Cmd/Ctrl+Enter (modifiers 9/5) keep falling
  through to `\r`, preserving their submit semantics (Cmd+Enter is Claude's
  submit chord).

Changes:
- `keymap.ts`: added the `if (modifier === 2) return "\n";` fallback after the
  modifyOtherKeys / kitty branches in `terminalMetaKeyBytes`.
- `keymap.test.ts`: updated every assertion that previously expected
  Shift+Enter -> null in a non-report-all state (pristine, disabled,
  disambiguate-only, alt-screen, flag-removed, popped, reset) to expect `\n`;
  added an explicit SUBMIT fallback test + a Cmd/Ctrl+Enter pass-through test.
  19 pass (was 18).

Static gate (my scope): svelte-check 0/0; keymap.test.ts 19 pass; full vitest
1565 pass / 1 fail. The single red is `dashboardTabAndCarousel.test.ts:158`,
driven by @@LaneB's in-flight A3/A4/A6/A7 files (EmptyPaneCarousel /
FileInfoBody / InspectorBody / AboutSlotConfig / new PlainScreensaverPreview) in
the shared worktree — NOT my change (I only touched keymap.ts + keymap.test.ts).
Same cross-lane in-flight pattern as round-1. Flagged to @@Architect.

Pending: real-agent smoke (a running claude/codex, not a shell) — batched onto
one wave-1 test server. Will verify: Shift+Enter inserts a newline in Claude's
draft (does not submit); plain bash still submits on Shift+Enter with no stray
bytes.

### CK-SUBMIT vs POKE-2.2 — IMPORTANT clarification for @@Architect

The round-2 docs say SUBMIT "gates poke auto-delivery" because "a bare `\n`
will not submit to an agent until [the Shift+Enter fix] lands." Tracing the
code, this framing conflates two OPPOSITE behaviors of `\n`:

- Shift+Enter wants `\n` -> insert newline (NOT submit). <- the SUBMIT fix.
- A completion poke wants its trailing byte -> SUBMIT.

Both are consistent only under one fact, which the existing live probe
confirms: **agents treat `\n` as newline-insert and the modifyOtherKeys chord
`\x1b[27;9;13~` as submit.** Therefore:

1. Landing SUBMIT does NOT make a bare `\n` submit. The opposite — it makes
   Shift+Enter emit `\n` precisely BECAUSE `\n` is a newline to an agent.
2. POKE-2.2 auto-submit must append **`AGENT_SUBMIT_CHORD` (`\x1b[27;9;13~`)**,
   not a bare `\n`. The bootstrap/coordination poke command
   `cs terminal write --tab-name=<t> 'poke ...\n'` will NOT auto-submit to a
   Claude agent as written.
3. The submit-chord infra already exists from round-1 (`submitMode.ts`
   `AGENT_SUBMIT_CHORD` + `encodeForAgentSubmit`; server mirror
   `terminal_sessions.rs::SubmitMode::submit_chord`; consumed by the team-work
   Cmd+Enter submit and the `dispatch_agent_event` survey-reply echo). So
   POKE-2.2 is ENABLED by existing infra, not blocked by the Shift+Enter fix —
   they share the theme (agent keyboard encoding) but are independent
   mechanisms.

Recommendation: POKE-2.2 should either reuse the `dispatch_agent_event` path
(which already appends the submit chord) or give `cs terminal write` an opt-in
`--submit`/`--enter` flag that appends the per-agent submit chord. The
bootstrap poke docs should be corrected away from the bare `\n`. codex
diverges (submits on `\r`, ignores the chord); single-chord (Claude encoding)
is the round-1 decision and stands for round-2 unless @@Host wants a per-agent
map.

### RELOAD — code-complete + statically gated; terminal smoke pending

`app.window.reload` now binds **Cmd+R on macOS, Ctrl+Shift+R on
Linux/Windows, never plain Ctrl+R** (which is the shell's reverse-search).
Root cause was three-layered: the `Mod+R` descriptor with
`escapeTerminal: true` made `shouldEscapeTerminal` swallow Ctrl+R over a
focused terminal on non-mac; the desktop bridge `case 'KeyR'` reloaded on
any meta (Cmd OR Ctrl); and App.svelte only ever reloaded on Cmd+R anyway.

The reload is the one chord that diverges by OS (not just by label), which
a single `Mod+R` string can't express. Rather than add a per-OS descriptor
field, I added one `osChord(s, platform, os)` resolver in `shortcuts.ts`:
it stores the macOS form and diverges reload to `Mod+Shift+R` on non-mac.
Every chord consumer routes through it so they agree:
- `shouldEscapeTerminal` (escapes Cmd+R on mac / Ctrl+Shift+R on non-mac;
  plain Ctrl+R is NOT escaped -> reaches the PTY).
- `chordFor` + Pane.svelte `chordLabel` (menu label correct per running OS).
- `renderTable` (help table).
Plus a `note: "Ctrl+Shift+R on Linux / Windows"` on the descriptor so the
macOS-rendered `chan serve --help` informs Linux/Windows readers.

Raw-event matchers branch on the same rule:
- `App.svelte` onWindowKey: `currentOS() === "mac"` ? Cmd+R : Ctrl+Shift+R.
- `desktop/src-tauri/src/serve.rs` KEY_BRIDGE_JS: `case 'KeyR'` in the
  no-shift branch gated on `e.metaKey` (mac Cmd+R); a new `case 'KeyR'` in
  the shift branch gated on `!e.metaKey` (non-mac Ctrl+Shift+R). Mirrors the
  existing Cmd+W / Cmd+Shift+I metaKey-gating idiom. Plain Ctrl+R now falls
  through to xterm on every platform.

Help table: regenerated `SERVE_LONG_ABOUT` via
`node web/scripts/shortcuts-table.mjs --serve-long-about`. NOTE: the
regenerate also corrected PRE-EXISTING drift the table had accumulated
(Settings->"Flip focused Hybrid", "Terminal team work"->"Team Work",
"Infographics Cmd+I"->"Dashboard Cmd+. i", added Bold/Italic). Those rows
were already the truth in shortcuts.ts at HEAD; main.rs had simply not been
regenerated. main.rs is my owned file, so this is in-scope cleanup, flagged
here so the diff's unrelated rows are understood.

Tests: rewrote `cmdRWindowReload.test.ts` (per-OS handler + osChord +
note); updated the serve.rs `key_bridge_wires_reload_and_devtools_ipc` pin
to the two new `case 'KeyR'` shapes.

Gate (my scope): svelte-check 0/0; full vitest 1579 pass / 0 fail; cargo fmt
clean; clippy -p chan-desktop -p chan --all-targets -D warnings clean;
chan-desktop bridge test passes; cargo build -p chan clean. Pending: a
real-terminal smoke on chan-desktop AND browser (Ctrl+R reaches the shell's
reverse-search on Linux; Ctrl+Shift+R reloads; Cmd+R reloads on macOS;
plain Ctrl+R on macOS reaches the shell). Batched with the SUBMIT smoke on
one wave-1 test server. CK-INDEX-IDLE (<- @@LaneC) makes the reload smoke
reliable; coordinate after their indexing fix lands.
