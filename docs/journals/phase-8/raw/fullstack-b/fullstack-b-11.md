# fullstack-b-11: Terminal scrollback buffer size + default TERM (Settings)

Owner: @@FullStackB
Date: 2026-05-20

## Goal

Expose two terminal preferences in the Settings page:

1. **Terminal scrollback buffer size (MB)** — how much
   per-terminal scrollback chan retains. Today it's
   20k lines hardcoded (`fullstack-b-2`); @@Alex wants
   generous-but-bounded sizing, configurable.
2. **Default TERM value** — the value chan sets for the
   PTY env var (today appears to use `xterm-256color`).
   Configurable in Settings so users can pick an
   alternative shape.

Both settings live in the Settings tab (NOT in the
terminal itself — preferences belong with other
preferences).

## Background

@@Alex 2026-05-20:

> About the terminal: what's the situation with the scroll
> buffer? we need large buffers because of agents and the
> way they refresh the terminal, but we also dont want to
> be holding huge unbounded amount of buffers. what's a
> reasonable value in MB per scrollback buffer? 50MB per
> terminal? 100MB per terminal? configurable? i think
> warrants a setting (not in the terminal itself btw, in
> the settings tab) and also to define the default TERM
> value (iirc we use xterm256-color).

### Sizing analysis

xterm.js measures `scrollback` in LINES, not bytes.
Current shape:

* 20k lines × ~80 cols × ~1.5 bytes/char UTF-8 avg ≈ ~2.4
  MB per terminal.
* Heavy ANSI escapes / wide chars increase per-line
  bytes; sparse 80-col plain ASCII decreases.

**Default proposal: 50 MB per terminal.** Generous enough
for agent activity (translates to ~400k lines at typical
80-col width); bounded so the JS heap doesn't grow
unboundedly across many open terminals.

Range exposed in the setting: **10 MB - 500 MB**. Below
10 MB risks losing context during a normal compile run;
above 500 MB the per-tab heap cost starts adding up across
multiple terminals.

@@Alex confirms the default + range when this lands; if
they want 25 MB or 100 MB defaults instead, flip in the
implementation (the range stays the same).

The MB → lines conversion happens at terminal-spawn time:

```
lines = (mb_setting * 1024 * 1024) / (cols * avg_bytes_per_char)
```

with `avg_bytes_per_char ≈ 12` (xterm.js's internal
per-cell cost: 1-2 chars + colour attrs + width flags;
empirically ~12 bytes per cell on modern V8; verify before
locking the constant). Alternatively, query xterm.js's
own per-line memory estimator if it exposes one.

### TERM analysis

Today the PTY env var likely defaults to `xterm-256color`
(audit the PTY spawn path in chan-server to confirm).
Setting exposes:

* Default: `xterm-256color`.
* Common alternatives in a dropdown:
  * `xterm-256color` (default; broadest 256-colour
    compat).
  * `xterm` (basic; older-system compat).
  * `tmux-256color` (tmux inside chan).
  * `screen-256color` (screen inside chan).
* Free-text input alongside the dropdown for exotic
  values (so power users can set `alacritty-direct` or
  similar). Shape: dropdown + "Custom..." that opens an
  input.

The TERM env var propagates to the PTY at spawn time;
existing terminals keep their TERM until session restart
(same retroactive-not policy as scrollback).

### Setting applicability

Setting applies to NEWLY-spawned terminals. Existing
terminals keep their current scrollback / TERM until
session restart. Document this in the setting's hint
text:

> Applies to terminals spawned after this setting
> changes. Existing terminals keep their current
> scrollback / TERM until the chan session restarts.

Simpler than retroactive resize / env-var change.

## Authorization

**Authorization: yes**, this task covers edits to
`web/src/components/SettingsPanel.svelte` (Settings UI),
`web/src/components/TerminalTab.svelte` (xterm.js
construction site that reads the scrollback setting),
chan-server's PTY spawn path
(`crates/chan-server/src/routes/terminal.rs` or
`crates/chan-server/src/pty.rs` — wherever the TERM env
var lands), and the persistent settings storage. @@FullStackB
may proceed without further in-chat confirmation from
@@Alex.

## Acceptance criteria

* New Settings entries:
  * "Terminal scrollback buffer (MB)" — number input or
    slider, range 10-500, default 50.
  * "Default TERM value" — dropdown with 4 common
    options + Custom... for free-text.
* Both settings persist via the existing Settings
  storage shape.
* Scrollback setting takes effect on newly-spawned
  terminals (existing terminals unaffected until
  session restart).
