# chan editor (CM6) — design

Load-bearing reference for the new chan editor. Mirrors the role of
`chan-core/crates/chan-drive/design.md` for the editor surface.

## Why this exists

The previous editor (under `web/src/editor/`, tiptap/ProseMirror-based) made
the document the *rendered* tree. Markdown source was reconstructed by
serialization and faked in/out around the caret via expand/collapse passes
plus per-pattern editing flags. Each new pattern (bold, italic, strike, code,
link, wikilink, image, naked URL, ...) added another collapse/render race and
another set of edge cases.

The recurring bug class: cannot edit `*a*` (1-char italic) although `*aa*`
works; pending-mark heuristics flicker; markdown round-trip needs escape
gymnastics (NBSP for blank paragraphs, `\#` for heading prefixes, defensive
image serializers); the autosave gate has to enumerate every active
expansion flag. The class kept biting because it was structural.

This editor flips the model. Same architecture as Obsidian's Live Preview.

## The contract (10 invariants)

1. **Doc invariant.** `view.state.doc.toString()` is the markdown source.
   Always. No transform layer. Autosave writes it directly.

2. **Token detection.** `syntaxTree(state).iterate({from, to, enter})` from
   `@codemirror/lang-markdown`. Plus a custom inline parser extension for
   `[[wikilink]]`. Plus a small block-start parser for YAML frontmatter so
   headings inside `---...---` are not promoted.

