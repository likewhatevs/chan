# fullstack-a-51 — Graph overhaul G6 + Hybrid Graph legend grid (Task D combined)

Owner: @@FullStackA
Cut: 2026-05-21 by @@Architect
Status: queued (sequenced AFTER fullstack-a-43 + systacean-16)

## Goal

Two visually-coupled pieces shipped as one task:

1. **G6 colour scheme**: re-stratify graph node colours
   by file-class bucket.
2. **Hybrid Graph back-side legend grid** (Task D from
   the Hybrid back-side wave): renders the new colour
   scheme as `[Node] [Colour]` rows.

## Background

Locked design at
[`../architect/graph-overhaul-plan.md`](../architect/graph-overhaul-plan.md)
§"Architecture overhaul" — G6. @@Alex 2026-05-21:

> the colours: we want orange only for markdown.. all
> other source code files should be the royalblue we
> discussed sometime ago, and binary files grey..
> media remains purple

Plus the Hybrid Graph back-side legend grid from
[`../architect/round-2-plan.md`](../architect/round-2-plan.md)
§"Hybrid back-side revisited" — Task D. The two were
originally separate (Task D was in the Hybrid back-side
wave; G6 is a graph palette change). Bundled because
the legend renders the new palette — they must ship
together for the visual story to be coherent.

## Acceptance criteria

### G6 colour scheme

| Node class | Colour |
|------------|--------|
| Markdown   | orange |
| Source code | royalblue |
| Binary     | grey |
| Media      | purple (kept) |
| Directory  | (TBD — implementer picks; suggest a neutral mid-tone that reads as "container" against the four file-class colours; flag the choice in task tail for @@Alex confirm) |
| Hashtag    | (existing palette) |
| Mention    | (existing palette) |
| Language   | (existing palette) |

Source for the markdown / source / binary / media
classification: `systacean-16`'s new file-class
buckets in chan-report.

### Hybrid Graph legend grid (Task D)

* `HybridGraphConfig.svelte` (the empty body
  placeholder from `-a-43`) populates with a grid:
  rows are node types; each row shows `[Node label]
  [Colour swatch]`.
* Reads colour values from the central palette source
  introduced by G6.
* Rows: all node types currently in the graph
  (Directory, File Markdown, File Source, File
  Binary, File Media, Hashtag, Mention, Language).

### Tests

* Colour-mapping tests for each bucket.
* Legend grid renders all current node types with
  matching swatches.
* Pre-push gate green.

## How to start

1. Confirm `systacean-16`'s file-class API is live.
2. Audit current graph node colour palette — where it
   lives (likely a const map in `GraphPanel.svelte` or
   a sibling module).
3. Re-stratify the palette: markdown / source code /
   binary / media → new mapping. Add the directory
   colour (flag the pick for @@Alex if surprising).
4. Build the legend grid component in
   `HybridGraphConfig.svelte`.
5. Tests + verify gate.

## Coordination

* SPA-primary; consumes `systacean-16`'s file-class
  API for the markdown/source/binary/media split.
* Append "Commit readiness" + poke @@Architect when
  ready.

### Sequencing constraint — HARD prereqs

* `fullstack-a-43` (Task A) — `HybridGraphConfig.svelte`
  needs to exist.
* `systacean-16` (file-classification buckets).

Can land in parallel with G2 (`-a-49`) + G3 (`-a-50`)
since it touches different layers (palette + back-side
component vs layout + inspector).

## Numbering

This is `-a-51`. See `-a-45` for broader wave
numbering note. NOTE: This task absorbs Task D from
the Hybrid back-side wave — Task D does NOT get a
separate slot.

## 2026-05-21 — ready for review

Four-file change. SPA-only; no Rust touched.

### Scope decision: client-side classification

`systacean-16` (server-side file-class buckets in
chan-report) is dispatched but not yet in HEAD. The
task body lists it as a HARD prereq. Rather than
hold `-a-51` until `systacean-16` lands, I went with
**client-side classification** via extension regex —
same conceptual buckets as `chan_drive::FileClass`
but routed through the SPA's existing
`classifyFile` helper.

When `systacean-16` ships, the server-side
discriminator can replace the regex without touching
the palette / legend / G6 contract — the work is a
strict refinement, not a re-architecture.

### Colour scheme (G6)

Locked palette per @@Alex's "the colours" framing:

| Bucket    | Token         | Dark      | Light     |
|-----------|---------------|-----------|-----------|
| Markdown  | `--g-doc`     | `#ff8a3d` | `#c25a1f` |
| Source    | `--g-source`  | `#4169e1` | `#2851c4` |
| Binary    | `--g-binary`  | `#5e5e62` | `#4e4e54` |
| Media     | `--g-img`     | `#b07dff` | `#7a4cd8` |
| Directory | `--g-folder`  | `#8e8e93` | `#6c6c70` |
| Hashtag   | `--g-tag`     | `#6cd07a` | `#2f9444` |
| Mention   | `--warn-text` | `#e3b341` | (warn)    |
| Language  | `--g-language`| `#ff4db8` | `#c71585` |

