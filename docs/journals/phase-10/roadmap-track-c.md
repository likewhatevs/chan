# Phase 10 Roadmap Track C: Hybrid Pane and Editor Polish

Status: in progress.

Track C collects the Hybrid pane, terminal rendering, and editor-close
polish that should land after the phase 9 validation waves.

Progress:

- 2026-05-24: started the Hybrid menu and shortcut slice:
  - wired direct Previous/Next pane shortcuts.
  - added Close all tabs and Kill pane to the Hybrid hamburger.
  - made close-all / kill-pane use a success/failure result before session
    persistence.
  - made whitespace-only Draft close auto-discard.
  - wired empty-pane close for `Control+D` and native `Cmd+W`.
- 2026-05-24: added File Browser expansion reload persistence for docked
  browsers and File Browser tabs.
- 2026-05-24: retuned Matrix screen-lock rain toward the dcragusa
  MatrixScreensaver reference.
- 2026-05-24: added visible MatrixScreensaver attribution in Settings About.
- 2026-05-24: started per-surface Hybrid body theme overrides for Editor,
  Terminal, File Browser, Graph, and Infographics back-side settings.
- 2026-05-24: queued screen saver presentation state machine cleanup for
  Settings Test and first-input unlock reveal.
- 2026-05-24: queued right-click menu placement and hover/focus motion polish
  from manual screenshots.
- 2026-05-24: queued Draft save to docked File Browser refresh regression.

## Objectives

- Keep terminal rendering stable across Hybrid pane focus changes.
- Make Hybrid hamburger commands match the shortcuts shown in the UI.
- Add transactional close operations for tabs and panes.
- Treat whitespace-only drafts as empty when closing.
- Verify the Graph is always rooted in the filesystem hierarchy.
- Persist File Browser expanded/collapsed directory state across reloads.
- Match Matrix screen-lock rain to the dcragusa MatrixScreensaver reference.
- Credit the MatrixScreensaver source repo wherever the screen-lock rain is
  documented or exposed to users.
- Make screen saver Test mode cover or hide Settings correctly, and reveal the
  unlock card only after first user input.
- Give every Hybrid back-side settings surface the same title-bar shape:
  surface name on the left, Dark / Light body theme switch on the right, and
  OK in a bottom-right footer.
- Keep global Appearance in the main Settings overlay only. It remains the
  default for all Hybrid element bodies unless a per-surface body theme
  override exists.
- Make right-click menus appear close to the click target on all surfaces.
- Standardize the tab-pill hover wobble across right-click menu rows and pane
  focus transitions.
- Ensure Draft writes travel through chan-drive, chan-server watchers, and UI
  tree refresh so saved Drafts appear in docked File Browser without reload.

## 1. Terminal pane rendering bugs

Terminal font disappearance:

- Repro:
  - Create two panes.
  - Place one terminal in each pane.
  - Switch between panes.
- Expected:
  - Terminal font and glyph rendering remain stable.
- Actual:
  - Terminal font rendering can disappear while switching panes.
- Notes:
  - Cover multiple terminals in separate panes.
  - Include ANSI-colored output in the repro matrix.
  - Include scroll-heavy output such as `ps aux` or similar long output.

## 2. Hybrid hamburger shortcut wiring

Next/Previous pane:

- The Hybrid hamburger menu shows:
  - Next pane: `Cmd+]`
  - Previous pane: `Cmd+[`
- Bug:
  - The displayed shortcuts are not wired for at least one manual tester.
- Expected:
  - Menu labels and keybindings agree.
  - The commands work from editor, terminal, and file-browser focus where
    global Hybrid navigation should apply.

## 3. Hybrid hamburger menu expansion

Add pane and tab closing actions after `Previous pane`.

Target order:

- New Draft
- Terminal
- File Browser
- Rich Prompt
- Graph
- Enter Hybrid Nav
- Split right
- Split bottom
- Next pane
- Previous pane
- Separator
- Close all tabs: `Cmd+. x`
- Kill pane: `Cmd+. Backspace`
- Separator
- Focus border colour:
  - blue
  - orange
  - green
  - pink

Behavior:

- `Cmd+. x` closes all tabs in the active pane.
- `Cmd+. Backspace` kills the active pane.
- Both commands are transactional like the other Hybrid Nav operations.
- Failed close/kill operations should leave the pane tree and tabs intact.

## 4. Empty pane close shortcuts

Behavior:

- If the user presses `Control+D` or `Cmd+W` on an empty pane, and the pane
  is not the last pane, close that pane.
- This is equivalent to `Cmd+. Backspace` only for empty panes.
- Do not close the last remaining pane.
- Do not treat a pane with hidden, background, or unsaved tabs as empty.

