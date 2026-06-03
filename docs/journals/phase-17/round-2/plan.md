# Phase 17 - round 2 plan

@@Alex's second report (draft.md + image*.png, moved here from ./alex-report-2
on 2026-06-02). Three items, triaged onto the existing lanes. Dispatched as
each lane finishes its round-1 Wave-2 work (append-only; no mid-task interrupt).

Source: docs/journals/phase-17/round-2/draft.md (+ image-1/2/3.png).

## Items

### R2-1 References / attribution (about page)  -> @@LaneD

Add the missing open-source attributions to the About page's bottom section,
and the line "built on strong open source foundation, chan is free and open
source software". @@Alex flagged svelte + tauri as missing; also add:
- mermaid        https://mermaid-cjv.pages.dev/
- xterm.js       https://github.com/xtermjs/xterm.js/
- codemirror     https://codemirror.net/
- d3-force       https://d3js.org/d3-force
Plus svelte + tauri (and check the rest of the real stack: axum, candle/BGE,
rust-embed, notify, yamux/h2). Locate the actual About page (likely
web-marketing; confirm whether there is also an in-app one). Folds with D1.

### R2-2 List paste-link indent bug  -> @@LaneC

Pasting a link into a list INDENTS the list (image-1); cmd+shift+tab makes it
worse (image-2). Editor list/paste handling - @@LaneC's domain (blocks.ts /
Wysiwyg / the editor paste path). Reproduce, fix the indent-on-paste + the
shift-tab outdent interaction.

### R2-3 Per-terminal survey  -> @@LaneB

Surveys must be PER-TERMINAL, not window-wide (image-3): each terminal has its
own survey and they do not impact each other. BubbleOverlay.svelte (@@LaneB).
Natural follow-on to B1's per-terminal rich-prompt pattern (key visibility +
state by tab id, not a window-global). NB: this also improves the
`cs terminal survey` channel the lead uses to ask @@Alex.

## Sequencing

Queue behind round-1 Wave-2 per lane:
- @@LaneB: B12 -> cs-surface -> B4 -> R2-3.
- @@LaneC: B9 -> R2-2.
- @@LaneD: B5 -> D1 (Wave-3) -> R2-1 (R2-1 folds into D1's about/website work).

Commit round-1 at its green boundary, then round-2. No mid-Wave interrupts.
