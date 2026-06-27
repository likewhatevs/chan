# Phase 13 - Graph/Dashboard rework, then the Team Work revamp

Status: closed (two rounds; round 1 cut v0.17.0, round 2 cut v0.18.0)
Span: 2026-05-28 to 2026-05-29 (estimate; git author dates run 2026-05-28 22:17 to 2026-05-29 11:16; the round boundary straddles the two days)
Versions: v0.17.0, v0.18.0
Tags: #bugfixes #editor #graph #features #release #desktop

## Roadmap (the asks)

Two rounds, each authored as a standalone roadmap file by @@Alex.

**Round 1** was a combined bug list plus three enhancement areas.

Bugs: new-document cursor not ready to type; a spurious "Unsaved changes" notice on brand-new documents; list markers not preserving what the user typed; an empty-pane highlight inconsistency; a missing pane hover wobble; Shift+Enter submitting instead of inserting a newline in the terminal.

Enhancements:
- Hybrid Inspector: show the absolute path with a copy button; per-kind "Graph from here" chips; workspace-root parity.
- Hybrid Graph KINDS rework: path/contact/hashtag/language layers, expand/collapse controls, symlink-colored edges.
- Infographics renamed to Dashboard: auto-resize carousel; About, Workspace, and Search widgets; Settings flip-back; retiring the settings overlay; rebinding Cmd+, to the per-pane settings toggle.

**Round 2** extended the UI with three areas and one large deletion.

- Desktop: a global chord (Cmd+Shift+N) to open a new window of the current workspace.
- Rich Prompt renamed to Team Work: full UI and code rename, removing the Spawn-agent dialog, adding a lead-first Cmd+P flow with a redesigned dialog.
- The entire fsnotify-watcher backend for agent-event coordination was deleted (routes, endpoints, spool, terminal dispatch). Notification bubbles were reduced to a frontend-only static stub with an explicit caveat: equivalent functionality would return in a later phase.
- Editor: list-rendering glyphs; Bold/Italic chords (Cmd+B / Cmd+I); new hamburger split-shortcut labels.

## Rounds and waves

**Round 1 (v0.17.0)**

Two lanes ran in parallel against the same git checkout. @@LaneA owned the content surfaces (editor, terminal, Inspector); @@LaneB owned the structural shell (pane, graph, Dashboard). One hard cross-lane dependency: the Inspector kind-chips gate on @@LaneB's graph KIND routes going in first. @@LaneB also owned the merge gate: it gated the combined tree and cut v0.17.0 after both lanes finished.

**Round 2 (v0.18.0)**

Same two-lane structure with no cross-lane dependencies this time. @@LaneA ran as Team Work full-stack lead and spawned four in-session subagents to handle the work in parallel: backend deletion, frontend foundation, the Team Work component, and the notification-bubble stub. @@LaneB owned the shared files (editor glyphs and chords, desktop chord) and again served as merge gate, cutting v0.18.0.

## Team and coordination

Agent roster is in `../agents/README.md`. There was no separate @@Architect handle this phase; @@Alex wore the planning hat, authoring both roadmaps and the closing briefs.

```
Handle    Role this phase
--------  -------------------------------------------------
@@Alex    owner and planner; authored roadmaps and briefs;
          sole authority to push tags
@@LaneA   R1: content surfaces (editor, terminal, Inspector)
          R2: Team Work full-stack lead; spawned subagents
@@LaneB   R1: structural shell (pane, graph, Dashboard);
          R2: editor lists, chords, desktop chord;
          both rounds: merge gate and release cut
```

Coordination scheme: append-only per-author directional event channels (`event-<from>-<to>.md`) and per-lane journals in the main checkout, with source code living in per-lane git worktrees. Round-2 entries were appended below a divider in each channel file so the same files served both rounds. @@LaneB serialized all merges; no remote push without an explicit @@Alex ask.

## What shipped, tried, and undone

**Shipped in round 1 (v0.17.0)**

- New-document cursor receives focus immediately on creation.
- Fresh-draft "Unsaved changes" suppression (the badge no longer appears on a document that has never been saved).
- List-marker source preservation (the typed character is kept).
- Terminal Shift+Enter inserts a newline instead of submitting.
- Hybrid Inspector: absolute path with copy button, workspace-root parity, per-kind "Graph from here" chips.
- Graph KIND rework: discriminated path/contact/hashtag/language layers with expand/collapse controls and symlink-colored edges, consistent across the backend discriminator, open-graph helpers, and Inspector chips.
- Infographics renamed to Dashboard: auto-resize carousel, About / Workspace / Search widgets, Settings flip-back, settings overlay retired, Cmd+, rebound to the per-pane settings toggle.

