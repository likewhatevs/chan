# chan web: colors and themes

Single reference for the chan frontend color system. Two theme
axes, one canonical semantic palette, and a fixed syntax-highlight
palette for fenced code. Update this file in the same commit as
any change to `App.svelte`'s palette blocks, the editor theme
sheets under `web/src/editor/themes/`, or `web/src/editor/highlight.ts`.

## Two theme axes

The frontend has two independent theme dimensions. Both are
attributes on `<html>` and both can change at runtime via the
settings panel.

1. **Color scheme** (`data-theme="light"` or `data-theme="dark"`).
   Drives the entire CSS-variable palette: backgrounds, text,
   accents, pill hues, graph node hues, etc. Defined in
   `App.svelte` as a `:root` block (dark, the default) plus a
   `[data-theme="light"]` override block.

2. **Editor theme** (`data-editor-theme="github"`, `"google_docs"`,
   or `"word"`). Drives the editor surface only: body font,
   heading scale, code font, link color, code-block slab bg,
   table borders, blockquote rule. Defined in
   `web/src/editor/themes/{base,github,google_docs,word}.css` as
   `--chan-editor-*` variables. `base.css` declares neutral
   defaults; each named theme overrides under
   `[data-editor-theme="<name>"]` plus a nested
   `[data-editor-theme="<name>"][data-theme="dark"]` block.

The axes are orthogonal. Any combination of color scheme by
editor theme is valid (6 combinations total). Only the
color-scheme axis affects app chrome (panes, status bar, file
tree, panels, modals); the editor-theme axis is scoped to the
editor surface.

A third, fixed dimension is the **syntax-highlight palette** for
fenced code. It is GitHub Primer (light or dark, branched off the
color scheme) and is shared across all three editor themes so a
python snippet reads identically regardless of which document
chrome is active. See `web/src/editor/highlight.ts`.

## Canonical semantic palette

Each concept gets one hue across surfaces (graph node, file-tree
row, info-pane border, editor pill). Picking a hue per concept
means the same item reads the same color whether you see it in
the graph, the editor, or the inspector.

```
Concept         Hue         Why
-----------     --------    --------------------------------------
Document        Orange      Brand color (chan ensō logo)
Media / image   Purple      Hue-separated from documents
Tag (#)         Green       Hue-separated from media
Contact         Yellow      Warm warning family (distinct from doc)
Date / time     Grey        Neutral, low emphasis
Broken / error  Red         Standard alert family
```

## Resolved values per surface

```
Concept    Graph node       File tree / info   Editor pill (fg/bg)
---------  ---------------  -----------------  -------------------
Document   --g-doc          (default text)     --pill-wiki-*
Media      --g-img          (none today)       --pill-image-*
Tag        --g-tag          (none today)       --pill-tag-*
Contact    --g-contact*     --warn-text        --pill-contact-*
Date       n/a              n/a                --pill-date-*
Broken     n/a              n/a                --pill-broken-*
```

*Note: `--g-contact` is not yet defined; the graph currently
falls back to `--warn-text` directly for contact nodes. Add a
dedicated `--g-contact` token when the graph needs to diverge.

### Hex values

```
Token              Dark         Light       Concept
-----------------  -----------  ----------  -----------------
--g-doc            #ff8a3d      #c25a1f     Document hue
--g-img            #b07dff      #7a4cd8     Media hue
--g-tag            #6cd07a      #2f9444     Tag hue
--g-binary         #58a6ff      #0969da     Binary file hue (FILE blue)
--g-folder         #8e8e93      #6c6c70     Directory hue (neutral grey)
--warn-text        #e3b341      #9a6700     Contact / warning
--pill-wiki-fg     #ff8a3d      #c25a1f     Document pill
--pill-image-fg    #b07dff      #7a4cd8     Media pill
--pill-tag-fg      #6cd07a      #2f9444     Tag pill
--pill-contact-fg  #e3b341      #9a6700     Contact pill
--pill-date-fg     #98989d      #6c6c70     Date pill
--pill-broken-fg   #ff6961      #c93232     Broken link pill
```

