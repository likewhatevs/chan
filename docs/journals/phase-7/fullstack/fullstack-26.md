# fullstack-26: drop MUTE entirely; BCAST is binary in-or-out

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

Simplify BCAST to its minimum: one group per Hybrid,
each tab in or out, pink indicator on the tab strip as
the only feedback. **Remove the MUTE concept entirely**
— no per-tab MUTE state, no `Cmd+Shift+I` mute toggle,
no MUTE-preserved-across-membership logic. Drop the
extra strip / bar that hosted mute + off buttons.

This supersedes the MUTE-related portions of
`fullstack-8` and `fullstack-22`. The window-wide-group
+ per-tab-toggle + no-self-entry semantics from
`fullstack-22` STAY.

## Relevant links

* [../request.md](../request.md) — B18 cluster, see the
  "Simplification (2026-05-19 04:00 BST)" sub-bullet.
* Predecessors:
  * [./fullstack-8.md](./fullstack-8.md) — original
    BCAST/mute cluster.
  * [./fullstack-22.md](./fullstack-22.md) — BCAST
    as window-wide group.

## Acceptance criteria

### Membership behavior (unchanged from fullstack-22)

* One BCAST group per Hybrid; all panes see it.
* Each tab's own "Broadcast input on/off" button is
  the canonical add/remove for that tab.
* Membership menu lists OTHER tabs (no self entry).
* A tab can be in BCAST by itself; that's a valid state.

### MUTE removal (new)

* Drop all per-tab MUTE state from the tab model.
* Drop the `Cmd+Shift+I` shortcut and any "bulk mute"
  affordances.
* Drop the extra bar / strip that hosted the mute and
  off buttons.
* Drop any backend or WS frames that exist solely to
  signal mute state. Broadcast input dispatch fans out
  to every IN-the-group tab unconditionally — there's
  no per-tab opt-out beyond leaving the group.
* Update tests: remove MUTE assertions; keep
  membership assertions.

### Single feedback surface

* The pink indicator on the tab strip is the ONLY
  visible signal that a tab is in BCAST.
* No `[BCAST]` text pill, no membership chip strip,
  no mute icon. Bubble overlay / rich prompt
  unchanged (this is per-tab terminal UI, not the
  bubble surface).

### Simplest test (the spec)

```
1. 4+ terminals up.
2. Select-all (via the membership menu's "select all"
   action): pink indicator on every tab.
3. Deselect a few from the menu: pink indicator clears
   on those, stays on the rest.
4. Deselect all: no pink indicators anywhere.
5. Select a few from the membership menu (or click
   their per-tab "Broadcast input on" button):
   pink indicator appears.
```

That sequence is the entire spec. Any state beyond
"in the group / not in the group / pink-or-not" is
out of scope.

## Out of scope

* Bubble overlay UI (separate surface, unchanged).
* Per-tab terminal output streaming (PTY mechanics
  unchanged).
* MUTE-equivalent features. If we want "pause
  broadcasting briefly" later, the spec is "leave the
  group, come back". Period.

## How to start

1. `web/src/state/tabs.svelte.ts` — drop MUTE fields
   from the tab type / state model.
2. `web/src/components/Pane.svelte` (or wherever the
   tab strip + extra bar live) — drop the mute /
   off-button strip.
3. `web/src/state/shortcuts.ts` — remove `Cmd+Shift+I`
   handler.
4. `crates/chan-server/src/...` — if there are any
   WS frames or backend state for broadcast-mute,
   remove them. Broadcast dispatch fans to every
   group member unconditionally.
5. Update tests: drop MUTE assertions.

## Hand-off

Standard. Pre-push gate green. Coordinate with
@@WebtestB for the deferred BCAST formal walkthrough —
the test sequence above replaces theirs. Ping via
`alex/event-fullstack-architect.md`.

## 2026-05-19 06:28 BST — hand-off

`fullstack-26` is committed and pushed on `main`.

Commit:

* `5806343` Drop terminal broadcast mute (fullstack-26)

Gate run: `npm run test -- tabs`, `npm run check`,
`npm run build`, and `bash -lc 'ulimit -n 4096; scripts/pre-push'`.

Notes: removed per-tab broadcast mute state, the `Cmd+Shift+I` mute
shortcut, the native bridge/help entry, and the in-terminal broadcast
strip. BCAST is now binary group membership with the tab-strip pink
indicator as the only visible state.
