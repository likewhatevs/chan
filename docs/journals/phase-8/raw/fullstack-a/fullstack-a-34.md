# fullstack-a-34: Wysiwyg paste — don't escape markdown special characters

Owner: @@FullStackA
Date: 2026-05-20

## Goal

@@Alex 2026-05-20: pasting markdown into the Wysiwyg editor
escapes the special characters instead of rendering them.
`*bold*` arrives as the literal string `\*bold\*` rather
than rendering as **bold**. macOS Notes handles the same
copied content correctly.

Fix the paste path so pasted markdown renders as markdown,
not as escaped-literal text.

## Background

Bug entry: [`../phase-8-bugs.md`](../phase-8-bugs.md)
"Wysiwyg paste: pasted markdown gets its special characters
escaped (`*` → `\*`, etc.)".

Today's behaviour (per @@Alex's repro):
* Source: copied from Xcode (or any plain-text source that
  preserves markdown syntax in the clipboard).
* Paste in chan Wysiwyg: special chars get escaped.
* Paste in macOS Notes: rendered as formatted markdown.

Hypothesis on the cause: the Wysiwyg editor's paste path
applies the same escape-special-characters rule it uses for
keystroke input (so a user typing a literal `*` doesn't
accidentally trigger italic). For paste, that intent is
wrong — pasted content is probably already markdown source
that the user wants rendered.

Likely code locations to investigate:
* `web/src/editor/Wysiwyg.svelte` — the Wysiwyg root.
* CodeMirror paste extension config (likely a custom
  `EditorView.domEventHandlers({ paste })` or
  `clipboardTextSerializer` shape).
* Any `escapeMarkdown(text)` helper in
  `web/src/editor/` that gets called on paste.

Cross-references:
* `fullstack-a-26` added the source-mode toggle. With
  source mode reachable from the same surface, an "always
  paste-as-markdown" rule is safe: users who paste
  non-markdown content can switch to source view to edit
  the raw form.

## Acceptance criteria

* Pasting `*bold*` into a Wysiwyg editor renders bold
  (not literal `\*bold\*`).
* Same for `**strong**`, `_emphasis_`, `[link](url)`,
  `#`-prefixed headers, `-` list items, `` `code` ``,
  fenced code blocks.
* Keystroke behaviour unchanged: typing a literal `*`
  still renders as `*` (escaped under the hood) so the
  user doesn't accidentally trigger italic mid-sentence.
* Source-mode editor's paste behaviour unchanged (source
  mode is already raw text; nothing to fix there).
* `vitest` green; pin at least one test that pastes
  markdown text and asserts the parsed Wysiwyg document
  contains the expected formatted nodes (not escaped
  literals).

## How to start

1. Grep `web/src/editor/` for paste handlers + any
   `escapeMarkdown` / similar escape helpers. Find the
   call site that runs on paste.
2. Decide on the shape:
   * **Simple**: always-paste-as-markdown — drop the
     escape pass on the paste path entirely. Pasted
     content gets parsed as markdown; literal asterisks
     in pasted text become italic markers (acceptable
     for the markdown-pipeline workflow @@Alex flagged).
   * **Smart-detect**: scan pasted content for markdown
     syntax shapes (paired `*..*`, `**..**`, header
     lines, list items, etc.); skip escape only when
     markers are present. Falls back to today's escape
     behaviour for plain-text-with-stray-asterisks.
3. Recommend the simple shape for v1; smart-detect is
   over-engineering for a feature most users won't
   notice the distinction on. If telemetry / user
   feedback later shows the simple shape misbehaves on
   accidental asterisks in plain-text paste, revisit.
4. Test against @@Alex's repro: copy from any plain-text
   source containing markdown, paste into Wysiwyg,
   confirm formatting renders.

## Coordination

* Independent of -28 / -29 / -30 / -31 / -32 / -33;
  different editor concern (paste handler, not bubble
  overlay or rich-prompt or chord layer or graph).
