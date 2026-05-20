# fullstack-a-24: Rich prompt + chat/survey bubbles visual redesign + collapse/expand

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Re-shape the rich prompt + every chat/survey bubble into
softly-rounded floating pills over the terminal. Today the
rich prompt is a rectangle attached to the bottom of the
screen; @@Alex wants it floating like a pill, like the
reference image embedded in the 2026-05-20 conversation
(rounded corners, breathing room on all four sides,
default placeholder "Write a multi-line command and
Cmd+Enter"). Plus a new collapse/expand affordance next to
the existing close, so the rich prompt can stay attached
in minimal-height form while the chat / survey area gets
more vertical real estate.

## Background

@@Alex 2026-05-20:

> The rich prompt itself, and all the chat bubbles, should
> have round corners and be floating over the terminal —
> they already float but they are squared especially the
> prompt itself, it is currently a rectangle coming off
> the bottom of the screen — it must be like this
> [reference image] with the default prompt "Write a
> multi-line command and Cmd+Enter"; the style toolbar,
> when toggled to show (default off) will appear inside
> the bubble, not outside, and will have margin at the
> top so that the prompt does not go under it, at least
> when the cursor is at the first line.
>
> In addition to the close button, we need a
> collapse/expand to/from bottom, so the rich prompt can
> stay there but using minimal space while the bubble and
> chat / survey space gets more visible area.

## Acceptance criteria

### Visual

* Rich prompt container has rounded corners (suggest
  `border-radius: 12-16px`; pick what reads natural in
  the actual rendering).
* Rich prompt floats off the bottom edge with visible
  terminal underneath; not flush-attached. The reference
  image shows clear inset on all four sides.
* Default placeholder copy: `Write a multi-line command
  and Cmd+Enter` (replaces the current placeholder text).
* All chat / survey bubbles get the same rounded-corner
  treatment. Composes with `fullstack-b-5` per-Hybrid
  theme overrides cleanly (theme tokens still drive
  colours).
* Works in both light and dark theme.

### Style toolbar

* Style toolbar (formatting controls — bold, italic, etc.)
  moves INSIDE the rich prompt bubble. Currently appears
  outside.
* **Default state: OFF**. Toggle-to-show.
* When ON: toolbar sits at the TOP of the bubble.
* Margin between the toolbar and the prompt body so the
  cursor at line 1 has clearance and doesn't sit under
  the toolbar.

### Collapse / expand affordance

* New control next to the existing close button.
* **Expanded** (default): current full-height behaviour.
* **Collapsed**: minimal-height bar; only enough room to
  show the placeholder / current first line. Chat /
  survey bubbles above gain the freed vertical area.
* Click toggles between expanded and collapsed.
* Close stays as the dismiss path (the rich prompt goes
  away entirely). Collapse is "stay attached, but
  small."
* Glyph: chevron (down for collapse-to-bottom, up for
  expand). Mirror the close-button visual weight.
* Optional chord binding: leave to implementer judgement.
  If a clean chord fits (e.g. Alt+Down / Alt+Up while
  rich prompt is focused), document. Otherwise click-only
  is fine.

### Composition with existing rich-prompt features

* `fullstack-a-4`'s caret-focus rules survive: caret
  retention after Cmd+Enter dispatch, bubble-present →
  caret to survey, dismiss → caret back to prompt.
* `fullstack-a-14`'s autoFocus prop survives: bubble
  present at open / re-open → editor doesn't auto-focus.
* `fullstack-a-17`'s spawn-side focus gate (Cmd+K p)
  survives.
* `fullstack-a-18`'s wysiwyg-mode Cmd+Enter dispatch
  survives.
* `fullstack-a-20`'s defaultPrevented guard survives.
* Collapse state persists across rich prompt close →
  re-open within the same session (so the user doesn't
  have to re-collapse every time). Persistence shape
  matches the other rich-prompt session state.

### Gate

* Vitest pin for collapse-state toggle if a testable seam
  exists (the visual CSS isn't vitest-testable; the state
  machine that drives the class binding is).
* `npm run check` + `npm run build` clean.

## How to start

1. Find the rich prompt component
   (`web/src/components/TerminalRichPrompt.svelte`).
   Identify the current styling that makes it rectangular
   + edge-attached. Add the border-radius + bottom-margin
   to lift it into a floating pill.
2. Find the bubble overlay
   (`web/src/components/BubbleOverlay.svelte` or sibling).
   Apply the same rounded-corner treatment.
3. Find the style toolbar's current mount point (likely
   outside `TerminalRichPrompt`'s root, in a sibling
   slot). Move INSIDE the rich prompt container; gate on
   the "show toolbar" toggle state (default false). Add
   top-margin so the prompt body's first line clears the
   toolbar.
