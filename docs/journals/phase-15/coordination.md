# Phase 15 — cross-lane coordination

## Lane map

| Lane | Theme | Subagents |
|------|-------|-----------|
| A | True two-face card flip + Dashboard carrousel redesign + move chan-reports / semantic / embedding / Index into Dashboard slot backs + Search inspector buttons | 2 (A-core, A-helper) |
| B | Search overlay cleanup: remove SCOPE selector, SEARCH STATUS button, delete the search-status overlay | 1 |
| C | Terminal Shift+Enter fix + tab-groups + group-scoped broadcast; `chan shell` / `cs` CLI | 2 (C-term, C-shell) |

## Shared files (region ownership — the #1 conflict risk)

- `web/src/state/tabs.svelte.ts`: **A** owns the `DashboardTab` type +
  `flipHybrid` (and removes the `paneFlip` bus). **C** owns the `TerminalTab`
  group field + broadcast/group fns. Open-tab fns are read-only for C.
- `web/src/state/store.svelte.ts`: **B** owns the `searchPanel`/scope region.
  **C** owns `handleWindowCommand` additions.
- `crates/chan-server/src/terminal_sessions.rs`: **C-term** owns the
  edits — `$CHAN_TAB_GROUP` env (like `$CHAN_TAB_NAME`) + storing
  `tab_name`/`tab_group` per live session in the registry. C-shell does not
  edit this file; it only takes a **read handle** to the registry (plumbed at
  the control-socket construction site in the AppState wiring) for
  `cs term write` / `term list`. Coordinate the handle at `CK-GROUP`.
- `web/src/components/Pane.svelte`, `EmptyPaneCarousel.svelte`,
  `HybridFileBrowserConfig.svelte`, `HybridDashboardConfig.svelte`: **A** only.
- `web/src/components/SearchStatusOverlay.svelte`: **A** *reads* it (to copy
  the Index widget out); **B** is the sole *deleter*. No shared edit.

## Checkpoints (the unavoidable sequence points)

1. **A-internal:** two-face flip (A1) lands before A builds the per-slot
   Dashboard back (A2).
2. **A -> B (`CK-INDEX`):** A confirms the Index widget renders in the
   Dashboard Search slot back. Then B deletes `SearchStatusOverlay.svelte`.
   B does everything else in parallel first.
3. **C-term -> C-shell (`CK-GROUP`):** C-term lands the tab-group concept
   (the `group` string per `TerminalTab` (SPA) + `$CHAN_TAB_GROUP` env +
   `tab_name`/`tab_group` stored per live session in the `TerminalRegistry`).
   Then C-shell's `cs term write` / `term list` can resolve groups via a read
   handle to the registry.
4. **A-helper -> A-core (`CK-COMPONENTS`):** A-helper hands over the new
   standalone components (screensaver preview + per-slot config bodies); then
   A-core imports them.

Lanes coordinate the details of each handoff peer-to-peer and tell @@Alex
when a checkpoint is reached so dependent lanes rebase.

## Gate

The pre-push gate in `bootstrap.md` is shared and non-negotiable. @@Alex
aligns all lanes on it before each merge. The release gate also builds the
gateway workspace.

## Merge cadence

Merge gated-green increments to `main` locally as they land. @@Alex
sequences merges that touch shared files so adjacent-region edits don't
collide. After a shared-file merge, the owning lane pings dependents to
rebase.
