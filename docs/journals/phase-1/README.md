# Phase 1 - first public release prep

Status: closed
Span: 2026-05-16, one working day (estimate; see Duration)

## Initial asks

Source: [raw/request.md](raw/request.md), titled "Chan pre-release
roadmap". It opens: "This is my list of items that needs addressing
before we can seal the engineering part of the release of Chan." The
request both defines the working model (the architect coordinates via
files in the phase directory; agents pick up `{name}-{n}.md` tasks and
report back) and lists work across four areas:

- Release cleanup: clear out migration code from older, pre-canonical
  versions of Chan, and add "crystal clear code comments and design
  documentation on the current decisions, as a snapshot, not a
  changelog".
- Search and graph: a graph-like index over directories and files
  (including symlinks and hardlinks, broken links shown as ghost nodes);
  a File Browser "Graph this" action; a Graph overlay scope of
  folder / parent folder / file; a search-status overlay separate from
  the File Browser inspector, with index reset and chan-report
  SLOC-per-language; report-backed search such as `language: Python`;
  and search-window arrow-nav scrolling fixes.
- Assistant (presentation only): keep chat pinned on scroll, stretch
  bubbles to full width, and converge on a single thinking indicator.
- Command-line parity with the web UI: `chan config get|set`,
  `chan graph` per scope, and `chan status` (drive / index / graph /
  report).

## Team, profiles, and coordination

Phase 1 predates the `@@handle` convention; the journals use bare role
names. Contact cards live under `../../agents/`.

```
role        what they did this phase                  card
----------  ----------------------------------------  ------------------
architect   plan, dispatch, design snapshot, sealing  architect.md
rustacean   Rust + Cargo, /api/fs-graph, CLI parity,  rustacean.md
            chan-core purge, watcher/self-upgrade     (-> Systacean)
syseng      low-level hardening, fixture drive, live  syseng.md
            probes, runtime-dependency audit          (-> Systacean)
webdev      frontend: Search Status overlay, fs-graph webdev.md
            render, Graph this, assistant polish      (-> FullStack A/B)
backend     a CLI-parity task, reconciled with the    backend.md
            rustacean CLI work                        (-> FullStack A/B)
webtest     ran the test server, end-to-end browser   webtest.md
            smoke, the CDP smoke runner               (-> Webtest A/B)
```

Legacy-to-current handle mapping is in
[../../agents/README.md](../../agents/README.md).

Coordination scheme: flat task files at the phase root, not one
directory per author. The architect owned a single `journal.md` (a
dispatch table, critical-path diagram, and a dated log). Each unit of
work was a flat `{name}-{n}.md` brief that the assigned agent edited in
place, advancing its own status toward REVIEW; the architect verified
and marked DONE. A second flat channel, `architect-{role}-{n}.md`,
carried agent-to-architect escalations and architect-to-agent dispatch.
This is the early, manual ancestor of the later event-channel model:
there were no per-author directories and no separate event-channel
files yet.

## Duration

Estimate: a single working day, 2026-05-16. Basis: every dated header
inside the journals reads 2026-05-16, and the webtest timestamps run
10:39 to 13:10 Europe/London. The git history for this directory shows
only the 2026-05-18 commit that relocated the journals into
`docs/journals/`, which is migration noise, not the work span.

## Highlights and lowlights

Highlights:
- The `/api/fs-graph` wire shape was frozen before the frontend started,
  so the UI did not have to guess the contract. The summary names this
  the pattern to repeat.
- Live hardening found a release blocker that unit tests missed: any
  symlink, hardlink, or FIFO in a drive pinned `/api/index/status` to a
  permanent error state. A pre-seal audit also caught a mid-path symlink
  escape on `/api/fs-graph` that earlier tests and the first live pass
  had missed.
- The CDP smoke found a real lazy-tree bug: `language:<name>` search only
  scanned the initially expanded root and missed unexpanded folders.
- The runtime-dependency audit confirmed the single static binary holds
  (system frameworks only, no Node, Python, or daemon).

Lowlights:
- The request spanned many boundaries at once (indexing, server routes,
  Svelte overlays, CLI parity, release cleanup), which the architect
  flagged as an idle-time risk if contracts were not frozen early.
- A late webtest "smoke refresh" note read as contradicting the
  already-completed interactive browser smoke; the architect had to
  reconcile the audit trail so it was not self-contradictory.

## Constructive feedback

- Freeze cross-boundary wire shapes early; it prevented frontend
  guessing here.
- Do not leave stale "remaining" sections in task files after later
  verification supersedes them; a short superseding note is enough.
- Web smoke notes must distinguish three different gates: feature
  implemented, HTTP reachable, and browser behavior observed.
  Conflating them caused the contradiction the architect had to clean
  up.
- Keep adjacent-repo work (the chan-core purge) visible in the journal
  before committing, because it changes cross-repo commit ordering.

## What shipped, tried, and undone

Shipped:
- Removed the pre-v3 contact-email backfill consumer and the orphaned
  producer-side helpers in the sibling chan-core checkout.
- New `/api/fs-graph` route: folder/file scope, bounded depth,
  symlink/hardlink/ghost classification, loop termination, escape
  checks, with module tests.
- CLI parity: `chan status`, `chan graph --scope all|file|folder`
  (folder/file scopes reuse the fs-graph builder; `--scope all` stays
  the semantic markdown graph), and `chan config get|set` over the
  editor, server, and assistant namespaces.
- Frontend: File Browser "Graph this", filesystem-graph rendering, a new
  search-status overlay (index plus chan-report SLOC/language) with index
  reset, report-backed `language:<name>` search, search-panel keyboard
  scrolling, and the assistant chat polish.
- Hardening: watcher events are lstat-classified so special or missing
  paths return index status to idle instead of pinning it to error.
- Self-upgrade hardening: downloads routed through the shared HTTPS
  guard; Windows installs park the running executable before replacing
  it.

Tried then corrected:
- `chan graph --scope file|folder` first queried the semantic content
  graph and emitted a synthetic node for a missing target; reconciled to
  use the fs-graph builder and to reject missing, escaping, or
  non-directory targets.
- `chan config` first shipped editor-namespace only; the assistant and
  server namespaces were added in a later reconciliation.

Deliberately not done (release risks recorded for later):
- The chan-core SQLite migration chain v0..v6 was kept rather than
  collapsed; folding it into a single create-from-scratch needs an
  owner decision.
- Signed release checksums and self-upgrade rollback-after-launch are
  post-release hardening items.

Terminology note: phase 1 predates the `chan-drive` to `chan-workspace`
rename. The journals say `chan-drive` and `chan-core`; that crate is
`chan-workspace` in the current tree.

## Raw material

- Source request: [raw/request.md](raw/request.md)
- Seal / summary (with agent ranking and feedback):
  [raw/summary.md](raw/summary.md)
- Architect journal (dispatch table, critical path, dated log):
  [raw/journal.md](raw/journal.md)
- Design snapshot: [raw/design-snapshot.md](raw/design-snapshot.md)
- Task files and the cross-agent handoff channel live alongside them in
  [raw/](raw/).
