# Phase 1 - first public release prep

Status: closed
Span: 2026-05-16, one working day (estimate; all dated headers in the journals read 2026-05-16, webtest timestamps run 10:39 to 13:10 Europe/London; the 2026-05-18 git commit is migration noise, not work span)
Versions: none (pre-release, no version tag cut this phase)
Tags: #features #bugfixes #reliability #graph #search #cli

## Roadmap (the asks)

Alex submitted a "Chan pre-release roadmap" that also defined the working model: the architect coordinates through files in the phase directory; agents pick up `{name}-{n}.md` task briefs and report back. Work fell into four areas:

**Release cleanup.** Remove migration code from older pre-canonical versions of Chan. Add crystal-clear code comments and design documentation reflecting current decisions as a snapshot, not a changelog.

**Search and graph.** Build a graph-like index over directories and files (including symlinks, hardlinks, and broken links shown as ghost nodes). Add a File Browser "Graph this" action and a graph overlay scope selector (folder / parent folder / file). Introduce a search status overlay separate from the File Browser inspector, with index reset and chan-report SLOC-per-language. Add report-backed search (e.g. `language: Python`). Fix search-window arrow-nav scrolling.

**Assistant (presentation only).** Keep chat pinned on scroll, stretch bubbles to full width, converge on a single thinking indicator.

**CLI parity.** `chan config get|set`, `chan graph --scope` per scope, and `chan status` (drive / index / graph / report).

## Rounds and waves

Single round. All work was dispatched and completed on 2026-05-16. The architect froze the `/api/fs-graph` wire shape before any frontend work started; that sequencing kept the Svelte overlay from having to guess the contract. The rustacean and syseng roles worked in parallel on the backend and hardening tracks; webdev and webtest followed once routes were green.

## Team and coordination

Phase 1 predates the `@@handle` convention; the journals use bare role names. The current agent roster and legacy-to-current handle mapping are in `../agents/README.md`.

```
role        work this phase                                 card
----------  ----------------------------------------------  ------------
architect   plan, dispatch, design snapshot, sealing        architect.md
rustacean   Rust + Cargo, /api/fs-graph, CLI parity,        rustacean.md
            chan-core purge, watcher/self-upgrade           (-> Systacean)
syseng      low-level hardening, fixture drive, live        syseng.md
            probes, runtime-dependency audit                (-> Systacean)
webdev      Search Status overlay, fs-graph rendering,      webdev.md
            Graph this, assistant polish                    (-> FullStack A/B)
backend     CLI-parity task, reconciled with rustacean      backend.md
            CLI work                                        (-> FullStack A/B)
webtest     test server, end-to-end browser smoke,          webtest.md
            CDP smoke runner                                (-> Webtest A/B)
```

Coordination scheme: flat task files at the phase root, not one directory per author. The architect maintained a single `journal.md` (dispatch table, critical-path diagram, dated log). Each unit of work was a flat `{name}-{n}.md` brief that the assigned agent edited in place, advancing its status toward REVIEW; the architect verified and marked it DONE. A second flat channel, `architect-{role}-{n}.md`, carried agent-to-architect escalations and architect replies.

This is the early, manual ancestor of the later event-channel model. There were no per-author directories and no separate event-channel files yet.

## What shipped, tried, and undone

**Shipped:**

- Removed the pre-v3 contact-email backfill consumer and the orphaned producer-side helpers in the sibling chan-core checkout.
- New `/api/fs-graph` route: folder/file scope, bounded depth, symlink/hardlink/ghost node classification, loop termination, and path-escape checks, with module tests.
- CLI parity: `chan status`, `chan graph --scope all|file|folder` (folder/file scopes reuse the fs-graph builder; `--scope all` stays the semantic markdown graph), and `chan config get|set` covering the editor, server, and assistant namespaces.
- Frontend: File Browser "Graph this" action, filesystem-graph rendering, a new search-status overlay (index plus chan-report SLOC/language breakdown) with index reset, report-backed `language:<name>` search, search-panel keyboard scrolling, and assistant chat polish.
- Hardening: watcher events are lstat-classified so special or missing paths return index status to idle instead of pinning it to error.
- Self-upgrade hardening: downloads routed through the shared HTTPS guard; Windows installs park the running executable before replacing it.

**Tried then corrected:**

- `chan graph --scope file|folder` first queried the semantic content graph and emitted a synthetic node for a missing target. Reconciled to use the fs-graph builder and to reject missing, escaping, or non-directory targets.
- `chan config` first shipped editor-namespace only; the assistant and server namespaces were added in a later reconciliation.

**Deliberately not done (recorded risks):**

- The chan-core SQLite migration chain v0..v6 was kept rather than collapsed; folding it into a single create-from-scratch needs an owner decision.
- Signed release checksums and self-upgrade rollback-after-launch are post-release hardening items.

## Retrospective

**Highlights:**

- Freezing the `/api/fs-graph` wire shape before the frontend started prevented contract guessing and kept the parallel tracks from blocking each other. The post-phase summary names this the pattern to repeat.
- Live hardening (syseng probing a fixture drive) found a release blocker that unit tests missed: any symlink, hardlink, or FIFO in a drive pinned `/api/index/status` to a permanent error state. A pre-seal audit also caught a mid-path symlink escape on `/api/fs-graph` that earlier tests and the first live pass had both missed. The lesson: static tests do not substitute for a live probe against realistic fixture content.
- The CDP smoke found a real lazy-tree bug: `language:<name>` search only scanned the initially expanded root and missed unexpanded folders. Browser-driven smoke caught what unit tests could not.
- The runtime-dependency audit confirmed the single static binary invariant holds (system frameworks only, no Node, Python, or daemon).

**Lowlights / contention:**

- The request spanned many area boundaries at once (indexing, server routes, Svelte overlays, CLI parity, release cleanup). The architect flagged idle-time risk if contracts were not frozen early; freezing them was the mitigation that worked.
- A late webtest "smoke refresh" note read as contradicting the already-completed interactive browser smoke. The architect had to reconcile the audit trail to prevent a self-contradictory record.

**Constructive feedback / lessons:**

- Freeze cross-boundary wire shapes before parallel tracks start. It worked here; the lesson generalizes to every phase with a backend-and-frontend split.
- Do not leave stale "remaining" sections in task files after later verification supersedes them. A short superseding note is enough. Stale sections create contradictory audit trails that cost architect time to reconcile.
- Web smoke notes must distinguish three distinct gates: feature implemented, HTTP endpoint reachable, and browser behavior observed. Conflating them caused the contradiction the architect had to clean up.
- Keep adjacent-repo work (here, the chan-core purge) visible in the journal before committing; it changes cross-repo commit ordering and can blindside peers who do not know the sibling checkout is in play.

## Notes

**Terminology drift:** phase 1 predates the `chan-drive` to `chan-workspace` rename and the `chan-core` to `chan-workspace` crate rename. The source journals say `chan-drive` (the workspace root directory concept) and `chan-core` (the library crate); those map to `chan-workspace` in the current tree. "folder" in the original request means "directory" in current project writing rules.

Raw working material (per-author journals, task briefs, request and roadmap files, coordination logs) is preserved in git history under `docs/journals/phase-1/`; it was removed from the working tree during the phase-15 docs cleanup and will be deleted at the close of phase 18.
