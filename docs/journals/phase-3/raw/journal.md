# Chan Pre-Release Phase 3 Journal

Owner: @@Architect. Host: Alex.

Source request: [request.md](./request.md).

## Plan summary

Phase 3 is a UI-heavy polish and correctness pass: rename Assistant to Agent
across visible surfaces and code where practical, fix agent CLI/session
resumption, make screen state reloadable through URLs, tighten editor/file
browser interactions, unify resource visual language, improve graph mode
consistency, and establish the tabless background as the primary dashboard.

## Request checklist

- [ ] Assistant -> Agent everywhere, code and surfaces.
- [ ] Status bar event click opens the related overlay.
- [ ] Agent CLI resume works correctly and shows the right banner.
- [ ] Screen state is reflected in the URL and reloadable.
- [ ] Supported agents have real banners rather than copied Claude banners.
- [ ] Document Cmd+F Enter places cursor at the beginning of the word match.
- [ ] File Browser overlay supports Cmd+F over expanded and visible entries.
- [ ] File Browser context menu opens near the clicked row/label.
- [ ] Agent overlay supports Cmd+F over current session chat history.
- [ ] Agent Cmd+I from selected text places caret after the inserted quote.
- [ ] Settings layout becomes standard/compact, with standard as default.
- [ ] New-file creation supports tab-complete.
- [ ] Resource type colors are consistent across inspector, browser, search,
  agent, and graph.
- [ ] Graph modes have consistent filters and scope options.
- [ ] Multi-level indent no longer de-indents the following long-sentence line.
- [ ] File/folder icons match GitHub style and file browser uses folder icons.
- [ ] Cursor height is not inherited from an image on the previous line.
- [ ] Document list guide lines work around images and auto-hide after 1.5s
  outside the list.
- [ ] Empty tab/background window becomes the primary dashboard.

## Dispatch

| Task | Owner | Status | Depends on |
|------|-------|--------|------------|
| [backend-1](./backend-1.md) | @@Backend+Rustacean | REVIEW | - |
| [frontend-1](./frontend-1.md) | @@Frontend | REVIEW | backend-3 landed; syseng-frontend-4 handles layout frontend wiring |
| [frontend-2](./frontend-2.md) | @@Frontend | REVIEW | stale-selection defense landed; needs webtest/browser repro for remaining image residuals |
| [frontend-3](./frontend-3.md) | @@Frontend | REVIEW | color/scope/folder filter landed; cross-mode filter normalization deferred |
| [backend-2](./backend-2.md) | @@Backend+Rustacean | REVIEW | - |
| [backend-3](./backend-3.md) | @@Backend+Rustacean | REVIEW | layout standard/compact backend landed; frontend wiring remains |
| [backend-rustacean-1](./backend-rustacean-1.md) | @@Backend+Rustacean | REVIEW | role handoff recorded; old backend slot can tear down |
| [backend-teardown-1](./backend-teardown-1.md) | @@Backend | REVIEW | original backend slot teardown complete |
| [rustacean-1](./rustacean-1.md) | @@Rustacean | REVIEW | backend-1 findings available |
| [syseng-1](./syseng-1.md) | @@Syseng | REVIEW | backend-1/backend-2/rustacean-1 reviewed |
| [webtest-1](./webtest-1.md) | @@Webtest | REVIEW | browser smoke complete; service teardown closed in webtest-5 |
| [webtest-2](./webtest-2.md) | @@Frontend | REVIEW | WebtestB smoke pass complete; findings filed |
| [webtest-3](./webtest-3.md) | @@Webtest | REVIEW | Settings/narrow pass complete; Agent overlay blocked by fixture |
| [webtest-4](./webtest-4.md) | @@Webtest | REVIEW | assistant-enabled Agent overlay smoke passed; deeper chat-history/banner sync cases recorded as gaps |
| [webtest-5](./webtest-5.md) | @@Webtest | REVIEW | services stopped; ports 5173/8787 free |
| [architect-decision-1](./architect-decision-1.md) | @@Architect | REVIEW | chose bounded assistant-enabled smoke attempt |
| [architect-verify-1](./architect-verify-1.md) | @@Architect | REVIEW | `scripts/pre-push` passed |
| [frontend-b-1](./frontend-b-1.md) | @@Syseng | REVIEW | historical FrontendB read-only findings; no active separate owner |
| [frontend-b-2](./frontend-b-2.md) | @@Syseng | REVIEW | historical FrontendB path prompt polish landed; no active separate owner |
| [frontend-idle](./frontend-idle.md) | @@Architect | REVIEW | frontend idle handoff |
| [syseng-frontend-1](./syseng-frontend-1.md) | @@Syseng | REVIEW | image-guide cap landed; cursor-height not reproduced |
| [syseng-frontend-2](./syseng-frontend-2.md) | @@Syseng | REVIEW | Agent quote caret placement fixed |
| [syseng-frontend-3](./syseng-frontend-3.md) | @@Syseng | REVIEW | File Browser validation passed; Agent overlay validation owed |
| [syseng-frontend-4](./syseng-frontend-4.md) | @@Syseng | REVIEW | Settings layout frontend wiring landed; browser validation owed |
| [architect-1](./architect-1.md) | @@Architect | TODO | backend idle handoff |
| [architect-2](./architect-2.md) | @@Architect | TODO | rustacean idle handoff |
| [architect-syseng-1](./architect-syseng-1.md) | @@Architect | TODO | syseng idle handoff |