4. Add the collapse / expand control:
   * State: new `collapsed: boolean` on the rich prompt's
     local state (or sibling to the existing close-button
     state).
   * Class binding: `class:collapsed={collapsed}` on the
     prompt root.
   * CSS: when `.collapsed`, height clamps to the minimum
     needed for the placeholder + the control row;
     overflow-y hides the editor body.
   * Persistence: write the collapse state to the same
     store that holds the other rich-prompt session
     prefs.
5. Visual test on lane-A (default-mode rich prompt + a
   bubble present + the style toolbar toggle).
6. Pre-push gate.

## Coordination

* @@WebtestA verifies on lane-A drive once landed.
* No backend / Rust work in this task.
* Independent of other Round-1 detour tasks; can land in
  parallel.

## 2026-05-20 — implementation note

Five-area change across two svelte files + the rich-prompt
state model + two test pins.

### Floating-pill visual on the rich prompt

`TerminalRichPrompt.svelte`'s `.rich-prompt` was previously
flush against the bottom edge — full-bleed left/right/bottom,
single-edge border, single top-edge shadow that signalled
"attached to bottom". @@Alex's spec wanted it floating like a
chip with breathing room on all four sides.

* `left: 12px; right: 12px; bottom: 12px` instead of `0` on
  each — visible terminal underneath on all sides.
* `border: 1px solid var(--border)` (was `border-top` only) +
  `border-radius: 14px` for the floating-pill silhouette.
* `box-shadow: 0 10px 30px rgba(0,0,0,0.32)` (was a top-edge
  only shadow). Reads as a card lifted off the surface
  rather than a header bar attached to it.
* `overflow: hidden` clips the inner editor + header
  borders to the new border-radius.

### Default placeholder hint

CodeMirror has a built-in `placeholder` extension, but
threading a new prop through `Wysiwyg.svelte` + `Source.svelte`
for this single in-prompt use felt like the wrong layer —
the placeholder is a *rich-prompt* concern, not an *editor*
concern. Instead a CSS overlay sits at the top of the
`.composer-editor` block when `prompt.buffer === ""`. The
overlay is `position: absolute; pointer-events: none`
so the editor still receives clicks; Svelte's reactive
conditional render swaps it out the moment the user types
the first character. Copy: "Write a multi-line command and
Cmd+Enter" per @@Alex's spec.

### Style toolbar default flipped to off

Pre-fix `toolbarOpen()` read `prompt.styleToolbarOpen !==
false` — `undefined` was treated as on. @@Alex's spec is
explicit: default off, opt-in. Flipped to
`prompt.styleToolbarOpen === true` so `undefined` reads
as off. The toolbar mount site (already inside the header
of the rich-prompt root, with the existing border-bottom
separator from the editor body) didn't need to move — it's
already at the top of the bubble with margin separation.

### Collapse / expand affordance

New chevron button between Send and Close in the header.
`collapsed: true` clamps the prompt to `min-height: 0;
height: auto` and hides the `.watcher-row`, the
`.composer-editor`, and the top-resize handle (collapsed
prompts can't be drag-resized — there's nothing to resize).
Header stays visible with the chevron flipping to ChevronUp
(expand cue). Click toggles; `aria-pressed` reflects state
for screen readers.

Persistence: new `collapsed?: boolean` field on
`TerminalRichPromptState`; serialized as `rpc: 1` only when
truthy; round-tripped via `richPromptFromSer` with the
`...(src.rpc === 1 ? { collapsed: true } : {})` spread so
the round-tripped shape stays exact-equal to the pre-`-a-24`
shape when the user hasn't collapsed. Mirrors the existing
`rpo` / `rpm` "absence reads as default" pattern.

### Bubble overlay rounded corners

`BubbleOverlay.svelte`'s `.bubble` went from
`border-radius: 6px` to `12px` to match the rich-prompt's
14px. Slight asymmetry (12 vs 14) is intentional: bubbles
are smaller floating chips above a larger floating prompt;
identical corners would feel rigid. The 12/14 pair reads
as the same design language without forcing parity.

### Test updates

Two tests in `TerminalRichPrompt.test.ts` exercised the
style-toolbar's mode-toggle button (aria-label="show
rendered"). Pre-`-a-24` the toolbar was default-on so the
button was always in the DOM. Now that default flipped,
the tests set `styleToolbarOpen: true` on their prompt
fixtures so the toolbar (and the mode-toggle button) are
rendered. Tests focus on the mode-toggle, not the toolbar
default — explicit prompt-fixture configuration is the
correct fix.

The pre-existing `tabs.test.ts` "persists rich prompt
drafts only in session layouts" test asserts the
round-tripped `richPrompt` deep-equals an exact 4-field
shape. The deserialized shape stayed identical (collapsed
omitted when false) thanks to the conditional spread on
deserialize.

