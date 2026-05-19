# fullstack-5: workspace tab D&D regression

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Fix a workspace tab drag-and-drop regression surfaced by
@@WebtestA during `webtest-a-2`: dragging the *active* tab
onto an adjacent inactive tab in the same tablist deletes the
active tab from the list instead of reordering.

This is *workspace* tab D&D, not side-pane related.
`fullstack-1` did not touch `tabs.svelte.ts`; likely
pre-existing or related to a different recent change. Treat
as its own bug to fix and land.

## Relevant links

* @@WebtestA's repro at
  [../webtest-a/webtest-a-2.md](../webtest-a/webtest-a-2.md)
  section "7. Tab D&D" (the caveat paragraph) — measurements
  and exact pixel coords included.
* Also referenced in
  [../alex/event-webtest-a-architect.md](../alex/event-webtest-a-architect.md)
  ("Out-of-scope side observation").
* Related cluster: B15 (left-click on empty pane opens
  right-click menu — pane click handler hygiene generally).

## Acceptance criteria

* Dragging an active tab onto an adjacent inactive tab in the
  same tablist either:
  * **Reorders** the tabs (swap positions or insert at drop
    point) — recommended, matches user expectation for a
    drag-onto-tab gesture.
  * OR rejects the drop and leaves the tablist unchanged.
* Either way, the active tab MUST NOT disappear from the
  tablist. The current behavior — drop-as-close — is the
  bug.
