# frontend-4: drive-rooted graph, inspector enrichment, royal-pink color

Owner: @@Frontend
Status: REVIEW

## Decisions locked by Alex 2026-05-18

* Royal-pink hex: `#C71585` light / `#FF4DB8` dark. Token
  `--chan-color-language` (alias `--chan-color-code`).
* "Graph from here" replaces "Graph this" across **all** surfaces
  (file tree row context menu, file editor menu, graph overlay
  openers, empty-pane menu). Default scope is drive.
* Frontmatter kinds beyond `contact` are next phase; this lane only
  wires the existing `contact` renderer via the registry from
  [backsystacean-4](./backsystacean-4.md).

## REVIEW backend payloads to consume

* [backsystacean-2](./backsystacean-2.md): `PathClass` on
  `/api/files` (TreeEntryView + FileResponse) and `/api/fs-graph`
  with permission / link metadata. Read-only directory dead-end
  behavior already in fs_graph.
* [backsystacean-3](./backsystacean-3.md):
  `/api/inspector?path=<rel>` returns
  `kind: drive | directory | markdown | text | media | binary |
  special` plus `path_class`, `report_file`, `report_summary`,
  `subtree` counts. Empty path returns drive-root payload.
* [backsystacean-4](./backsystacean-4.md): `CHAN_KIND_REGISTRY`
  with `contact` only; render contact pill through registry-driven
  lookup.

## Goal

Implement the architectural refresh from [request.md](./request.md)
on the web side: graph defaults to drive scope, every inspector
surface (file browser, graph, search) carries the new file-classifier
and chan-report payloads, and the language chip / graph language
ring uses the royal-pink token.

## Relevant links

* Request: [request.md](./request.md)
* Design memo: [architect-2.md](./architect-2.md)
* Journal: [journal.md](./journal.md)
* Inspector components: `web/src/components/Inspector.svelte`,
  `FileInfoBody.svelte`, `DriveInfoBody.svelte`
* Graph: `web/src/components/GraphPanel.svelte`,
  `web/src/components/GraphCanvas.svelte`
* Scope state: `web/src/state/scope.svelte.ts`
  (`defaultScopeId`)

## Scope

### Drive-rooted graph default

* `defaultScopeId()` (or the call sites that derive it) returns
  the drive scope when no preference exists.
* Every entry into the graph that does not specify a scope
  resolves to drive:
  * Empty-pane menu "Graph this" with no current file.
  * Graph overlay opened from a global shortcut.
  * Hash-restored graph state with a missing or stale scope id.
* Per-file / per-dir "Graph this" actions still scope to that
  file / dir on click. Wording stays "Graph this" unless Alex
  picks "Graph from here" per [architect-2](./architect-2.md)
  open question 2.
* Locked / read-only directories render as dead-end nodes (no
  subtree expansion). Permission flag comes from the classifier
  payload @@Backsystacean ships in
  [backsystacean-1](./backsystacean-1.md).
* Symlinks pointing outside the drive: render but do not traverse.

### Inspector enrichment

Across file browser, graph, and search:

* Drive inspector: existing widget + new sections from
  [backsystacean-2](./backsystacean-2.md) (language breakdown,
  file kind counters, total bytes, total file count).
* Directory inspector: same shape as drive, scoped.
* File inspector subcases:
  * Markdown: existing + frontmatter kind badge if present (use
    the renderer registry from [backsystacean-3](./backsystacean-3.md);
    the contact pill is the reference renderer).
  * Text: language chip, chan-report data lines, byte size, line
    count, encoding, mtime.
  * Binary: byte size, kind from extension sniff, mtime. No
    content read.
* Special-file flags (symlink / hardlink / read-only) surface as
  small badges in the inspector. The graph dead-end rendering is
  handled in the graph component, but the inspector should still
  state the reason.

### Royal-pink color token

* Add `--chan-color-language` to the design tokens (look in
  `web/src/` for the existing palette definitions; the existing
  tag green is the placement reference).
* Light mode: `#C71585`. Dark mode: `#FF4DB8`.
* Aliased as `--chan-color-code` for component templates that read
  better with that name; same value.
* Apply to:
  * Language chips in inspectors (drive / dir / text file).
  * Graph: the language ring or stroke that today picks up the
    tag green; switch to the language token.
* Confirm against the live palette on the test service before
  commit; propose alternates if either hex looks off in context
  and flag in [architect-2](./architect-2.md) open question 1.

## Out of scope

* The classifier itself (in [backsystacean-1](./backsystacean-1.md)).
* The chan-report aggregation (in [backsystacean-2](./backsystacean-2.md)).
* The frontmatter registry (in [backsystacean-3](./backsystacean-3.md)).
* Terminology codemod for "folder" -> "directory"
  (in [frontend-5](./frontend-5.md)).

## Acceptance criteria

* "Graph this" with no explicit scope lands on drive across every
  entry point.
* Inspectors show the new payload shape for drive / dir / markdown /
  text / binary, with permission / symlink / hardlink badges where
  applicable.
* Royal-pink language token is the source of language chip color
  and the graph language ring; tag green is no longer reused for
  language anywhere.
* Read-only dir nodes do not expand subtree edges; symlink-outside-
  drive nodes render but do not traverse.

## Tests

* Vitest coverage for default-scope resolution, inspector body
  rendering across subcases (snapshot for the chip set is fine if
  it stays compact), graph dead-end rule.
* Visual confirmation on the test service: graph default scope,
  language chip color across light + dark, drive breakdown.
* `npm --prefix web run check` clean.
* `npm --prefix web test -- --run` green.
* `npm --prefix web run build` clean.

## Review and hardening

* @@Frontend self-review for token application coverage
  (`git grep` for the old green class names is a quick way to
  audit).
* @@WebtestA + @@WebtestB live verification on the test service
  per [webtest-1](./webtest-1.md) and [webtest-2](./webtest-2.md).

## Dependencies

Blocked-by: [backsystacean-1](./backsystacean-1.md),
[backsystacean-2](./backsystacean-2.md),
[backsystacean-3](./backsystacean-3.md) need to land or expose
enough of the payload shape that this lane can integrate against.
Parallel scaffolding of the UI is fine.

## Progress notes

* Added locked royal-pink tokens:
  `--chan-color-language` / `--chan-color-code`
  (`#FF4DB8` dark, `#C71585` light), and routed `--g-language`
  through them.
* Updated GraphCanvas language fallback colors to the dark
  royal-pink value.
* Replaced user-facing `Graph this` labels with `Graph from here`
  across file tree, editor, inspector, graph, tag, and search-status
  surfaces.
* Added typed `/api/inspector?path=<rel>` client support.
* Shared file inspector now consumes inspector payloads across file
  browser, graph, and search surfaces.
* Surfaced classifier badges for read-only paths, symlinks, special
  files, hardlinks, and outside-drive targets.
* Directory inspectors use backend subtree counts/bytes/file-kind
  counters when available, falling back to the tree-derived counts.
* Existing chan-report sections remain in place; the inspector
  payload supplements the UI without removing the COCOMO roll-up.

## Completion notes

Verification:
* `rg -n "Graph this|#d66bff|#9b2bbf" web/src` returns no matches.
* `npm run check` in `web` passed.
* `npm test -- --run` in `web` passed: 18 files, 170 tests.
* `npm run build` in `web` passed with existing chunk-size warnings.
