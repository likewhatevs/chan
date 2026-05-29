# Chan Pre-Release Phase 3 Summary

Owner: @@Architect.

Status: ready for commit coordination.

## Outcome

Phase 3 implementation, review, browser smoke, final service teardown, and one
full pre-push gate are complete.

The work is not committed yet. The repository still has the phase source
changes and coordination files in the working tree.

## Highlights

- Visible Assistant terminology moved to Agent across the main UI surfaces.
- Agent overlay gained session-scoped Cmd+F, scoped Esc behavior, correct
  Cmd+I quote caret placement, and real backend-specific banner rendering.
- Status-bar click routing, dashboard shell, and URL state for key overlays
  landed.
- Settings layout is now `standard | compact`, with `standard` default and
  legacy `tight` normalized to `compact`.
- Document find, File Browser find, File Browser context-menu positioning,
  path prompt Tab completion, new-file completion, nested-list indenting,
  stale selection defense, and image/list guide behavior were fixed.
- Resource colors and graph scope/filter improvements landed, including
  centralized resource classification and parent/common-ancestor scope options.
- Backend/Rust config, CLI, preferences, and graph audits are complete.
- Webtest services were stopped; ports 5173 and 8787 are free.

## Bugs Fixed

- CODEx banner showing on a Claude conversation due to frontend state-sync.
- File Browser find next/previous stuck on the first match.
- Esc in overlay find bars bubbling and closing the whole overlay.
- Agent find effect loop risk.
- Cmd+I quote prompt caret landing at the start of the quote.
- File Browser right-click menu appearing far from the click.
- Editor stale native selection rectangles around image/list blocks.
- Image-bearing list guide bars stretching across image height.
- Long wrapped nested list lines de-indenting.
- Document find Enter not placing caret at the beginning of the matched word.
- Path prompt Tab completion requiring Enter instead of normal Tab flow.

## Validation

- `scripts/pre-push` passed on 2026-05-17:
  - `cargo fmt --check`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test --all-targets`
  - `cargo build --no-default-features`
- Frontend lanes recorded clean `cd web && npm run check` and
  `cd web && npm test -- --run` runs during implementation.
- Browser smoke covered desktop and narrow viewports, File Browser workflows,
  settings layout persistence, dashboard, graph/search URL state, resource
  colors, image-guide behavior, context menu placement, Agent overlay mount,
  Agent Cmd+F open/Esc close, and Cmd+I quote caret placement.

## Lowlights

- Agent overlay browser validation needed late fixture/config work because the
  phase-3 drive started with no enabled LLM backend.
- The same physical agent slot was briefly addressed as both @@Syseng and
  @@FrontendB; this was corrected in task files.
- Several journal follow-ups went stale during rapid handoffs and needed a
  final cleanup pass.
- `cargo test --all-targets` appears to write a `assistant.claude_cli.cmd`
  value into the live user config, pointing at a test artifact. This predates
  the phase but was observed again during teardown.

## Remaining Follow-Ups

- Commit coordination: review the dirty tree and commit in coherent units.
- Agent overlay deeper smoke: Enter/Shift+Enter navigation over populated chat
  history was not fully exercised.
- Banner state-sync deeper smoke: Claude conversation vs global Codex selector
  was not fully exercised in browser.
- Graph: full cross-mode filter normalization and markdown-mode synthetic
  folder/path nodes remain deferred from [frontend-3.md](./frontend-3.md).
- Decide whether the Settings Agent section should expose an in-UI enable
  toggle for backends.
- Investigate test pollution of live `assistant.claude_cli.cmd` config.
- Product audit: only ask Alex if any remaining visible "Assistant" string is
  intentional protocol/config compatibility rather than user-facing copy.

## Agent Review

1. @@Frontend: high output and broadest product impact. Landed the main UX
   surface area quickly: Agent rename, dashboard, URL state, banners,
   status-bar routing, resource colors, graph scope work, editor/file-browser
   fixes. Main weakness was overloading one lane with too many independent
   slices before validation had caught up.
2. @@Syseng: strongest late-phase closer. Picked up focused frontend fixes,
   resolved BUG-FE2-A, Esc propagation, Agent quote caret, image-guide residuals,
   and Settings layout wiring with clean tests. Good discipline around scoped
   fixes.
3. @@Webtest / @@WebtestB: effective at finding real UI regressions and keeping
   validation concrete. The BUG-FE2-A report and image-guide repro were useful.
   Fixture setup for Agent validation lagged, but Webtest-4 and teardown closed
   the operational loop.
4. @@Backend+Rustacean: solid backend/Rust execution and review. Kept backend
   changes narrow, added layout config compatibility, and avoided unnecessary
   API churn after auditing graph/status capabilities.
5. @@Architect: coordination recovered from role churn and kept work moving
   through task files. Improvement for next phase: create explicit role-change
   tasks earlier and prune stale journal follow-ups continuously, not just at
   the end.

## Final State

- All implementation/review task files are REVIEW.
- Webtest services are stopped.
- Pre-push gate passed.
- Known gaps are documented above.
- Working tree remains dirty and ready for commit coordination.