## 5. Draft close empty-content classification

Current issue:

- Closing a Draft prompts to save even when the only content is whitespace
  or newlines.

Target behavior:

- Classify whitespace-only and newline-only Drafts as empty.
- Auto-discard empty Drafts on close.
- Prompt to save only when the Draft has non-whitespace content or other
  meaningful unsaved state.
- Keep the rule scoped to Draft close behavior. Do not change normal markdown
  file save semantics.

## 6. Graph filesystem hierarchy contract

Current issue:

- The Graph can show filesystem-backed nodes without their filesystem parent
  chain, making the graph look detached from the drive hierarchy.

Target behavior:

- The Graph spine is always the filesystem hierarchy, starting from the drive
  root.
- With the folder filter enabled, files and directories should not appear
  without edges to a parent directory or to the drive root.
- Turning off the folder filter may show non-folder layers without the same
  parent-directory visibility constraint.
- When a file or directory is used as the graph root, the graph still shows
  that node's parent directory chain up to the drive root.
- Show all files as graph nodes, matching File Browser coverage.
- Reuse the existing file-type color coding.
- Clicking a file node opens that file type's existing inspector.
- Markdown document links to tags, mentions, contacts, and other markdown
  documents are an overlay on top of filesystem edges.
- Language bubbles are another overlay layer, with edges to directories.

## 7. File Browser expansion persistence

Current issue:

- File Browser directories can lose their expanded/collapsed state after a
  screen reload.
- The issue must be covered for both docked File Browsers and File Browser
  tabs.

Target behavior:

- Docked File Browsers remember expanded/collapsed directories after a reload.
- File Browser tabs remember their per-tab expanded/collapsed directories
  after a reload.
- Per-tab File Browser state stays independent between multiple File Browser
  tabs.
- The root directory remains available even when every visible child
  directory is collapsed.

## 8. Matrix screen-lock rain reference

Current issue:

- The Matrix screen-lock rain does not closely match the reference at
  `https://dcragusa.github.io/MatrixScreensaver/`.
- Attribution for the visual reference must remain visible because the
  behavior is adapted from the MIT-licensed dcragusa MatrixScreensaver repo:
  `https://github.com/dcragusa/MatrixScreensaver`.

Target behavior:

- Keep the existing Matrix intro text and timing.
- Use the reference rain cadence and cell geometry:
  - 40 ms draw interval.
  - 11 px horizontal spacing.
  - 19 px vertical spacing.
  - dense staggered columns with randomized delays.
- Use the reference color tiers:
  - near-white head glyph.
  - pale lead glyphs.
  - green body glyphs.
  - black per-cell fade and clear behind the trail.
- Keep the implementation self-contained inside the app bundle. Do not add a
  runtime dependency on the external reference site.
- Show a user-visible credit for `dcragusa/MatrixScreensaver` in Settings
  About.

## 8b. Screen saver presentation state machine

Current issue:

- Pressing Test from Settings can leave the Settings overlayshell visually
  competing with the screen saver.
- The lock or unlock card can appear immediately, before the user has clicked
  or pressed a key.

Target behavior:

- When the user clicks Test in Settings, the screen saver should cover the
  Settings overlayshell or the Settings overlayshell should be hidden while
  the screen saver runs.
- When exiting Test mode, the app should return to the Settings overlayshell.
- In normal auto-lock mode, the screen saver should cover the app without
  exposing any lock or unlock card at first.
- The first click or key press reveals the unlock card.
- The unlock card keeps the existing with-PIN and no-PIN behavior after it is
  revealed.
- An acceptable implementation is to make the screen saver Test presentation
  layer sit above the Settings overlayshell, as long as Settings returns after
  unlock or exit.

Smoke:

- Open Settings, then press Test.
- Confirm Settings is not visible above the screen saver.
- Confirm no unlock card is visible until the first click or key press.
- Press a key or click. Confirm the unlock card appears.
- Exit the screen saver. Confirm Settings is visible again.
- Enable auto-lock, close Settings, wait for lock, and confirm the same
  first-input unlock reveal behavior.

## 9. Hybrid surface body themes

Current issue:

- Hybrid back-side settings have inconsistent top bars and local Appearance
  sections.
- Theme overrides previously applied too broadly to the pane chrome instead of
  only to the active element body.

Target behavior:

- Hybrid Terminal, Hybrid Editor, Hybrid File Browser, Hybrid Graph, and
  Infographics settings use a shared back-side shell:
  - left side: surface name.
  - right side: Dark / Light body theme switch.
  - footer: OK button at bottom right.
