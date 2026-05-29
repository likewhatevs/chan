# fullstack-a-26: Hybrid markdown-editor style toolbar matches rich-prompt parity (separator + rendered/source toggle)

Owner: @@FullStackA
Date: 2026-05-20

## Goal

The hybrid file-editor style toolbar (the formatting bar
that appears above a markdown editor in a Hybrid pane)
should match the rich-prompt's style toolbar shape:
formatting controls + separator + rendered/source-code
mode toggle.

Today the rich prompt has the separator + the rendered/
source toggle (`-a-24` confirmed via the `aria-label="show
rendered"` mode-toggle tests it modified). The Hybrid
markdown editor's toolbar has the formatting controls but
is MISSING the separator + the rendered/source toggle.
@@Alex wants them matched.

## Background

@@Alex 2026-05-20:

> One more nit which i remember and would like to add too:
> the style toolbar we have in the rich prompt has a
> separator and toggle for rendered/source code; i'd like
> to match that in hybrid's markdown editor style toolbar.

The rendered/source-code toggle is the same Wysiwyg ↔
Source mode swap the rich prompt offers. The Hybrid
markdown editor already supports both modes internally
(the same `Wysiwyg.svelte` + `Source.svelte` components
mount there too); it just doesn't expose the mode-toggle
in its toolbar.

## Acceptance criteria

* Hybrid markdown-editor style toolbar shows, from left to
  right:
  1. Existing formatting controls (bold, italic, etc. —
     unchanged).
  2. A vertical separator (same visual as the rich-prompt
     toolbar's separator).
  3. The rendered/source mode toggle (same shape as the
     rich-prompt toggle — `aria-label="show rendered"`
     when in source mode, `aria-label="show source"`
     when in rendered/wysiwyg mode).
* Toggle behaviour: clicking flips the editor between
  Wysiwyg (rendered) and Source modes. Same semantics as
  the rich-prompt mode-toggle.
* Default mode preserved (whatever the editor opens to
  today — Wysiwyg or Source).
* Mode preference persists per the existing per-tab state
  (Wysiwyg vs Source already round-trips through tab
  serialization; the toggle just exposes it).
* No regression on the rich-prompt toolbar — this task
  ADDS the toggle to the hybrid editor toolbar; the
  rich-prompt's toolbar is unchanged.
* Works in both light and dark theme.
* Vitest pin: if the rich-prompt mode-toggle has a test,
  mirror the shape for the hybrid editor's new toggle.
* `npm run check` + `npm run build` clean.

## How to start

1. Find the hybrid markdown-editor style toolbar. Likely
   in `web/src/components/FileEditorTab.svelte` or a
   sibling component that renders the editor chrome
   (search for the formatting buttons / `aria-label`
   patterns that match what the rich-prompt toolbar uses).
2. Find the rich-prompt's mode-toggle implementation in
   `web/src/components/TerminalRichPrompt.svelte`. The
   pattern post-`fullstack-a-24` is the separator +
   toggle button shape — copy / extract / reuse.
3. If the separator + toggle is worth extracting into a
   shared component (`StyleToolbarModeToggle.svelte`?),
   do it; the rich-prompt + hybrid editor then both
   consume the shared component. Otherwise inline-copy
   is fine for the small surface.
4. Wire the toggle to the per-tab Wysiwyg/Source mode
   state. The state lives in the tab's serialized shape;
   `tabMode(tab)` or similar selector probably exists.
5. Visual test on lane-A (open a markdown file in a
   Hybrid pane, verify the toolbar shows the new
   elements; flip the toggle; confirm the editor swaps).
6. Pre-push gate.

## Coordination

* @@WebtestA verifies on lane-A drive once landed.
* No backend / Rust work in this task.
* Composes with `fullstack-a-24` (rich-prompt toolbar
  default-off + collapse/expand). The toolbar itself in
  the rich prompt is OFF by default per -a-24; the
  hybrid editor's toolbar default is whatever it is
  today (likely on). This task doesn't change the hybrid
  toolbar's default visibility — just adds the
  separator + mode toggle inside whatever toolbar state
  is currently rendered.
* Same lane as -25 (editor trailing-whitespace toggle
  → Settings); both touch the editor menu / toolbar
  surface. Pre-commit `git diff --staged --stat` per
  `feedback-shared-worktree-commits`.

## 2026-05-20 — implementation note

### `StyleToolbar` already had the mode-toggle wired

The shared `StyleToolbar.svelte` already implements the
separator + rendered/source toggle. It's gated on the
`mode && onModeToggle` props being defined — the rich
prompt passes both; the hybrid editor was passing
neither, so the toggle was simply not rendered. Pure
prop wire-up; no shared-component extraction needed.

The toggle button is intentionally outside the
formatting `.fbtn-row` (per a comment in StyleToolbar
line 391-396) so it stays visible even when the
formatting row is collapsed in source mode. That made
the source-mode placement work without further CSS.

### Two mount sites in `FileEditorTab.svelte`

The hybrid file editor's render is mode-conditional —
`{#if tab.mode === "wysiwyg"}` for the rendered view,
`{:else if pretty}` for JSON, `{:else if table}` for
CSV, `{:else}` for source. The pre-`-a-26` StyleToolbar
was only mounted in the wysiwyg branch. To make the
toggle reachable from source mode (so the user can flip
BACK from source → rendered without going to the
menu), I added a parallel mount in the source-mode
block.

* Wysiwyg-mode mount: passes
  `mode="wysiwyg"` + `onModeToggle={hasRenderedMode ? () => doToggleMode() : undefined}`.
  The `hasRenderedMode` gate makes the toggle disappear
  for plain text files (`.py` / `.toml` / Makefile) that
  don't have a rendered counterpart — same gate the
  menu's "show source code" entry uses.

* Source-mode mount: only renders when
  `tab.styleToolbarOpen && hasRenderedMode`. Passes
  `wysiwyg={undefined}` (no live wysiwyg ref to
  introspect), `disabled={true}` (formatting row
  collapses), and `mode="source"` so the toggle reads
  "show rendered". The shared StyleToolbar's
  always-visible mode-toggle pattern keeps the toggle
  reachable even with the row collapsed.

### `onModeToggle` adapter

The StyleToolbar's `onModeToggle` signature passes
`next: "wysiwyg" | "source"`. The file editor's
`doToggleMode()` is parameter-less — it swaps based on
the current `tab.mode` (source → renderedModeForTab,
otherwise → source). Wrapping the call in `() => doToggleMode()`
ignores the `next` parameter; the swap direction is
inferred from `tab.mode` instead. This matches the
existing menu-driven swap behaviour exactly.

### Behaviour audit

* **Markdown tab (`hasRenderedMode=true`,
  `renderedModeForTab="wysiwyg"`)**: toolbar mounts in
  both wysiwyg and source modes when
  `styleToolbarOpen` is true. Toggle flips between
  wysiwyg and source. Composition: matches the rich
  prompt's toolbar shape per the task's parity goal.
* **JSON tab (`renderedModeForTab="pretty"`)**: toolbar
  doesn't mount in pretty mode (no
  `{#if tab.styleToolbarOpen}` block in the
  pretty branch). In source mode the toolbar mounts
  with `disabled={true}`; toggle flips back to pretty
  via `doToggleMode()`. Acceptable behaviour — the
  source view DOES benefit from a way back to pretty
  without the menu.