Statuses: TODO, IN_PROGRESS, BLOCKED, REVIEW, DONE.

## Initial ownership map

- @@Backend+Rustacean owns backend/Rust implementation and Rust quality review
  for its own backend/config changes unless @@Architect asks for a separate
  review.
- @@Frontend owns visible Agent terminology, URL state, status bar routing UI,
  banners, layout presets, resource colors, document/file browser interaction,
  graph mode UX, icons, list guide behavior, and dashboard shell.
- @@Rustacean owns Rust code quality for CLI/session/backend changes, tests,
  naming hygiene, dependency discipline, and commit readiness review.
- @@Syseng owns filesystem/process/session hardening, graph edge cases, and
  operational validation for resume/status/index behavior.
- @@Webtest owns the live test service and browser smoke coverage.
- Reassignment on 2026-05-16: @@Syseng is now a frontend implementation/support
  lane in [syseng-frontend-1.md](./syseng-frontend-1.md), focused on deferred
  editor image-selection residuals after a browser repro. @@Frontend is now
  @@WebtestB in [webtest-2.md](./webtest-2.md), using the existing webtest
  service without restarting it.
- Identity cleanup on 2026-05-16: the earlier @@FrontendB lane was the same
  agent slot that is now @@Syseng. Treat [frontend-b-1](./frontend-b-1.md) and
  [frontend-b-2](./frontend-b-2.md) as historical REVIEW tasks; do not ping a
  separate @@FrontendB identity.
- Role update on 2026-05-16: the backend slot is now Backend+Rustacean. Route
  backend/Rust implementation and ordinary Rust review for backend-owned changes
  to @@Backend+Rustacean; ask for a separate @@Rustacean pass only for high-risk
  or cross-cutting Rust changes.
- Coordination should happen through task files, not pasted prompts. New role
  changes and assignments should be recorded as tasks in this directory.

## Notes & decisions

- Preserve compatibility intentionally where "assistant" is part of an external
  config/API/schema name. Rename public/user-visible surfaces first; internal
  code rename should be staged if it risks broad churn.
- URL state should be scoped enough to reload the active overlay/screen and
  selected resource without encoding volatile transient state.
- Graph mode consistency likely needs a frontend filter model first. Backend
  should only grow endpoints if current graph data cannot represent folder/path
  nodes or cross-link overlays safely.
- Resource colors should come from a shared frontend classification/theme path
  rather than one-off CSS in each component.
- @@FrontendB flagged one frontend-1 ambiguity: `AppStatusBar.svelte` currently
  avoids agent activity by design, while the request lists agent chats as an
  example status-bar target. Treat existing status sections as in scope; adding
  a new agent-activity status-bar section needs explicit product confirmation.

## Log

- 2026-05-16 @@Architect: read [process.md](./process.md), profile, architect
  guide, and prior phase coordination format. Created [request.md](./request.md),
  this journal, and first-wave task briefs.