Pill background variables (`--pill-*-bg`, `--pill-*-bg-hover`) are
alpha tints of the foreground hue at ~0.18 dark / ~0.12 light.

## Kind taxonomy

`web/src/state/kinds.ts` defines the unified taxonomy used by every
chip, tree icon, and (eventually) inspector header glyph. Three
families:

- **FileKind**: things that exist as files in the workspace.
  `document` | `contact` | `text` | `media` | `binary`.
- **EntityKind**: graph-only entities (tokens extracted from markdown
  bodies, no file backing). `tag` | `mention` | `date`.
- **ContainerKind**: `folder` (directory rows in the file tree).

`classifyEntry(entry)` / `classifyFile(path, serverKind?)` is the
single classifier. It applies the server-provided `kind`
discriminator first (today only `"contact"`), then falls back to
extension-based image / editable-text detection. `text` is reserved
here for the phase 2 widening that lets any non-binary file open in
the source-only editor; today's classifier returns `document` for
.md / .txt and `binary` for everything else.

`web/src/components/KindChip.svelte` is the single chip component.
Inspector headers pass `block` (flex:1 fill); the search results
list passes `compact` (smaller font + fixed-width column). `ghost`
and `dim` modify opacity for graph ghost rows and search filename-
match rows respectively.

### Per-kind mapping

```
kind        label       palette token       lucide icon
----------  ----------  ------------------  -----------
document    document    --g-doc             FileText
contact     contact     --warn-text         User
text*       text        --g-doc (alias)     FileCode
media       media       --g-img             Image
binary      binary      --g-binary          File
tag         tag         --g-tag             Hash
mention     mention     --warn-text         User
date        date        --text-secondary    Calendar
folder      folder      --g-folder          Directory
```

*Reserved. `classifyFile` does not emit `text` until chan-workspace
exposes the wider editable-text class (phase 2 of the editor
widening). Until then a non-markdown text file falls into `binary`.

A `mention` shares the contact palette by design: a resolved mention
points at a contact file, an unresolved mention is the same concept
without a backing file. Distinguishing the two is the role of the
inspector ("Contacts" section, dimmed pill state for unresolved),
not the chip.

## Functional and chrome variables

Color-scheme axis (`App.svelte`):

```
Group       Variable               Use
----------  ---------------------  --------------------------------
Surface     --bg                   Canvas
            --bg-card              Card / chip / inline-code bg
            --bg-elev              Elevated panel
            --inspector-bg         Right inspector panel
            --tab-active-bg        Active tab face
            --tab-inactive-bg      Inactive tab face
            --code-bg              Inline + fence default code bg
                                   (overridden by editor theme)
Text        --text                 Primary body
            --text-secondary       Secondary / dim
            --text-heading         Heading-specific tone
            --link                 Hyperlink color
Lines       --border               Hairline
            --separator            Resize-handle bar
            --separator-hover      Resize-handle hover
States      --hover-bg             Hover tint
            --selection-bg         Text selection
            --page-shade           Off-page tint when page width is
                                   capped
            --zebra-bg             Alternating tree row tint
            --smart-bg             Smart-suggest tint
            --pane-focus           Active pane outline
Functional  --accent               Success green
            --warn-text            Warning amber (= contact hue)
            --danger-text          Errors / destructive
            --info-text            Unsaved-buffer dot
Buttons     --btn-bg               Button face
            --btn-border           Button border
            --btn-hover            Button hover ring
```

Editor-theme axis (`web/src/editor/themes/*.css`):