3. **Decoration taxonomy.**
   - **Hide markers**: `Decoration.replace({})` over `*`, `**`, `~~`, `` ` ``,
     `[`, `](`, `)`, `![`, `[[`, `]]`, ```` ``` ````, `> `, `# ` prefixes.
   - **Inline marks**: `Decoration.mark({class})` over the *content* between
     markers — emphasis, strong, strike, inline-code, link-label.
   - **Atomic widgets**: `Decoration.replace({widget, block: false})` over the
     *whole* range — wikilink, image, date, tag pill, contact pill (rendered
     as a wikilink subtype). `EditorView.atomicRanges` registered for these.

4. **Visibility rule (per token kind).**
   - **Marks** (bold/italic/strike/code/link-markers/image-markers/wikilink-
     brackets): hide unless the active selection intersects `[from, to]`.
     Equality at the boundary counts as intersection.
   - **Block prefixes** (heading `#`, list `-` / `1.`, blockquote `>`,
     fence ```` ``` ````): hide unless the caret line intersects the token's
     line. Selection-intersect alone causes flicker as the caret crosses the
     prefix mid-line.
   - **Atom widgets**: show widget unless selection intersects the source
     range; on intersect, suppress the widget and reveal source so the user
     can edit literally. (Non-atomic decorations always rendered as marks;
     only the widget's `replace` flips off.)

5. **Atom strategy (split by token type).**
   - **Wikilinks (`[[note]]` and `[label](path)` where `path` is internal)**:
     atomic widget. Editing means caret-adjacent reveals raw text, OR click
     pill → opens wiki bubble in edit mode.
   - **External markdown links `[label](https://...)`**: hide markers only
     (`[`, `](`, `)`); `link` mark on label; URL stays visible-but-dimmed
     when selection intersects, hidden otherwise. URL editable in place.
   - **Naked URLs**: mark only, no hide.

6. **Selection rule for ranges.** A non-empty selection that crosses any
   token's range reveals all of those tokens uniformly. No special cases.

7. **Bubbles** (`[[`, `![`, `#`, `@`) open/close from a single
   `StateField<BubbleSpec | null>` driven by `EditorView.updateListener`.
   Trigger detection inspects the doc text immediately around
   `state.selection.main.head`. Debounced via transaction batching: 5
   transactions of `[[abc` produce one open bubble showing "abc". Bubble
   keymap intercepts before CM6's defaults via a high-precedence
   `keymap.of`. Bubble must NOT call `view.focus()` mid-flow — that breaks
   the caret-stay feel.

8. **Find** uses the existing `find.ts` `scanMatches`. The `findField` and
   `FindAdapter` shape from the legacy `Source.svelte` lifts into
   `editor-cm6/base.ts` and is shared by both Source and WYSIWYG modes.

9. **Fold** uses `@codemirror/language` `foldService` with a heading-level-
   aware computer: line `^#{n} ` folds end-of-line → start of next `#{<=n}`
   line (or doc end). `foldGutter()` for the chevron.

10. **Autosave** writes `view.state.doc.toString()` on `update.docChanged` to
    the bindable `value` prop. The existing tab-side debounced
    `scheduleAutosave` pipeline is unchanged. No serialize step. No
    `editing*Original` flags. The `applyingExternal` guard is preserved
    (tab-swap external sync). The CAS contract on `PUT /api/files` is
    unchanged.

## Why this design fixes the `*a*` class

Under the old model, italic on `a` produces a 1-char marked text node. The
"caret strictly inside the mark" check has no integer position satisfying
`from < caret < to` when `to - from == 1`. Each pattern (bold, italic,
strike, code, link, wikilink, image) needs its own boundary patch.

Under this model, `*a*` is three real characters in the doc: `*`, `a`, `*`.
The `*` markers at `[0, 1]` and `[2, 3]` get hide-decorations whenever the
selection does NOT intersect them. A caret at offset 1 (between `*` and `a`)
intersects both `[0, 1]` (caret == to) and `[2, 3]` (caret == from); both
markers reveal. No special case. Backspace deletes a real `*` character the
user can see. Round-trip is the identity function.

## Layout

```
editor-cm6/
├── design.md              this file
├── base.ts                shared CM6 setup: theme compartment, $bindable
│                           sync helper, applyingExternal guard, density attr,
│                           lineWrapping, findField + setFindEffect +
│                           buildFindDecos + makeFindAdapter
├── Wysiwyg.svelte         the new editor (full decoration stack + widgets +
│                           bubbles + overlays + format commands)
├── Source.svelte          plain CM6 source mode; reuses base.ts; no widgets
├── markdown/
│   ├── grammar.ts         @codemirror/lang-markdown + GFM + custom lezer
│   │                       extension for [[wikilinks]]
│   └── frontmatter.ts     small block-start parser for ---...---
├── decorations/
│   ├── walker.ts          ViewPlugin: walks viewport syntaxTree, builds
│   │                       RangeSet, runs on
│   │                       docChanged | viewportChanged | selectionSet
│   ├── selection.ts       selectionInRange / lineIntersect helpers
│   ├── marks.ts           bold/italic/strike/code/link-label
│   ├── blocks.ts          headings, lists, task lists, blockquote, hr,
│   │                       fenced code (language slot inline)
│   └── naked_url.ts       https?:// detection, link mark
├── widgets/
│   ├── wikilink.ts        atom widget; replaces extensions/wikiLink.ts
│   ├── image.ts           atom + drag-resize handle + alignment fragments;
│   │                       replaces extensions/image.ts + imageBubble.ts
│   ├── date.ts            atom + calendar popover; uses dateFormats.ts
│   ├── tag.ts             decoration-only pill on #word
│   └── checkbox.ts        widget on `[ ]` / `[x]` for task items
├── bubbles/
│   ├── controller.ts      StateField<BubbleSpec | null> + updateListener;
│   │                       keymap precedence
│   ├── wiki.ts            reuses today's wikiLink bubble UI logic
│   ├── image.ts           reuses imageBubble UI logic
│   ├── tag.ts             reuses tagPicker UI logic
│   └── contact.ts         reuses contactPicker UI logic
├── overlays/
│   └── image_action.ts    zoom + edit pills on rendered image hover
├── commands/
│   └── format.ts          toggleBold/Italic/.../setBlockKind/...
│                           (text mutations: insert ** around selection,
│                            prepend `- ` to lines)
└── fold.ts                heading-level-aware foldService
```

## Files reused as-is from the previous tree

- `editor/bubble.ts` — framework-agnostic popover shell
- `editor/extensions/popover.ts` — viewport-watching positioner
- `editor/dateFormats.ts` — date catalog + matcher
- `editor/find.ts` — pure scanMatches
- `editor/links.ts` — wikiLinkToMarkdown, normalizeHref, etc.
- `editor/extensions/wikiBlocks.ts` — block-anchor parser

These will move alongside the rewrite at cutover (step 11) but their content
does not change.

## Server contract (unchanged)

The new editor calls the same endpoints as the old:

| Method | Path | Purpose |
|--------|------|---------|
| GET    | `/api/files/{path}`              | load markdown |
| PUT    | `/api/files/{path}`              | autosave (CAS via `expected_mtime_ns`, 409 on conflict) |
| GET    | `/api/search/files`              | wiki picker |
| GET    | `/api/resolve-link`              | wiki pill kind classification |
| GET    | `/api/headings/{path}`           | `[[file#` mode |
| GET    | `/api/contacts`                  | `@` picker |
| GET    | `/api/graph`                     | tag picker source |
| GET    | `/api/files`                     | image catalog |
| POST   | `/api/attachments`               | multipart upload (50MB cap) |
| WS     | `/ws`                            | watch events (self-writes filtered) |

## Out of scope (v2)

- Language-aware syntax highlighting inside fenced code blocks (CM6 nested
  parsers via `@codemirror/language` + per-language packs).
- Tables: v1 keeps them as plain source. Real grid rendering is widget work.
- Pasting structured content from outside (HTML clipboard) — v1 pastes plain
  markdown text; HTML→markdown conversion is v2.
- Collaborative editing remains an explicit non-goal per `CLAUDE.md`.
