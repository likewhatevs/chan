# @@WebtestA's phase-8 journal

Author: @@WebtestA
Date: 2026-05-19

Authoritative walkthrough lane A. Drives a persistent test server
and the Chrome MCP browser, appends verdicts to task files. Pairs
with @@WebtestB for parallel coverage.

Append-only. New entries go at the bottom under a dated heading.

## 2026-05-19 — boot

Fresh phase-8 session. Read:

* `docs/agents/webtest-a.md` + `skills/webdev.md`.
* `docs/journals/phase-8/process.md` + phase-7 process for the
  event protocol.
* `docs/journals/phase-8/request.md` +
  `docs/journals/phase-8/phase-8-bugs.md`.
* `webtest-a/webtest-a-1.md` (only open task: reproduce every
  v0.11.0 bug, with the @@WebtestA / @@WebtestB coverage split).

No events with me yet. Working tree on `main` carries phase-7
deltas + untracked phase-8/ scaffolding (matches what
@@Architect cut).

Filed permission event for terminal exec + Chrome browser at
`alex/event-webtest-a-alex.md`. Waiting on @@Alex's written
approval before launching `chan serve` and Chrome MCP.

## 2026-05-19 22:00 BST — approval landed, starting Round 1 walkthrough

@@Architect transcribed @@Alex's verbal approval on
`alex/event-webtest-a-alex.md`: standing through Round 1 (bug
sweep + new build), Round 2 features need a fresh ask. Scope
matches what I requested.

Plan:

1. Throwaway drive at `/tmp/chan-test-phase8-wa/`, seeded with
   a copy of the chan repo (excluding `.git`, `target`,
   `node_modules`, `web/dist`) so the graph bug and other
   "seed with chan source" repros work.
2. `cargo build -p chan` then `./target/debug/chan serve` it,
   capture the URL with bearer token.
3. Walk my coverage cluster top-to-bottom against the bug list,
   append repro notes to `webtest-a-1.md`.

## 2026-05-20 00:25 BST — Round 1 close

All 15 bugs in my coverage cluster have verdicts in
`webtest-a-1.md`. Two active repros (bug 8 graph
false-missing, bug 11 image-insert push), two partials
(bug 6, bug 20), one could-not-reproduce (bug 14), ten fix-
holds-on-HEAD. Filed a `poke` event at
`alex/event-webtest-a-architect.md` summarising the verdicts
+ side observations + handing off the test-server URL.

Test server left running at
`http://127.0.0.1:8787/?t=Am6NjQ7pSNeH2ibHCyaftLu8m8MuNntm`
against `/tmp/chan-test-phase8-wa/` (chan repo seed). Watcher
attached to `watcher-events/` inside the drive; sample survey
events + a reply file left in place for @@Alex to inspect.

Standby for fix-verification cadence as patches land.