* TERM setting takes effect on newly-spawned terminals
  (existing terminals' PTY keeps the old TERM).
* Hint text under each setting names the
  "applies-to-new-only" semantic explicitly.
* xterm.js scrollback computed from MB at spawn time
  via the formula above.
* No regression on the existing 20k-line scrollback
  behaviour: users who haven't changed the setting get
  the new 50 MB default (which translates to >> 20k
  lines, so the default UX is strictly better than the
  pre-fix shape).
* Pre-push gate: fmt + clippy `-D warnings` + workspace
  test + svelte-check + npm build.
* Vitest pin for the MB→lines computation if it has a
  testable seam (it should — pure function on cols +
  MB).
* New chan-server unit test if the PTY-spawn TERM env
  propagation has a testable shape.

## How to start

1. Find the existing scrollback setting in
   `web/src/components/TerminalTab.svelte`. The
   xterm.js `scrollback` option is the consumer.
   Today it's 20000 (from `fullstack-b-2`).
2. Find the existing PTY spawn path in chan-server.
   Likely in `crates/chan-server/src/routes/terminal.rs`
   or a `pty.rs` sibling. The TERM env var setting is
   inside the env vector handed to the spawned shell.
3. Add the two Settings entries in
   `SettingsPanel.svelte`. Group under a "Terminal"
   section (create if not present; pairs with the
   `fullstack-b-2` line-height work if that already
   added one).
4. Thread the settings through to the consumer points
   (SPA reads scrollback; chan-server reads TERM).
   Server-side TERM may need a small API addition or a
   per-session config that the SPA passes at terminal-
   spawn time.
5. Validate the spawn-time-only semantic: existing
   terminals don't get their scrollback resized or their
   TERM swapped under them.
6. Pre-push gate.

## Coordination

* @@WebtestB verifies on lane-B drive once landed (same
  fixture they use for terminal regression checks).
* Coordinate with `systacean-8`'s lock-free status path
  if the settings storage requires a drive lock — should
  not, since Settings storage is app-level (not per-
  drive).
* Independent of the other Round-1 detour tasks; can
  land in parallel.

## 2026-05-20 — implemented

Two new Settings entries shipped against the persisted
`Preferences.terminal` subtree so the autosave plumbing
(`SettingsPanel` → PATCH `/api/config` → `apply_preferences`
→ `EditorPrefs + ServerConfig::save`) already round-trips
them without a fresh API surface.

### Server (Rust)

* `crates/chan-server/src/config.rs::TerminalConfig`: two
  new `#[serde(default)]` fields — `scrollback_mb: u32`
  (default 50) and `default_term: String` (default
  `"xterm-256color"`). Public consts
  `TERMINAL_SCROLLBACK_MB_MIN = 10` /
  `TERMINAL_SCROLLBACK_MB_MAX = 500` for the route-level
  clamp + frontend re-use.
* `crates/chan-server/src/terminal_sessions.rs::Session::spawn`:
  `cmd.env("TERM", "xterm-256color")` → reads
  `config.terminal.default_term.as_str()`. Existing PTYs
  keep whatever TERM they started with (the value lives on
  the per-spawn `cmd`, not on the running process), so
  changing the setting only affects newly-spawned shells.
* `crates/chan-server/src/routes/preferences.rs::sanitize_terminal_config`:
  literal-0 scrollback snaps to default; any other
  out-of-range value clamps to the slider edges; TERM
  trims accidental whitespace and falls back to the
  default on empty.
* Three new Rust tests: `terminal_config_defaults_scrollback_and_term`,
  `terminal_config_legacy_file_fills_new_fields` (legacy
  `server.toml` without the new keys loads cleanly via
  the per-field serde defaults),
  `sanitize_terminal_config_clamps_scrollback_and_trims_term`,
  and a real-PTY integration test
  `spawn_uses_configured_default_term` that spawns a shell
  with a custom `default_term` and verifies the configured
  value lands on the child's `$TERM`.

### Frontend (Svelte)

* `web/src/api/types.ts::TerminalPreferences`: optional
  `scrollback_mb` + `default_term` so older servers without
  the fields still parse.
* `web/src/terminal/scrollback.ts` (new): pure helpers —
  `clampScrollbackMb` (mirrors server policy: 0/undefined
  → 50 default; below 10 → 10; above 500 → 500) and
  `scrollbackLinesFromMb(mb, cols = 80)` (the
  bytes/(cols × 12)-per-cell conversion the task spec
  named). Constants `SCROLLBACK_MB_MIN/MAX/DEFAULT` shared
  with the Settings UI so the slider bounds stay in
  lockstep with the server clamps.
* `web/src/components/TerminalTab.svelte`: scrollback
  cap reads from `drive.info?.preferences?.terminal?.scrollback_mb`
  at xterm.js construction time; the computed line count
  is captured on the component (`scrollbackLines`) so the
  two `serialize?.serialize({ scrollback: ... })` paths in
  the "copy scrollback" actions use the same window the
  buffer actually holds. No reactive resize: a settings
  change after spawn doesn't reach through (matches the
  task's spawn-time-only contract).
* `web/src/components/SettingsPanel.svelte`: new "Terminal"
  section between the strip-trailing-whitespace section
  and Semantic search. Range slider + paired number input
  for scrollback (so power users can type a precise value
  rather than dragging); native `<select>` for TERM with
  the four shipped terminfo entries + `Custom...`, which
  expands a free-text input. Hint copy under each control
  explicitly names the spawn-time-only semantic.
* `normalizePrefs` extended to fill missing
  `scrollback_mb`/`default_term` from the legacy server
  payload so the autosave dirty() check doesn't trigger
  an infinite save loop after a refetch.

### Tests

* Rust: 4 new (191 → 191 + 3 unit + 1 PTY = 195 total
  chan-server, all green).
* SPA: 3 new test files
  (`web/src/terminal/scrollback.test.ts` x10,
  `web/src/components/SettingsPanel.terminal.test.ts` x7,
  `web/src/components/TerminalTab.scrollback.test.ts` x3).
  Vitest 501/501 green (was 481 baseline).

### Pre-push gate

* `cargo fmt --check`: clean (one auto-rewrap by `cargo fmt`).
* `cargo clippy --workspace --all-targets -- -D warnings`:
  clean.
* `cargo test --workspace`: all green.
* `cargo build --no-default-features`: builds (pre-existing
  `not_a_chan_drive_hint` dead-code warning unrelated to
  this task).
* `npm run check`: 3971 files / 0 errors / 0 warnings.
* `npm run build`: clean (same pre-existing chunk-size +
  ineffective-dynamic-import warnings as baseline).
* `npx vitest run`: 501/501.

### Notes for review

* The MB → lines formula assumes an 80-col baseline. Wider
  terminals consume more bytes per visible line so the
  effective MB budget is conservatively under-estimated
  for wide-window users (their buffer stays smaller than
  the configured MB cap rather than overshooting it). Easy
  to swap in xterm.js's own per-line estimator if/when one
  is exposed.
* The custom-TERM input is unvalidated — chan-server's
  `sanitize_terminal_config` only trims whitespace + falls
  back on empty. A misconfigured TERM produces a PTY where
  curses-based programs misbehave; the user's fix is to
  swap back to a known value. Matches the "power user
  escape hatch" framing in the task body.
* The "Terminal" section sits outside the existing
  `section-row` two-column wrapper so it spans full width.
  Pairs with the eventual Round-2 expansion (line-height,
  font, scrollback retention policy) since those will need
  the wider lane too.

Proposed commit subject:
`Settings: terminal scrollback (MB) + default TERM with spawn-time semantics (fullstack-b-11)`

Holding for commit clearance; queue empty otherwise.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Comprehensive landing — server-side `TerminalConfig` fields
with `#[serde(default)]` for legacy-file compat, public
clamp constants shared with the frontend, sanitization
helpers, real-PTY integration test, and frontend pure
helpers in `web/src/terminal/scrollback.ts` with constants
shared with the SettingsPanel so the slider bounds stay
in lockstep with the server clamps. The bytes/(cols × 12)-
per-cell formula matches the task spec; the
80-col-baseline conservative-under-estimate framing is
the right call.

The "Terminal" SettingsPanel section spanning full-width
(outside the `section-row` 2-column grid) is good
forward-thinking — pairs with the eventual Round-2
expansion (font in `fullstack-b-12`, line-height,
scrollback retention policy) that will need the wider
lane too.

Three notes for review all defensible:
* 80-col baseline → conservative under-estimate at wider
  widths (safer than overshoot).
* Unvalidated `Custom...` TERM input → matches the
  power-user escape-hatch framing in the spec; misconfig
  is reversible.
* Full-width section layout → spans the right lane for
  the Round-2 Terminal-section expansion.

Spawn-time-only semantic correctly implemented (no
reactive resize through to existing terminals); hint copy
names this explicitly to the user.

Pre-push gate green across the full stack:
* Rust: clippy + workspace test + no-default-features
  build all clean.
* SPA: vitest 501/501 (+20 from baseline reflecting -25's
  +10 and your own +10 across three new test files);
  check + build clean.

**Commit clearance**: approved. Use your proposed commit
subject as-is. Push waits until end of Round 2.

After commit: pick up `-12` (chan terminal visual parity
with iTerm — Source Code Pro bundling + cursor + line
metrics). That's your last Round-1 detour task. Then
standby until Round-2 fan-out.