- 2026-05-16 @@Backend: completed [backend-1](./backend-1.md) and
  [backend-2](./backend-2.md). Renamed "assistant config" -> "agent config"
  error context strings in `crates/chan/src/main.rs`; documented that the
  CODEx-on-CLAUDE banner symptom is a frontend state-sync bug (handed off to
  @@Frontend with the specific files and lines); confirmed the existing
  `/api/llm/*` and `/ws "llm.*" / "progress"` surfaces already expose
  everything frontend-1 needs for agent metadata + status routing; audited
  graph endpoints and confirmed no backend changes are required for the
  phase-3 graph-consistency / URL-state work. Both tasks REVIEW, ready for
  @@Rustacean (backend-1) and @@Syseng (both) review per the task files.
  Idle and ready for more work.
- 2026-05-16 @@Architect: reconciled backend update. [frontend-1](./frontend-1.md)
  is unblocked: banner bug is frontend conversation-state selection, status
  routing can use existing `/ws` progress + `llm.*` frames, and banners can use
  existing `/api/llm/status`, `/api/llm/cli-detection`, and preferences data.
  [frontend-3](./frontend-3.md) is unblocked: graph consistency can compose
  existing `/api/graph` and `/api/fs-graph` using shared path identifiers.
  Updated [rustacean-1](./rustacean-1.md) and [syseng-1](./syseng-1.md)
  dispatch states to IN_PROGRESS.
- 2026-05-16 @@Architect: Alex confirmed @@Frontend is already working on
  [frontend-3](./frontend-3.md). Marked it IN_PROGRESS and left a note for
  @@Frontend to fill in implementation details as they land.
- 2026-05-16 @@Architect: polled task files after Alex asked whether the team is
  blocked on frontend. [rustacean-1](./rustacean-1.md) and
  [syseng-1](./syseng-1.md) are REVIEW with no blocking findings; @@Backend,
  @@Rustacean, and @@Syseng each filed idle handoff tasks
  ([architect-1](./architect-1.md), [architect-2](./architect-2.md),
  [architect-syseng-1](./architect-syseng-1.md)). [webtest-1](./webtest-1.md)
  is IN_PROGRESS with service up at `http://127.0.0.1:5173/`, but browser smoke
  is waiting on implementation slices. Current critical path is @@Frontend work,
  especially [frontend-3](./frontend-3.md), plus unstarted
  [frontend-1](./frontend-1.md) and [frontend-2](./frontend-2.md).
- 2026-05-16 @@Architect: added Alex's newly reported stale editor selection bug
  to [request.md](./request.md) and [frontend-2.md](./frontend-2.md). The
  symptom is large blue selection rectangles around image/list blocks that do
  not clear normally, including after the caret moves to another line.
- 2026-05-16 @@Architect: reverted the tentative @@FrontendB split after Alex
  showed @@Frontend already has frontend-1, frontend-2, and frontend-3 in its
  active queue. [frontend-2](./frontend-2.md) ownership stays with @@Frontend.
  If we add @@FrontendB, create a fresh non-overlapping task instead of pointing
  it at frontend-2.
- 2026-05-16 @@Architect: created [frontend-b-1](./frontend-b-1.md) as a
  read-only frontend support review task. It can be assigned to the idle backend
  agent with the frontend/webdev skill loaded; it must not edit source files or
  duplicate @@Frontend's implementation queue.
- 2026-05-16 @@Frontend: moved [frontend-2](./frontend-2.md) to REVIEW. Landed
  document find caret placement, File Browser Cmd+F over visible expanded rows,
  new-file tab completion, nested-list hang indent, list-guide auto-hide, and
  GitHub-style chevrons/folder icons. Verification recorded there:
  `cd web && npm run check` clean and `cd web && npm test -- --run` with 130
  tests passing. Deferred pending browser repro: cursor height after images,
  image/list guide breakage around images, and stale selection rectangles around
  image/list blocks. @@Webtest should smoke the landed cases and try to
  reproduce the deferred cluster before @@Architect treats frontend-2 as
  complete.
- 2026-05-16 @@Architect: added Alex's new Agent overlay Cmd+F requirement to
  [request.md](./request.md), [frontend-1.md](./frontend-1.md), and
  [webtest-1.md](./webtest-1.md). It belongs to frontend-1: overlay-scoped find
  over the current Agent conversation/session with next/previous navigation.
- 2026-05-16 @@FrontendB: moved [frontend-b-1](./frontend-b-1.md) to REVIEW.
  Recorded read-only findings for @@Frontend: current dirty-work map, remaining
  Agent rename audit points, SERVE_LONG_ABOUT regeneration, banner state-sync
  fix location, status-bar click ambiguity, existing URL hash coverage and gaps,
  layout standard/compact compatibility options, dashboard implementation map,
  frontend-2 residual hypotheses, and frontend-3 color-token/filter risks. No
  source files changed by @@FrontendB.
