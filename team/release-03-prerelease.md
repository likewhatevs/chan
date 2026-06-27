# Phase 3 - UI polish and the Assistant-to-Agent rename

Status: closed
Span: 2026-05-16 to 2026-05-17 (estimate; dated headers in journals are 2026-05-16 and 2026-05-17; the 2026-05-18 commit is a docs migration, not part of the work window)
Versions: none
Tags: #features #bugfixes #editor #refactor #search #graph

## Roadmap (the asks)

Alex submitted a flat checklist of bug fixes and feature requests, derived from screenshots of the running app.

Rename and identity:
- Rename "Assistant" to "Agent" everywhere in the UI and code, with an explicit preservation boundary: external API routes, `assistant.*` config keys, the on-disk directory name, and protocol role strings were not to be changed.

Editor fixes:
- Cmd+F moving the caret to the word match on Enter.
- Cmd+I quoting selected text and landing the caret after the quote.
- Indent de-indenting wrapped lines.
- Caret height near images.
- List guides around images.
- Stale selection.
- GitHub-style file and folder icons.

Agent overlay:
- The Codex banner wrongly appearing on Claude sessions.
- CLI resume not working.
- Real per-agent banners instead of a copy of one agent's banner.
- Cmd+F and Esc scoped to the overlay.

Navigation and URL state:
- Clicking a status-bar event pops the related overlay.
- "We should have the exact state of each screen reflected in the URL so that we can reload the page."
- Context menu opening next to the clicked label (not at a fixed position).

Settings and colors:
- Layout changing to standard|compact with standard as default.
- Consistent resource color coding: markdown orange, contacts yellow, media purple, binary blue, tag green, folder grey.

File Browser:
- New-file path Tab completion.
- Cmd+F over visible rows.

Graph:
- Graph mode consistency, parent-dir scope, common-ancestor scope.
- Empty pane becomes the primary dashboard.

Several items (stale selection, context-menu placement, Agent Cmd+F, Cmd+I caret, path Tab polish) were appended during the phase as Alex reported them, not part of the original checklist.

## Rounds and waves

Single round. Work ran over two calendar days. The architect dispatched tasks in a single pass from the initial checklist. There was one mid-phase reassignment (@@syseng redirected from backend support to frontend-support tasks). No formal waves or gating between them; the team worked in parallel and converged on a shared pre-push gate at the end.

## Team and coordination

Agent roster is in ../agents/README.md. Handles as used in the journals:

```
handle       role this phase
-----------  -------------------------------------------------
@@architect  plan, dispatch, decisions, summary
@@webdev   highest-output lane: Agent rename, URL state,
             dashboard shell, banners, resource colors
Backend    backend rename, layout config + CLI,
             graph/URL audit
@@rustacean  Rust review
@@syseng     reassigned mid-phase to frontend-support tasks:
             image guides, Cmd+I caret, File Browser find
Webtest    live service, browser smoke, teardown
```

The journals also mention FrontendB and WebtestB. Both are the same physical slots reused under different handles mid-phase, not additional headcount. These were recorded as identity reconciliations and directly motivated the later single-canonical-handle convention.

Coordination scheme: flat task files at the phase root using the `{agent}-{n}.md` pattern. The architect maintained a single `journal.md` covering the request checklist, dispatch table, ownership map, and dated log. No per-author directories and no separate event-channel files. Role churn was handled by writing role-change notes into the journal and creating reassignment task files rather than rewriting prior ones, which introduced addressing confusion.

## What shipped, tried, and undone

Shipped:
- The visible Agent rename across the editor, panes, settings, overlays, and the serve docstring; three backend error-context strings also updated. External API routes, config keys, and protocol role strings were intentionally left unchanged.
- A compact line-spacing option with standard as default; the legacy value normalized via a serde alias.
- Banner resolution by conversation backend (fixes the Codex-on-Claude symptom by walking the conversation's own switch history).
- Status-bar click routing to the related overlay.
- A search-scope URL hash param; a dashboard shell on the lone pane.
- Agent overlay: scoped Cmd+F and Esc, the correct Cmd+I quote caret.
- File Browser find over visible rows.
- Context menu portaled to escape a transformed ancestor.
- Editor: document-find caret at word start, nested-list hang indent, list-guide auto-hide, GitHub-style icons, a stale-selection defense.
- Centralized resource color tokens consumed by tree, inspector, search, and graph.
- Parent-dir and common-ancestor graph scopes; filesystem-mode folder filter.

Tried then abandoned or not reproduced:
- An attempted FrontendB split was reverted once Alex confirmed @@webdev already owned those tasks.
- A guessed payload to enable the LLM backend returned 200 but changed no preferences; the bounded fix reused the phase-1 fake-Codex fixture instead.
- The "cursor as tall as the image" symptom could not be reproduced after the selection fix; left open pending the original screenshot.

Deferred:
- Full cross-mode graph filter normalization.
- Synthetic ancestor nodes for the markdown/link graph.
- Deeper Agent overlay smoke with a live backend.
- An in-UI backend enable toggle.
- The test-config pollution fix: `cargo test --all-targets` writes a real value into the live user config; observed this phase but not fixed.

## Retrospective

Highlights:
- The Agent rename landed across the full UI surface in a single phase, with a clean and documented preservation boundary for the external API layer. That boundary prevented a much larger and riskier rename.
- Conversation-aware banner resolution was a non-obvious fix: walking the conversation's own switch history to pick the right banner is the kind of design decision that is hard to reconstruct later.
- Centralized resource color tokens gave the tree, inspector, search, and graph a single source of truth; previously each surface had its own ad-hoc mapping.
- A full pre-push gate passed before close; services were torn down cleanly.

Lowlights and contention:
- Agent overlay browser validation was blocked for most of the phase because the fixture drive had no enabled LLM backend and the agent shell never mounted. The workaround (reusing the phase-1 fake-Codex fixture) came late, which meant the smoke happened near the deadline.
- Identity/role churn: one physical slot was addressed as @@syseng, then FrontendB, before being corrected. This caused task-routing confusion and was the direct motivation for the single-canonical-handle rule adopted in later phases.
- @@webdev received many independent task slices in parallel before any validation loop closed. Later phases split this work across multiple named lanes explicitly.

Constructive feedback:
- Create explicit role-change tasks earlier and prune stale journal follow-ups continuously, not only at close. A journal that accumulates stale entries becomes a coordination liability.
- Provision an enabled-backend fixture at phase start, not as a late workaround. Agent overlay validation is always blocked without it.
- Give each physical slot one canonical handle for the entire phase. Slot reuse is fine; switching handles mid-phase is not.
- Avoid loading one lane with many independent slices before any validation round-trip. Parallel dispatch works, but the validation surface has to be prepared in advance.

## Notes

Terminology drift: "Assistant" was the in-app and code term for what is now called "Agent" (the rename is what this phase shipped). "chan-drive" appears in the raw journals in references to the workspace root; it later became "chan-workspace". "folder" appears for what the codebase calls "directory". These names are updated throughout the current codebase.

Raw working material (per-author journals, task/request/roadmap files, coordination logs) lives in git history under `docs/journals/phase-3/`; that tree was removed from the working tree during the phase-15 docs cleanup.