```
Group       Variable                          Use
----------  --------------------------------  ----------------------
Body        --chan-editor-body-family         Body font
            --chan-editor-body-size           Body size
            --chan-editor-body-color          Body ink
            --chan-editor-bg                  Editor bg
Heading     --chan-editor-heading-family      Heading font
            --chan-editor-heading-color       Heading ink
            --chan-editor-h{1..6}-size        Size
            --chan-editor-h{1..6}-weight      Weight
            --chan-editor-h{1..6}-line-height Line height
            --chan-editor-h6-color            H6 override (dim)
            --chan-editor-h1-border-bottom    GitHub-style page rule
            --chan-editor-h1-padding-bottom   "
            --chan-editor-h2-border-bottom    "
            --chan-editor-h2-padding-bottom   "
Code        --chan-editor-code-family         Code font
            --chan-editor-code-size           Code size
            --chan-editor-source-size         Source-view size
            --chan-editor-inline-code-bg      Inline code bg
            --chan-editor-inline-code-color   Inline code ink
            --chan-editor-code-block-bg       Fence slab bg
            --chan-editor-code-block-color    Fence ink
            --chan-editor-code-block-border   Fence border
Inline      --chan-editor-link-color          Editor link color
Block       --chan-editor-quote-color         Blockquote ink
            --chan-editor-quote-border        Blockquote bar
            --chan-editor-hr-color            `---` rule color
            --chan-editor-table-border        Table borders
            --chan-editor-table-header-bg     Table header bg
            --chan-editor-table-stripe-bg     Table stripe bg
```

## Axis intersection

A fenced code block exercises all three layers:

```
                github theme      google_docs theme   word theme
light slab bg   #f6f8fa           #f8f9fa             #f7f7f7
dark slab bg    #151b23           #28292b             #1f1f1f
syntax (light)  Primer Light      Primer Light        Primer Light
syntax (dark)   Primer Dark       Primer Dark         Primer Dark
H1/H2 rule      yes (1px hr)      no                  no
```

Slab bg and the H1/H2 hairline rule track the editor theme; the
syntax palette only tracks the color scheme.

## Adding a new concept

1. Pick a hue family. Try to reuse an existing one (document
   orange, media purple, tag green, contact yellow, neutral
   grey, error red) before introducing a new hue. Each new hue
   has to defend its hue distance from the five already in use.
2. Add the dark + light hex to the two `App.svelte` palette
   blocks under domain-specific variable names
   (`--<concept>-fg`, `--<concept>-bg`, etc.).
3. Pipe the new variable into every surface that should display
   the concept (graph node, file tree row, info border, editor
   pill, etc.). Each surface reads its own variable name so a
   future hue swap is a one-line palette edit.
4. Add the row(s) to this document.

## Adding a new editor theme

1. Create `web/src/editor/themes/<name>.css`. Override only the
   `--chan-editor-*` tokens that should diverge from `base.css`;
   missing tokens fall through to the color-scheme palette.
2. Light goes under `[data-editor-theme="<name>"]`; dark goes
   under `[data-editor-theme="<name>"][data-theme="dark"]`.
3. Import the sheet in `web/src/main.ts` (or wherever the other
   editor themes are imported).
4. Register the option in the settings panel and the editor
   theme enum (`web/src/api/types.ts`).
5. Decide whether the theme wants the GitHub-style H1/H2 rule;
   opt in by setting `--chan-editor-h{1,2}-border-bottom` and
   `--chan-editor-h{1,2}-padding-bottom` (base.css defaults
   these to `none` / `0`).

The new theme inherits the GitHub Primer syntax-highlight palette
automatically; it is not part of the editor-theme contract.

## Source-of-truth pointers

- `web/src/App.svelte` palette blocks: color-scheme axis.
- `web/src/editor/themes/`: editor-theme axis.
- `web/src/editor/highlight.ts`: syntax-highlight palette
  (GitHub Primer, shared across editor themes).
- `web/src/editor/base.ts` `themeExtensions()`: how CodeMirror
  picks the highlight + chrome (cursor, gutter) per color scheme.
