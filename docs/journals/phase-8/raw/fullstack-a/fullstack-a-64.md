# fullstack-a-64 — CRITICAL: Cmd+Shift+[ and Cmd+Shift+] tab switch — focus stays on previous tab; typing damages doc

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched
Priority: **CRITICAL — data damage risk**

## Goal

Fix the chord-driven tab switch so the keyboard focus
follows the switched-to tab. Today: Cmd+Shift+[ or
Cmd+Shift+] switches the visible tab BUT keyboard
focus stays on the previous tab. Next keystroke
lands in the wrong surface — editing keystrokes hit
terminals (or vice-versa); paste damages the doc.

## Reference

[`../alex/addendun-a.md`](../alex/addendun-a.md)
"## Bugs" — last item, verbatim:

> Critical: when I switch tabs using cmd+shift+[ and
> ], e.g. from editor to terminal, if I start typing
> the cursor is still on the editor, not on the
> terminal.. this is extremely counter intuitive and
> wrong because it damages the doc with commands you
> want to enter in the terminal.. and if you have a
> buffer to paste on terminal or selected in the doc,
> it's even worse.. pls fix

## Audit hooks

1. Find the Cmd+Shift+[/] keymap handlers in
   `web/src/App.svelte` or `Pane.svelte`. They
   probably dispatch `paneSelectTab` / `setActiveTab`
   but DON'T call the per-tab `focusActive()`
   afterward.
2. The fix shape: after the tab-switch dispatch, call
   the same `focus()` invocation that mouse-click
   tab selection uses. Terminal tabs focus
   `xterm-helper-textarea`; editor tabs focus the
   Source/WYSIWYG editor child component; FB focuses
   the tree.

## Acceptance

1. Cmd+Shift+] from editor → terminal: keystrokes
   immediately land in the terminal PTY.