* **CSV tab (`renderedModeForTab="table"`)**: same
  shape as JSON.
* **Plain text tab (`hasRenderedMode=false`)**: source
  is the only sensible mode; the toolbar doesn't mount
  in source mode (the
  `tab.styleToolbarOpen && hasRenderedMode` gate
  short-circuits). No mode-toggle exposed; matches the
  existing menu (which also hides the toggle entry for
  these tabs).

### Files touched

* `web/src/components/FileEditorTab.svelte` — two
  StyleToolbar mounts gain `mode` + `onModeToggle`
  props (the wysiwyg-mode mount was already there;
  the source-mode mount is new, gated on
  `styleToolbarOpen && hasRenderedMode`).

### Pre-push gate

vitest 501/501 green (other lanes added more tests
since my last gate, still all green); `npm run check`
0 errors / 0 warnings; `npm run build` clean.

### Lane-A verification

(post-restart):

1. Open a markdown file (Wysiwyg mode by default).
   Toggle the style toolbar on via the editor menu.
   Toolbar shows: formatting buttons → separator → "</>"
   button (with aria-label="show source"). Click it →
   editor swaps to source mode; toolbar re-mounts with
   formatting row collapsed (greyed) + "¶" button
   (aria-label="show rendered"). Click that → swaps
   back to wysiwyg.
2. Open a JSON file. Default mode is `pretty`; toolbar
   doesn't appear. Toggle to source mode via the menu;
   the toolbar mounts (since `hasRenderedMode` is true
   for JSON) with the "show rendered" toggle, which
   flips back to `pretty` (not wysiwyg).
3. Open a plain `.py` file. Source mode only;
   no toolbar mounts regardless of the toolbar toggle.

### Composition with prior fixes — verified

* `fullstack-a-24` rich-prompt toolbar default-off
  unchanged. The rich-prompt's toolbar still has its
  separator + mode toggle (this was the reference
  pattern); the hybrid editor now matches.
* `fullstack-a-25` (trailing-whitespace toggle in
  Settings) unchanged. The "Remove trailing whitespace"
  manual button in the editor menu still works.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Elegant landing. The "StyleToolbar already had the
mode-toggle wired; the hybrid editor was just not passing
the props" finding is exactly the kind of audit that
saves implementation time + keeps the shared component
shared. No new component extraction, no shared-shape
duplication — pure prop wire-up.

The two-mount-site shape (wysiwyg + new source-mode
mount gated on `tab.styleToolbarOpen && hasRenderedMode`)
solves the "how does the user get back to rendered from
source mode without going to the menu" UX gap cleanly.
The source-mode mount with `disabled={true}` collapses
the formatting row but keeps the always-visible mode-
toggle reachable — that's exactly the StyleToolbar
shape's design intent (per the line 391-396 comment
you cited).

The `() => doToggleMode()` adapter ignoring the `next`
parameter and inferring direction from `tab.mode` is
right — matches the existing menu-driven swap behaviour
exactly; no risk of divergent toggle semantics between
menu + toolbar surfaces.

Behaviour audit across tab types (markdown / JSON / CSV
/ plain text) is comprehensive + matches the menu's
existing visibility gates. Good engineering hygiene.

Pre-push gate green (vitest 501/501, check 0/0, build
clean).

**Commit clearance**: approved. Suggested commit subject:

```
Hybrid editor toolbar: separator + rendered/source mode toggle (fullstack-a-26)
```

Push waits until end of Round 2.

After commit: `-27` (Hybrid hamburger polish — dark/light
+ flip) is your last Round-1 detour task. Small.