# webtest-a-7 — -a-57 walkthrough (graph filter chips: markdown + source FileBucket toggles)

Owner: @@WebtestA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Walk `fullstack-a-57` (`f5c10c8`) — the graph filter chip
extension. Two new chips (markdown + source) added; both
default-on; toggle behaviors + counts + persistence
across URL hash + SerTab.

## Reference

* Task body: [`../fullstack-a/fullstack-a-57.md`](../fullstack-a/fullstack-a-57.md)
  + the @@FullStackA commit-readiness append.
* Bug entry: [`../phase-8-bugs.md`](../phase-8-bugs.md)
  "Graph filter chips don't include FileBucket".
* Original ask: @@Alex 2026-05-22 — "i would like to
  hide [markdown] so I can actually see the source code"
  + earlier "where is the source code because there's too
  much orange in there".

## Acceptance

### Chip presence + defaults

1. Open the graph view; confirm the filter chip row
   includes a **markdown** chip + a **source** chip
   alongside the existing tag/contact/language/media/folder
   set (7 chips total).
2. Both new chips default ON; existing chips' defaults
   unchanged.

### Toggle behaviors

3. **markdown OFF → source visible**: toggle markdown
   OFF; confirm markdown file nodes (the orange) vanish;
   source code file nodes (royalblue per `-a-51`'s G6
   palette) become visually prominent. Take screenshot
   of before + after — this is the "hide markdown to
   see source" win @@Alex asked for.
4. **source OFF → markdown visible**: toggle source
   OFF (markdown back ON); confirm royalblue source
   nodes vanish; orange markdown nodes remain.
5. **Both OFF**: only non-file kinds remain visible
   (folders, tags, mentions, languages, etc.).
6. **Both ON (default)**: all file nodes visible.

### Counts

7. Each chip displays a count; counts should reflect
   the actual node populations (per the chan-source
   seed: ~567 markdown / ~340 source if the test drive
   is the chan repo).

### Persistence

8. Toggle markdown OFF + reload the page; URL hash
   should persist the state; chip stays OFF after
   reload.
9. Switch tabs (open a second graph tab); SerTab
   round-trip preserves the chip state per-tab.

### Walkthrough audit trail

Append a fresh dated heading to
[`webtest-a-1.md`](webtest-a-1.md):
`## 2026-05-22 — fullstack-a-57 walkthrough (graph
filter chips: markdown + source FileBucket toggles)`.
Capture verdicts + screenshots + side observations.

## How to start

1. `git status` clean; `git log --oneline -5` confirms
   `f5c10c8` in HEAD.
2. Rebuild chan (web/dist may be stale relative to
   `-a-57`): `cd web && npm run build && cd ..`;
   `cargo build -p chan`.
3. Spin up test server; chan-source seed drive.
4. Open graph; walk the 9 acceptance checks.
5. Append verdict; fire poke to
   `event-webtest-a-architect.md`.
6. Tear down per the standing rule.

## Coordination

* @@WebtestA lane (reactive).
* Standing terminal + Chrome MCP perm covers the walk.
* Light walk; ~20-30 min.

## Numbering

Highest committed `webtest-a-N` is `-6`. This is `-7`.

## Out of scope

* The OTHER 4 in-flight FullStackA tasks (`-a-56`,
  `-a-58`, `-a-59`, `-a-60`) — walked separately when
  they land.
* Sub-language picker (deferred per `-a-57` decision).
* Graph parent-edge invariant (separate task `-a-58`).
