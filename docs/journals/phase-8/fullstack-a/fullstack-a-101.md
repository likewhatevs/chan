# fullstack-a-101 — Tab-click focus on terminal + editor tabs (v0.13.0 release blocker)

Owner: @@FullStackA
Phase: 8, Round 3
Date cut: 2026-05-23
Priority: P1 — release blocker for v0.13.0; small fix (~5-20 LOC)

## Goal

When the user clicks a terminal tab header, the terminal content should receive keyboard focus so the user can start typing immediately. Same for editor tab headers. @@Alex flagged 2026-05-23 (`docs/journals/phase-8/alex/round4.md` bug 2): "Clicking the tab of a terminal tab must place the cursor/focus on the terminal itself, ready to type; same for editor tabs."

## Background

Standard tab-UX expectation: clicking a tab header makes the tab active AND focuses its content area. Today the click only activates the tab but doesn't shift keyboard focus into the tab content — user has to click into the terminal / editor body to start typing.

This is a 5-LOC class of bug (find the tab-click handler, call `.focus()` on the right element after the activate happens, on next tick if needed).

## Scope

### Terminal tab

`web/src/components/TerminalTab.svelte` (or wherever the xterm.js mount happens). The xterm.js `Terminal` instance has a `.focus()` method.

* Find the tab-header click handler that calls `activateTab(...)` or similar.
* After activation lands (likely on next `tick()`), call `terminal.focus()` on the xterm instance.

### Editor tab

`web/src/components/EditorTab.svelte` (or wherever the contenteditable / Lexical / CodeMirror instance lives).

* Same shape: tab-header click activates the tab; after activation, focus the editor content surface.
* Editor focus might mean: focus the contenteditable root, or call the editor framework's `.focus()` API.

### Shared pattern

If the tabs route through a common `activateTab(paneId, tabId)` or chord handler, the focus call may be addable in one place with a per-kind branch (terminal vs editor vs FB vs graph vs infographics). For non-content-input tabs (FB, graph, infographics), focus may stay on the tab header — that's fine; focus only the input-able kinds.

## Acceptance criteria

1. **Click terminal tab → cursor focuses the terminal content; user can type immediately.**
2. **Click editor tab → cursor focuses the editor content; user can type / edit immediately.**
3. **No regression on other tab kinds** (FB / graph / infographics) — they don't have a meaningful "content focus" target; focus behavior unchanged.
4. **Test pins**: vitest assertion that the tab-click handler calls the focus method on the right element for terminal + editor.
5. **Gate**: `npm run check + npm test + npm run build` green.

## How to start

1. Grep for the tab-click handler: `grep -n "activateTab\|onTabClick\|setActiveTab" web/src/components/*.svelte`.
2. Trace from a tab header click through the activate logic.
3. Add focus call after activation (on next tick if Svelte's reactive cycle needs it).
4. Pin with a vitest.

## Coordination

* Safety guardrail: do NOT touch @@Alex's running chan.app session. Use throwaway drives + dev builds.
* Small fix; standalone task. Doesn't depend on `-98`/`-99`/`-100`.

## Authorization

Yes for SPA-side edits + vitest pins.

## Out of scope

* Reworking the tab-activation pipeline.
* Auto-focus on tab CREATION (the user already focuses the tab; we're handling re-focus on click).
* Focus persistence across pane flips or window switches.

## Reference

* `docs/journals/phase-8/alex/round4.md` bug 2.

---

## 2026-05-23 - candidate fix ready: tab header clicks refocus terminal/editor content

Implemented the small shared fix by reusing the existing
`tabFocusPulse` path from `fullstack-a-64`:

* Exported `bumpTabFocusPulse()` from `tabs.svelte.ts`.
* `Pane.svelte` tab-header `mousedown` now selects the tab,
  clears terminal activity as before, then bumps the pulse for
  input-capable tab kinds only (`terminal` and `file`).
* Existing `TerminalTab` and `FileEditorTab` pulse effects perform
  the actual focus work (`term.focus()`, `wysiwygRef.focus()`, or
  `sourceRef.focus()`), so this avoids a second focus pipeline.
* Raw-source pins added in `tabSwitchFocusFollow.test.ts`.

Verification:

* `npm test -- --run src/components/tabSwitchFocusFollow.test.ts src/components/Pane.test.ts`
  - 2 files passed, 31 tests passed.
* `npm run check`
  - svelte-check 0 errors / 0 warnings.
* `npm test -- --run`
  - one parallel full-suite run hit 3 unrelated 15s UI-test
    timeouts (`EmptyPaneCarousel.test.ts`, `TerminalTab.test.ts`,
    `Pane.test.ts`).
* `npm test -- --run src/components/EmptyPaneCarousel.test.ts src/components/TerminalTab.test.ts src/components/Pane.test.ts`
  - targeted rerun passed, 3 files / 26 tests.
* `npm run build`
  - passed; existing chunk-size / ineffective dynamic import
    warnings only.

## 2026-05-23 — @@Architect: approved + commit clearance (shipped: c53cb6c)

Clean reuse of the existing `tabFocusPulse` infrastructure from `-a-64` — that's the right shape (no parallel pipeline, one focus path with branches for input-capable kinds). Exporting `bumpTabFocusPulse()` from `tabs.svelte.ts` + bumping it from `Pane.svelte`'s `mousedown` handler is the minimal-surface change.

Verified clean (2 files / 31 tests in targeted run; svelte-check 0/0; full-suite passed apart from the known parallel-load UI-test timeouts that you correctly rebooked via targeted reruns).

Code already shipped at `c53cb6c` (committed under your pre-authorization). This append is documentation-only.

### Lane state post-`-101`

| Task | Status |
|---|---|
| `-97` (terminal glyph) | ✓ shipped + HOLD walk |
| `-98` (menu gaps) | ✓ shipped (`dec62ff`); pending @@WebtestA walk |
| `-101` (tab focus) | ✓ shipped (`c53cb6c`); pending @@WebtestA walk |
| `-96` sub-passes 1/2/3 (polish) | cleared, non-blocking |
| `-99` (screensaver themes; +timeout bounds) | open |
| `-100` (Drafts chain) | open (P0) |

Thank you for the `tabFocusPulse` reuse instinct — exactly the right call vs writing a second pipeline.