- 2026-05-16 @@Architect: added Alex's newly reported File Browser context-menu
  positioning bug to [request.md](./request.md), [frontend-2.md](./frontend-2.md),
  and [webtest-1.md](./webtest-1.md). Symptom: right-clicking a file/folder
  label opens the menu far toward the lower-right instead of adjacent to the
  click/row, visible with inspector pane open.
- 2026-05-16 @@Frontend: updated [frontend-2](./frontend-2.md) after REVIEW with
  a fix for File Browser context-menu positioning. Root cause was transformed
  overlay panel containing `position: fixed`; fix portals the context menu to
  `document.body`, matching `HamburgerMenu.svelte`.
- 2026-05-16 @@Frontend: moved [frontend-3](./frontend-3.md) to REVIEW. Landed
  centralized binary/folder color tokens, graph/file-tree usage of those tokens,
  parent-dir and common-ancestor scope options with tests, and a filesystem
  graph folder filter with URL-hash compatibility. Deferred full cross-mode
  graph filter normalization and markdown-mode synthetic folder/path nodes.
- 2026-05-16 @@Frontend: updated [frontend-1](./frontend-1.md) to IN_PROGRESS
  partial. Landed visible Agent rename, search scope URL state, dashboard shell,
  and Agent overlay Cmd+F over current chat. It reports backend blockers for
  banners/status routing/layout; @@Architect reconciled with [backend-1](./backend-1.md):
  banner/status data already exists unless a specific missing field is filed.
  Created [backend-3](./backend-3.md) for the real backend dependency:
  standard/compact layout config and CLI support.
- 2026-05-16 @@Frontend: filed [frontend-idle](./frontend-idle.md), reporting all
  frontend-only work landed, checks/tests/build green, and waiting on backend-3
  plus @@Webtest smoke.
- 2026-05-16 @@Architect: added Alex's path prompt Tab-completion polish request
  to [request.md](./request.md) and created [frontend-b-2](./frontend-b-2.md)
  for @@FrontendB. Scope: new file/new folder/rename-move path inputs should
  complete with Tab, preserve directory trailing `/`, suggest a `.md` filename
  in new-file flows, and keep Enter as confirmation.
- 2026-05-16 @@Architect: adjusted staffing per Alex. @@Syseng moves into frontend
  support via [syseng-frontend-1](./syseng-frontend-1.md), scoped to deferred
  editor image-selection residuals after browser repro. @@Frontend moves to
  WebtestB via [webtest-2](./webtest-2.md), using the existing
  `http://127.0.0.1:5173/` service and focusing on browser smoke, not source
  edits.
- 2026-05-16 @@Frontend: moved [frontend-1](./frontend-1.md) to REVIEW. Landed
  CODEx-on-CLAUDE banner state-sync by preferring the active conversation's
  latest `assistant_switch`, AppStatusBar click routing (index -> Settings,
  import -> File Browser, transient status -> clear), Agent overlay Cmd+F,
  dashboard shell, search-scope URL state, visible Agent rename, and
  SERVE_LONG_ABOUT regeneration. Only Settings layout standard/compact remains
  gated on [backend-3](./backend-3.md).
- 2026-05-16 @@Frontend: also pulled one [frontend-2](./frontend-2.md) deferred
  item forward by adding CodeMirror `drawSelection()` to Wysiwyg to address
  stale native selection rectangles around image/list blocks. Remaining
  image-related residuals still need browser repro: cursor height after images
  and image-line guide bar breakage.
- 2026-05-16 @@FrontendB: [frontend-b-2](./frontend-b-2.md) is IN_PROGRESS.
- 2026-05-16 @@FrontendB: moved [frontend-b-2](./frontend-b-2.md) to REVIEW.
  Landed PathPromptModal Tab completion polish: Tab accepts highlighted
  suggestions, LCP is scoped to directory suggestions, new-file flows add
  `<dir>/untitled.md` placeholder with stem selection, and
  `pathValidate.test.ts` covers the helper contracts. Verification recorded:
  `cd web && npm run check` clean and `cd web && npm test -- --run` green.
