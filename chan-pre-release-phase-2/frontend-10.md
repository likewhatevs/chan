# @@Frontend task 10: Swap the fs-graph folder-node glyph

Owner: @@Frontend. Status: REVIEW. Phase-2 disposition: small
follow-up; ride along on the phase commit if it lands in time,
otherwise it's its own commit.

Sourced from @@Rustacean's aside in
[[chan-pre-release-phase-2/rustacean-1.md]]:

> the "tags from source code" symptom Alex flagged on screenshot
> earlier in this kickoff was a UX read of the frontend `#` glyph
> used for fs-graph folder nodes, not a backend leak. ... Frontend
> follow-up: swap the `#` glyph for a folder icon on fs-graph
> `kind: "folder"` nodes so the glyph stops colliding with
> semantic-graph tag nodes.

## Goal

In the fs-graph overlay mode (`graphOverlay.mode === "filesystem"`),
folder nodes currently share the `#` glyph the semantic graph
uses for tags. The result reads as "this folder is a tag", which
is the symptom Alex screenshotted at phase-2 kickoff. Replace it
with a folder glyph so fs-graph folder nodes read as folders.

## Relevant links

- [[chan-pre-release-phase-2/journal.md]] item F2.
- [[chan-pre-release-phase-2/rustacean-1.md]] aside.
- `web/src/components/GraphCanvas.svelte` (kind-to-glyph
  mapping; see `loadIcon` and the `PATH_*` / `iconImages` table).
- `web/src/components/GraphPanel.svelte::mapFsNodes` for how
  `fs-graph` folder nodes get mapped onto the canvas
  `RenderedNode` kind (`folder` → `tag`). The current mapping
  is the source of the glyph collision — folder fs-graph nodes
  are rendered as `kind: "tag"` so they pick up the tag glyph.

## Architecture direction

Two options; either is acceptable:

### A. Stop mixing fs-graph folder nodes onto the tag kind

Add a new canvas-side `DKind = "folder"` and update
`mapFsNodes` so fs-graph folder nodes carry that kind. Wire a
folder-icon `PATH_FOLDER` in `GraphCanvas.svelte` and load it
in the icon table. Smallest surface change; cleanest semantics.

### B. Conditional glyph on the existing tag kind

Keep the `tag` canvas kind but switch the glyph based on the
node's `path` carrying a slash / being in `filesystemMode`.
Quick but smudges the `tag` kind's contract; not preferred.

Lean: option A.

## Acceptance criteria

1. In fs-graph mode, folder nodes render with a folder glyph,
   not the `#` tag glyph.
2. Semantic-graph tag nodes are unchanged.
3. The fix is theme-neutral (no hardcoded colors).
4. No regressions in GraphCanvas's existing layout / pan-zoom /
   selection behaviour.

## Test expectations

* `cd web && npm run check`.
* `cd web && npm test -- --run` (existing vitest).
* Visual smoke: load a folder in fs-graph mode; confirm the
  folder nodes render with the new glyph; confirm tag nodes in
  the semantic graph still render with `#`.

## Hardening / review

* @@Webtest picks up the visual smoke as part of the phase-2
  smoke matrix; coordinate via
  [[chan-pre-release-phase-2/architect-9.md]] / webtest-2.

## Phase fit

If frontend cycles are free before the phase commit, fold this
in. Otherwise commit it standalone after the phase commit. Not a
blocker.

## Progress notes

* 2026-05-16 @@Frontend: changed fs-graph folder nodes in
  `GraphPanel.mapFsNodes` to render as canvas `kind: "folder"`
  instead of reusing semantic `kind: "tag"`. `GraphCanvas` already
  had the folder render kind and theme-neutral folder icon path.

## Completion notes

* Files changed: `web/src/components/GraphPanel.svelte`,
  `chan-pre-release-phase-2/frontend-10.md`.
* Verification:
  * `cd web && npm test -- --run`
  * `cd web && npm run check`