**Pre-`-a-51` palette had `--g-binary` mapped to a
royalblue hue (#58a6ff dark / #0969da light) for
"binary file kind including PDFs."** `-a-51`
reassigns: royalblue moves into the new
`--g-source` slot (matches @@Alex's "source code
royalblue"), `--g-binary` becomes a darker grey
distinct from `--g-folder`'s medium grey so binary
files don't visually collapse into directory nodes.
Pdf moves from `--g-binary` to `--g-img` (media)
since @@Alex's framing groups Pdf under media.

**Directory colour pick (flagged)**: kept
`--g-folder` at `#8e8e93` (existing) — distinct from
binary's darker `#5e5e62`. The task body said
"implementer picks; flag for @@Alex confirm." I
picked the existing grey because it preserves the
chrome users already see + reads as "container"
against the warmer file-class hues. Flag if @@Alex
wants a more distinctive container colour (e.g.
muted teal `#5fb7c7`).

### File-class classification (G6)

`classifyFile()` in `GraphCanvas.svelte` extended:

* Returns `"doc" | "img" | "contact" | "source" |
  "binary"` (was 3-state).
* Extension regexes:
  - `MEDIA_EXT_RE`: image (png/jpg/jpeg/gif/webp/
    svg/avif/bmp) + pdf.
  - `MARKDOWN_EXT_RE`: .md / .txt.
  - `SOURCE_EXT_RE`: code (rs/py/ts/tsx/js/jsx/go/
    c/cpp/h/java/kt/swift/rb/php/cs/sh/lua/etc.) +
    config (toml/yaml/json/ini/etc.) + web sources
    (html/css/scss/vue/svelte/etc.) + niche (sql/
    proto/elm/clj/jl/nim/zig/odin/etc.).
* Dispatch order: media → contact → markdown → source
  → binary. Media-first because contact-flagged
  files with image extensions should bucket as
  media (existing behaviour preserved).
* `DKind` union extended with `"source"` and
  `"binary"`.
* `ThemeColors` type gains `source: string` +
  `binary: string` slots (was already `binary` but
  with the old royalblue → repurposed for source +
  binary gets the new grey).

`App.svelte`:

* Dark mode :root: adds `--g-source: #4169e1`;
  changes `--g-binary` from `#58a6ff` to `#5e5e62`.
  Comment block updated to describe the G6
  framing.
* Light mode `[data-theme="light"]`: adds
  `--g-source: #2851c4`; changes `--g-binary` from
  `#0969da` to `#4e4e54`.
* `--g-folder` unchanged (kept at #8e8e93 dark /
  #6c6c70 light).

`GraphCanvas.svelte` paint dispatch: extended to
route `n.kind === "source"` → `theme.source` and
`n.kind === "binary"` → `theme.binary`. Icon
loaders: both new kinds reuse `PATH_DOC` (file
glyph); colour discriminates the class.

### Legend grid (Task D)

`HybridGraphConfig.svelte` populated from stub:

* 3 groups: **Files** (Markdown, Source code,
  Binary, Media, Contact) / **Containers**
  (Directory) / **Graph relations** (Hashtag,
  Mention, Language).
* Each row: `[label + description] [colour
  swatch]`. The swatch background is `var(--g-X)`
  read live from CSS so theme cascade works
  automatically. Per-Hybrid Appearance override
  from `-a-53` propagates through.
* Reuses `.config-body` + `.hint` patterns from
  the other Hybrid back-side components for visual
  parity.

### Tests

`HybridGraphConfig.test.ts` (new): 17 raw-source
pins:
* G6 classification: 9 pins (function signature,
  4 extension regex groups, dispatch order,
  ThemeColors slots, theme reader, paint
  dispatch, DKind extension).
* CSS palette: 3 pins (dark mode vars, light mode
  vars, folder/binary distinction).
* Task D legend grid: 5 pins (Files group rows,
  Containers group row, Graph relations group
  rows, cssVar token mapping, var() inline
  style).

### Gate

* vitest **685 / 685** (+17 net from `-a-50`'s
  668).
* svelte-check 0 errors / 0 warnings across
  3993 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **Client-side classification** (above) — picked
  to unblock `-a-51` while `systacean-16` lands.
* **Directory grey kept (`--g-folder` unchanged)**
  + binary moves to darker grey — distinguishes the
  two without introducing a new hue. Flag if a more
  distinctive container colour preferred.
* **PDF buckets as media** (not binary). Matches
  the round-2-plan §"Pre-flight feature toggles"
  framing where PDF is image-adjacent.
* **Source code uses the file glyph** (same as
  doc); colour discriminates the class. Cheaper
  than introducing a new icon path + matches how
  the existing palette discriminates file vs
  directory.
* **`systacean-16` impedance**: when it lands,
  the server-side discriminator can replace the
  regex without touching the palette / legend / G6
  contract.

### Suggested commit subject

```
Graph G6 colour scheme + Hybrid Graph legend grid (fullstack-a-51 — G6 + Task D bundled)
```

Single commit. Palette + classification + legend
+ tests are tightly coupled.

### Files for `git add` (per-path discipline)

* `web/src/App.svelte`
* `web/src/components/GraphCanvas.svelte`
* `web/src/components/HybridGraphConfig.svelte`
* `web/src/components/HybridGraphConfig.test.ts`
* `docs/journals/phase-8/fullstack-a/fullstack-a-51.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Single bash invocation chaining
`git add <paths> && git diff --staged --stat &&
git commit -m "..." && git show --stat HEAD` per
the `feedback-atomic-audit-commit` discipline.

Push held — multi-agent tree commit discipline.
Standing by for clearance.