- The Dark / Light switch is per surface type, not per pane:
  - changing one Editor setting changes all Editor bodies.
  - changing one Terminal setting changes all Terminal bodies.
  - the same rule applies to File Browser tabs, Graph tabs, and Infographics.
- The override applies only to the element body, not the full Hybrid pane,
  tab strip, focus border, or other chrome.
- The back side itself remains themed by the current global UI theme. A change
  made there becomes visible when the tab flips back to the front body.
- The main Settings overlay keeps the only Appearance section. It controls the
  default body theme for all Hybrid elements without a surface override.
- Rich Prompt follows the Terminal body theme for now because it lives inside
  Terminal. Future work may split Rich Prompt into its own settings surface and
  theme override.

Smoke:

- Global dark mode:
  - create an Editor tab.
  - right-click the tab, open Settings, switch Editor to Light.
  - confirm the back side stays visually unchanged.
  - click OK and confirm only the editor document body becomes light while the
    surrounding Hybrid remains dark.
- Global light mode:
  - create a Terminal tab.
  - right-click the tab, open Settings, switch Terminal to Dark.
  - confirm the back side stays visually unchanged except for the switch state.
  - click OK and confirm only the terminal body becomes dark while the
    surrounding Hybrid remains light.
- Four-pane smoke:
  - open two Editors and two Terminals.
  - changing one Editor surface theme updates both Editor bodies.
  - changing one Terminal surface theme updates both Terminal bodies.

## 10. Right-click menu placement and hover motion

Current issue:

- Some right-click menus open far from the click position.
- Manual screenshots show the offset on Terminal, Graph, and Editor tab menus.
- File Browser appears closer to the click and should be used as the reference
  behavior.
- Some right-click menu rows are missing the same hover wobble effect used by
  tab pills. Terminal menu rows are a visible example.
- Pane hover and Hybrid Nav focus changes should use the same focus in/out
  motion language as mouse hover.

Target behavior:

- Right-click menus anchor near the pointer across Terminal, Editor, Graph,
  File Browser, pane chrome, empty panes, and Infographics.
- Menu positioning should account for transformed or flipped ancestors,
  viewport clamping, scroll offsets, and right-edge collisions.
- File Browser menu placement remains correct.
- All right-click menu rows share the tab-pill hover wobble feel, while
  preserving disabled-row styling and reduced-motion behavior.
- Hovering a pane applies the same motion treatment as focusing it.
- Switching panes through Hybrid Nav applies the same focus in/out motion as
  mouse hover.

Investigation notes:

- Compare File Browser's menu path against Terminal, Editor, and Graph tab menu
  paths before changing the shared primitive.
- Check whether the offset comes from menu coordinates being interpreted inside
  a transformed pane, portal root, or clamped local coordinate system.
- Check Terminal's menu row CSS separately because it may bypass the shared
  `HamburgerMenu` row classes.
- Confirm motion does not resize menu rows, pane chrome, or tab pills.

Smoke:

- Right-click the active Terminal tab and terminal body. Menu appears near the
  click.
- Right-click the File Browser tab and File Browser body. Menu remains near
  the click.
- Right-click Graph and Editor tabs. Menus appear near the click.
- Hover Terminal, Editor, Graph, and File Browser menu rows. Each row uses the
  same wobble style as tab pills.
- Hover each pane with the mouse. Pane focus motion matches tab hover motion.
- Use Hybrid Nav to switch panes. Pane focus in/out motion matches mouse
  hover focus in/out.

## 11. Draft save to File Browser refresh

Current issue:

- Creating a Draft with `Cmd+N`, then saving it, does not make the saved Draft
  appear in the docked File Browser.
- The expected pipeline is:
  - editor save writes through chan-server.
  - chan-server writes through chan-drive.
  - chan-drive write triggers the watcher path.
  - chan-server emits the file/tree update to the frontend.
  - the frontend refreshes the visible docked File Browser tree.

Target behavior:

- Saving a new Draft creates or updates the physical Draft file through the
  normal chan-drive write path.
- The watcher or post-save invalidation path reliably reaches all visible File
  Browser surfaces.
- Docked File Browser shows the saved Draft without manual reload.
- File Browser tabs also update, while preserving their own expanded state and
  selection.
- The fix should cover both watcher-delivered updates and any save path that
  currently bypasses the watcher because the write originates from the same
  process.

Investigation notes:

- Trace the Draft `Cmd+N` path, editor save path, chan-server file route,
  chan-drive write, watcher emission, and frontend `refreshTree` trigger.
- Check whether same-process writes suppress or coalesce watcher events.
- Check whether Drafts are filtered out of the File Browser tree after save.
- Check whether docked File Browser and File Browser tab variants consume the
  same tree refresh signal.
