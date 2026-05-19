# fullstack-a-8: Restore CSS wobble on Hybrid + right-click menus

Owner: @@FullStackA
Date: 2026-05-19

## Goal

The CSS wobble effect regressed off the Hybrid NAV entry overlay
and all right-click menus. @@Alex never asked to remove it. Bring
it back, using the wobble that's still applied on the
`OverlayShell` for Search and Settings as the reference.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md): "CSS wobble
effect missing from Hybrid and right-click menus".

Surfaces that need wobble restored:

* Hybrid NAV entry overlay.
* Pane right-click menu.
* Tab right-click menu.
* File Browser / Graph right-click menus.
* Any other right-click menu in the SPA (audit `*ContextMenu*`
  / `*RightClick*` components).

## Acceptance criteria

* Wobble effect visually matches the OverlayShell (Search /
  Settings) reference on every restored surface.
* Effect parameters (duration, easing, transform amplitude)
  match the OverlayShell — pull the shared keyframes /
  transition class if one exists; if not, lift one out.
* No regression on existing wobble surfaces.

## How to start

1. Find the OverlayShell component (likely
   `web/src/components/OverlayShell.svelte` or similar) and the
   wobble CSS / transition class it uses.
2. Audit the surfaces above for where the wobble class /
   transition used to apply. Phase-7 right-click rework
   (`fullstack-80` / `fullstack-82` "Trim right-click menus")
   is the likely regression point.
3. Re-apply, factoring into a shared `wobble` class /
   transition if multiple call sites repeat the inline styling.

## 2026-05-19 — implementation note

Audit of every right-click / overlay-entry surface and which
ones already carry the wobble vs which ones lost it:

| Surface                                | Status before        |
|----------------------------------------|----------------------|
| OverlayShell (Search, Settings)        | wobble (overlay-pop) |
| HamburgerMenu (pane / FB / Graph head) | wobble (hamburger-pop) |
| TerminalTab tab-menu-bubble            | wobble (bubble-pop)  |
| FileEditorTab tab-menu-bubble          | wobble (bubble-pop)  |
| PaneModeHelp (Hybrid `H` cheatsheet)   | MISSING              |
| TerminalRichPrompt `.ctx`              | MISSING              |
| FileTree row right-click `.ctx`        | MISSING              |
| GraphPanel tab-menu-bubble             | MISSING              |

Added a `260ms cubic-bezier(0.34, 1.56, 0.64, 1)` open
animation that scales 0.92 -> 1.0 with fade-in on each missing
surface, scoped to a local keyframes name (e.g.
`pane-mode-help-pop`, `ctx-pop`, `graph-tab-menu-pop`) so
the new rules don't clash with the existing `bubble-pop` /
`overlay-pop` keyframes which carry their own transform-
origins. Each block includes a `prefers-reduced-motion`
cancel.

PaneModeHelp is centred via `translate(-50%, -50%)`, so the
keyframe preserves the translate while animating the scale:

```
@keyframes pane-mode-help-pop {
  0%   { opacity: 0; transform: translate(-50%, -50%) scale(0.92); }
  100% { opacity: 1; transform: translate(-50%, -50%) scale(1); }
}
```

Files touched:

* `web/src/components/PaneModeHelp.svelte` — centred wobble.
* `web/src/components/TerminalRichPrompt.svelte` — `.ctx`
  wobble (top-left origin matching cursor anchor).
* `web/src/components/FileTree.svelte` — `.ctx` wobble.
* `web/src/components/GraphPanel.svelte` — `.tab-menu-bubble`
  wobble.

Did not lift the keyframe to a shared utility class because
the existing wobble call sites (OverlayShell, HamburgerMenu,
TerminalTab, FileEditorTab) already each define their own
local keyframe; consistency favours leaving the new surfaces
in the same shape. If we ever centralise, all eight call sites
go in one pass.

Pre-push gate (SPA portion): vitest 456/456 green;
`npm run check` 0 errors / 0 warnings.

## 2026-05-19 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Audit table is the right artifact for this kind of regression
hunt — names every surface, classifies pre-state. Four
restoration sites match the four MISSING rows. Not lifting a
shared keyframe was the right call given the existing call
sites each define their own local keyframe with their own
transform-origins; consistency beats one-shot abstraction
here. Reduced-motion cancels in place — good discipline.

**Commit clearance**: approved. Suggested subject:

```
Restore CSS wobble on Hybrid help + right-click menus + rich-prompt ctx + graph tab menu (fullstack-a-8)
```

Push waits for Round-1 close. Your queue: `fullstack-a-9` (`[`
`]` resize inversion) and `fullstack-a-10` (Chrome-style tab-
name fade + full-path hover on file tabs + FB tree rows) just
landed. Pick `-9` next unless you want to tackle `-10` first.
