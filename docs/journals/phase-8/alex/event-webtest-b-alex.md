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

## 2026-05-20 — STANDING approved (transcribed by @@Architect)

@@Alex (in chat post-v0.11.1): "i grant permission now,
pls record on the docs so i dont need to do again later"

**Tauri-launch extension upgraded to STANDING (indefinite
scope)**: the prior "through Round-1 close" framing is
replaced. @@WebtestB has standing authorization for
chan-desktop runtime walkthroughs going forward. Scope:

* `npm run tauri dev` / `cargo build -p chan-desktop` /
  launching `Chan.app` against throwaway drives for any
  per-task verification queue.
* Driving the chan-desktop UI (click cycles, drive
  open / close, window LRU restore behaviour, etc.) to
  the extent the available tooling allows (Chrome MCP
  doesn't reach Tauri's WKWebView; `osascript` blocked
  by Accessibility — these are still tooling
  limitations, not permission limitations).
* Standard test-server-workflow tear-down (kill
  process, `rm -rf` throwaway drive, `chan remove`
  registry entry, chan-desktop config restored to
  pre-walk state).

Boundaries that still apply:
* Production-tag pushes still gated on @@Alex.
* Persistent side effects outside the throwaway set
  are NOT covered; always tear down.

The previously-parked `fullstack-b-1` empirical LRU
click cycle remains tooling-blocked (Chrome MCP +
Accessibility); permission isn't the gate, but if
@@Alex grants Accessibility OR adds a chan-desktop
`--drive <path>` CLI arg (Round-3 polish), this
permission covers the walk.

Recorded as a standing permission in
[../../../agents/bootstrap.md](../../../agents/bootstrap.md)
"Standing permissions" section so fresh @@WebtestB
sessions inherit on bootstrap.

## 2026-05-21 — permission (canonical fresh-Mac Gatekeeper walk for chan-v0.11.2 DMG)

Per [`../webtest-b/webtest-b-2.md`](../webtest-b/webtest-b-2.md)
+ the pause-and-warn rule from
[`event-architect-webtest-b.md`](event-architect-webtest-b.md)
"Scope clarification...".

Gatekeeper-clean walkthrough for `Chan_0.11.2_x64.dmg`
(first signed+notarized chan-desktop release, live on
GitHub Release `chan-v0.11.2`) requires either:

* **(a)** pausing your current chan-desktop session +
  closing `/Applications/Chan.app` + resuming via iTerm
  with the tightened scope rules. The (a) path requires
  you to consciously close your working Chan.app —
  that's a destructive action I cannot make unilaterally.
* **(b)** running on your secondary Mac.
* **(c)** decline / defer — I walk the binary in
  throwaway-drive shape only on this Mac (no DMG
  install, no /Applications touch), capture the
  keychain-independent signals (`spctl --assess` against
  the mounted DMG + `.app` in `/Volumes/Chan`,
  `stapler validate`, `codesign --verify`) on a custom
  sandbox path I own end-to-end, and document the
  partial. This is the safest option and the one I'll
  default to if you don't reply.

Which?

Standing chan-desktop runtime perm (against throwaway
drives only) does NOT cover any of (a)/(b); the
tightened scope explicitly excludes
`/Applications/Chan.app`, system-path `xattr` writes,
and `pkill -f chan-desktop`-shaped process triage.
Option (c) operates strictly within standing scope.

While you decide, I'll start the lane-B
throwaway-drive walkthrough (terminal cluster, watcher
dialog, indexing chart, CLI scriptability) on a freshly
built `chan-v0.11.2`-equivalent binary — those surfaces
need no permission ask.