- Check whether Draft metadata paths and UI-facing `Drafts/...` paths resolve
  differently for saved files.

Smoke:

- Open a docked File Browser.
- Press `Cmd+N` to create a Draft.
- Type non-whitespace content.
- Save the Draft.
- Confirm the docked File Browser shows the saved Draft without reload.
- Open a File Browser tab and repeat. Confirm the tab updates without losing
  expanded state.
- Repeat after collapsing and expanding parent directories.

## Test plan

- Hybrid menu tests:
  - Verify menu ordering and separators.
  - Verify labels for `Cmd+. x`, `Cmd+. Backspace`, `Cmd+[`, and `Cmd+]`.
  - Verify Next/Previous pane shortcuts dispatch the matching commands.
- Transaction tests:
  - Close all tabs succeeds.
  - Close all tabs failure leaves all tabs and pane structure unchanged.
  - Kill pane succeeds.
  - Kill pane failure leaves the pane tree unchanged.
- Empty pane tests:
  - `Control+D` closes a non-last empty pane.
  - `Cmd+W` closes a non-last empty pane.
  - Neither shortcut closes the last pane.
  - Neither shortcut closes a pane containing tabs.
- Terminal visual tests:
  - Two panes with one terminal each.
  - Pane switching preserves font and glyph rendering.
  - ANSI-colored output remains stable.
  - Long scrollback output remains stable.
- Draft tests:
  - Empty Draft closes without save prompt.
  - Whitespace-only Draft closes without save prompt.
  - Newline-only Draft closes without save prompt.
  - Non-whitespace Draft still prompts to save.
- Graph tests:
  - Drive-root graph shows directory and file nodes from the filesystem tree.
  - File Browser and Graph expose the same file-node coverage.
  - No file or directory appears without a parent-directory or drive-root edge
    while the folder filter is enabled.
  - Rooting the graph at a nested file shows parent directories up to the
    drive root.
  - Rooting the graph at a nested directory shows parent directories up to
    the drive root.
  - Markdown tag, mention, contact, and wiki-link edges render as overlays
    while filesystem edges remain present.
  - Language bubbles connect to directories as an overlay layer.
  - File-node clicks open the matching inspector for that file type.
- File Browser tests:
  - Docked File Browser expansion writes a reload-safe snapshot.
  - Reload restore applies the last same-screen expansion snapshot.
  - File Browser tab expansion writes into that tab's serialized layout state.
  - Multiple File Browser tabs keep independent expansion state.
- Matrix screensaver tests:
  - Pin intro message sequence and timing.
  - Pin rain spacing, cadence, density, and color tiers.
  - Verify reduced-motion still renders a static Matrix frame.
- Screen saver state tests:
  - Settings Test hides or covers the Settings overlayshell while active.
  - Settings returns after exiting Test mode.
  - No unlock card renders until the first click or key press.
  - First click or key press reveals the existing unlock card.
  - Auto-lock and Test mode share the same first-input reveal behavior.
- Hybrid surface theme tests:
  - Back-side settings for Editor, Terminal, File Browser, Graph, and
    Infographics use the shared title-bar and bottom-right OK shell.
  - Back-side settings no longer contain local Appearance sections.
  - Global Appearance changes still recolor body surfaces without overrides.
  - Per-surface overrides recolor only the matching front-side body.
  - Two Editors update together after changing one Editor setting.
  - Two Terminals update together after changing one Terminal setting.
- Right-click menu tests:
  - Terminal, Editor, Graph, File Browser, empty-pane, and Infographics context
    menus open within a small offset from the pointer unless clamped by the
    viewport.
  - File Browser placement remains the reference passing case.
  - Menu placement is stable in split panes, flipped Hybrid backs, and narrow
    right-edge positions.
  - Shared menu row hover wobble applies to Terminal, Editor, Graph, File
    Browser, pane chrome, empty-pane, and Infographics menus.
  - Disabled menu rows do not wobble as active commands.
  - Pane mouse hover and Hybrid Nav pane switch use the same focus in/out
    motion.
- Draft save refresh tests:
  - Saving a new Draft makes it appear in the docked File Browser without a
    reload.
  - Saving a new Draft makes it appear in an open File Browser tab without
    losing that tab's expansion state.
  - Watcher or explicit invalidation events fire for same-process Draft saves.
  - Draft UI paths and physical metadata paths resolve to the same visible
    File Browser entry.

## Assumptions and non-goals

- Track C is web and embedded app work, not desktop-native shell work.
- Linux desktop launch failures live in Track A.
- Terminal renderer stability is tested visually in Browser/iab and, when
  useful, through automated DOM or canvas checks.
