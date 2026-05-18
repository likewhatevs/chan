# frontend-3: same-name tab disambiguation

Owner: @@Frontend
Status: REVIEW

## Goal

When two open file tabs share a basename (`foo.md`, `foo.c`, etc),
shorten the tab title to disambiguate using their shortest common
ancestor. Full path on hover. The existing "show in file browser"
button stays as the precise navigation jump.

## Relevant links

* Request: [request.md](./request.md)
* Journal: [journal.md](./journal.md)
* Tab state: `web/src/state/tabs.svelte.ts`

## Scope

* Add a derived "display title" per tab that:
  1. Defaults to the basename.
  2. If another tab shares the basename, expand to the shortest
     unambiguous suffix made of segments separated by `[...]` when
     the divergence is deeper than one segment, otherwise just
     prepend the divergent ancestor segment.
  3. Aim for shortest readable form. Examples:
     * `a/foo.md`, `b/foo.md` -> `a/foo.md`, `b/foo.md`.
     * `a/x/foo.md`, `a/y/foo.md` -> `x/foo.md`, `y/foo.md`.
     * `a/x/p/foo.md`, `a/x/q/foo.md` -> `p/foo.md`, `q/foo.md`.
     * `a/x/p/foo.md`, `a/y/q/foo.md` -> `x/[...]/foo.md`,
       `y/[...]/foo.md` (collapse the inner unique tail).
* Title attribute (`title=...`) on the tab carries the full drive-
  relative path so hover surfaces it.
* Tabs without conflicts keep their plain basename.
* When the set of open tabs changes, recompute disambiguation for
  the affected basename group only.

## Algorithm sketch

```
group_by_basename(tabs)
  for each group with >= 2 tabs:
    let segs_per_tab = split each path into segments minus basename
    find the longest common prefix segments across the group, call it P
    find the longest common suffix segments across the group, call it S
    each tab's unique part = segments between (after P, before S)
    if unique part has exactly one segment:
      display = unique_segment + "/" + basename
    else:
      display = unique_part[0] + "/[...]/" + basename
```

The function lives in `web/src/state/tabs.svelte.ts` or a sibling
helper. Add unit coverage in `tabs.test.ts` (new) covering the four
example shapes above plus single-tab no-op + close-tab recompute.

## Out of scope

* Terminal tab naming (handled separately in
  [frontend-1](./frontend-1.md) with `Terminal-N`).
* Sorting tab order.

## Acceptance criteria

* Same-basename file tabs render with disambiguated titles per the
  algorithm.
* Hover (title attribute) shows the full drive-relative path.
* No regression for unique-basename tabs (still plain basename).
* Recomputation runs on tab open / close.

## Tests

* Vitest: shapes above, plus an open-close cycle that re-collapses
  to plain basename when the conflict goes away.
* `npm --prefix web run check` clean.
* `npm --prefix web test -- --run` green.
* `npm --prefix web run build` clean.

## Review and hardening

* @@Frontend self-review for performance on large tab counts (the
  recompute is per-basename-group, so worst case is bounded).

## Progress notes

* `tabLabelInPane` now groups same-basename file tabs and renders
  the shortest divergent directory segment.
* Deep divergent tails collapse as `x/[...]/foo.md`; unique
  basenames stay as plain filenames.
* Tab hover continues to use the full drive-relative path through
  the existing `title` attribute.
* Added Vitest coverage for the examples in this file plus the
  open/close recompute case.

## Completion notes

Verification:
* `npm --prefix web run check` passed.
* `npm --prefix web test -- --run` passed: 19 files, 185 tests.
* `npm --prefix web run build` passed with existing Vite warnings.
* Webtest round 4 scenario 8 passed for same-basename tab
  disambiguation and hover titles.
