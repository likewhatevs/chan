# Phase 3 - UI polish and the Assistant-to-Agent rename

Status: closed (ended at "ready for commit coordination": all task files
at REVIEW, the pre-push gate green, the working tree intentionally left
dirty for a later commit pass)
Span: 2026-05-16 to 2026-05-17 (estimate; see Duration)

## Initial asks

Source: [raw/request.md](raw/request.md), titled "Chan pre-release phase
3 request", derived from Alex's bug-fix / feature-request screenshots. It
is a flat checklist. Representative items, as written:

- "Big change: Assistant -> Agent everywhere, code and surfaces."
- Clicking a status-bar event pops the related overlay.
- The Codex banner wrongly appearing on Claude sessions; assistant CLI
  resume not working.
- "We should have the exact state of each screen reflected in the URL so
  that we can reload the page."
- Real per-assistant banners instead of a copy of one assistant's banner.
- Cmd+F moving the caret to the word match on Enter; File Browser and
  Agent overlay Cmd+F; context menu opening next to the clicked label.
- Cmd+I quoting selected editor text and landing the caret after the
  quote.
- Settings layout changing to standard|compact with standard as default.
- New-file path Tab completion.
- Consistent resource color coding (markdown orange, contacts yellow,
  media purple, binary blue, tag green, folder grey).
- Graph mode consistency, parent-dir and common-ancestor scopes, plus a
  set of editor bugs (indent de-indenting wrapped lines, caret height
  near images, list guides around images, stale selection).
- GitHub-style file and folder icons; the empty pane becomes the primary
  dashboard.

Several items (stale selection, context-menu placement, Agent Cmd+F,
Cmd+I caret, path Tab polish) were appended to the request during the
phase as Alex reported them.

## Team, profiles, and coordination

Handles as written in the journals; cards under `../../agents/`, mapped
to current successors via
[../../agents/README.md](../../agents/README.md).

```
handle       role this phase                           card
-----------  ---------------------------------------   ------------------
@@Architect  plan, dispatch, decisions, summary        architect.md
@@Frontend   highest-output lane: Agent rename, URL    frontend.md
             state, dashboard shell, banners, colors    (-> FullStack A/B)
@@Backend    backend rename, layout config + CLI,      backend.md
             graph/URL audit (renamed Backend+Rust)     (-> FullStack/Sys)
@@Rustacean  Rust review                                rustacean.md
                                                        (-> Systacean)
@@Syseng     reassigned to a frontend-support lane     syseng.md
             (image guides, Cmd+I caret, FB find)       (-> Systacean)
@@Webtest    live service, browser smoke, teardown     webtest.md
                                                        (-> Webtest A/B)
```

The journals also mention @@FrontendB and @@WebtestB; both are the same
physical slots reused, not extra headcount, and are recorded as identity
reconciliations.

Coordination scheme: flat task files at the phase root using the
`{agent}-{n}.md` pattern, all dispatched by the architect through a
single `journal.md` (request checklist, dispatch table, ownership map,
dated log). There were no per-author directories and no separate
event-channel files. Role churn was handled by writing role-change notes
into the journal and creating reassignment task files rather than
rewriting prior ones, which caused some addressing confusion that the
later handle-mapping convention was created to prevent.

## Duration

Estimate: 2026-05-16 to 2026-05-17, two calendar days. Basis: the only
dated headers in the journals are 2026-05-16 and 2026-05-17. The single
2026-05-18 git commit is the bulk migration into `docs/journals/`, not
the work window.

## Highlights and lowlights

Highlights:
- The visible Assistant-to-Agent rename landed across the UI, with a
  deliberate boundary: external API routes, `assistant.*` config keys,
  the on-disk directory, and protocol role strings were intentionally
  preserved.
- The Agent overlay gained scoped Cmd+F and Esc, the correct Cmd+I quote
  caret, and conversation-aware banner resolution (which fixed the
  Codex-on-Claude symptom by walking the conversation's own switch
  history).
- Centralized resource colors via one shared palette, consumed by tree,
  inspector, search, and graph.
- A full pre-push gate passed and services were torn down cleanly.

Lowlights:
- Agent overlay browser validation lagged: the fixture drive had no
  enabled LLM backend, so the assistant shell never mounted and the smoke
  was blocked until a late fixture workaround.
- Identity/role churn: one slot was addressed as both @@Syseng and
  @@FrontendB before correction.
- `cargo test --all-targets` writes a real value into the live user
  config (a pre-existing issue, observed but not fixed this phase).

## Constructive feedback

- Create explicit role-change tasks earlier, and prune stale journal
  follow-ups continuously rather than only at the end.
- Avoid overloading one lane with many independent slices ahead of
  validation.
- Provision an enabled-backend fixture earlier so the Agent overlay smoke
  is not blocked.
- Give each physical slot one canonical handle; the slot reuse here is
  what the later handle-mapping convention formalized.

## What shipped, tried, and undone

Shipped:
- The visible Agent rename across the editor, panes, settings, overlays,
  and the serve docstring, plus three backend error-context strings.
- A compact line-spacing option with standard as default and the legacy
  value normalized via a serde alias.
- Banner resolution by conversation backend, status-bar click routing, a
  search-scope URL hash param, and a dashboard shell on the lone pane.
- Agent overlay Cmd+F, the Cmd+I quote caret, File Browser find over
  visible rows, context menu portaled to escape a transformed ancestor,
  and Esc propagation fixes.
- Editor: document-find caret at word start, nested-list hang indent,
  list-guide auto-hide, GitHub-style icons, a stale-selection defense.
- Centralized resource color tokens, parent-dir and common-ancestor
  graph scopes, and a filesystem-mode folder filter.

Tried then abandoned or not reproduced:
- An attempted @@FrontendB split was reverted once Alex showed @@Frontend
  already owned the tasks.
- A guessed payload to enable the LLM backend returned 200 but changed no
  preferences; the bounded fix reused the phase-1 fake-Codex fixture.
- The "cursor as tall as the image" symptom could not be reproduced after
  the selection fix; left open pending the original screenshot.

Deferred follow-ups: full cross-mode graph filter normalization, synthetic
ancestor nodes for the markdown/link graph, deeper Agent smoke, an in-UI
backend enable toggle, and the test-config pollution fix.

## Raw material

- Source request: [raw/request.md](raw/request.md)
- Summary: [raw/summary.md](raw/summary.md)
- Process contract: [raw/process.md](raw/process.md)
- Master journal: [raw/journal.md](raw/journal.md)
- Lane task files, decisions, and verification notes live alongside them
  in [raw/](raw/).