**Shipped in round 2 (v0.18.0)**

- Desktop Cmd+Shift+N chord opens a new window of the current workspace.
- Rich Prompt renamed to Team Work across UI and code (160 references, 35 files); Spawn-agent dialog removed; lead-first Cmd+P flow with redesigned dialog.
- Editor list glyphs and Bold/Italic chords (Cmd+B / Cmd+I).
- New hamburger split-shortcut labels.

**Removed in round 2, returning later**

The fsnotify-watcher agent-event coordination backend was deleted outright: the event watcher, the rich-prompt HTTP routes and endpoints, the terminal-session dispatch, and the workspace spool. The Team Work notification bubble overlay was reduced to a frontend-only static stub. Equivalent notification functionality is planned to return in a later phase. The orchestration skill docs under `../../agents/orchestration/` intentionally retain the removed system as a blueprint for the returning implementation; a fuller rewrite lands when the replacement does.

**Tried then undone (within round)**

- A first list-glyph attempt using absolute-positioned pseudo-elements was reverted to in-flow CSS after a browser smoke caught a gutter detachment on nested glyphs.
- A blanket-scrub edit during the rich-prompt rename flipped a vitest absence-guard; caught by vitest and fixed.
- A narrow residual-cleanup commit was superseded by the broader scrub.

**Post-cut note**

v0.18.0 cut green across all CI jobs. A `chan.app/dl/*` 404 observed afterward was CDN propagation lag from GitHub Pages, not a release-cut failure. The 0.17.0-to-0.18.0 self-upgrade path via `/dl` was left as an @@Alex desktop verify.

## Retrospective

**Highlights**

- Both round-1 lanes closed end to end: every roadmap item and every closing-smoke item shipped. The KIND graph rework landed coherently across all three layers (backend discriminator, open-graph helpers, Inspector chips) in parallel without cross-lane rework.
- Round-2 auto-merge was clean across three files touched by both lanes; the one real overlap was declared cross-lane before any edit.
- Browser smoke earned its keep twice in round 2: it caught the nested-glyph gutter detachment AND confirmed the Cmd+, per-pane invariants and the rich-prompt wire rename end to end, in both cases after the static gate had passed the broken intermediate.
- The 160-reference, 35-file rich-prompt scrub landed with zero svelte-check collisions and a green browser smoke on the first attempt.

**Lowlights**

- A round-1 Cmd+, regression on the desktop (WKWebView) could not be root-caused from the CLI because the Chrome MCP cannot drive WKWebView. A defensive matcher shipped; the fix was flagged empirically unverified at cut time.
- The round-1 merge gate missed three of @@LaneA's closing slices on the first cycle because it compared against a stale status snapshot rather than reading the channel tail. @@Alex's nudge was required to surface the gap.
- A round-2 list-glyph first attempt was wrong and only the browser smoke caught it, producing one avoidable round-trip.

**Lessons**

- Reading the channel tail (not a last-noted status snapshot) before each merge-gate cycle is the correct discipline; stale status misses work that arrived after the last check.
- Desktop-only bugs that cannot be reproduced with the Chrome MCP should be escalated to @@Alex as soon as they are confirmed WKWebView-specific, not left as unverified patches.
- Blanket string-scrubs need a pre-scan for assertions that check the old string's ABSENCE; those turn into false positives after the rename.
- Stating "no legacy identifiers, pre-release so no back-compat" up front in the roadmap lets the responsible lane do the full scrub in its own commit rather than leaving it for the merge gate.
- When a lane owns in-session subagents, declaring shared-file ownership before the subagents start prevents the silent contamination that a concurrent `git add` + `git commit` can cause.

## Notes

**Terminology drift**

- "Rich Prompt" was the earlier name for the Team Work feature. Round 2 performed the full rename across UI and code. In phase-13 journals and commit messages, "rich prompt" and "Rich Prompt" refer to what is now called "Team Work."
- "Hybrid Inspector" and "Hybrid Graph" refer to the Inspector panel and Graph overlay as they existed from phase 12 onward; "Hybrid" was the internal working name for the multi-pane split layout.

Raw working material (per-author journals, task/request/roadmap files, coordination logs) lives in git history under `docs/journals/phase-13/`.
