# fullstack-a-57 — Graph filter chips: add FileBucket toggles (markdown / source) + sub-language picker

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Add bucket-based filter chips to the graph view so users
can hide markdown documents to see the source-code
population (or vice versa). Surfaces the `FileBucket`
data from `systacean-16` that's already in the wire
shape via `/api/report/file` + the graph nodes.

## Reference

[`../phase-8-bugs.md`](../phase-8-bugs.md) "Graph filter
chips don't include FileBucket (Markdown / SourceCode);
user can't hide markdown to see source" — full bug body
with audit-confirmed file pointers + counts framing.

## Scope

### Minimum

Two new chips alongside the existing `tag` / `mention` /
`language` / `img` / `folder` set:

* `markdown` chip — toggles visibility of file nodes
  with `FileBucket = Markdown`.
* `source` chip — toggles visibility of file nodes with
  `FileBucket = SourceCode { language }` (collective
  toggle for all languages).

### Stretch

When `source` chip is ON, expose a small sub-picker (or
collapsible row) of per-language toggles (rust / ts /
svelte / py / etc.) so user can hide TypeScript while
showing Rust, etc. Implementer's call: if sub-picker
adds significant UX complexity, ship the collective
toggle in this task + flag sub-picker for a follow-up.

### Wire glue

Audit needed: the `FileBucket` field already lands in
the graph node payload via `-16` + the SPA's existing
endpoint consumption (per the proactive walk
`a63c8cb` verdict). Confirm the bucket field is on the
graph-node payload OR add it if missing. If chan-server
emit needs a small extension, fire scope poke + I
route to @@Systacean.

## Acceptance

1. **markdown chip exists + functions**: toggle OFF →
   all markdown file nodes hidden; toggle ON → visible.
2. **source chip exists + functions**: toggle OFF → all
   source-code file nodes hidden; toggle ON → visible.
3. **Combined**: toggle markdown OFF + source ON → only
   source nodes + non-file kinds (tags, mentions,
   languages, folders, etc.) visible.
4. **Counts in chips**: each chip shows its current
   node count (mirror the existing 5 chips' shape).
5. **(Stretch)** sub-language picker under source if
   shipped.

### Tests

Vitest pins for chip toggle behavior; count rendering;
node visibility under combined toggles.

### Gate

* `npm test -- --run` green.
* `npm run check` 0e/0w.
* `npm run build` clean.

## Coordination

* @@FullStackA lane primary.
* If chan-server-side bucket emit gap surfaces at
  pickup: fire scope poke → @@Architect routes to
  @@Systacean (small).
* Atomic-audit-commit discipline.

## Authorization

**Yes** for `web/src/components/GraphPanel.svelte` +
test files + task tail + outbound.

If chan-server emit needs a small additive change:
**fire scope poke first**; don't expand into chan-server
unilaterally.

## Numbering

This is `-a-57`.
