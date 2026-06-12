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
gymnastics. See "Why 1-char marks work" below.

## The contract (10 invariants)

1. **Doc invariant.** `view.state.doc.toString()` is the markdown source.
   Always. No transform layer. Autosave writes it directly.

2. **Token detection.** `syntaxTree(state).iterate({from, to, enter})` from
   `@codemirror/lang-markdown` + GFM, extended with two custom lezer parsers:
   `[[wikilink]]` (inline) and YAML frontmatter (block-start, so headings
   inside `---...---` are not promoted). Fenced code bodies parse with
   lazy-loaded per-language packs (`markdown/code_languages.ts`). Tokens that
   are not lezer nodes - `#tag`, `@@mention`, dates - are matched by regex in
   their own ViewPlugins, skipping code ranges.

3. **Decoration taxonomy.**
   - **Hide markers**: `Decoration.replace({})` over `*`, `**`, `~~`, `` ` ``,
     `[`, `](`, `)`, and `# ` heading prefixes. Blockquote `>`, list markers,
     `---` rules, and ```` ``` ```` fences are NOT hidden: the marker is the
     visual cue (Obsidian convention) and hiding `---` / fences makes the
     block structure harder to edit.
   - **Inline marks**: `Decoration.mark({class})` over the *content* between
     markers - emphasis, strong, strike, inline-code, link-label.
   - **Line decorations**: heading levels (`cm-md-h1..6`), list lines,
     blockquote lines, fence opener/content/closer rows - CSS paints size,
     indent, borders, slab background.
   - **Atomic widgets**: `Decoration.replace({widget})` over the *whole*
     range - wikilink/internal-link pill, image, date pill, GFM table grid,
     mermaid diagram, page break. `EditorView.atomicRanges` registered for
     each so caret motion skips them in one keystroke. The task checkbox is
     a replace widget over just the `[ ]` / `[x]` marker (not atomic; the
     click toggles the source).

4. **Visibility rule (per token kind).**
   - **Marks** (bold/italic/strike/code/link markers): hide unless the active
     selection intersects the OUTER token range `[from, to]`. Equality at the
     boundary counts as intersection, and the outer-range rule (not
     per-marker) means a caret near `*a*` reveals both `*` together instead
     of `*a` then `a*`.
   - **Heading prefixes** (`# `): hide unless the caret line intersects the
     heading's line. Selection-intersect alone causes flicker as the caret
     crosses the prefix mid-line.
   - **Atom widgets**: show widget unless selection intersects the source
     range; on intersect, suppress the widget and reveal source so the user
     can edit literally.
   - **Always-visible markers** (`>`, list markers, `---`, fences): styled
     via marks/line decorations, never hidden.

5. **Atom strategy (split by token type).**
   - **Wikilinks (`[[note|alias#anchor]]` and `[label](path)` where `path`
     is internal)**: atomic pill widget. Pill kind (file / contact / image /
     broken) resolves via `GET /api/resolve-link`, cached per target. Editing
     means caret-adjacent reveals raw text, OR click pill -> wiki bubble.
   - **External markdown links `[label](https://...)`**: hide markers only
     (`[`, `](`, `)`); `link` mark on label; URL editable in place.
   - **Naked URLs**: mark only, no hide.
   - **Tables**: read-only grid widget; click drops the caret at the source
     start, which reveals the pipe form for editing.
   - **Mermaid**: a closed ```` ```mermaid ```` fence renders as a diagram
     atom while the caret is outside; caret inside reveals source. The
     mermaid library is dynamic-imported on first render.
   - **Tag `#word` / mention `@@name` pills**: mark-based (no replace), with
     click handling delegated through one content-DOM listener.

6. **Selection rule for ranges.** A non-empty selection that crosses any
   token's range reveals all of those tokens uniformly. No special cases.

