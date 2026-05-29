# fullstack-8: BCAST + mute state cluster (B17 + B18)

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Make the BCAST membership menu and the per-tab MUTE state
behave consistently and predictably under a 6+ terminal
workload. @@Alex hit live state drift today; the cluster is
covered by B17, B18, and the 2026-05-18 18:50 BST
clarification in `request.md`.

## Relevant links

* [../request.md](../request.md) Bugfixes — see B17, B18, and
  the sub-bullet clarification under B18.

## Acceptance criteria

### B17 — `Cmd+Shift+I` always toggles all tabs

* `Cmd+Shift+I` toggles MUTE on every terminal tab, period.
  It must not be a no-op when state has drifted.
* After the bulk toggle, the user can still flip individual
  tabs on/off without affecting the others.
* The per-tab MUTE state is preserved across subsequent
  bulk toggles (so the user's manual exceptions stick).

### B18 — BCAST UI consistency + membership isolation

* The tab strip's `[BCAST]` text pill is replaced by the
  broadcast (radio) icon used in the membership chip area.
  The tab strip and the menu use the same icon.
* The BCAST membership menu ticks toggle ONLY the clicked
  terminal. Currently they leak across tabs. Fix the state
  binding so each row's checkbox is independent.
* Select-all / deselect-all in the membership menu are
  bulk operations that **preserve each tab's pre-existing
  individual MUTE state**. BCAST membership and per-tab MUTE
  are independent axes; mutating one must not implicitly
  mutate the other.

### Defensive

* Reproduce the 6-terminal stress flow: BCAST on/off twice
  plus mute/unmute a subset, end state visually consistent
  on every tab.
* Add a unit / component test that nails the membership +
  MUTE matrix so the cluster doesn't regress.

## Out of scope

* Re-architecting the broadcast wire format.
* PTY-level mute mechanics (already handled).

## How to start

1. Locate the `[BCAST]` tab pill in `web/src/components/`
   (likely a sibling of the broadcast icon used in the
   chip area). Swap text for icon.
2. Audit the BCAST membership menu component: any `bind:`
   that references a shared store slot is the leak.
3. Audit `cmd+shift+I` handler in
   `web/src/state/shortcuts.ts` or wherever the keybind
   lives; confirm it iterates every tab and preserves the
   pre-toggle MUTE map.

## Hand-off

Standard. Pre-push gate green. Ping via
`alex/event-fullstack-architect.md`.

## 2026-05-18 18:09 BST — implementation

Implemented the BCAST / mute state cluster.

Changed files:

* `web/src/App.svelte`
* `web/src/components/Pane.svelte`
* `web/src/components/TerminalTab.svelte`
* `web/src/state/shortcuts.ts`
* `web/src/state/tabs.svelte.ts`
* `web/src/state/tabs.test.ts`

What changed:

* Replaced the tab strip `[BCAST]` text pill with the same radio icon used in
  the terminal broadcast menu/strip.
* Made BCAST membership a per-source terminal target list instead of syncing
  target rows into every member's state.
* The membership menu now lists other terminals only; checking/unchecking a row
  mutates only the active terminal's target list.
* BCAST membership changes no longer clear `broadcastMuted`; MUTE is a
  separate per-terminal axis.
* `Cmd+Shift+I` now toggles broadcast mute across all terminal tabs and does
  not change BCAST membership.
* Added state tests for membership isolation, mute preservation, input fan-out,
  and the bulk mute shortcut.

Verification:

* `npm run test -- tabs` from `web/`
* `npm run check` from `web/`
* `npm run build` from `web/`
* `scripts/pre-push`

Notes:

* No manual 6-terminal browser walkthrough performed in this lane.
