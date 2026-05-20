# event-webtest-b-alex.md

From: @@WebtestB
To: @@Alex
Date: 2026-05-19

## 2026-05-19 — permission

Need terminal exec + Chrome MCP browser sessions for Round 1
bug-sweep walkthrough (lane B).

Scope:

* Terminal: `cargo build -p chan` and
  `./target/debug/chan serve <drive-path>` (and the surrounding
  `chan list` / `chan remove` lifecycle) against a throwaway
  drive distinct from @@WebtestA's, per
  [`../webtest-b/webtest-b-1.md`](../webtest-b/webtest-b-1.md).
  Also includes the standard test-server workflow: stop the
  process between rebuilds, `rm -rf` the temp drive at teardown.
  Lane-B drive will be seeded with the chan repo itself (same
  seed as lane A is fine — separate drive paths + ports keep
  the two lanes from colliding).
* Chrome: `mcp__claude-in-chrome__*` sessions against the
  lane-B test-server URL with bearer token, to drive the
  editor + terminal and capture repro notes / screenshots for
  my coverage slice in [`../phase-8-bugs.md`](../phase-8-bugs.md).

Coverage in this lane: native window-config persistence,
terminal cluster (Cmd+T, scrollback, line adjustment), watcher
dialog cluster, indexing-chart pan/zoom, CLI scriptability
(systacean-1). Matches the @@WebtestA / @@WebtestB split in
`webtest-b-1.md`.

Standing approval for the duration of Round 1 would be ideal —
the walkthrough cycles through serve restart + browser reload
per fix verification. Otherwise I'll re-request at each session
boundary.

## 2026-05-19 22:00 BST — approved (transcribed by @@Architect)

@@Alex approved this verbally in chat: "approve webtest-a/b
standing through round 1". Scope covered:

* Terminal exec: `cargo build -p chan`, `./target/debug/chan
  serve <drive-path>`, `chan list` / `chan remove` lifecycle
  against a lane-B drive distinct from @@WebtestA's, including
  stop-between-rebuilds and `rm -rf` teardown.
* Chrome MCP: `mcp__claude-in-chrome__*` sessions against the
  lane-B test-server URL with bearer token.
* Coverage slice: native window-config persistence, terminal
  cluster, watcher dialog cluster, indexing-chart pan/zoom,
  CLI scriptability.
* Standing through the entirety of Round 1 (the bug sweep +
  new build). New scope (Round 2 features) requires a fresh
  permission event.

## 2026-05-20 — approved (transcribed by @@Architect) — Tauri launch extension

@@Alex approved this in chat: "1. granted" in response to
@@Architect's framing of the WebtestB Tauri-launch permission
extension ask.

Scope add-on to your standing Round-1 permission:

* `npm run tauri dev` / `make run` / `Chan.app` runtime launch
  against a throwaway drive.
* Driving the launched chan-desktop window via Chrome MCP or
  manual click as needed for `fullstack-b-1` (window-config
  LRU) runtime walkthrough.
* Standard tear-down rule applies: kill the chan-desktop
  process when the walkthrough is finished.
* Standing through the entirety of Round 1.

You may pick up `fullstack-b-1`'s runtime walkthrough on lane
B with this extension. Append your verdict to
[`../webtest-b/webtest-b-1.md`](../webtest-b/webtest-b-1.md)
when done.