7. **Bubbles** (`[[`, `![`, `@@`, `@`, `#`) open/close from
   `computeBubbleSpec` (`bubbles/triggers.ts`), which inspects the doc text
   around `state.selection.main.head` on every transaction via
   `bubbleListener`; the host (Wysiwyg.svelte) mounts/reuses the bubble UI.
   Triggers also fire in "raw" mode when the caret sits inside an existing
   Link/Image URL slot or `[[...]]` body, so commit replaces the right range.
   Triggers never fire inside code ranges, and the reserved macro words
   (`@today`, `@date`, `@pagebreak`, `@break`) suppress the contact bubble.
   The bubble keymap intercepts before CM6's defaults via a high-precedence
   `keymap.of`. Bubbles must NOT call `view.focus()` mid-flow - the caret
   stays in the document and the popover runs alongside it.

8. **Find** uses `find.ts` `scanMatches`. The `findField` and `FindAdapter`
   shape live in `base.ts` and are shared by both Source and WYSIWYG modes.

9. **Fold** uses `@codemirror/language` `foldService` with a heading-level-
   aware computer: line `^#{n} ` folds end-of-line -> start of next `#{<=n}`
   line (or doc end). The chevron gutter is custom (headings only):
   `foldGutter()` would chevron every foldable block because lang-markdown
   marks paragraphs, quotes, and fences foldable too.

