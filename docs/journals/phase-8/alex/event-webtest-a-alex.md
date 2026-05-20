# event-webtest-a-alex.md

From: @@WebtestA
To: @@Alex
Date: 2026-05-19

## 2026-05-19 — permission

Need terminal exec + Chrome MCP browser sessions for Round 1
bug-sweep walkthrough.

Scope:

* Terminal: `cargo build -p chan` and
  `./target/debug/chan serve <drive-path>` (and the surrounding
  `chan list` / `chan remove` lifecycle) against a throwaway
  drive seeded with the chan repo itself, per
  [`../webtest-a/webtest-a-1.md`](../webtest-a/webtest-a-1.md).
  Also includes the standard test-server workflow: stop the
  process between rebuilds, `rm -rf` the temp drive at teardown.
* Chrome: `mcp__claude-in-chrome__*` sessions against the
  test-server URL with bearer token, to drive the editor +
  terminal and capture repro notes / screenshots for each entry
  in [`../phase-8-bugs.md`](../phase-8-bugs.md).

Coverage in this lane: file-browser tab, status bar, Cmd+K
cluster, rich-prompt cluster, editor cluster, graph (matches
the @@WebtestA / @@WebtestB split in `webtest-a-1.md`).

Standing approval for the duration of Round 1 would be ideal —
the walkthrough cycles through serve restart + browser reload
per fix verification. Otherwise I'll re-request at each session
boundary.

## 2026-05-19 22:00 BST — approved (transcribed by @@Architect)

@@Alex approved this verbally in chat: "approve webtest-a/b
standing through round 1". Scope covered:

* Terminal exec: `cargo build -p chan`, `./target/debug/chan
  serve <drive-path>`, `chan list` / `chan remove` lifecycle
  against a throwaway drive seeded with the chan repo itself,
  including stop-between-rebuilds and `rm -rf` teardown.
* Chrome MCP: `mcp__claude-in-chrome__*` sessions against the
  test-server URL with bearer token, driving the editor and
  terminal and capturing repro notes + screenshots for each
  entry in `phase-8-bugs.md`.
* Coverage slice: file-browser tab, status bar, Cmd+K cluster,
  rich-prompt cluster, editor cluster, graph.
* Standing through the entirety of Round 1 (the bug sweep +
  new build). New scope (Round 2 features) requires a fresh
  permission event.
