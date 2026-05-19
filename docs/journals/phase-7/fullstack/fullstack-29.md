# fullstack-29: Phase 1 migration audit — match the spec, drop the additions

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-19

## Goal

Audit your `fullstack-14` Phase 1 migration (Graph + File
Browser overlays → first-class tabs) and the subsequent
menu work for **two directions of drift**:

1. **Things added that weren't in the spec.** Anything
   you introduced for your own convenience or to fill a
   gap that wasn't asked for — remove or surface to
   @@Architect for explicit sign-off.
2. **Things the spec asked for that aren't actually
   working end-to-end.** Specifically the "open the
   File Browser tab for this path" action across every
   surface that exposes it.

@@Alex has noticed both directions in recent walkthrough
sessions; this task scopes a clean pass to match the
shipped behavior to the spec.

## Known concrete misses (per @@Alex 2026-05-19)

These are confirmed broken. They are the entry points to
the wider audit; do not stop at fixing just these.

1. Terminal tab right-click → `Show Dir` — doesn't
   spawn a File Browser tab.
2. Graph tab inspector → `Show Directory` / `Show
   File` — currently changes the focused graph node
   instead of opening a File Browser tab.
3. **Every inspector pane that exposes a "Show
   Directory" / "Show File" button** must spawn (or
   focus) a File Browser tab for that path. Inspectors
   live on Graph tabs, File Browser tabs, and any
   future inspector surface — same contract for all of
   them.

## Known concrete additions to review

* `Focused` checkbox at the bottom of the terminal tab
  right-click menu — already removed in `fullstack-25`
  (confirmed not intentional, surface of the broken
  state model).
* **Inline `×` close button on the Graph surface's
  SCOPE bar** (top right, next to the kebab inspector
  toggle). This is the old OverlayShell internal-close
  affordance; now that Graph is a first-class tab with
  its own tab-strip `×`, the inline one is redundant.
  Drop it.
* **Same inline `×` on the File Browser surface** if
  it exists (per @@Alex's note 2026-05-19 05:00 BST:
  "i havent seen the file browser yet but if it does
  have the same X button we can remove it"). Audit
  and drop.
* Audit the rest of the menus you added or modified in
  `fullstack-6`/`-14`/`-21` for anything similar:
  controls that aren't called for in `request.md` or in
  the relevant task file's acceptance criteria. List
  each one in this task's hand-off note with a
  one-line "drop / keep + why".

## Acceptance criteria

### Drop direction

* For every UI element added during Phase 1 (`fullstack-14`)
  or the pane-menu work (`fullstack-6`, `-21`, `-28`) that
  is NOT in the matching task file's acceptance criteria
  or the `request.md` source bullets:
  * If removing it has no user impact: drop it cleanly.
  * If you're unsure: list it in the hand-off note +
    one-line rationale, leave in tree, tag @@Architect
    for sign-off via event.

### Complete direction

* Every "show this path in the File Browser" call site
  spawns the new first-class FileBrowser tab. Specifically:
  * Terminal tab right-click → `Show Dir` (uses CWD).
  * Graph tab inspector → `Show Directory` / `Show
    File` for the focused/selected node.
  * File Browser tab inspector → if it exposes any
    similar action (audit and confirm).
  * Doc-tab right-click → `Show in file browser` (added
    in `fullstack-6`; confirm it lands).
* If a File Browser tab is already open at the same
  path, focus it instead of spawning a duplicate.
* No OverlayShell calls for File Browser anywhere (per
  `fullstack-14` removal).
* Regression test for at least the terminal + graph
  inspector call sites.

### Audit summary

* Append a "2026-05-19 — audit summary" section to this
  task file with:
  * Each call site you checked + verdict (works /
    fixed / removed).
  * Each menu/UI element you reviewed + verdict (in
    spec / not in spec, kept or dropped).
  * Any items you're flagging back to @@Architect for
    a sign-off decision.

## Why this task exists

Phase 1 was shipped quickly. Some things landed that
weren't asked for; some asked-for behaviors didn't land
end-to-end. This is a discipline pass to match the
shipped behavior to the spec — both ways. The next
phase's bootstrap will reference this as the model for
how to handle scope drift cleanly.

## Out of scope

* Backend changes — Phase 1 was frontend-only.
* New File Browser features beyond what the overlay
  already shipped pre-`fullstack-14`.
* Bigger refactors of the pane / inspector machinery.

## How to start

1. `grep -RIn "FileBrowser\|file_browser\|show.*dir\|show.*file\|inspector" web/src/` —
   inventory every reference. Map to call sites.
2. For each call site, decide: does it spawn the new
   tab? Does it pass the right path?
3. List inspector buttons across `Pane.svelte`,
   `GraphPanel.svelte`, `FileBrowserSurface.svelte`,
   and any sibling components.
4. For each menu item / button you touch, check the
   matching task file's acceptance criteria. If it's
   not there, decide drop/keep per the criteria
   above.

## Hand-off

Standard. Pre-push gate green. The audit summary append
is part of the hand-off — don't skip it. Ping via
`alex/event-fullstack-architect.md`.
