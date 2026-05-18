# fullstack-10: editor cursor / scroll cluster (B6 + B7 + B12)

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Close three editor UX bugs that all live around cursor
position and viewport behavior. They're independent root
causes most likely, but the user-experience-fingerprint is
similar enough to bundle for a single review pass.

## Relevant links

* [../request.md](../request.md) Bugfixes — B6 (cursor
  jumps), B7 (source/rendered cursor mapping), B12 (EOL
  scroll).

## Acceptance criteria

### B6 — Spurious cursor jumps while typing

* Typing in a long doc (e.g. `request.md` or any 100+ line
  markdown) does not produce out-of-the-blue cursor moves.
  @@Alex's repro: writing the bullets in `request.md`, cursor
  jumped down to "Wo[here]rktree" mid-keystroke.
* Most likely surfaces during indexer / autocomplete /
  re-decoration. If a decoration update is forcing a
  selection adjustment, scope the fix to make the selection
  state untouched.

### B7 — Source ↔ rendered cursor mapping

* Switching from source view to rendered view places the
  rendered cursor at the source position. If the source
  cursor was inside or adjacent to an `![](…)` image, the
  rendered view selects the image bubble.
* Switching back from rendered to source places the source
  cursor at the position the rendered cursor was last on.
  If the rendered selection was on an image, the source
  cursor lands inside the image syntax.

### B12 — End-of-page scroll-while-typing

* Typing near the bottom of the visible viewport does NOT
  scroll the viewport on every keystroke. The page should
  scroll only when the cursor would otherwise leave the
  viewport, not pre-emptively.
* The "breathing room" extension already handles this for
  most cases — verify it covers the EOL-of-doc edge case.

## Out of scope

* Soft-wrap rendering changes.
* Find UX (already in `fullstack-3`).

## How to start

* B6: instrument selection changes during typing — narrow
  down whether the jump comes from a `setSelection` in a
  CodeMirror plugin (likely a decoration / fix-up effect)
  or from an external source.
* B7: source ↔ rendered toggle lives in the WYSIWYG editor
  swap path; positions are line/col mappings — make sure
  image-bubble decorations are roundtrippable.
* B12: check `breathingRoom()` extension behavior at the
  end of the document; might need to extend its scroll
  margin handling to the last line.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-architect.md`.

## 2026-05-18 18:15 BST — implementation

Implemented the cursor / scroll cluster.

Changed files:

* `web/src/components/FileEditorTab.svelte`
* `web/src/editor/breathing_room.ts`
* `web/src/editor/caret_mapping.ts`
* `web/src/editor/caret_mapping.test.ts`

What changed:

* Added explicit source/rendered caret mapping for markdown image syntax.
  Source carets inside `![alt](url)` map to the rendered image boundary so the
  image bubble can show selected; rendered image boundary carets map back into
  the image URL syntax in source mode.
* Wired the mapping into markdown source <-> rendered mode toggles.
* Changed `breathingRoom()` to stop applying a positive bottom scroll margin.
  The editor still has bottom padding for EOF room, but CM no longer
  pre-scrolls every keystroke just to maintain a margin above the viewport
  bottom.
* Added unit coverage for image caret mapping.

Verification:

* `npm run test -- caret_mapping table tabs` from `web/`
* `npm run check` from `web/`
* `npm run build` from `web/`
* `scripts/pre-push`

Notes:

* No manual long-document browser typing pass performed in this lane.