- 2026-05-16 @@Architect: added Alex's Agent quote insertion caret bug to
  [request.md](./request.md) and dispatched [syseng-frontend-2](./syseng-frontend-2.md)
  to @@Syseng's frontend lane. Symptom: selecting editor text and pressing
  Cmd+I inserts the quote correctly, but the Agent prompt caret lands at the
  beginning of the quote instead of the first editable line after it.
- 2026-05-16 @@Syseng: moved [syseng-frontend-2](./syseng-frontend-2.md) to
  REVIEW. Fixed Agent quote insertion caret placement by seeding a one-shot
  `assistantOverlay.promptCaretTarget`, consuming it in `InlineAssist.svelte`,
  and adding `focusAt(pos)` to prompt editor components. Verification recorded:
  `cd web && npm run check`, focused store tests, and full `cd web && npm test -- --run`
  all passed.
- 2026-05-16 @@Webtest: updated [webtest-1](./webtest-1.md) with desktop smoke.
  Several frontend-2 cases pass, including context-menu portal positioning.
  New blocker: BUG-FE2-A, File Browser find next/prev never advances beyond
  match 1 of 3. Also noted minor web-only Cmd+F leakage risk when browser native
  find does not appear. Image/cursor residual repro is still inconclusive.
- 2026-05-16 @@Architect: cleaned up role confusion after Alex noted the same agent
  slot was being addressed as both @@Syseng and @@FrontendB. Retired active
  @@FrontendB pings; future implementation/review pings for that slot should use
  @@Syseng, while [frontend-b-1](./frontend-b-1.md) and
  [frontend-b-2](./frontend-b-2.md) remain historical REVIEW notes.
- 2026-05-16 @@Backend: moved [backend-3](./backend-3.md) to REVIEW. Landed
  `LineSpacing::Compact` with `#[serde(alias = "tight")]`, default `Standard`,
  CLI parse/label support for `standard | compact` with legacy `tight` alias,
  tests, fmt, clippy, and @@Rustacean review. Frontend wiring remains.
- 2026-05-16 @@WebtestB: moved [webtest-2](./webtest-2.md) to REVIEW. Smoke pass
  confirms graph/search URL hashes, parent-dir scope option, Agent Cmd+F,
  Agent Cmd+I quote caret, drawSelection behavior, and several color checks.
  Confirmed BUG-FE2-A and found Esc in overlay find bars bubbles to close the
  whole overlay. Could not reproduce cursor-height-after-image; did reproduce
  image-height guide bar chunkiness.
- 2026-05-16 @@Syseng: moved [syseng-frontend-1](./syseng-frontend-1.md) to
  REVIEW. Fixed only the reproduced image-guide residual by marking
  image-bearing list lines and capping their guide bar to a text-height segment.
  No fix for cursor height/stale selections because smoke could not reproduce
  them after drawSelection.
- 2026-05-16 @@Architect: created [syseng-frontend-3](./syseng-frontend-3.md) for
  BUG-FE2-A and Esc propagation in overlay find bars.
- 2026-05-16 @@Architect: created [backend-rustacean-1](./backend-rustacean-1.md)
  so the Backend+Rustacean slot switch and standby expectations are visible via
  task files instead of requiring pasted prompts.
- 2026-05-16 @@Architect: marked [backend-rustacean-1](./backend-rustacean-1.md)
  REVIEW. Backend tasks [backend-1](./backend-1.md), [backend-2](./backend-2.md),
  and [backend-3](./backend-3.md) are REVIEW; original backend slot has no
  active implementation work and can be torn down once the replacement
  Rustacean+Backend slot is active.
- 2026-05-16 @@Architect: created [backend-teardown-1](./backend-teardown-1.md)
  for the original backend slot teardown. Future backend/Rust work stays with
  the Rustacean+Backend slot.
- 2026-05-16 @@Backend: moved [backend-teardown-1](./backend-teardown-1.md) to
  REVIEW. Original backend slot is closed; no owned processes, temp files, or
  branches remain. Shared Webtest service left running.
- 2026-05-16 @@Syseng: moved [syseng-frontend-3](./syseng-frontend-3.md) to
  REVIEW. Fixed File Browser find next/prev cursor preservation and verified
  Esc handlers stop propagation in File Browser and Agent find bars. Also fixed
  an Agent find effect loop found by WebtestB. Browser validation still owed.
- 2026-05-16 @@Architect: created [syseng-frontend-4](./syseng-frontend-4.md) for
  the remaining frontend side of Settings Layout `standard | compact`, now that
  [backend-3](./backend-3.md) has landed.
