# chan editor (CM6) - design

Load-bearing reference for the chan editor. Mirrors the role of
`crates/chan-workspace/design.md` for the editor surface.

## Model

The document text IS the markdown source. `view.state.doc.toString()` is the
file on disk; there is no separate rendered tree and no serialization layer.
The editor decorates the source in place (hide markers, render widgets) so it
reads like rendered markdown while every character stays editable. This is the
Live Preview model, the same architecture as Obsidian's.

Because the source is the single source of truth, the editor sidesteps a class
of structural bugs a rendered-tree model is prone to: editing 1-char marks
like `*a*`, flickering pending-mark heuristics, and markdown round-trip escape
gymnastics (NBSP for blank paragraphs, `\#` for heading prefixes, defensive
image serializers). See "Why 1-char marks work" below.

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
     markers - emphasis, strong, strike, inline-code, link-label.
   - **Atomic widgets**: `Decoration.replace({widget, block: false})` over the
     *whole* range - wikilink, image, date, tag pill, contact pill (rendered
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
     pill -> opens wiki bubble in edit mode.
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
   `keymap.of`. Bubble must NOT call `view.focus()` mid-flow - that breaks
   the caret-stay feel.

8. **Find** uses `find.ts` `scanMatches`. The `findField` and `FindAdapter`
   shape live in `base.ts` and are shared by both Source and WYSIWYG modes.

9. **Fold** uses `@codemirror/language` `foldService` with a heading-level-
   aware computer: line `^#{n} ` folds end-of-line -> start of next `#{<=n}`
   line (or doc end). `foldGutter()` for the chevron.

10. **Autosave** writes `view.state.doc.toString()` on `update.docChanged` to
    the bindable `value` prop. The tab-side debounced `scheduleAutosave`
    pipeline owns the write. No serialize step. No `editing*Original` flags.
    The `applyingExternal` guard handles tab-swap external sync. The CAS
    contract on `PUT /api/files` is the conflict gate.

## Why 1-char marks work

`*a*` is three real characters in the doc: `*`, `a`, `*`. The `*` markers at
`[0, 1]` and `[2, 3]` get hide-decorations whenever the selection does NOT
intersect them. A caret at offset 1 (between `*` and `a`) intersects both
`[0, 1]` (caret == to) and `[2, 3]` (caret == from), so both markers reveal.
No special case. Backspace deletes a real `*` character the user can see, and
round-trip is the identity function. A rendered-tree model that represents
`*a*` as a single marked node has no integer caret position satisfying
`from < caret < to` when `to - from == 1`, which is the structural reason that
model needs a per-pattern boundary patch and this one does not.

## Layout

```
editor/
├── design.md              this file
├── base.ts                shared CM6 setup: theme compartment, $bindable
│                           sync helper, applyingExternal guard, density attr,
│                           lineWrapping, findField + setFindEffect +
│                           buildFindDecos + makeFindAdapter
├── Wysiwyg.svelte         the editor (full decoration stack + widgets +
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
│   ├── wikilink.ts        atom widget for [[note]] and internal links
│   ├── image.ts           atom + drag-resize handle + alignment fragments
│   ├── date.ts            atom + calendar popover; uses dateFormats.ts
│   ├── tag.ts             decoration-only pill on #word
│   └── checkbox.ts        widget on `[ ]` / `[x]` for task items
├── bubbles/
│   ├── controller.ts      StateField<BubbleSpec | null> + updateListener;
│   │                       keymap precedence
│   ├── wiki.ts            wikiLink bubble UI
│   ├── image.ts           image bubble UI
│   ├── tag.ts             tag picker UI
│   └── contact.ts         contact picker UI
├── overlays/
│   └── image_action.ts    zoom + edit pills on rendered image hover
├── commands/
│   └── format.ts          toggleBold/Italic/.../setBlockKind/...
│                           (text mutations: insert ** around selection,
│                            prepend `- ` to lines)
└── fold.ts                heading-level-aware foldService
```

## Shared support files

Framework-agnostic helpers the editor builds on:

- `editor/bubble.ts` - popover shell
- `editor/extensions/popover.ts` - viewport-watching positioner
- `editor/dateFormats.ts` - date catalog + matcher
- `editor/find.ts` - pure scanMatches
- `editor/links.ts` - wikiLinkToMarkdown, normalizeHref, etc.
- `editor/extensions/wikiBlocks.ts` - block-anchor parser

## Server contract

The editor calls these endpoints:

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

## Out of scope

- Language-aware syntax highlighting inside fenced code blocks (CM6 nested
  parsers via `@codemirror/language` + per-language packs).
- Tables render as plain source; real grid rendering is widget work.
- Pasting structured content from outside (HTML clipboard): paste is plain
  markdown text; HTML to markdown conversion is not implemented.
- Collaborative editing is an explicit non-goal per `CLAUDE.md`.
