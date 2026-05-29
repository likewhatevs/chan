# fullstack-a-12: Graph inspector "not in current file listing" warning even when server says missing=false

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Drop (or correctly gate) the SPA-side second-ghost check in
`GraphPanel.svelte::isFileGhost` so a graph node whose server-
side `missing` flag is `false` does not still render the
"not in the current file listing (try Reload / chan index)"
inspector warning when the file's parent directory has not
been lazy-loaded into the file-browser tree.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md) "Graph
inspector falls back to 'not in current file listing' even
when the server-side missing flag is false (SPA second-ghost
on lazy-tree path)" (filed 2026-05-20).

Companion to `@@Systacean`'s `systacean-2` which expanded the
server-side resolver universe to all on-disk files (not just
markdown + images). Server-side now correctly reports
`missing: false` for plain non-markdown files. The remaining
red-flag in the inspector is the SPA layer: `GraphPanel.svelte`
derives a second "ghost" state from `tree.entries`, which is
the lazy-loaded file-browser tree. For a selected file in an
unexpanded FB subtree the lazy tree has no record of the path
and the inspector tips into ghost-mode despite the server
flag.

@@WebtestA confirmed the user-visible symptom on the lane-A
sweep:

> 5 plain files (LICENSE, desktop/LICENSE, two
> crates/chan-drive/src/*.rs, a docs journal shell script)
> flagged "not in current file listing" in the inspector,
> despite being on disk and despite the server's resolver
> universe including them post-`systacean-2`.

@@Systacean flagged this as the SPA follow-up in
[`../systacean/systacean-2.md`](../systacean/systacean-2.md)
"Out of scope / follow-ups". The fix shape is firmly in the
SPA lane.

## Acceptance criteria

* Selecting a non-markdown file node (e.g., `LICENSE`,
  `crates/chan-drive/src/util.rs`) in the Graph view does
  NOT show the "not in the current file listing" warning
  when the server reports `missing: false`.
* The inspector still shows the warning when the server
  reports `missing: true` (true ghost case — file genuinely
  not on disk).
* The lazy-tree state is no longer load-bearing for the
  ghost decision; the server flag is the source of truth.
* No regression on the broken-link inspector path
  (`./does-not-exist.md` style links still get the warning).
* Verify against `@@WebtestA`'s test server
  (`/tmp/chan-test-phase8-wa/`, URL bearer in
  `event-architect-alex.md` 2026-05-20) once
  `systacean-2`'s server-side fix is committed and the
  binary rebuilt; the 5 false-flag files should clear.

## How to start

1. Find `isFileGhost` (or the equivalent ghost derivation)
   in `web/src/components/GraphPanel.svelte`.
2. Read the current decision tree. It almost certainly
   has the shape: `if (server.missing) → ghost; else if
   (!tree.entries.has(path)) → ghost`. The second branch is
   the bug.
3. Drop the lazy-tree branch entirely, OR gate it on
   `tree.loadedDirs` covering `dirname(path)` so the check
   only fires when we actually have ground truth from the
   FB tree. The simpler "drop it" route is preferable if
   the server flag now covers the universe — keep one
   source of truth.
4. Pin with a small SPA test if the ghost logic has a
   testable extraction; otherwise visual verification on
   the lane-A server is acceptable.

## Coordination

* Depends on `systacean-2` being committed + binary rebuilt
  for the verification leg. The fix code is independent —
  can land in parallel.
* @@WebtestA verifies on lane-A drive after both land.

## 2026-05-20 — implementation note

Confirmed shape in `GraphPanel.svelte`. The pre-fix derivation was:

```ts
const treeHasPath = $derived(new Set(tree.entries.map((e) => e.path)));
const isFileGhost = $derived<boolean>(
  selectedNode != null &&
    selectedNode.kind === "file" &&
    (selectedNode.missing || !treeHasPath.has(selectedNode.path)),
);
```

The second branch (`!treeHasPath.has(...)`) read from `tree.entries`,
which is only populated for FB subtrees the user has expanded. Any
file living under an un-expanded directory missed the set lookup
and tipped the inspector into ghost mode regardless of what the
server reported.

Post-`systacean-2`, the server's resolver universe covers every
on-disk file (markdown + non-markdown), so `selectedNode.missing`
is the authoritative ghost flag. Dropped the lazy-tree branch
and the now-unused `treeHasPath` derivation; `isFileGhost` is
just `selectedNode.missing === true` now.

Refreshed the leading docstring on `isFileGhost` to reflect the
new single source of truth (and to record why the lazy-tree
fallback was dropped, since the previous comment about "search
index not rebuilt" was misleading post-fix).

Other ghost paths inspected for regression:

* `kind === "ghost"` server-side nodes (line 917-923) still
  flow through the existing branch — those are broken-link
  sentinels emitted by the resolver, separate code path from
  `file` nodes; unaffected.
* Broken-link inspector path (`isFileGhost && selectedNode.missing`
  branch in the template at line 1308) still fires the warning
  for true ghosts. The text `./does-not-exist.md` would resolve
  to a `kind === "ghost"` node, not a `file` with `missing: true`,
  but server-side `missing: true` on a file node maps to the same
  inspector branch.

Files touched:

* `web/src/components/GraphPanel.svelte` — drop `treeHasPath`
  derivation, simplify `isFileGhost`, update the doc-comment.

Pre-push gate (SPA portion): vitest 475/475 green;
`npm run check` 0 errors / 0 warnings; `npm run build` clean.

To verify on the lane-A server (post-restart so the rebuilt
binary picks up the new bundle): select one of the 5 plain
files @@WebtestA flagged (LICENSE, desktop/LICENSE, two
`crates/chan-drive/src/*.rs`, the docs journal shell script)
without first expanding their parent directories in the FB
tree. Inspector should NOT show "not in the current file
listing". For a real ghost (e.g., a broken `./does-not-exist.md`
wiki link from somewhere in the README), the warning should
still appear.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Clean. The two-branch `isFileGhost` was always a brittle
shape — once `systacean-2` made the server the authoritative
universe, the lazy-tree fallback became actively wrong (it
fires on every FB subtree the user hasn't expanded yet, which
is most of them on first open of a large drive). Dropping it
and reducing to `selectedNode.missing === true` is the right
collapse to a single source of truth.

The regression audit of the other ghost paths is exactly
what we want — `kind === "ghost"` server-side nodes flow
through their own branch and stay unaffected; the
broken-link inspector path on `isFileGhost && missing` still
fires for true ghosts. So the change is a strict tightening:
fewer false positives, no false negatives.

Updating the leading docstring + recording why the lazy-
tree fallback got dropped is the right hygiene — the prior
"search index not rebuilt" comment was a documented
misread; killing it dead now beats letting it confuse a
future reader.

Pre-push gate green. Visual verification path on lane-A is
spelled out and correct (binary rebuild + server restart →
select an unexpanded-subtree plain file → no warning; pick
a true broken link → warning still fires).

**Commit clearance**: approved. Suggested commit subject:

```
Graph inspector: drop lazy-tree second-ghost; server missing flag is source of truth (fullstack-a-12)
```

Push waits for Round-1 close.

Carry on with `fullstack-a-14` (rich prompt re-open focus)
next per the queue.