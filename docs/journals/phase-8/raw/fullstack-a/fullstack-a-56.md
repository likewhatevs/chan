# fullstack-a-56 — Cmd+P 3-state contract + depth-slider shallow-scope discoverability (bundled UX polish)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Two small UX papercuts bundled in one commit:

1. **Cmd+P 3-state contract** — fix the rich-prompt chord
   so it follows the active tab + supports toggle-to-hide
   on re-press.
2. **Depth slider discoverability on shallow scopes** —
   visual cue when slider max=1 already reveals
   everything forward-reachable.

## Reference

* Cmd+P bug: [`../phase-8-bugs.md`](../phase-8-bugs.md)
  "Cmd+P (rich prompt) doesn't honor the active-tab + doesn't toggle-to-hide on re-press"
  — full 3-state contract + audit-confirmed root causes
  + fix shape (single function rewrite of `showOrSpawnRichPromptInFocusedPane`)
  + 3 test pins.
* Depth slider observation: from `webtest-a-6` verdict
  (`cf383d8`) — "slider max can be misleading for shallow
  scopes (no visual cue that depth=1 already reveals
  everything forward-reachable)". Small visual cue (disable
  the slider OR show "[max]" label OR similar) when scope's
  depth-cap is hit at value 1.

## Acceptance

### Cmd+P 3-state

* Current tab IS a terminal + prompt NOT showing → open
  prompt on current terminal.
* Current tab IS a terminal + prompt IS showing → HIDE
  the prompt (toggle off).
* Current tab is NOT a terminal → spawn a fresh terminal
  + open prompt.
* 3 vitest pins per the bug body.

### Depth-slider discoverability

* When the scope's depth-cap is 1 (e.g. a single-file
  graph that has only one forward hop), the slider
  shows a clear visual that depth-1 already reveals
  the full scope. Implementer picks the cue — could be
  a `[max]` suffix label, disabling the slider when
  max=1, or a one-line caption.

### Gate

* `npm test -- --run` green.
* `npm run check` 0e/0w.
* `npm run build` clean.

## Coordination

* @@FullStackA lane. SPA-only.
* Atomic-audit-commit discipline per memory rule.

## Authorization

**Yes** for `web/src/state/tabs.svelte.ts`,
`web/src/components/GraphPanel.svelte`, the relevant test
files, plus the task tail + outbound.

## Numbering

`-a-55` is highest cut. This is `-a-56`.

## 2026-05-22 — ready for review

Three-file change (1 SPA state + 1 SPA component + 1
test patch + 1 new test). SPA-only; no Rust touched.

### Cmd+P 3-state (state 1, 2, 3 all addressed)

`web/src/state/tabs.svelte.ts`
`showOrSpawnRichPromptInFocusedPane()`:

* **Read p.activeTabId** instead of
  `p.tabs.find((t) => t.kind === "terminal")` — the
  PRE-`-a-56` shape always picked the first terminal
  in the pane regardless of which tab was active.
* **Case 1** (active terminal + prompt closed):
  call `openActiveTerminalRichPrompt()` (no
  activeTabId mutation needed; it reads p.activeTabId
  on its own).
* **Case 2** (active terminal + prompt open): set
  `activeTab.richPrompt.open = false` + early return.
  Pure toggle-off path; was missing pre-`-a-56`.
* **Case 3** (active NOT terminal): spawn a fresh
  terminal via `openTerminalInPane(p.id, {})` +
  `openActiveTerminalRichPrompt()`. Picked
  spawn-fresh over switch-to-existing per the task
  body's recommendation — "doesn't surprise the user
  with a tab-switch."

### Depth-slider shallow-scope cue

`web/src/components/GraphPanel.svelte`:

* New `depthShallow` `$derived.by` at the top-level
  state block (hoisted out of `{@const}` since
  `{@const}` can't sit inside a `<div>` parent).
  Gates on `!languageMode && !disabled && depthCap
  <= 1`.
* `.depth-row` template gains `class:shallow` +
  `title` attributes when shallow.
* `<input type="range">` gets `disabled={depthDisabled
  || depthShallow}` — no point dragging a slider that
  has nothing to reveal.
* `.depth-value` markup branches to render `<span
  class="depth-cue">[max]</span>` when shallow.
* CSS: `.tab-menu-bubble .depth-row.shallow
  .depth-value { width: auto }` (the existing
  `1.6em` width is too tight for the suffix); new
  `.depth-cue` rule with dimmer + smaller text.

### Tests

`web/src/components/cmdPRichPrompt3State.test.ts`
(new): 10 raw-source pins covering the 3-state
contract + the depth-shallow cue's $derived,
markup, and CSS.

`web/src/state/tabs.test.ts` existing pin
("focuses an existing terminal in the pane
(fullstack-50)") rewritten to match the new
spawn-fresh case-3 behaviour (`-a-56` replaces
the pre-existing focus-existing semantic).
Renamed + commented as fullstack-a-56 to
reflect the new contract.

### Gate

* vitest **732 / 732** (+10 net from `-a-62`'s
  722).
* svelte-check 0 errors / 0 warnings across
  3999 files.
* npm build clean.
* Rust gate not re-run.

### Decisions

* **Case 3 = spawn fresh** — per task body's
  "doesn't surprise the user with a tab-switch"
  framing. Switch-to-existing would silently move
  the user from their current (non-terminal) tab
  to a terminal they may not have intended to
  surface.
* **`$derived.by` over `{@const}`** for depthShallow
  — Svelte requires `{@const}` to be a direct child
  of `{#if}` / `{#each}` / etc., not a `<div>`. The
  derived also keeps the computation visible
  alongside `depthCap` (next-line declaration).
* **Disable slider when shallow** — not just visual
  cue; the slider can't meaningfully move so making
  it interactive would be misleading.

### Suggested commit subject

```
Cmd+P 3-state contract + depth slider shallow-scope cue (fullstack-a-56)
```

Single commit. Two small UX papercuts bundled
per task body.

### Files for `git add` (per-path discipline)

* `web/src/state/tabs.svelte.ts`
* `web/src/components/GraphPanel.svelte`
* `web/src/state/tabs.test.ts`
* `web/src/components/cmdPRichPrompt3State.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-56.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only; the
working tree carries unrelated WIP from other lanes
(`docs/journals/phase-8/alex/event-ci-architect.md`,
`docs/journals/phase-8/ci/ci-14.md`, etc.) that
must NOT be swept into this commit.

Push held. Standing by for clearance.
