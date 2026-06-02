# @@LaneD — Frontend/UX + Team Work GUI

Read `round-1-plan.md` first. Round-1: land the SMALL WINS first (F1, F2,
F3, F6, TW3), then start F4 (design first). TW1 waits on @@LaneA's C1
contract. The full menu map for F4 is in `menu-inventory.md`.

## Round-1 tasks

- **F1** [S] List-bullet glyph too big. `web/src/editor/Wysiwyg.svelte`
  `::before { font-size: var(--chan-editor-body-size,11pt) }` (:991-995);
  top glyph `\25CF` (:999), nested hollow (~:1002). Shrink both.
- **F2** [S] Inspector section separators (buttons / file size / code /
  contacts). `web/src/components/Inspector.svelte` (no menu — just dividers).
- **F3** [S] Terminal hamburger: move the whole "Broadcast input on/off"
  section to the TOP, right after the Group row.
  `web/src/components/TerminalTab.svelte` (~:1422-1475 group/broadcast
  block).
- **F6** [S/M] Theme-toggle icon consolidation. Adopt chan-desktop's
  sun/moon ICON as the shared affordance. Replace the SPA per-tab back-side
  "dark/light" TEXT toggle and (coordinating with @@LaneE) the web-marketing
  "sun/moon" TEXT. EXCEPTION: the tri-state system/dark/light configurator
  keeps its labeled form. Extract a shared icon component.
- **TW3** [S] Bug: the Cmd+P Team Work dialog ignores ESC; ESC should
  cancel/close it.
- **F4** [L] Context-menu overhaul (DESIGN first, then implement). Today the
  editor, terminal, and tab-name all share ONE `tabMenu` bubble; the editor
  menu (`FileEditorTab.svelte:501-1006`) is NOT selection-aware. Split into
  body-context (selection-aware Copy/Cut/Paste + contextual items) vs
  tab-context (prune body-only items). Plus link affordances: markdown
  preview (terminal read-only) + external-link "open" bubble (external
  browser, or new tab on web — `web/src/editor/external_links.ts`
  `openExternalUrl`). See `menu-inventory.md`.
- **TW1** [M] Team Work LOAD dialog (Cmd+P): path autocomplete, FORCE the
  path to directories, visually surface the config file in the chosen dir.
  MIRRORS @@LaneA's C1 path+spawn semantics — read C1's contract from
  `event-lane-a.md` before finalizing. Start after the small wins.
- **TW4** [N,S] Bug (filed by @@Host). In the Spawn-agents dialog
  (`web/src/components/TeamDialog.svelte`) the per-member "cell N" badge
  wraps to two lines and renders as a circle for at least one member
  (cell 4 / @@LaneC observed; others are single-line pills). Root cause
  candidate: `.team-member-cell-badge` (`:673-682`) declares
  `font-size: 0.7rem` (:674) but then `font: inherit` (:681) — the `font`
  shorthand resets font-size back to the larger inherited value — and there
  is no `white-space: nowrap`, so the badge exceeds its intended size and
  can wrap; `border-radius: 999px` then makes the square box look round.
  Fix: drop or reorder the `font` shorthand so `0.7rem` sticks, and add
  `white-space: nowrap`. Repro in the actual client first (the screenshot
  was chan-desktop/WKWebView), then confirm fixed across all 6 badges.

## Files you OWN

`web/src/editor/{Wysiwyg.svelte,external_links.ts,links.ts}`,
`web/src/components/{Inspector.svelte,TerminalTab.svelte,FileEditorTab.svelte,
Pane.svelte}`, the Team Work dialog component, and the shared theme-toggle
component.

## Coordination

- `web/src/state/store.svelte.ts` is @@LaneA's this round — don't edit it.
- F6 in `web-marketing/` overlaps @@LaneE's D1 copy work: agree file split
  (you = icon component, E = copy) before editing web-marketing.
- TW1 blocks on C1 (@@LaneA) — coordinate via poke, don't guess the path
  semantics.

## Verify

`make pre-push` green (svelte-check + npm build included). Browser-smoke the
reactive bits (menus, ESC, theme toggle, glyph size) — static gates miss
Svelte-5 runtime reactivity. Post the commit sha to `event-lane-d.md` and
poke @@Lead per slice.
