# Phase 21 - Terminal cross-window awareness + workspace-terminal parity

Status: closed with v0.29.0.
Span: 2026-06-09 to 2026-06-10.
Versions: v0.29.0.
Tags: #terminal #broadcast #cross-window #roster #release

## Roadmap (the asks)

Follow-up to phase-20 (standalone terminal windows). Phase-20 shipped a single
shared `/terminal` tenant (one PTY registry across all terminal windows) with a
global `Terminal-N` counter, session-preserving cross-window drag, and a
`broadcast-input` WS frame that fans a terminal's input to same-group sessions
in OTHER windows. Two gaps remained, both because the SPA only knows its OWN
window's terminals (`allTerminalTabs()` is the local layout).

**21.1 - Cross-window broadcast roster.** The broadcast UI was per-window: the
target list showed only same-window terminals, receiving terminals in other
windows showed no sign, and the cross-window fan reached every same-group
session regardless of that terminal's own broadcast toggle (the same-window fan
respected the per-member selection; the cross-window fan did not). Fix: a
cross-window roster the SPA can read, plus syncing each terminal's broadcast
toggle to the server so the fan can honor it.

**21.2 - Workspace-terminal parity.** The global counter and cross-window
broadcast were standalone-only. A second window of the SAME workspace restarted
`Terminal-N` at 1 (the counter was a process-global static, and the SPA used the
local per-window name in workspace mode). Bring both to workspace windows, which
already share a workspace's tenant + PTY registry.

## Rounds and waves

An architect-led solo round: a planning pass (parallel read-only Explore agents
mapped the server registry/routes/event bus and the SPA terminal/broadcast/ws
surfaces), then the cross-surface wire contract was fixed up front and both
surfaces implemented against it. A live review loop with the human then
surfaced four follow-ups, each verified end-to-end before moving on.

### Server (chan-server)

- **Per-tenant `Terminal-N` counter.** Moved off the process-global
  `TERMINAL_NAME_ORDINAL` static onto the per-tenant `Registry`
  (`next_terminal_name`). One registry serves one tenant, so standalone
  terminal windows share one sequence and each workspace gets its own.
  `GET /api/terminal/next-name` mounted on the full router too.
- **Cross-window roster.** `RosterEntry { id, tab_name, tab_group, window_id,
  broadcast }`; `GET /api/terminals/roster` for the reconnect seed and a
  `terminal_roster` `/ws` push on every change. The registry signals a
  `tokio::sync::Notify` on any mutation (create / restart / close / toggle); a
  `spawn_roster_broadcaster` task (sibling of the pruner/drainer) coalesces and
  republishes the full snapshot onto the global `events_tx` bus.
- **Broadcast toggle sync + gate.** A `set-broadcast` WS frame stores the
  toggle on the `Session` (an `AtomicBool`); `broadcast_input_cross_window` now
  skips receivers whose toggle is off - closing the limitation and matching the
  same-window semantics.
- **Cross-window broadcast control.** `POST /api/terminals/:session/broadcast`
  routes a `terminal_broadcast` window-command to the session's owning window
  (reusing the existing `window_command` bus), so the menu's group-wide Select
  All / per-row toggles reach terminals in other windows.

### Web (SPA)

- A cross-window roster `$state` seeded on `/ws` ready and refreshed by
  `terminal_roster` pushes, feeding the broadcast menu (same-group terminals in
  other windows) and the indicator count.
- The per-tenant counter is used in workspace mode too, resolving the name
  BEFORE the WS connects (a transient `pendingGlobalName` flag) so the session
  spawns with its final name - the roster and `cs term list` show it, not the
  local placeholder.
- The broadcast toggle syncs to the server (`set-broadcast`) on toggle and on
  (re)connect via a `$effect`.
- Select All / Deselect All span the whole group across windows (the
  cross-window members POST to the broadcast endpoint); the `Cmd+Shift+I` chord
  is shown on the button (macOS-native only, via `chordFor`).
- Terminal-name uniqueness is tenant-wide: `uniqueTerminalName` dedups against
  the cross-window roster across all windows and groups, not just the local
  window.

## Verification

- **Gates:** `make pre-push` green - rustfmt, clippy (`-D warnings`),
  `cargo test --all-targets` (chan-server +6 new tests: cross-window fan gate,
  per-tenant naming, roster shape, `set-broadcast` decode, broadcast endpoint
  emit + 404), `--no-default-features` build, gateway-build, web-check
  (svelte-check + 1700 vitest, +3 new), web-marketing-check.
- **Scripted (real PTYs):** a Node `/ws` client confirmed roster pushes fire at
  exactly create/toggle-on/toggle-off/close and the seed endpoint agrees, and
  the cross-window input gate (OFF receiver does not receive, ON receiver does,
  source never self-echoes).
- **Browser smoke (Chrome, multi-window):** group scoping verified across three
  windows (A / A / B - Select All on an A window lit both A windows and left B
  off); cross-window Select All / Deselect All toggled another window's sign;
  the duplicate-name fix verified live (rename to a cross-window name resolves
  to `-N`); per-workspace counter verified (a second window continued the
  sequence instead of restarting at 1).
- A signed `Chan.app` / `.dmg` was rebuilt locally for the human's desktop
  (WKWebView) pass; the upstream sign/notarize runs on Actions at tag time.

## Retrospective

**Highlights.**
- Contract-first again: fixing the roster wire shape (the `terminal_roster`
  frame + `RosterEntry`) before touching either surface let the server and SPA
  land without integration drift.
- Reuse over new transport: the roster rides the existing `events_tx` `/ws`
  bus, and cross-window broadcast control reuses the `window_command` bus that
  already targets a window_id - no new channel.
- The symmetric-mesh insight (broadcast membership is the toggle) collapsed
  21.1's indicator requirement: once the fan is gated on the toggle, a receiver
  necessarily has the toggle on, so the existing sign already shows; the work
  reduced to making the count cross-window-aware.
- The stale-desktop diagnosis was empirical: a green local build but a "broken"
  desktop traced to rust-embed baking `web/dist` at release-compile time, so the
  installed app ran phase-20 code until rebuilt - not a product bug.

**Lowlights / contention.**
- Three of the four follow-ups (name-before-connect, group-wide Select All,
  tenant-wide names) were gaps the initial pass missed because they only show
  with two real windows of a tenant - static gates and single-window smokes
  could not catch them; the live human review did.
- A reproduction artifact (stacked confirm dialogs in automation) briefly looked
  like a stale-session roster bug; a clean single-restart pass disproved it.
  Drive UI flows atomically, including modal confirms.
- The roster's `tab_name` / `tab_group` are spawn-time on the server (they
  refresh on the next restart, matching `$CHAN_TAB_GROUP`); the SPA shows a
  rename/regroup immediately but the cross-window view lags until restart. An
  accepted trade-off, not a live-sync.

**Lessons worth carrying forward.**
- Cross-window features need a two-window empirical pass; reasoning + a
  single-window smoke is not enough.
- When a desktop "regression" contradicts a green build, suspect the embedded
  bundle first (the installed app links its own `web/dist` + server).
- Name/group uniqueness and scoping are tenant-wide invariants, not
  per-window; check them against the roster, not just the local layout.

## Notes

Built solo (architect + Explore subagents for the read passes) on branch
`phase-21-terminal-cross-window`, merged to main with `--no-ff` (matching the
phase-20 merge). The original scratch plan lived in `dev/phase-21/plan.md`
(gitignored). No desktop code changed - the desktop app only needed a rebuild
to embed the new bundle + server.
