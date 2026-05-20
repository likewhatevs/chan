# @@FullStackA's phase-8 journal

Author: @@FullStackA
Date: 2026-05-19

Frontend + backend lane A. Same profile as @@FullStackB; operates
in parallel to clear the bug queue and (Round 2) feature queue.

Append-only. New entries go at the bottom under a dated heading.

## 2026-05-19 — Round 1 sweep, fullstack-a-1 through -8

Cleared the initial Round-1 queue in one session. All eight
tasks closed; -1, -2, -3 committed locally with architect
clearance; -4 through -8 awaiting commit clearance.

| Task            | Topic                                        | Status         |
|-----------------|----------------------------------------------|----------------|
| fullstack-a-1   | FB tab title = parent-dir + trailing slash   | committed      |
| fullstack-a-2   | Status-bar clicks + watcher dot yellow       | committed      |
| fullstack-a-3   | Hybrid label + drop flash + 1/2/3 immediate  | committed      |
| fullstack-a-4   | Rich prompt cluster (focus, overlay, spawn)  | review pending |
| fullstack-a-5   | Editor cluster (img scroll, empty pane, repop) | review pending |
| fullstack-a-6   | Cmd+K F focuses search input                 | review pending |
| fullstack-a-7   | Hybrid NAV: Cmd+K → Cmd+.                    | review pending |
| fullstack-a-8   | Restore wobble on Hybrid + right-click menus | review pending |

Highlights:

* `SpawnDialog` lifted to App root via a new
  `state/spawnDialog.svelte.ts` singleton (-4). Fixes the
  "backdrop without dialog" visibility regression by moving the
  dialog out of every ancestor stacking context that clipped
  its `position: fixed` (rich-prompt's z-index: 20, pane's
  overflow-hidden, Hybrid NAV's filter on unfocused panes).

* New `BrowserLabelCtx = { driveName?; selectedIsDir? }` plumbed
  from `Pane.svelte` through `tabLabel`/`tabLabelInPane` (-1).
  Tree-derived `is_dir` lookup keeps the FB tab title honest
  even when the selected file just got deleted — the lookup
  returns undefined and the title falls through to the drive
  display name.

* `BubbleOverlay.visibleEvents` now filters surveys whose `id`
  matches a sibling `survey-reply` event (-5). Picked option (b)
  from the task's three fix options since the chan-server reply
  endpoint already writes a pair-by-id record; pure SPA-side
  fix, no cross-stack coordination.

* Audited eight right-click / overlay surfaces for the
  easeOutBack wobble (-8). Four were missing it after the
  phase-7 `fullstack-80` / `-82` rework. Added a 260ms
  cubic-bezier(0.34, 1.56, 0.64, 1) open animation to each,
  scoped to local keyframes + `prefers-reduced-motion` cancel.

Gate green throughout: vitest 452/456, `npm run check` clean,
`npm run build` clean. Pre-push Rust gate not touched (no Rust
changes in any of -1 through -8).

Awaiting architect clearance on -4 through -8 before
committing. No push gate cleared yet for any commit — pushes
wait for Round-1 close per the architect's standing rule.
