# Phase 15 round 2 - cross-lane coordination

## Model

@@Architect is the hub, gate, and coordinator (the only agent talking to
@@Host). Three worker lanes (@@Lane-1 / @@Lane-2 / @@Lane-3) own disjoint
scopes and append-only journals/tasks, and coordinate through @@Architect.
@@Host owns product/scope calls, push, tag, and the release cut. See
`bootstrap.md` for the full role + process definition.

## Completion / poke protocol

On finishing a task a worker (a) appends to its event file and (b) pokes the
target: `cs term write --tab-name=<target> 'poke from <agent>: check <path>\n'`.
Host-targeted pokes are meant to surface as survey bubbles over the Lead
terminal (feature 2.3); until that lands they route via the event file +
@@Architect.

## Lane map (TBD - @@Architect fills after decomposition)

@@Architect decomposes `round-2-part-1.md` + `round-2-part-2.md` into the three
`lane-<x>-tasks.md` files, then records the theme split here. Representative
round-2 surface to distribute:

- `round-2-part-1.md`: About-front license placement (A6), About-back preview
  reacts to theme (A7), Dashboard right-click slot menu + Settings (A3),
  Search-slot inspector buttons (A4).
- `round-2-part-2.md`: indexing-complete signal (bugs 1+2), terminal links,
  Ctrl+R reload remap, editor stale-conceal-on-tab-switch, shift+Enter /
  agent-submit, the `cs` CLI surface (rename, prefix-match, restart, search,
  flags) + `chan open`, Team Work (group field, poke protocol 2.2, survey
  rebuild 2.3).

## Shared files (region ownership - the #1 conflict risk)

@@Architect records the round-2 region splits here once lanes are assigned.
The round-1 hot spots recur and need explicit owners:

- `web/src/state/tabs.svelte.ts` - `DashboardTab` slot state, `TerminalTab`
  group/keyboard-protocol fields, `TeamWorkState`.
- `web/src/state/store.svelte.ts` - `handleWindowCommand` (cs additions),
  status/index state.
- `web/src/components/TerminalTab.svelte` - key handling, restart, team send.
- `crates/chan-server/src/{control_socket.rs,terminal_sessions.rs,routes/*}` -
  cs control-socket commands + registry, the rebuilt survey watcher.

Rule (verbatim from round-1): when committing a shared file, chain
`git add <paths>` + `git diff --staged --stat` + commit + `git show --stat
HEAD`. `git add <path>` does not unstage peers.

## Checkpoints (TBD - @@Architect fills)

Sequence points are recorded here as the decomposition firms up (e.g. the
`cs terminal` rename landing before the poke docs reference it; the
agent-submit fix landing before agent-to-agent poke delivery; the survey
watcher backend landing before the BubbleOverlay reconnect). Lanes coordinate
each handoff peer-to-peer and tell @@Architect when a checkpoint is reached so
dependents rebase.

## Gate

The pre-push gate in `bootstrap.md` is shared and non-negotiable. @@Architect
aligns all lanes on it before each merge. The release gate also builds the
gateway workspace.

## Merge cadence

Merge gated-green increments to `main` locally as they land. @@Architect
sequences merges that touch shared files so adjacent-region edits don't
collide. After a shared-file merge, the owning lane pings dependents (via
@@Architect) to rebase.