10. **Autosave** writes `view.state.doc.toString()` on `update.docChanged` to
    the bindable `value` prop (`createValueSync` in `base.ts` guards the
    echo so a prop write-back can't clobber the caret). The debounced
    `scheduleAutosave` pipeline in `state/tabs.svelte.ts` owns the write. No
    serialize step. The CAS contract on `PUT /api/files`
    (`expected_mtime_ns`, 409 + `current_mtime_ns` on conflict) is the
    conflict gate; a watcher event for a non-self write flags a "changed on
    disk" banner instead of auto-reloading. `state/editorBuffer.ts` keeps a
    debounced localStorage copy keyed by path for hang-recovery.

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

## Modes

`FileEditorTab.svelte` hosts the editors and owns a per-tab mode:
`wysiwyg` | `source` | `pretty` | `table`. Markdown-class files (.md/.txt)
pair WYSIWYG with source; JSON opens as a collapsible tree
(`JsonPretty.svelte`) and CSV/TSV as an editable grid (`CsvTable.svelte` +
`csv.ts`), each with source as the toggle. Any other text-kind file is
source-only - source IS the sensible surface for a .py / .toml / Makefile.
Source mode highlights by extension via the same lazy language packs.

## Layout

```
editor/
├── design.md              this file
├── base.ts                shared CM6 setup: themeExtensions (Primer
│                           highlight + chrome), theme compartment,
│                           findField + setFindEffect + makeFindAdapter,
│                           createValueSync ($bindable echo guard)
├── Wysiwyg.svelte         the editor (decoration stack + widgets +
│                           bubbles + format commands + paste/drop)
├── Source.svelte          plain CM6 source mode; per-extension
│                           highlighting; reuses base.ts; no widgets
├── JsonPretty.svelte      read-only collapsible JSON tree (+ JsonNode)
├── CsvTable.svelte        editable CSV/TSV grid
├── highlight.ts           GitHub Primer syntax palettes (light/dark)
├── markdown/
│   ├── grammar.ts         lang-markdown + GFM + WikiLink + Frontmatter
│   │                       + lazy codeLanguages
│   ├── wikilink.ts        lezer inline parser for [[...]]
│   ├── frontmatter.ts     block parser for ---...--- at doc start
│   ├── code_languages.ts  per-language packs, one vite chunk each
│   └── debug.ts           dev-console syntax-tree dump
├── decorations/
│   ├── walker.ts          ViewPlugin: walks viewport syntaxTree, runs
│   │                       on docChanged | viewportChanged |
│   │                       selectionSet | geometryChanged
│   ├── index.ts           handler registry (chanDecorations)
│   ├── selection.ts       selectionInRange / lineIntersect helpers
│   ├── marks.ts           bold/italic/strike/code/links/naked URL
│   ├── headings.ts        heading line classes + prefix hide
│   └── blocks.ts          lists, task lists, blockquote, hr, fences
├── widgets/
│   ├── wikilink.ts        atom for [[note]] + internal links
│   ├── image.ts           atom + drag-resize + #w=N / #left / #right
│   ├── date.ts            atom; click opens the calendar popover
│   ├── table.ts           GFM table grid atom (read-only)
│   ├── mermaid.ts         diagram atom for closed mermaid fences
│   ├── tag.ts             mark-based pill on #word
│   ├── mention.ts         mark-based pill on @@name
│   └── checkbox.ts        widget on `[ ]` / `[x]` task markers
├── bubbles/
│   ├── controller.ts      bubbleListener + high-prec bubbleKeymap
│   ├── triggers.ts        computeBubbleSpec (caret-context scan)
│   ├── types.ts           BubbleSpec / BubbleHandle
│   ├── anchor.ts          1x1 caret-anchored host element
│   ├── wiki.ts            wiki picker: search, `#` headings, `^`
│   │                       blocks (CAS-writes the anchor), create-note
│   ├── image.ts           image picker + upload
│   ├── image_drop.ts      editor-level drop/paste image upload
│   ├── heic.ts            HEIC -> WebP conversion before upload
│   ├── tag.ts             tag picker (graph tags)
│   ├── contact.ts         `@` contact / `@@` mention picker
│   └── empty_state.ts     shared empty / still-indexing states
├── overlays/
│   ├── date_popover.ts    month grid + format dropdown for date pills
│   └── preview_popover.ts read-only file preview (read mode, locked)
├── commands/
│   ├── format.ts          toggleBold/Italic/.../setBlockKind
│   ├── list.ts            Enter-continuation, indent/outdent (regex
│   │                       on the current line, not the syntax tree)
│   ├── date_macros.ts     @today / @date expansion
│   └── page_break.ts      @pagebreak / @break -> page-break atom
├── extensions/
│   ├── popover.ts         viewport-watching positioner
│   ├── wikiBlocks.ts      client-side ^block target parser
│   ├── image.ts           image src resolve/encode helpers
│   └── list_guide_visibility.ts  fade list guides after caret leaves
├── fold.ts                heading-only foldService + custom gutter
├── find.ts                pure scanMatches
├── bubble.ts              popover shell (openBubbleShell)
├── links.ts               wikiLinkToMarkdown, normalizeHref, etc.
├── dateFormats.ts         date catalog + matcher
├── breathing_room.ts      bottom padding + smooth scroll at EOF
├── click_caret.ts         dead-zone clicks still place the caret
├── caret_mapping.ts       source<->rendered caret map on mode flips
├── clipboard.ts           context-menu Cut/Copy/Paste (plain text)
├── paste_html.ts          HTML clipboard -> markdown (lazy turndown)
├── print.ts               print / export-to-PDF pipeline
├── external_links.ts      open http(s)/mailto/tel (browser or Tauri)
├── link_preview.ts        wiki-pill preview from the context menu
├── image_drag_indicator.ts  drop-line + badge during image drag-move
├── right_click_no_select.ts suppress CM6 right-click selection
├── csv.ts                 CSV/TSV parser + serializer
├── mermaid_render.ts      lazy mermaid loader/renderer
└── tools.ts               trailing-whitespace highlight/strip,
                            collapse-all-code-blocks
```

## Server contract

The editor calls these endpoints:

| Method | Path | Purpose |
|--------|------|---------|
| GET    | `/api/files/{path}`              | load file (`?stream=1` for large) |
| PUT    | `/api/files/{path}`              | autosave + `^block` anchors (CAS via `expected_mtime_ns`, 409 on conflict) |
| GET    | `/api/search/files`              | wiki picker |
| GET    | `/api/link-targets`              | wiki picker target list |
| GET    | `/api/resolve-link`              | wiki pill kind classification |
| GET    | `/api/headings/{path}`           | `[[file#` mode |
| GET    | `/api/contacts`                  | `@` picker, `@@` pill resolution |
| GET    | `/api/mentions`                  | mention rows in the contact picker |
| GET    | `/api/graph`                     | tag picker source |
| GET    | `/api/files`                     | image catalog |
| POST   | `/api/attachments`               | multipart upload (50MB cap) |
| WS     | `/ws`                            | watch events (self-writes filtered) |

## Out of scope

- In-cell table editing: the grid atom is read-only; edits happen in the
  revealed pipe/dash source.
- YAML highlighting inside frontmatter: the block is isolated and dimmed,
  the body is unstyled.
- Collaborative editing is not implemented.
