# task LaneA -> Lead (1): Editor wave-1 done (items 1-3); item 4 HOLD

@@LaneA Editor lane, round-1 wave-1. Items 1-3 landed + empirically
smoked. Item 4 (`[[` path autocomplete) HELD for the @@Alex survey, with
client-side recon below.

## Scoped own-gate: GREEN
- svelte-check (`npm run check`): pass (whole tree).
- vitest scoped to my files (blocks.test.ts + list.test.ts): 44/44 pass.
- `npm run build`: pass (built in 2.71s, with all peers' WIP in tree).
- NOTE / cross-lane heads-up: full `make web-check` (all vitest) shows ONE
  red: `web/src/components/fileTreeSelectionMenu.test.ts` (Delete/Copy
  Path/Rename rows). That file AND `FileTree.svelte` are both peer-modified
  (@@LaneC's File Browser WIP) - a concurrent-WIP false-red, NOT my change.
  My 5 files are the only `web/src/editor/**` changes in the tree.

## Pathspec (for your atomic commit)
- base HEAD: d5f7dd38
- files (the ONLY ones I touched):
  - web/src/editor/decorations/blocks.ts
  - web/src/editor/decorations/blocks.test.ts
  - web/src/editor/commands/list.ts
  - web/src/editor/commands/list.test.ts
  - web/src/editor/Wysiwyg.svelte
- `git diff -- <those 5> | git hash-object --stdin` = 50dc82ea274421822d688c9e1235f73032f84c88
- No shared-file touch. blocks.ts decorations are consumed ONLY by
  Wysiwyg.svelte (verified: RichPrompt / others do not import
  chanDecorations), so the hyphen-class change is contained to the
  hybrid editor - no cross-lane effect.

## Per-item status

1. DONE - Bullet/hyphen cursor parity with ENUMERATED lists.
   Root cause (reproduced in Chrome): `*`/`+` markers render as a
   zero-width source char + CSS ::before glyph, so a vertical move whose
   goal column lands left of the glyph drops the caret AT the glyph
   (inside the prefix). Ordered keeps a real visible marker so its goal
   column maps onto the text. Fix: list.ts listAwareArrowDown/Up run the
   normal cursorLineDown/Up then snap a caret that landed in the prefix to
   the first text column (reusing clampListCaretPosition); only
   re-dispatches when it actually moves, so ordered/hyphen keep native
   goal-column tracking. Wired in Wysiwyg Prec.high keymap AFTER mermaid's
   stepInto (earlier extension -> tried first) + escapeFence.
   Smoked: ArrowDown AND ArrowUp on star + hyphen nested lists now land
   caret past the marker (e.g. star nested: caretX 286 > markerRight 281;
   was 268 == glyph), matching the ordered reference. Mermaid ArrowDown
   still de-renders into the block (composition intact).

2. DONE - Restore distinct HYPHEN lists.
   blocks.ts bulletMarkerDecoration(markerChar, depth): `-` -> HYPHEN_MARK
   (`cm-md-ul-marker cm-md-ul-hyphen`, literal dash, NO depth cycle);
   `*`/`+` keep the Google-Docs depth glyphs (disc/circle/square).
   Wysiwyg `.cm-md-ul-hyphen` CSS keeps the dash visible (no font-size:0 +
   ::before), matched glyph-to-text margin. Smoked: hyphen list renders
   dashes at every level, visually distinct from star/plus glyphs; star/
   plus glyphs unchanged. Side benefit: hyphen markers are now real text,
   so they get ordered-list cursor behavior for free.
   blocks.test.ts updated: the old tests encoded the REGRESSED "all
   markers -> depth glyph" behavior; now `*`/`+` = depth glyph, `-` =
   hyphen, plus a new hyphen-distinct test + cm-md-ul-hyphen source-pins.

3. DONE - Trackpad free-scroll hang.
   Root cause (reproduced): `scroll-behavior: smooth` on `.cm-scroller`
   animates EVERY scrollTop write, including CM6's own height-estimation
   corrections while scrolling a tall, mostly-estimated doc; during a
   trackpad pan those animated corrections fight the pan ("hang, jump
   opposite, settle"). Confirmed: setting scrollTop=3000 read back 2.
   Fix: removed `scroll-behavior: smooth` + the reduced-motion block from
   Wysiwyg `.cm-scroller` (WHY documented inline). The Google-Docs lift
   comes from the 60px bottom padding + scrollMargin (breathing_room.ts),
   not from smooth, so the lift is preserved. Smoked: scrollBehavior now
   "auto"; scrollTop=3000 applies instantly.
   CAVEAT: Blink synthetic events can't reproduce real trackpad momentum,
   so the definitive "no stall" pass is @@Alex's hand-smoke on a real
   trackpad (Chrome or chan-desktop). Mechanism + fix are confirmed.
   Note (NOT my lane): Source.svelte:461 has the same
   `scroll-behavior: smooth` on its `.cm-scroller`. If @@Alex hits the
   stall in source mode too, that's the parallel fix - route to whoever
   owns Source.svelte (unassigned in the plan).

## Item 4 - `[[` workspace-PATH autocomplete: HELD for @@Alex survey

Did NOT touch any route (cross-lane). Client-side recon for your survey:
- Today: bubbles/wiki.ts fetchFile -> api.linkTargets -> /api/link-targets
  (basename/title/heading FUZZY match), NOT a path-prefix completion.
- An EXISTING client method `api.list(dir)` -> GET /api/files returns
  TreeEntry[] (path, is_dir, kind) = real workspace paths. So:
  - If @@Alex wants PATHS only (or both): a path-completion mode could be
    built CLIENT-SIDE off api.list, NO chan-server route change needed
    (I own bubbles/wiki.ts + triggers.ts).
  - If @@Alex wants /api/link-targets itself to also return directory/path
    candidates: that IS a chan-server route change (cross-lane -> route
    through you).
- Survey question to fold in (1 decision, <=4 options): should `[[`
  complete (a) workspace PATHS only, (b) keep the existing
  filename/heading/block link-targets, or (c) both? Pick (a) or (c) and I
  can do it client-side this round; (b)-only = no change needed.

Journal: docs/journals/phase-18/team/journals/journal-LaneA.md
Ready for your gate + commit on items 1-3; awaiting the item-4 decision.
