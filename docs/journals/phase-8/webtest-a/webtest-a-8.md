# webtest-a-8 — Bundled walk: -a-62 (FB fade) + -22 (graph contact filtering + bucket emit)

Owner: @@WebtestA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Walk two recently-landed changes in a single beat —
both visual / data-shape checks against the chan repo
seed:

1. **`fullstack-a-62`** (`1d3d200`) — docked FB fades
   long filenames at the right edge instead of
   wrapping to 2 lines.
2. **`systacean-22`** (`6443b98`) — graph view filters
   unreferenced contact File nodes + emits FileBucket
   on graph nodes.

## Reference

* `-a-62` task body: [`../fullstack-a/fullstack-a-62.md`](../fullstack-a/fullstack-a-62.md).
* `-22` task body: [`../systacean/systacean-22.md`](../systacean/systacean-22.md).
* Bug entries: `phase-8-bugs.md` "Docked file browser
  wraps long filenames" + "Contact-node count seems
  anomalously high (UPDATED diagnosis post-`systacean-22`)".

## Acceptance

### -a-62: FB fade (4 visual checks)

1. **Long filename fades**: open the docked FB with
   chan-source seed; locate a long filename like
   `chan-desktop-onboarding-redesign.md`,
   `phase-9-desktop-native-vision.md`. Confirm it
   renders on ONE line with fade on the right edge.
   No 2-line wrap. Screenshot before + after.
2. **Resize widens visible text**: drag the FB column
   wider; more of the long filenames become visible
   with less fade extent.
3. **Resize narrows visible text**: drag the FB column
   narrower; more filenames fade off the right edge;
   none ever wrap to 2 lines.
4. **Right-dock mirror**: switch FB to right-dock if
   the UI supports the switch; confirm fade direction
   mirrors (fades to LEFT edge since text right-
   aligns).

### -22: Graph contact filtering + bucket emit (5 data + visual checks)

5. **Contact count drops** on the chan-source seed
   (which has no imported contacts): graph chip
   `contact` count should drop from ~1973 (pre-`-22`)
   to ~49 (only mentioned handles in markdown). On a
   plain chan-source-only drive (no imported
   contacts), count should be roughly the unique-
   `@@<Name>` handle count.
6. **Mention edges preserved**: each `@@Handle`
   reference in markdown still produces a mention
   edge to the deduped contact node. Pick a node
   visibly + verify edges point at it.
7. **Synthesized contacts test (optional)**: spin up
   a throwaway drive with 3 contact-frontmatter
   files (`alice.md`, `bob.md`, `charlie.md`) +
   markdown referencing only `@@alice`. Confirm
   graph emits only alice as a contact node; bob +
   charlie are absent.
8. **Bucket emit visible**: inspect
   `/api/graph?scope=drive` JSON via curl; confirm
   file nodes carry `bucket: "Markdown"` or
   `bucket: "SourceCode"` field (Option<FileBucket>).
   None for ghost/fs-graph-merge file nodes is
   acceptable.
9. **Composition with `-a-57` filter chips**: toggle
   markdown / source chips — visible counts should
   update accordingly. (Bucket emit is server-side;
   chips read it.)

### Walkthrough audit trail

Append a fresh dated heading to
[`webtest-a-1.md`](webtest-a-1.md):
`## 2026-05-22 — bundled walk: fullstack-a-62 (FB
fade) + systacean-22 (contact filtering + bucket emit)`.
Capture verdicts + screenshots for each check + side
observations + tear-down.

## How to start

1. `git status` clean; `git log --oneline -5` confirms
   `1d3d200` + `6443b98` in HEAD.
2. Rebuild chan (web/dist stale): `cd web && npm run
   build && cd ..`; `cargo build -p chan`.
3. Spin up test server; chan-source seed drive.
4. Walk `-a-62` checks (1-4): visual FB fade.
5. Walk `-22` checks (5-9): graph data + counts.
6. (Optional) synthesized contacts test for #7.
7. Append verdict; fire poke to
   `event-webtest-a-architect.md`.
8. Tear down per the standing rule.

## Coordination

* @@WebtestA lane.
* Standing terminal + Chrome MCP perm covers the walk.
* Medium walk; ~30-45 min (two changes; light each).

## Numbering

Highest committed `webtest-a-N` is `-7` (the proactive
`-a-58` walk wasn't numbered; appended to `webtest-a-1.md`).
This is `-8`.

## Out of scope

* The OTHER 4 in-flight FullStackA tasks (`-a-56`,
  `-a-59`, `-a-60`) — walked separately when they
  land.
* Re-walking `-a-58` (already 3/4 HOLD per proactive
  walk `7175c1a`).
* `-a-61` (PAUSED pending Alex's draft-folder design).
