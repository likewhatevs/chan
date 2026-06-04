# task LaneA -> Lead (5): bullet CLEANUP (task-7) + indent refinement (@@Alex direct)

Two things in one editor commit (same files/domain): the task-7 bullet cleanup
(supersedes the snap), and a small indent refinement @@Alex asked me for
DIRECTLY in chat while watching the test (flagging that for your awareness -
he reached me directly, not via you).

## Heads-up: @@Alex contacted me directly in chat
While I was smoke-testing, @@Alex (the session user) asked in chat for the
nested-glyph indent alignment (with a reference image). I folded it into this
cleanup since it's the same bullet-rendering domain. Routing the OUTCOME to you
per process.

## Part A - task-7 cleanup: real bullet markers, snap DELETED
- Diagnosis confirmed: `*`/`+` used a zero-width source char + CSS ::before
  glyph -> visual glyph decoupled from source position -> click/cursor coords
  mapped into the prefix -> needed snap band-aids. The scaffolding WAS the bug.
- blocks.ts: `*`/`+` marker is now `Decoration.replace({widget:
  BulletGlyphWidget})` rendering the REAL glyph CHARACTER (● / ○ / ■ by depth,
  real width) - like the task checkbox replace-widget (no atomicRanges). Hyphen
  stays a literal `-` mark. DOC unchanged (render-only; round-trip writes `*`).
- DELETED: clampListCaretPosition, listCaretGuard, isListEolClick,
  listAwareArrowDown/Up, verticalMoveClampingPrefix (list.ts); the
  ArrowDown/ArrowUp keymap entries + listCaretGuard() extension; the
  cursorLineDown/Up import; the font-size:0 + ::before glyph CSS. KEPT
  clickToPlaceCaret (general blank-area helper) + listLineAt (image_drop).
- NO TENSION to flag: got BOTH the Google-Docs glyph AND correct positioning
  (a real-character glyph widget gives both, as you hoped).

## Part B - indent refinement (@@Alex)
- Nested glyphs now align under the parent line's first text char (outline
  indent). Wysiwyg.svelte .cm-md-list-line gains
  `margin-left: calc(var(--cm-md-list-depth) * var(--chan-editor-body-size) * 0.6)`.
  margin (not text-indent) so the hanging indent is preserved. Value tuned live
  in-browser.

## Gate: GREEN
- svelte-check 0 ERRORS (1 WARNING = pre-existing RichPrompt a11y, not mine).
- editor vitest 221/221. npm run build OK.

## Smoke (fresh binary, regression matrix - bullet/hyphen/ordered x depth 0/1/2)
- click MID-text of nested -> caret where clicked (task-7 bug GONE): bullet,
  hyphen, ordered all land mid-text past the glyph.
- click EOL -> line end: all 8 cases (synthetic sweep via clickToPlaceCaret).
- arrow-down AND arrow-up between items -> caret past the glyph (native goal
  column, the ORIGINAL item-1 bug gone WITHOUT any snap).
- indent: nested glyph aligns under parent first text char - bullet exact at
  depth 1 AND 2; hyphen +2px, ordered +3px (within "a few pixels").
- glyphs render: ●/○/■ depth cycle, dashes, ordered numbers.

## Pathspec (supersedes task-LaneA-Lead-4; this is the bullet rework + indent)
- base HEAD: 948faed1
- files (7): web/src/editor/decorations/blocks.ts(+test),
  web/src/editor/commands/list.ts(+test), web/src/editor/Wysiwyg.svelte,
  web/src/editor/click_caret.ts, web/src/editor/clickCaret.test.ts
- `git diff -- <those 7> | git hash-object --stdin` = 361b789417e12323b03bc36985a4825801be10c4
- Net -81 lines (146 ins / 227 del) - "less bullet-specific code". No shared-
  file touch.
- Commit guidance: lands as its own refactor commit (supersedes the bullet
  glyph bits of c9ea3c56 + the task-6 snap). Your call whether to rework
  c9ea3c56 or append; I recommend append (clean diff, append-only history).

Journal: docs/journals/phase-18/team/journals/journal-LaneA.md
Bullet lists are now plain CodeMirror cursor/click/arrow with the Google-Docs
glyph + outline indent. Ready for your commit. Hand-smoke for @@Alex unchanged
(real-trackpad item-3 no-stall).

## RE task-Lead-LaneA-8 (crossed with this report) - already satisfied
Your task-8 (HOLD task-4 EOL commit; fold into the task-7 cleanup; remove the
guards) is EXACTLY what this task-5 cleanup does. Reconciliation:
- task-4's EOL listCaretGuard branch is NOT committed standalone - it's DELETED
  along with the entire listCaretGuard, clampListCaretPosition, isListEolClick,
  and listAwareArrowDown/Up. Zero bullet-specific guard code remains.
- The negative-text-indent root cause was the lever, as you said: real-width
  glyph-widget markers (no zero-width char + ::before) let CM6 resolve clicks
  NATIVELY. click-in-text (your task-7 bug) + EOL + arrow all work at depth 1
  AND 2 for bullet/hyphen/ordered with no guards.
- The ONLY remaining click helper is clickToPlaceCaret - that is the GENERAL
  blank-area click helper (places the caret on a click past any short line /
  below the doc, for ALL content, not just lists). It pre-dates the bullet
  work and is NOT bullet-specific scaffolding, so I kept it. Removing it would
  regress general blank-click UX everywhere. FLAGGING so it's your/@@Alex's
  call if you want even that gone - but it is not the list scaffolding @@Alex
  flagged.
- No "minimal guard unavoidable" flag needed: the marker geometry fix made the
  bullet path fully native. The wrap hanging-indent stays (margin-based outline
  indent) and CM6 resolves it correctly with real markers.
So: nothing further to do for task-8 - this task-5 IS the unified close-out.
