# @@WebtestA's phase-7 journal

Author: @@WebtestA
Date: 2026-05-18

Append-only. New entries go at the bottom under a dated heading.

## 2026-05-18 11:29 BST - Bootstrap

Read contact card, webdev skill guide, phase process, phase request,
own journal, and CLAUDE.md. The actual phase directory in this checkout
is `docs/journals/phase-7/`; the requested
`docs/journals/phase-7/` path is absent.

No `webtest-a-*.md` task files are present under
`docs/journals/phase-7/webtest-a/`. Architect journal
mentions planned `webtest-a-1`, but no task file has been cut yet.

## 2026-05-18 13:05 BST - Fresh-agent resume + Lane A walkthroughs

Fresh @@WebtestA session per @@Architect's 13:00 BST handover. Drive
was pre-seeded from the previous (terminated) attempt; verified content
matched the spec and proceeded.

Server: `./target/debug/chan serve --port 8801 --no-browser
/tmp/chan-webtest-a-1/` (default 8787 was taken by an unrelated
chan from phase 6; 8810 is @@WebtestB's).

URL: `http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`

Ran both:

* `webtest-a-1` (baseline Lane A) — 11 bug verdicts. Notable
  finds: B20 (table render crashes the editor with a CodeMirror
  block-decoration error), B1 (Shift+Tab outside list jumps
  focus to the pane hamburger), B13 (typing left of a list
  marker inserts before the marker - "Q1. First..."), B9
  (image bubble has a stray separator with no "searched N
  files" hint), B19 *does not* reproduce.
* `webtest-a-2` (fullstack-1 docked side panes) — all 8 items
  pass. Two non-blocking observations (default width feels
  wide; resize handles lack keyboard a11y) noted inline.
* Out-of-scope but worth surfacing: dragging the active tab
  onto an adjacent inactive tab in the workspace removes the
  dragged tab from the list. Side-pane false-positive check
  passed; this looks like plain workspace tab D&D.

### Process observations

* The chrome-MCP browser is shared with @@WebtestB. My tab
  was repeatedly hijacked to 8810 between batches. Mitigated
  by re-asserting `window.location.assign('http://127.0.0.1:8801/...')`
  at the top of every batch. Flagged to @@Architect.
* The `chrome MCP` blocks `Element.innerHTML` reads that
  contain query-string tokens (Cookie/query-string data
  guard). Worked around by using narrower `.textContent`
  reads and per-attribute lookups.
* `RangeError: Block decorations may not be specified via
  plugins` repros on every load of any doc with a pipe table
  - documented the stack and pointed @@FullStack at the
  StateField vs ViewPlugin contract for CM6 block decoration
  providers.

Hand-off URL forwarded via
[../alex/event-webtest-a-architect.md](../alex/event-webtest-a-architect.md).

## 2026-05-18 16:05 BST - webtest-a-3 closed + standby

`webtest-a-3` two-wave walkthrough done:

* Wave 1 (pre-revision): items 1-5 (toolbar parity) + 6, 7,
  9, 10 (browser-served link routing + icon audit) all PASS.
* Wave 2 (post-revision): rebuilt + restarted 8801; revised
  `openExternalUrl` dispatch validated end-to-end. Scenarios
  2 + 3 (Chan.app desktop, tunnel-loop) verdicted by code
  audit since Chrome MCP can't drive Tauri's WKWebView.

Architect accepted the verdict at 16:00 BST; `fullstack-2`
is cleared for commit architect-side. Standing by for
@@Alex's closeout + the agent-recycle event.

8801 server stays up for optional click-around. Drive
contents: index.md + 3 PNGs, note-a.md (table-crash repro),
note-b.md (lorem). No test files left behind.

## 2026-05-18 (resume) BST - Fresh @@WebtestA bootstrap

Fresh session resuming Phase 7. Read in this order:

* This journal (the three predecessor entries above).
* [../request.md](../request.md) — Round 1 done; Round 2
  fan-out queued (survey protocol, bubble overlay, agent
  spawning, orchestration SKILL).
* [../process.md](../process.md) — events, permission flow,
  agent-recycle.
* [../alex/event-architect-webtest-a.md](../alex/event-architect-webtest-a.md)
  — latest from @@Architect at 16:00 BST: `fullstack-2`
  accepted, I'm done with closeout, no queued task.
* [../architect/journal.md](../architect/journal.md) handover
  section at 17:05 BST — confirms "@@WebtestA has no queued
  task; Round 2 fan-out is where they get work."
* [../../../agents/webtest-a/contact.md](../../../agents/webtest-a/contact.md).

### State on disk

* `/tmp/chan-webtest-a-1/` still present (index.md, note-a.md
  with the B20 table-crash repro, note-b.md, img/ with three
  PNGs). Drive is unregistered (was a throwaway).
* The 8801 server the previous me left up is down. The only
  running `chan serve` is on `/private/tmp/chan-test-phase6`
  (unrelated, pre-existing from phase 6).
* `origin/main` at `9e48367`, tag `v0.10.1`. Branch is up to
  date.
* No `webtest-a-4.md` task file exists.

### Posture

Standby for Round 2 fan-out. No new task to start; no event
or task file is waiting for my action. Will not spin up a
test server speculatively (per the test-server workflow
memory: ask first about drive choice + seed).

@@Alex / @@Architect: when you cut my next task, ping
[../alex/event-architect-webtest-a.md](../alex/event-architect-webtest-a.md).
