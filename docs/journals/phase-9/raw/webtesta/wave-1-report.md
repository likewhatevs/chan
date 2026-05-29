# WebtestA Wave 1 report

Author: @@WebtestA
For: @@Architect
Date: 2026-05-23

## Scope

Lane: live browser and visual regression testing for terminal, editor,
search, markdown, and list bugs from `roadmap-round1.md`.

Current limitation: the Browser plugin is installed, but this session has no
`iab` browser instance. `agent.browsers.list()` returned `[]`, and
`agent.browsers.get("iab")` failed with `Browser is not available: iab`.
No long-running chan server was started.

## Repro status

| Bug | Current-main status | Evidence | Suspected owner/module |
| --- | --- | --- | --- |
| Terminal fonts not rendering after switching tabs | Not live-reproduced. Static coverage for font chain and tab-focus exists and passes. | `TerminalTab.font.test.ts` and `tabSwitchFocusFollow.test.ts` pass. | Frontend: `web/src/components/TerminalTab.svelte`, `web/src/components/Pane.svelte`, `web/src/state/tabs.svelte.ts`. |
| Terminal cannot render characters, e.g. em dash | Not live-reproduced. Static coverage checks WebGL renderer wiring, font chain, atlas refresh on SGR sequences. No pixel/glyph screenshot possible here. | `TerminalTab.renderer.test.ts` and `TerminalTab.font.test.ts` pass. | Frontend: xterm setup in `TerminalTab.svelte`; possible runtime WebGL/context-loss/font fallback issue. |
| Tab switching and terminal/editor focus | Static coverage passes. No live click/typing repro. | `tabSwitchFocusFollow.test.ts` passes. | Frontend: `Pane.svelte` tab mousedown, `TerminalTab.svelte`, `FileEditorTab.svelte`, editor focus exports. |
| Search or another path mutates the edited file | Not reproduced in WebtestA tooling. Needs live drive with before/after content hashes while invoking SearchPanel and index updates. | No server/browser run. | Cross-lane: Frontend save/applyExternal path in `FileEditorTab.svelte` and `editor/base.ts`; backend watcher/indexer likely Systacean owns root cause. |
| Bullet lists indenting like `-` lists do | Static list command and list-decoration coverage passes. No visual screenshot for indentation. | `list.test.ts`, `blocks.test.ts`, `blocks.list_trigger.test.ts` pass. | Frontend editor: `web/src/editor/commands/list.ts`, `web/src/editor/decorations/blocks.ts`, WYSIWYG CSS list guide rules. |
| `[[` search yields no results while Search shows results | Not live-reproduced. Static read finds different search surfaces: `[[` uses filename autocomplete `/api/search/files`; global Search uses content search `/api/search/content` plus graph-derived rows. This can explain "Search shows results, `[[` does not" when the match is in file content but not in path. | Source read: `openWikiBubble()` calls `api.search()`, client maps that to `/api/search/files`; `SearchPanel.svelte` uses content and graph paths. | Frontend/product boundary: `web/src/editor/bubbles/wiki.ts`, `web/src/api/client.ts`, `web/src/components/SearchPanel.svelte`; server filename route in `crates/chan-server/src/routes/search.rs`. |
| Markdown `---` rendering as `<hr>` unexpectedly | Not live-reproduced. Static read confirms WYSIWYG intentionally decorates parsed HorizontalRule as a visual HR when caret is off the line and reveals raw `---` only when caret intersects the line. | Source read: `handleHorizontalRule` in `web/src/editor/decorations/blocks.ts`; toolbar `insertHorizontalRule()` inserts `---`. | Frontend editor decorations: `web/src/editor/decorations/blocks.ts`; possible product decision if desired behavior changed. |
| Scroll fighting when cursor at bottom and user scrolls up | Not live-reproduced. Existing image-load scroll restore test passes, but roadmap repro is general bottom-cursor scroll fight and may be separate from image load. | `imageScrollCaretLost.test.ts` passes; source read shows image load dispatches `EditorView.scrollIntoView(head, { y: "nearest" })` only when caret is off-screen. | Frontend editor: CodeMirror scroll effects in `web/src/editor/widgets/image.ts`, `web/src/editor/base.ts`, `web/src/editor/breathing_room.ts`, `FileEditorTab.svelte` focus/selection effects. |

## Tests run

```bash
npm run test -- src/components/TerminalTab.font.test.ts src/components/tabSwitchFocusFollow.test.ts src/editor/commands/list.test.ts src/editor/decorations/blocks.test.ts src/editor/decorations/blocks.list_trigger.test.ts src/search/results.test.ts src/components/editorBugBundle.test.ts
```

Result: 7 files passed, 76 tests passed.

```bash
npm run test -- src/components/TerminalTab.renderer.test.ts src/editor/widgets/imageScrollCaretLost.test.ts src/editor/commands/format.test.ts src/editor/find.test.ts src/editor/bubbles/empty_state.test.ts src/components/BubbleOverlay.test.ts
```

Result: 6 files passed, 49 tests passed.

## Proposed fix order

1. Terminal glyph/font visual repro first once browser/server is available.
   It is user-visible, screenshot-driven, and likely sensitive to runtime
   renderer/font loading rather than unit logic.
2. Editor scroll fight next. Add a live repro against a long note before
   touching scroll effects; source has multiple legitimate scroll paths.
3. `[[` search behavior. Decide whether `[[` should remain filename-only or
   include content/global Search results; then align tests to that product
   call.
4. Markdown `---`. Decide whether WYSIWYG should render HR off-caret or keep
   raw markdown visible. Current source behavior looks intentional.
5. Bullet/list visual indentation. Existing command/decorator tests pass, so
   live screenshot/CSS measurement should precede changes.
6. Search/edit interference. Needs Systacean root cause first if watcher/index
   writes are involved; WebtestA can supply before/after browser repro after
   server is up.

## Required live baseline seed

Use a throwaway drive, e.g. `/tmp/chan-test-phase9-webtesta`, with:

- `terminal-glyphs.md`: reference text for commands to paste into terminal.
- `editor-long-scroll.md`: 300+ lines, cursor-bottom scroll repro.
- `markdown-lists-hr.md`: `-`, `*`, nested list, task list, ordered list,
  and standalone `---` cases.
- `wikilink-targets/alpha.md`, `wikilink-targets/beta.md`,
  `content-only-hit.md` with a unique token in body but not path.
- `bulk/` with 500+ small markdown files for search/index churn.

## Pass/fail evidence format

For each live rerun:

- build SHA and command used to launch server
- browser/tool surface and viewport
- exact repro steps
- screenshot for visual failures
- file hash/content diff for mutation claims
- console/server stderr excerpt for runtime failures

## Files changed

- `docs/journals/phase-9/webtesta/wave-1-report.md`

## Root cause summary

No live root cause is claimed. Static suspects:

- terminal glyphs: runtime WebGL/font path in `TerminalTab.svelte`
- scroll fight: CodeMirror scroll/focus effects in editor modules
- `[[` mismatch: filename autocomplete versus content/global search split
- `---`: intentional HR decoration while caret is off-line
- list indentation: likely CSS/visual layer, not list command behavior

## Behavior changed

None. Report-only change.

## Known gaps

- No in-app browser available.
- No chan server launched.
- No visual screenshots captured.
- No live file mutation/hash repro captured.

## Recommended commit boundary

Do not commit this alone unless @@Architect wants journal-only coordination
commits. Keep it as a report artifact, separate from implementation fixes.