- 2026-05-17 @@Syseng: moved [syseng-frontend-4](./syseng-frontend-4.md) to
  REVIEW. Frontend Settings/Layout now uses `standard | compact`, standard
  fallback/default, compact editor densities, and legacy `tight` normalization.
  `cd web && npm run check` and `cd web && npm test -- --run` passed.
- 2026-05-17 @@Webtest: updated [webtest-1](./webtest-1.md). File Browser
  find next/previous and Esc behavior now pass after [syseng-frontend-3](./syseng-frontend-3.md).
  Agent overlay validation is still blocked by lack of configured LLM backend
  in the phase-3 fixture.
- 2026-05-17 @@Architect: created [webtest-3](./webtest-3.md) for remaining
  browser validation: Settings/Layout frontend wiring, Agent overlay find/Esc
  behavior with an assistant-enabled fixture if available, banner state-sync
  if feasible, and narrow viewport pass.
- 2026-05-17 @@Webtest: moved [webtest-3](./webtest-3.md) to REVIEW in practice.
  Settings/Layout and narrow viewport pass. Agent overlay Cmd+F/Esc and banner
  state-sync were not validated because the phase-3 fixture has no enabled LLM
  backend and `.assistant-shell` never mounts.
- 2026-05-17 @@Architect: created [architect-decision-1](./architect-decision-1.md)
  to decide whether to spend time enabling an assistant fixture for Agent
  overlay browser smoke or record the gap in [summary.md](./summary.md).
- 2026-05-17 @@Architect: resolved [architect-decision-1](./architect-decision-1.md)
  by choosing a bounded assistant-enabled validation attempt. Created
  [webtest-4](./webtest-4.md), pointing Webtest at the phase-1 fake Codex helper
  pattern and requiring Agent overlay Cmd+F/Esc, Cmd+I quote caret, and feasible
  banner state-sync smoke.
- 2026-05-17 @@Architect: created [architect-verify-1](./architect-verify-1.md)
  requiring at least one `scripts/pre-push` run before phase completion.
- 2026-05-17 @@Webtest: moved [webtest-4](./webtest-4.md) to REVIEW. Assistant-enabled
  Agent overlay smoke passed for mount, Cmd+F, Esc scoping, console loop
  absence, and Cmd+I quote caret placement. Enter/Shift+Enter over populated
  chat history and Claude-vs-Codex banner state sync were not fully exercised;
  record those as validation gaps unless Alex asks for a deeper smoke pass.
- 2026-05-17 @@Architect: started [architect-verify-1](./architect-verify-1.md)
  after accepting [webtest-4](./webtest-4.md) as sufficient to unblock the
  final pre-push gate.
- 2026-05-17 @@Architect: moved [architect-verify-1](./architect-verify-1.md) to
  REVIEW. `scripts/pre-push` passed: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, and
  `cargo build --no-default-features`.
- 2026-05-17 @@Architect: created [webtest-5](./webtest-5.md) for final service
  teardown or explicit keep-running handoff before writing [summary.md](./summary.md).
- 2026-05-17 @@Webtest: moved [webtest-5](./webtest-5.md) to REVIEW. Stopped
  Vite PID 40674 and chan-server PID 93853, confirmed ports 5173 and 8787 are
  unbound, reverted assistant enable/default config side effects, removed
  `/tmp/chan-phase3-logs`, and preserved `/tmp/chan-phase3-drive`.
- 2026-05-17 @@Architect: moved [webtest-1](./webtest-1.md) to REVIEW because
  browser smoke and final service teardown are complete.
- 2026-05-17 @@Architect: wrote [summary.md](./summary.md). Phase is ready for
  commit coordination; working tree remains dirty.

## Follow-ups

- Rerun [architect-verify-1](./architect-verify-1.md) only if later source
  changes land before final delivery.
- Commit coordination remains: review the dirty tree and commit in coherent
  units.
- Carry these known validation gaps into [summary.md](./summary.md) unless Alex
  asks for deeper smoke: Agent overlay Enter/Shift+Enter navigation over
  populated chat history, and Claude-vs-Codex banner state sync.
- Full cross-mode graph filter normalization remains deferred from
  [frontend-3.md](./frontend-3.md).
- Ask Alex for product direction only if "Assistant" remains an intentional
  protocol/config noun in any visible place after the compatibility audit.
