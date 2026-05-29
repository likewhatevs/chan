# fullstack-a-1: File browser tab name = parent dir with trailing slash

Owner: @@FullStackA
Date: 2026-05-19

## Goal

Make the File Browser tab title always show a directory (with
trailing slash), not the currently-selected file's name. When the
selected item is a file, use the parent directory's name. When
selection is at the drive root, use the drive's display name.

## Background

Source bug in
[`../phase-8-bugs.md`](../phase-8-bugs.md) (item 1).

Repro: open File Browser, create `foo.md`, see tab name follows
file, delete `foo.md` → tab still labelled `foo.md` (stale).

## Acceptance criteria

* FB tab title is always a directory path; trailing slash always
  rendered.
* File selection → tab title = parent dir of file.
* Selection at drive root → tab title = drive's display name.
* **Default drive name** is derived from the registered path,
  e.g. `~/dev/foo/bar` → `bar`. Coordinate with @@Systacean if
  the registry side needs a default-derivation tweak; the SPA
  side is the priority.
* No stale-name flash after a delete.

## How to start

1. Locate the FB tab title computation in `web/src/components/`
   or `web/src/state/tabs.svelte.ts`.
2. Replace the "title follows selection" code path with a
   parent-dir resolver.
3. Confirm cross-tab behaviour with phase-7 multi-FB-tab tests
   (`fullstack-47` / `fullstack-58` patterns).

## 2026-05-19 — implementation note

Reshaped `browserTabLabel` to accept an optional
`BrowserLabelCtx = { driveName?: string; selectedIsDir?: boolean }`.
The label now always ends in `/` and resolves as:

| Selected state                | Label                        |
|-------------------------------|------------------------------|
| no selection / whitespace     | `<driveName>/` (or tab title) |
| file directly under drive root | `<driveName>/`               |
| file under a subdir            | `<parentDirBasename>/`       |
| directory                      | `<dirBasename>/`             |

`ctx.selectedIsDir` disambiguates dir vs file when the path
string alone is ambiguous; a trailing `/` on `selected` is the
fallback signal so unit-only callers stay clean.

`tabLabel` and `tabLabelInPane` accept the same optional ctx and
pass it through (non-browser kinds ignore it).

Pane.svelte builds the ctx via a small `browserCtxFor(tab)`
helper. `selectedIsDir` is derived from a tree-path → `is_dir`
`Map` rebuilt with `tree.entries`; `driveName` comes from a new
`driveDisplayName()` export in `state/store.svelte.ts` that
prefers `drive.info.name` and falls back to the basename of
`drive.info.root` (e.g., `~/dev/foo/bar` → `bar`). When a tree
entry is removed (file deleted), the lookup returns `undefined`,
which falls through to the file branch and the parent-dir or
drive-name fallback. No stale-name flash possible.

Tests updated under `state/tabs.test.ts::browserTabLabel`
covering: empty selection (drive name), file-at-root (drive
name), file-in-subdir (parent dir), directory (dir basename),
trailing-slash dir signal, and cross-tab divergence. Full vitest
suite passes (446/446). Frontend `npm run build` green.

Files touched:

* `web/src/state/tabs.svelte.ts` — new `BrowserLabelCtx`,
  reshaped `browserTabLabel`, pass-through on `tabLabel` and
  `tabLabelInPane`.
* `web/src/state/store.svelte.ts` — `driveDisplayName()` helper.
* `web/src/components/Pane.svelte` — wiring + `treeIsDir` map.
* `web/src/state/tabs.test.ts` — new test block.

No chan-server changes; drive name default-derivation lives on
the SPA side as the task specified.

## 2026-05-19 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Shape matches the spec: parent-dir resolver, trailing slash
always rendered, drive-name fallback derived from the path
basename when registry lacks an explicit display name, no
stale-name path possible because the tree-entry lookup falls
through cleanly on deletion. The `BrowserLabelCtx` reshape is
the right abstraction — keeps the lookup explicit instead of
threading dir/file detection through every caller.

Test coverage matches the acceptance criteria (empty selection,
file-at-root, file-in-subdir, directory, trailing-slash signal,
cross-tab divergence). 446/446 vitest green + `npm run build`
green is the bar.

**Commit clearance**: approved. Commit `fullstack-a-1` as a
standalone change. Suggested commit subject:

```
FB tab title = parent-dir-with-trailing-slash; drive name fallback (fullstack-a-1)
```

Push waits for the Round-1 close commit-grouping plan; do NOT
push yet.

Pick up `fullstack-a-2` next (status-bar click events + flash
colour blue → yellow).
