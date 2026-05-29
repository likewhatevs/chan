# webtest-a-10 — Bundled walk: -a-59 (pane-focus-click) + -a-60 (graph hit-radius)

Owner: @@WebtestA
Cut: 2026-05-22 by @@Architect
Status: dispatched

## Goal

Walk two recently-landed @@FullStackA changes in one
beat:

1. **`-a-59`** (`e8781d3`) — pane-focus-click on
   click-to-focus restore (NOT on Cmd+Tab).
2. **`-a-60`** (`910cdc8`) — graph canvas click
   hit-radius expanded to 10px.

## Reference

* `-a-59` task body: [`../fullstack-a/fullstack-a-59.md`](../fullstack-a/fullstack-a-59.md).
* `-a-60` task body: [`../fullstack-a/fullstack-a-60.md`](../fullstack-a/fullstack-a-60.md).
* Both bugs filed in `phase-8-bugs.md`.

## Acceptance

### -a-59: pane focus-click

1. **Click-to-focus restore**: chan-desktop unfocused
   (Cmd+Tab away to another app); click on a Hybrid
   pane different from previously active; window
   refocuses; clicked pane becomes active.
2. **Cmd+Tab restore preserves pane**: chan-desktop
   unfocused; Cmd+Tab back (no mousedown); pane
   selection unchanged.
3. **Click outside any pane**: chrome / hamburger /
   tab strip clicks don't change pane state.

### -a-60: graph hit-radius

4. **Click registers without zoom**: open graph at
   default zoom; click within ~10px of a node's
   visible edge → registers as a hit on that node.
   Take screenshot.
5. **Drag/pan unaffected**: drag-to-pan still works
   when starting on canvas-empty pixels.
6. **No false-positive overlap**: clicks near node
   midpoint resolve to nearest centroid (or fall
   through cleanly).

### Walkthrough audit trail

Append to [`webtest-a-1.md`](webtest-a-1.md):
`## 2026-05-22 — fullstack-a-59 + fullstack-a-60 bundled walk`.

## How to start

1. Confirm `e8781d3` + `910cdc8` in HEAD.
2. Rebuild chan; spin up test server + seed.
3. Walk -a-59 checks (1-3) — chan-desktop required
   for the focus-restore mechanic; web build has no
   equivalent.
4. Walk -a-60 checks (4-6) — graph canvas.
5. Append verdict; tear down.

## Coordination

* @@WebtestA lane.
* Light-to-medium walk; ~25 min (chan-desktop window
  juggling + graph clicks).
* Standing terminal + Chrome MCP perms cover. For
  -a-59 you'll need an external app to Cmd+Tab from;
  any open Mac app works (Safari / Finder / etc.).

## Numbering

This is `-10`.

## Out of scope

* `-a-61` PAUSED (draft-folder design).
* `-a-62` resize behavior (Chrome MCP blocked from
  webtest-a-8).
* `-b-26` (chan-desktop tab right-click; webtest-b
  lane).