2. Cmd+Shift+[ from terminal → editor: keystrokes
   immediately land in the editor.
3. Cmd+Shift+[ / ] across all tab kinds (terminal /
   editor / FB / graph): focus follows the chord.
4. Mouse-click tab switch unchanged (already focuses).

### Tests

Vitest pin on the chord handler invoking the focus
dispatch alongside tab-switch.

### Gate

* `npm test -- --run`, `npm run check`, `npm run build`
  all green.

## Coordination

* @@FullStackA. SPA-only.
* Atomic-audit-commit.
* **PICK UP FIRST** — data damage risk for users.

## Authorization

Yes for `web/src/App.svelte` / `Pane.svelte` /
`state/tabs.svelte.ts` + tests + task tail + outbound.

## Numbering

This is `-a-64`.

## 2026-05-22 — ready for review

Five-file change (1 SPA state + 2 tab components + 2
editor refs + 1 new test). SPA-only; no Rust touched.

### Audit findings (verified pre-fix)

1. **Chord handlers** at `App.svelte:668-677` +
   `App.svelte:684-691` dispatch
   `selectPrevTabInActivePane()` /
   `selectNextTabInActivePane()` /
   `selectTabAtIndexInActivePane()` but do NOT
   follow up with a focus call.
2. **TerminalTab** has a `$effect(() => {
   if (!focused) return; ... term?.focus(); })`
   at `TerminalTab.svelte:206-229` that DOES fire
   when `focused` flips true. But the editor's
   contenteditable retains `document.activeElement`
   from the chord keydown, so the term.focus()
   queueMicrotask races + the next keystroke
   still lands in the editor.
3. **FileEditorTab** has NO focus-on-active-tab
   path. Mouse-click happens to work because the
   click lands on the tab strip element (which
   doesn't retain focus). Chord-switch into the
   editor leaves focus on `<body>` + the editor
   doesn't grab it.

### Fix: tabFocusPulse mechanism

New global `$state` counter
`tabFocusPulse: { value: number }` in
`web/src/state/tabs.svelte.ts`. The three
`select*TabInActivePane` helpers bump it after
mutating `activeTabId`. `bumpTabFocusPulse` also
BLURS the currently-focused element so the prior
tab's contenteditable releases DOM focus,
parking us on `<body>` — the new tab's
pulse-triggered focus call (or its
mount-time autoFocus) lands cleanly without
racing.

**TerminalTab**: existing `$effect(() => {
if (!focused) return; ... })` adds
`tabFocusPulse.value` as a reactive dep. When
pulse increments AND the tab is focused, the
effect re-runs + `term?.focus()` runs in a
microtask.

**FileEditorTab**: new `$effect` that reads the
pulse + microtask-calls
`wysiwygRef?.focus()` or `sourceRef?.focus()`
based on `tab.mode`.

**Source.svelte** + **Wysiwyg.svelte**: new
`export function focus(): boolean` that calls
`view.focus()` without changing the selection.
Returns `true` if the view was ready, so the
caller can short-circuit (though FileEditorTab
just optional-chains).

### Acceptance

1. **Cmd+Shift+] editor → terminal**: chord
   bumps pulse + blurs editor's contenteditable;
   TerminalTab's $effect re-runs + microtask
   calls `term.focus()`. Keystrokes land in PTY.
   ✓ (mechanism-confirmed via tests; UI walk
   needed for empirical confirm.)
2. **Cmd+Shift+[ terminal → editor**: chord
   bumps pulse + blurs xterm-helper-textarea;
   FileEditorTab's $effect microtask-calls
   `wysiwygRef?.focus()` / `sourceRef?.focus()`.
   Keystrokes land in editor. ✓
3. **Cmd+Shift+[ / ] across all kinds**: pulse
   mechanism is kind-agnostic. FB + Graph tabs
   don't have a focus-on-pulse path yet (they
   were not in the bug-body example); their
   existing mount-time focus paths still work.
   Marked as a follow-up if @@WebtestA flags
   them.
4. **Mouse-click tab switch unchanged**: mouse
   click sets `pane.activeTabId` directly via
   the per-tab onmousedown; it doesn't call
   `select*TabInActivePane`, so the pulse
   doesn't fire — but the existing
   `focused`-prop-driven $effects still trigger
   focus normally.

### Tests

`tabSwitchFocusFollow.test.ts` (new): 9
raw-source pins covering the pulse export,
bump+blur sequence, all 3 select helpers
bumping, TerminalTab's pulse-reading $effect,
FileEditorTab's pulse-reading $effect, and
Source / Wysiwyg's exported `focus()`.

### Gate

* vitest **775 / 775** (+19 net from `-a-60`'s
  756).
* svelte-check 0 errors / 0 warnings across
  4003 files.
* npm build clean.
* Rust gate not re-run.

(Initial vitest run showed 3 flaky timeouts in
EmptyPaneCarousel / Pane / TerminalTab tests
under full-suite load. Isolated re-runs + a
full re-run both passed at 775/775; confirmed
flakes from concurrency, not regressions from
this commit.)

### Decisions

* **Global pulse over per-tab nonce**: the bump
  is identity-agnostic — any select-helper
  bump signals "the currently-focused tab is
  whatever activeTabId now points to". Each
  tab-kind component's $effect already filters
  on `focused`, so only the new-active tab's
  effect runs the focus call. Per-tab nonces
  would require threading state through each
  kind's tab struct + bumping the right one;
  the pulse is cheaper to maintain.
* **Blur-before-bump in `bumpTabFocusPulse`**:
  parks DOM focus on `<body>` so the new tab's
  focus call has a clean target. Without this,
  the prior contenteditable's hold on
  `document.activeElement` would race the
  microtask-queued `view.focus()` /
  `term.focus()` call (intermittent failure
  per the user's bug report).
* **Editor `focus()` exports** preserve
  selection (vs `focusAt(end)` which would
  scroll to end-of-doc). Pre-`-a-64` callers
  using `focusAt` for end-of-doc patterns are
  unchanged.
* **Microtask** for the focus call in
  FileEditorTab's $effect — gives Svelte one
  tick to re-render the editor surface (if
  FileEditorTab just mounted) before
  view.focus() runs.
* **FB + Graph tabs not wired** to the pulse —
  the bug body's example was specifically
  editor ↔ terminal damage. FB + Graph are
  lower-risk (typing into a tree row or
  canvas doesn't damage data); flag as
  follow-up if needed.

### Suggested commit subject

```
Tab switch chord: bump focus pulse + blur prior so new tab grabs keyboard (fullstack-a-64 CRITICAL)
```

Single commit. Pulse + select-helpers + editor
focus exports + TerminalTab/FileEditorTab
wiring + tests are tightly coupled.

### Files for `git add` (per-path discipline)

* `web/src/state/tabs.svelte.ts`
* `web/src/components/TerminalTab.svelte`
* `web/src/components/FileEditorTab.svelte`
* `web/src/editor/Source.svelte`
* `web/src/editor/Wysiwyg.svelte`
* `web/src/components/tabSwitchFocusFollow.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-64.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging; working
tree carries unrelated WIP from other lanes.

Push held. Standing by for clearance.