* No regression in:
  * Drag a tab between distinct tabbars (pane-to-pane move).
  * Drop on the docked side panes (still false-positive
    rejected per `fullstack-1`'s walkthrough item 7).
  * Single-tab tablists (drag has no peer to drop on).
* Add a frontend test covering the drag-onto-adjacent-tab
  case so this can't regress silently.

## Reproduction

Per @@WebtestA's notes (paraphrased):

* Drive: `/tmp/chan-webtest-a-1/` (still running at
  `http://127.0.0.1:8801/?t=9UWmi4wMtSzcpaCESRhVBZAQPHWmiJbY`).
* Workspace has note-b.md (active) + index.md (inactive).
* Mouse down on active tab (coord example `(490, 17)`), drag
  onto adjacent inactive tab (example `(340, 17)`), release.
* Result: active tab disappears from tablist; only the
  adjacent tab remains.
* Drag start x was inside the tab body, not on the `×` close
  button (per measured tab rect `l=433..548`).

## Out of scope

* Tab reordering ergonomics polish (animation, drop indicator
  bar). Just make it not destructive.
* Side-pane tab D&D — already passes.

## How to start

1. Locate the tab D&D handlers in `web/src/` (likely under
   `tabs.svelte.ts` or a tab-tablist component).
2. Inspect what happens when `dragend` / `drop` fires with
   the source and target both in the same tablist on
   adjacent tabs. Suspect: the drop target is being
   interpreted as a close-button hit or as a "remove from
   list" signal.
3. Fix + frontend test.

## Hand-off

Standard: append a "Specialist review requested" entry on
completion and fire
`alex/event-fullstack-architect.md` (type `poke`).

## 2026-05-18 14:15 BST — Priority bump: @@Alex hit this firsthand

@@Alex reproduced this in the running 8801 server while
clicking around: tried to rearrange tabs within the same
pane, tabs DISAPPEARED, **could not recover**. Two
implications:

* **Data-loss class**: if any of the disappeared tabs held
  unsaved content, that content is gone with no undo. Even
  for saved docs the tab-state (cursor position, scroll, find
  buffer) is gone with no "reopen recently closed tab"
  affordance.
* Bumping this from "non-blocking follow-up" to **wave 1
  priority**. Land it before the closeout patch ships;
  shipping a release where everyday tab reordering can
  silently delete a tab is not a good first impression.

While fixing the core regression, please also:

* Add a **"Reopen closed tab"** affordance (menu + keyboard,
  e.g. `Cmd+Shift+T` on native — note browser reserves that
  one, so use the same native-vs-web detection pattern we
  agreed for `Cmd+T`). This is defense in depth even after
  the regression is fixed; tabs can also be closed by
  intentional X-click and people misclick.
* If you find that closing a tab also discards an in-memory
  unsaved buffer, flag it — that's a separate latent issue.

## 2026-05-18 13:45 BST — Specialist review requested

Implemented the tab D&D regression fix plus the requested reopen affordance.

Files changed:

* `web/src/state/tabs.svelte.ts`
* `web/src/state/tabs.test.ts`
* `web/src/components/Pane.svelte`
* `web/src/components/FileEditorTab.svelte`
* `web/src/components/TerminalTab.svelte`
* `web/src/state/shortcuts.ts`
* `desktop/src-tauri/src/serve.rs`

Behavior:

* Same-window tab drops now mark the drag as locally handled before
  `reorderTab` / `moveTab` mutates layout. The source tab's `dragend`
  consumes that marker and does not close a tab that is still present
  after a same-pane reorder.
* Cross-pane tab moves still work through `moveTab`; cross-window moves keep
  the prior "accepted move closes the source tab" behavior.
* Added an in-memory recently-closed tab stack (limit 20) and a
  `reopenClosedTab()` command. File tabs reopen with their in-memory buffer
  and caret preserved; terminal tabs reopen as fresh sessions so a closed PTY
  is not reattached accidentally.
* Added "Reopen Closed Tab" to the empty-pane, file-tab, and terminal-tab
  menus.
* Added keyboard bindings:
  * web: `Ctrl+Alt+T`
  * native: `Mod+Shift+T` via the desktop key bridge

Unsaved-buffer note:

* Closing a dirty file after confirmation previously discarded the in-memory
  buffer. The new reopen stack preserves that buffer, so an accidental close
  is recoverable. This is not durable crash recovery; it is an in-session
  undo affordance.

Verification:

* `npm run check`
* `npm run test -- tabs`
* `npm run build` (passes with existing large-chunk / ineffective dynamic
  import warnings)
* `cargo fmt`

Known gaps:

* No manual browser walkthrough yet for the drag gesture or the new keyboard
  shortcut.
* `crates/chan/src/main.rs`'s static `chan serve --help` shortcut table was
  not updated here because that file currently carries unrelated
  `systacean-1` edits in the dirty tree. The in-app shortcut registry is
  updated.

## 2026-05-18 14:35 BST — @@Architect review: APPROVED for commit (gated on @@Alex)

Solid — and the keyboard-shortcut choices show real thought:

* `Cmd+Shift+T` on native (free; Tauri intercepts before any browser-level
  reservation could matter).
* `Ctrl+Alt+T` on web (because `Cmd+Alt+T` is the web binding for new
  terminal per the earlier `Cmd+T` decision, and `Cmd+Shift+T` is reserved
  by the browser for its own "reopen closed tab"). The `Ctrl` modifier on
  macOS is unusual but unambiguous and justified by the constraint stack.

The local-drag-marker pattern is correct: same-window reorders consume the
marker so `dragend` doesn't double-close. Cross-pane and cross-window
moves keep their prior semantics. The 20-entry recently-closed stack with
the file-buffer-preserved / terminal-fresh asymmetry is the right
separation — terminal "reopen as fresh PTY" avoids the trap of
reattaching to a dead session.

The dirty-buffer survival note is an honest bonus (closing a dirty file
now preserves the buffer for in-session undo, not crash recovery).

### Commit clearance

**APPROVED architect-side.** Gated on @@Alex.

### Follow-ups (not blocking)

* `crates/chan/src/main.rs` help table — `systacean-1` (6c53c2d) is now
  committed, so the tree is no longer dirty on that file. Re-check `git
  status` and add the `Reopen Closed Tab` entry to the help table in the
  same commit as `fullstack-5` if it's a one-line touch; otherwise file
  a tiny follow-up.
* Manual browser walkthrough for the drag gesture + the new keyboard
  shortcut on both native and web. Will fold into a future
  `webtest-a-N` task once you confirm commit.

### Sequencing update

`fullstack-5` lands after `fullstack-1`, before `systacean-2`. Reason:
both `fullstack-5` and `systacean-2` touch `web/src/state/tabs.svelte.ts`
for different purposes (D&D handlers vs autosave serialization). Whoever
commits second rebases — and that's @@Systacean, since the autosave
serialization is the smaller patch.

Updated sequence:

1. ✓ `systacean-1` (6c53c2d) — done.
2. `fullstack-1` — commit now (rebase on systacean-1; mostly
   `store.svelte.ts`).
3. `fullstack-5` — commit immediately after fullstack-1, same agent
   session so the tabs.svelte.ts state is fresh in your head.
4. `systacean-2` — last; @@Systacean rebases on your tabs.svelte.ts
   changes.

Ping me via event when fullstack-1 and fullstack-5 are both in.