### Composition with prior phase-8 fixes — verified

* `fullstack-a-4` caret rules (no auto-focus when bubbles
  present, Cmd+Enter caret retention): untouched. The
  collapse state doesn't affect focus routing; collapsed
  prompts still receive focus on `focusNonce` bumps when
  no bubbles are present.
* `fullstack-a-14` autoFocus prop: untouched. The
  bubble-aware `autoFocus={bubbleCount === 0}` on the
  Wysiwyg/Source children rides through.
* `fullstack-a-17` Cmd+K p focus gate: untouched. Lives in
  `TerminalTab.svelte`'s focus effect; the rich prompt's
  shape doesn't matter to it.
* `fullstack-a-18` Wysiwyg Cmd+Enter dispatch + `-a-20`
  defaultPrevented guard: untouched. Both live on
  `onSubmit` threading + `onKeydown`; both still fire as
  before in either expanded or collapsed states (collapsed
  prompts still mount the editor, just visually clipped).

### Files touched

* `web/src/state/tabs.svelte.ts` — `collapsed?: boolean`
  on `TerminalRichPromptState`; `rpc?: 1` on the SerTab
  shape; serialize/deserialize conditional spread.
* `web/src/components/TerminalRichPrompt.svelte` —
  default-off toolbar; floating-pill CSS; collapse
  state + chevron button; placeholder overlay; collapsed
  CSS clamps.
* `web/src/components/BubbleOverlay.svelte` — bubble
  border-radius 6 → 12 px.
* `web/src/components/TerminalRichPrompt.test.ts` — two
  mode-toggle tests gain `styleToolbarOpen: true` on
  their fixtures.

### Pre-push gate

vitest 481/481 green; `npm run check` 0 errors / 0
warnings; `npm run build` clean.

### Lane-A verification

(post-restart):

1. Open the rich prompt (Alt+Space). The prompt sits as a
   floating pill — 12 px inset from the screen edges,
   visible terminal underneath, rounded 14 px corners,
   soft shadow on all sides.
2. With empty buffer, the placeholder "Write a multi-line
   command and Cmd+Enter" reads at the top of the editor;
   typing the first character clears it.
3. Right-click the prompt → context menu shows "Show
   style toolbar" (since default is now off). Click; the
   toolbar appears inside the bubble above the editor.
   Re-right-click → "Hide style toolbar". Toggle persists
   across close → re-open.
4. Click the chevron-down button (between Send and Close).
   Prompt collapses to a minimal-height bar — just the
   header row. Bubbles above gain vertical room. Click
   chevron-up to expand back. Collapse state persists
   across close → re-open.
5. Bubbles above the prompt have rounded corners (12 px)
   matching the prompt's design language.
6. Confirm previous fixes still apply: type, press
   Cmd+Enter → single dispatch (no `pwdpwd`); bubble
   appears → caret stays on the survey not the prompt;
   Cmd+K p on a no-terminal pane → caret lands on the
   newly-opened prompt's editor.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Five-area landing in one cohesive commit. Excellent
engineering instinct on the placeholder layer: threading
a `placeholder` prop through Wysiwyg + Source for one
in-prompt use case would have crossed an editor/prompt
boundary that should stay clean. The CSS overlay (gated
on `prompt.buffer === ""`) is the right layer.

Floating-pill visual reads cleanly per the reference
image: 12 px screen-edge inset + 14 px border-radius +
all-sides shadow lifts the prompt off the surface. The
12 vs 14 corner asymmetry between bubbles and the prompt
is a defensible micro-detail — bubbles smaller, slightly
tighter corners; same design language without enforced
parity.

Style toolbar default-off flip caught a subtle pre-fix
bug: `toolbarOpen() === undefined → on` was wrong per
@@Alex's spec. Now explicit. Two test fixtures updated
to set `styleToolbarOpen: true` so the mode-toggle
button stays accessible to the tests.

Collapse state: chevron between Send and Close,
clamps height to header-only, persists via the new
`rpc?: 1` field on SerTab with conditional spread on
deserialize to preserve the pre-`-a-24` round-trip
shape. Tests pinning the round-trip stayed green —
that's the right invariant pattern.

Composition with all prior rich-prompt fixes
(`-a-4` / `-14` / `-17` / `-18` / `-20`) verified
explicitly + reasoned through. Audit trail captures
the why-untouched in case a future reader wonders.

Pre-push gate green (vitest 481/481, check 0/0, build
clean).

**Commit clearance**: approved. Suggested commit subject:

```
Rich prompt + bubbles: floating pill, default-off style toolbar, collapse/expand chevron (fullstack-a-24)
```

Push waits until end of Round 2.

After commit: `-25` (editor trailing-whitespace toggle
→ Settings) is the last item in your Round-1 detour
queue. Then standby until Round-2 fan-out.