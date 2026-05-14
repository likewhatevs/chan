# Editor UX

How the WYSIWYG editor in `web/` is meant to behave. This document is
the source of truth for the live-preview interaction model. Match it
when reading or changing the editor; flag deviations explicitly.

## Core principle

The editor is **live-preview**: any line or inline element shows its
rendered form when the cursor is elsewhere, and reveals its markdown
source when the cursor enters it. Moving the cursor away hides the
source again. No mode toggle, no separate "source view" pane.

This mirrors Obsidian's Live Preview model. The user types markdown
freely; the editor renders it on the fly and re-shows the markers as
soon as the user needs to edit them.

## Per-element behavior

### Headings

- WYSIWYG: large/bolded heading line, fold chevron on the left.
- Cursor on the line: `#`, `##`, ... prefix appears in a muted color
  at the start of the line. Chevron stays.
- Cursor leaves: prefix hides again.
- Typing/erasing `#` characters works as plain text edit; the heading
  level follows the marker count.

### Wiki links `[[...]]`

- WYSIWYG: rendered as a styled pill with the target's display label.
- Cursor enters: `[[` and `]]` brackets become visible around the
  label; the label inside stays editable.
- Typing inside the brackets reopens the SAME search popup that opens
  when the user originally types `[[` in a paragraph. Result list,
  preview, and `Type # / Type ^ / Type |` hint row are identical.
- If the user breaks a marker (e.g. deletes one `]`), the editor
  serializes the literal text and renders it broken. No auto-repair.

### Tags `#tag`

- Same flow as wiki links: typing `#` opens a search popup over the
  drive's existing tags, ranked by frequency (top tags surface first).
- WYSIWYG: rendered as a styled pill.
- Cursor on the pill: reveals the `#` and lets the label edit.
- Click on a `#tag` pill: opens the graph view filtered to that tag.

### Mentions `@contact`

- Same flow as wiki links and tags: typing `@` opens a search over the
  contacts API.
- WYSIWYG: rendered as a styled pill.
- Cursor on the pill: reveals `@` and lets the label edit.
- Click on an `@mention` pill: opens the graph view focused on the
  contact (mirrors `#tag` click behavior).

### Images `![alt](src)`

- Typing `![` opens a bubble similar to `[[`, but image-specific:
  a search over drive images with a thumbnail preview for the
  highlighted result, plus an "upload image" button.
- Upload writes the file to the directory of the markdown being
  edited (drive-relative `./name.png`). On filename collision, the
  server appends `-1`, `-2`, ... until unique.
- Pasting an image from the clipboard: same upload-to-current-dir
  path; inserts `![](./name.png)` at the caret.
- WYSIWYG rendering, when an image shares the line with text
  (e.g. `foo ![alt](src) bar`): image bottom-aligned with the text
  baseline, so the text sits on the image's bottom edge.
- Optional width is encoded in the URL fragment: `#w=N` (pixels).
  Other renderers ignore the fragment.

### Image interactions

- Click on an image: shows two action buttons floating on the
  rendered image: "zoom" and "edit".
- Cursor navigation (arrow keys) onto an image: jumps directly to the
  "edit" state, not "zoom". The image is treated as a single cursor
  position; arrowing past it deselects.
- "Edit": reveals the image's markdown inline (`![alt](src#w=N)`),
  places the caret at the start of the markdown, and selects the
  block. Moving the caret deselects. The markdown stays revealed
  while the caret is inside it; leaving collapses back to the image.
- Editing the `alt` text is plain text editing.
- Editing the `src` opens a search dropdown anchored to the markdown
  with image-result previews (same shape as the insert flow). When
  the path doesn't resolve, render an inline error row under the
  markdown: `"<path>" could not be found.`

### Calendar shortcuts `!/today`, `!/date`

- Typing `!/today` or `!/date` opens a calendar picker over the
  supported formats. The picker is **editor-only**: it inserts a
  plain date string into the source. The markdown contains no
  special marker.
- Dates are never indexed. No graph edges, no FTS column, no
  per-date search. (See the chan-core task at the bottom.)

### Lists `- item`, `1. item`, `- [ ] task`

- WYSIWYG: source markers (`-`, `*`, `+`, `1.`, `1)`) stay visible on
  every list line. Task items render the GFM checkbox via the
  `Task` widget; the `[ ]` / `[x]` source is replaced by a clickable
  box and reappears when the caret enters the line.
- Enter at end of a list line: inserts a fresh marker on the next
  line. Bullets reuse the line's marker char; ordered lists
  increment the number and keep the original `.` / `)` separator;
  task items always start as `- [ ] ` regardless of the source
  line's checked state.
- Enter on an empty list item (just the prefix, no content): strips
  the prefix entirely. This is how the user exits the list.
- Enter mid-line on a non-empty item: falls through to a literal
  newline. Auto-continuing mid-sentence would split a paragraph
  with a stray bullet.
- Tab on a list line: indents the item by 2 spaces. Multi-line
  selections indent every list line in range; non-list lines in the
  range stay untouched.
- Shift-Tab: outdents one level (strips up to 2 leading spaces)
  when the line is already indented; no-op at the top level so the
  default Shift-Tab behavior is preserved.

### Inline formatting `**bold**`, `*italic*`, `~~strike~~`

- WYSIWYG: rendered with the appropriate style.
- Cursor enters the span: the `**` / `*` / `~~` markers become
  visible around the styled text. Markers stay until the cursor
  leaves.
- Cmd/Ctrl + B, I work as today. **No underline.**

## Cross-cutting rules

- **One bubble pattern.** Wiki, tag, mention, and image bubbles
  share the same keyboard model (Arrow Up/Down to navigate, Enter
  to commit, Esc to dismiss, click to commit). Each bubble owns
  its own results / preview content, but the interaction is
  uniform. Anchored under the caret; flips above when out of room.
- **Broken markdown is preserved.** If the user deletes part of a
  marker, the source keeps what the user typed; the renderer just
  fails to recognize the construct and shows the text plainly.
  Never auto-repair.
- **Last-line `---`.** When the file's last line is `---`, the user
  must still be able to land the caret on it (revealing `---` per
  the principle) and press Enter to create a new line below. The
  current renderer traps the caret above; this needs to be fixed.

## Out of scope (intentional)

- No underline.
- No date indexing, date search, or date graph edges. Dates are an
  editor convenience for typing absolute dates fast.

## Companion task: chan-core date tokens

There is residual date-extraction code in chan-core that should be
removed once the principle above lands. Inventory at the time of
writing:

- `crates/chan-drive/src/markdown/tokens.rs:1-12` — header doc
  about date tokens
- `crates/chan-drive/src/markdown/tokens.rs:22` — `Token::Date`
  enum variant
- `crates/chan-drive/src/markdown/tokens.rs:65-79` — date pattern
  match + token emission
- `crates/chan-drive/src/markdown/tokens.rs:182-248` — date tests
- `crates/chan-drive/src/drive.rs:1306-1309` — explicit
  `Token::Date` skip in `build_edges`

The skip already prevents date tokens from polluting the graph, so
no behavior changes in production today. Delete the variant + tests
when convenient to drop the carrying cost.