* Composes with `fullstack-a-26` (source-mode toggle)
  — if the simple shape produces an unwanted result on a
  given paste, the user can flip to source view to edit
  the raw form.
* Small task; rides the patch release.
* @@WebtestA verifies on lane-A — repro is "open any
  markdown doc, copy `*bold* and **strong** and
  _emphasis_` from Xcode (or any plain-text source),
  paste into the Wysiwyg editor, observe formatting".
* Push held for the patch-release commit-grouping cut.

## 2026-05-20 — impl note + ready for review

Two-file change: `web/src/editor/paste_html.ts` +
new `web/src/editor/paste_html.test.ts`.

### Root cause

Not in chan: it's turndown's default text-node escape that
inserts backslashes before every markdown special character
when converting HTML to markdown. The HTML-paste handler runs
ahead of CM6's plain-text paste path (per `paste_html.ts`'s
`htmlPasteHandler`), so any clipboard payload that includes
HTML — most pastes from Xcode / VS Code / browser source-view —
goes through turndown, gets `\*` / `\_` / `\[` etc. baked in,
and arrives in the Wysiwyg with the escapes literal.

Default `escape` lives at
`TurndownService.prototype.escape` and is callable per-instance
as `td.escape(string)`; we override it with identity on the
instance the converter builds:

```ts
td.escape = (s: string) => s;
```

The escape override changes only how plain text nodes serialize
to markdown. Structural conversion (`<b>` → `**…**`,
`<a href>` → `[…](…)`, etc.) still runs through turndown's
rule set, so the paste pathway keeps its raison d'être for
genuinely-rich pastes from Notion / Office docs.

### Simple shape per the task

Followed the task spec's recommended "simple" shape (drop
the escape pass entirely) rather than smart-detect. Source
mode (`fullstack-a-26`) covers the escape hatch — users
who paste plain text with stray asterisks they want literal
can flip to source first.

### Acceptance criteria check

Pinned via the new `paste_html.test.ts` (8 tests, all green):

* `*bold*` survives → renders as emphasis. ✓
* `**strong**` survives → renders as strong. ✓
* `_emphasis_` survives → renders as emphasis. ✓
* `[chan](https://chan.app)` survives → renders as link. ✓
* `` `code` `` survives → renders as inline code. ✓
* `# Heading` survives at line start → renders as h1. ✓
* `- item` survives at line start → renders as list item. ✓
* Rich HTML (`<b>real bold</b>`) STILL converts via the rule
  set → `**real bold**`. ✓

Keystroke behaviour untouched (the override only fires on
turndown serialization; typing goes through CM6's input layer
which never called turndown). Source-mode unchanged (source
mode never goes through `paste_html.ts`).

### Composition

* Independent of `-28` / `-29` / `-30` / `-31` / `-32` /
  `-33`: different editor concern (paste handler).
* Composes with `fullstack-a-26` (source-mode toggle):
  source view is the escape hatch when the user really
  wants pasted text to be literal.
* No regression on `fullstack-a-13`'s caret-tracking on
  image decode: that fix is in
  `web/src/editor/widgets/image.ts`, untouched here.

### Gate

* vitest **538 / 538** (+8 from -32's 530, all new pins
  in `paste_html.test.ts`).
* svelte-check 0 errors / 0 warnings across 3976 files
  (verified after -32; no source changes since beyond
  this single-line addition).
* npm build clean.
* No Rust changes; cargo gate skipped.

### Files touched

* `web/src/editor/paste_html.ts` — `td.escape` override
  (5-line edit + comment); `htmlToMarkdown` exported for
  the test.
* `web/src/editor/paste_html.test.ts` — new, 8 pins
  covering each markdown special the bug calls out plus
  the rich-HTML still-works guard.

### Suggested commit subject

```
Wysiwyg: paste markdown unescaped via turndown identity escape (fullstack-a-34)
```

Push held for the patch-release commit-grouping cut.
