# Phase 10 Roadmap Track C: Hybrid Pane and Editor Polish

Status: complete.

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
- 2026-05-24: verified per-surface Hybrid body theme overrides for Editor,
  Terminal, File Browser, Graph, and Infographics back-side settings.
- 2026-05-24: verified screen saver presentation state machine cleanup for
  Settings Test and first-input unlock reveal.
- 2026-05-24: queued right-click menu placement and hover/focus motion polish
  from manual screenshots.
- 2026-05-24: fixed right-click menu placement by portaling custom Terminal,
  Editor, and Graph menus to the document body; shared row hover motion across
  menus and pane focus.
- 2026-05-24: fixed Draft promotion refresh for docked File Browsers by
  including docked browsers in watcher scopes and explicitly refreshing the
  promoted drive path after same-process Draft saves.
- 2026-05-24: tightened the Graph filesystem spine so scoped filesystem
  graph responses emit ancestor chains to the drive root, and the semantic
  graph fills filesystem nodes from the unified tree independent of visual
  depth.
- 2026-05-24: verified Terminal renderer stability guards, Hybrid menu
  transactions, empty-pane close shortcuts, File Browser expansion
  persistence, right-click chrome, and Draft promotion refresh with focused
  frontend tests.
- 2026-05-24: fixed the live Terminal pane-switch repro by moving pane
  hover/focus/wobble scale animation off the pane element and onto a
  pointer-transparent chrome layer so xterm WebGL canvases are not transformed
  during focus changes.
- 2026-05-24: queued File Browser drag-and-drop transfer work: browser-side
  drop-to-upload, status-bar progress and cancel, plus native desktop
  drag-out/download follow-up in Track A.
- 2026-05-24: implemented File Browser drop-to-upload with an exact-name
  `/api/files/upload` route, status-bar progress and cancel, conflict refusal,
  and visible tree refresh after successful uploads.
- 2026-05-25: queued actionable persistent drive warnings after live IAB showed
  `Broken draft Drafts/team-phase-10: missing draft.md` as a passive status-bar
  label with no click target, dialog, repair path, or delete path.
- 2026-05-25: implemented typed drive-warning status actions and a Drive
  Warnings dialog with copy path, session dismiss, and safe broken Draft
  metadata discard through `/api/drafts/discard`.
- 2026-05-25: confirmed the live Terminal pane-switch font/glyph bug is gone
  after retesting two split panes with terminal output, ANSI-colored output,
  and focus changes.
- 2026-05-25: restored browser ownership of plain `Cmd+L`; chan screen lock is
  only `Cmd+. L`.
- 2026-05-25: tightened the Matrix follow-up scope: replace the local
  approximation with a high-fidelity port or adaptation of the MIT-licensed
  `dcragusa/MatrixScreensaver` source code, preserving visible credit and
  license attribution.
- 2026-05-25: ported the Matrix lock closer to the upstream
  `dcragusa/MatrixScreensaver` implementation, bundled the upstream Matrix
  font assets and MIT notice, and raised the lock layer above ambient status
  chrome after live IAB showed a drive-warning pill leaking over the
  screensaver.
- 2026-05-25: scoped the next transfer wave after Track A's desktop smoke:
  browser/right-click download paths exist, native desktop drag-out still
  needs Track A's bridge, and Track C should add Upload/Download actions to
  the shared file/directory inspector.
- 2026-05-25: implemented the initial shared-inspector transfer wave:
  file/directory inspectors expose Upload and Download, file Upload replaces
  the selected file through the multipart upload route, directory Upload adds
  into the selected directory, and chan-drive rejects non-UTF-8 raw bytes for
  editable text targets.
- 2026-05-25: implemented Draft editor explicit Save-to-drive action. Draft
  tabs replace the menu-top `Name` row with Save, reuse the existing Draft
  promotion dialog, and continue on the promoted drive path after Save.
- 2026-05-25: tightened docked File Browser context menus. Upload and Download
  are docked-only right-click rows, Open in File Browser opens a normal File
  Browser tab with Details focused on the selected row, and the no-selection
  fallback opens Details for the drive itself.
- 2026-05-25: consumed Track A's relationship NDJSON streams in the browser:
  typed report/backlink/graph readers, streaming FileInfoBody report/backlink
  state, graph node upserts, graph edge dedupe, and in-flight reload
  cancellation guards.
- 2026-05-25: live-smoked the streaming UI on a throwaway drive. Editor load,
  editor Details for `CHANGELOG.md`, backlinks, semantic Graph from the active
  file, graph reload, and the three stream endpoints all passed with no
  current-run console errors.
- 2026-05-25: live-regressed shared inspector transfer affordances across
  Editor details, File Browser tab file and directory details, and Graph file
  and directory node details. Direct endpoint smoke passed for file replace,
  directory upload, file download, and directory archive download.
- 2026-05-25: live-regressed Draft explicit Save-to-drive. Save replaced the
  Draft path row, the promotion dialog saved to `notes/track-c-saved-draft.md`,
  the tab continued on the promoted path, and the docked File Browser showed
  the saved file without reload.
- 2026-05-25: live-regressed File Browser expansion restore after window
  reload. Docked and tab File Browsers restored expanded `nested/` and
  `notes/` state.
- 2026-05-25: noted a backend-owned transfer gap for Track A coordination:
  replacing a text-class markdown file with non-UTF-8 bytes is rejected, but
  the route currently returns HTTP 500 instead of a user/actionable 4xx.
- 2026-05-25: fixed docked File Browser empty-menu drive path click. The drive
  row now opens a normal File Browser tab with drive Details, matching the
  docked-only Open in File Browser action when there is no selection.
- 2026-05-25: completed the next live Track C regression pass on top of
  Track A `9e16a4b`. Focused web tests, `npm run check`, `npm run build`,
  and `cargo build -p chan` passed. Live Browser smoke passed for transfer
  endpoints, streaming inspector/report/backlink/graph intake, graph reload
  cancellation, docked File Browser drive-row actions, Terminal ANSI
  scroll-heavy pane switching, Graph filesystem spine, File Browser expansion
  restore, Matrix lock coverage, broken Draft warning dialog, Rich Prompt
  submit/archive/clear/race behavior, Spawn agents clipboard/preflight, and
  rapid editor autosave/index convergence.
- 2026-05-25: rechecked the prior Track A transfer gap after `9e16a4b`.
  Non-UTF-8 replacement into editable markdown now returns HTTP 415 with the
  expected upload failure path.
- 2026-05-25: rapid-edit validation showed the editor buffer, saved file bytes,
  and BM25 index all converging on the final browser edit. Search hit the final
  `TRACKC_FINAL_C` content and did not retain the old payload terms for
  `plain.txt` after reload.
- 2026-05-25: added the empty-Hybrid masked `chan-mark.png` treatment to the
  plain screen-lock theme so the plain lock no longer reads as a blank grey
  cover. Matrix lock rendering remains unchanged.
- 2026-05-25: wrapped Track C. The final plain screen-lock follow-up landed in
  `b845531`, all Track C-owned focused tests, builds, and embedded Browser
  smokes passed, and no Track C-owned code or validation item remains open.

Closeout status:

- Final handoff and verification log:
  `docs/journals/phase-10/track-c-next-agent-handoff.md`.
- Shared inspector Upload/Download, Draft explicit Save, File Browser expansion
  restore, docked File Browser drive-row actions, streaming inspector/graph UI,
  Rich Prompt browser validation, rapid-edit validation, Matrix lock, plain
  screen-lock mark, persistent drive warnings, Terminal pane switching, and
  Graph filesystem spine all have current Track C coverage.
- Keep native desktop drag-out/download scoped to Track A.
- No Track C teardown action remains beyond leaving the workspace clean of
  Track C temp drives and servers.
- Any future findings should be cut as new follow-up work with explicit owner
  scope.

## Objectives

- Keep terminal rendering stable across Hybrid pane focus changes.
- Make Hybrid hamburger commands match the shortcuts shown in the UI.
- Add transactional close operations for tabs and panes.
- Treat whitespace-only drafts as empty when closing.
- Verify the Graph is always rooted in the filesystem hierarchy.
- Persist File Browser expanded/collapsed directory state across reloads.
- Match Matrix screen-lock rain to the dcragusa MatrixScreensaver reference.
- Reuse the empty-Hybrid grey logo treatment in the plain screen-lock theme.
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
- Add File Browser drag-and-drop upload and download flows without toolbar
  buttons.
- Add Upload and Download actions to shared file and directory inspectors so
  File Browser, Graph, and Editor details surfaces expose the same transfer
  controls.
- Replace the Draft editor menu's file-path `Name` row with an explicit
  Save-to-drive action that uses the same Draft promotion workflow as closing
  a Draft and choosing Save.
- Make persistent drive warnings actionable instead of passive status text,
  starting with broken Draft metadata warnings.

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
- Do not keep tuning a hand-rolled lookalike. Port or adapt the upstream
  `matrix.js` and `matrix.css` behavior into the Svelte component unless an
  app-shell constraint requires a small wrapper.
- Keep upstream rain cadence and cell geometry as the source of truth:
  - 40 ms draw interval.
  - 11 px horizontal spacing.
  - 19 px vertical spacing.
  - dense staggered columns with randomized delays.
- Keep upstream color tiers:
  - near-white head glyph.
  - pale lead glyphs.
  - green body glyphs.
  - black per-cell fade and clear behind the trail.
- Prefer the upstream Matrix font assets when bundling and licensing are
  acceptable. If a font asset cannot be bundled, document the fallback and
  verify the visual delta.
- Keep the implementation self-contained inside the app bundle. Do not add a
  runtime dependency on the external reference site.
- Show a user-visible credit for `dcragusa/MatrixScreensaver` in Settings
  About, and include the MIT license notice with any copied or substantially
  adapted source.

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

## 12. File Browser drag-and-drop transfer

Current issue:

- File Browser has no direct drag-and-drop transfer path for moving local files
  into a drive or dragging drive files back to the user's desktop.
- The desired UI should avoid extra toolbar buttons and use the existing File
  Browser and status bar surfaces.

Target behavior:

- Dropping a file onto a docked File Browser or File Browser tab uploads it
  into the drive.
- Dropping onto a folder row uploads into that folder.
- Dropping onto the root or empty browser area uploads into the current visible
  root.
- Uploads use the normal chan-server and chan-drive write path, including path
  sandboxing, editable-text rules where applicable, and special-file refusal.
- Conflict behavior must be explicit. Do not silently overwrite an existing
  file.
- The status bar shows active upload progress, including filename or count,
  bytes where available, completion, and failure.
- The status bar exposes a cancel affordance for active uploads.
- Cancelling an upload leaves no visible partial file, or cleans up any staged
  temporary file before the tree refreshes.
- File Browser refreshes after upload while preserving expanded/collapsed
  directory state.
- Dragging a file from File Browser toward the OS desktop should download or
  export that file where the browser or native host supports drag-out.
- Dragging a directory from File Browser toward the OS desktop should export
  that directory where supported. The export format can be a real directory or
  an archive, but it must preserve the tree and names.
- Shared inspectors for files and directories should expose Upload and
  Download buttons:
  - file upload replaces that file's bytes through a chan-drive-backed route;
  - directory upload adds the uploaded file inside that directory;
  - file download returns that file's bytes;
  - directory download returns the existing directory archive flow.
- File Browser right-click menus should expose Upload and Download only in the
  docked File Browser, because docked mode has no inspector panel. Tab and
  overlay File Browser menus should not show those transfer rows.
- Docked File Browser right-click menus should not expose Settings. They should
  expose Open in File Browser, which creates a normal File Browser tab with the
  selected file or directory selected and the inspector open.
- If the docked File Browser has no selected file or directory, Open in File
  Browser should create a normal File Browser tab with the drive itself
  selected in the inspector.
- The docked File Browser right-click transfer block order is separator,
  Upload, Download, separator.
- Upload and Download actions should include an info affordance or hover text
  explaining that File Browser also supports drag/drop where the platform
  allows it.
- Uploading binary bytes over a `.md` or other editable text path must not
  let the rendered editor interpret arbitrary binary as markdown. The
  chan-drive boundary should reject or classify such content more effectively;
  the frontend must show a binary/non-renderable state instead of rendering
  unsafe text if such content already exists.

Investigation notes:

- Use standard browser drag/drop `DataTransfer` for inbound file drops where
  possible.
- Treat File System Access APIs as optional capability checks, not a required
  dependency for the embedded app.
- Upload progress may require an XHR path or a fetch streaming path with
  `AbortController` cancellation, depending on browser and webview support.
- Directory upload can be a later expansion if it requires non-standard
  browser entry APIs.
- Browser drag-out/download feasibility is uncertain, especially for
  directories. Native desktop support is tracked in Track A.
- Track A's 2026-05-25 macOS smoke found that WKWebView did not create a
  Finder file from the browser `DownloadURL` drag payload. Track A owns the
  native bridge using the same `download=1` URL.
- Keep any temporary upload or export staging outside the user-content tree
  until the final chan-drive write or explicit desktop export step.

Smoke:

- Drop a small text file onto the File Browser root and confirm it appears
  without reload.
- Drop an image or other binary file onto a nested folder and confirm it
  appears in that folder.
- Start a large upload, cancel it from the status bar, and confirm no partial
  file remains.
- Drop a file that conflicts with an existing path and confirm the UI does not
  silently overwrite.
- Repeat upload smoke in docked File Browser and File Browser tabs.
- Confirm docked File Browser right-click menus show separator, Upload,
  Download, separator, while File Browser tab and overlay right-click menus
  omit Upload and Download.
- From docked File Browser, use Open in File Browser on a file and a directory.
  Confirm the new File Browser tab selects that row and opens Details.
- From docked File Browser with no row selected, use Open in File Browser and
  confirm the new File Browser tab opens Details for the drive itself.
- From a file inspector in File Browser, Graph, and Editor details, use Upload
  to replace the selected file and Download to retrieve it.
- From a directory inspector in File Browser and Graph, use Upload to add a
  file to the selected directory and Download to retrieve the directory
  archive.
- Upload binary bytes to a markdown path and confirm the editor does not
  render the file as markdown.
- Drag a file from File Browser to the desktop where supported and confirm the
  exported file bytes match.
- In the desktop build, drag a directory from File Browser to the desktop and
  confirm the exported tree or archive preserves names and contents.

## 12b. Draft editor explicit Save action

Current issue:

- Draft editor tab menus show the same editable `Name` row as regular files,
  so `Drafts/untitled/draft.md` looks like a direct rename target.
- Drafts are promoted into the drive through the Draft close Save workflow,
  not by renaming the metadata path in place.

Target behavior:

- For Draft file tabs only, replace the menu-top `Name` row with a Save
  button.
- Pressing Save opens the same destination workflow as closing a Draft and
  choosing Save.
- Single-file Drafts save to a drive file.
- Draft workspaces with attachments save to or merge into a drive directory.
- After a successful explicit save, the tab should continue on the promoted
  drive path instead of leaving the user on the stale `Drafts/...` metadata
  path.
- Regular non-Draft file tabs keep the editable `Name` row.

Smoke:

- Create a Draft with `Cmd+N`, open the editor menu, and confirm the top row
  is Save, not `Name`.
- Click Save, choose a destination, and confirm the Draft promotes into the
  drive and the tab now points at the promoted path.
- Repeat with a Draft workspace that has an attachment and confirm the
  destination is a directory.
- Open a normal markdown file and confirm the editable `Name` row still
  appears.

## 13. Actionable persistent drive warnings

Current issue:

- Drive boot can surface broken Draft metadata through a persistent status-bar
  message.
- Live IAB evidence on 2026-05-25:
  `Broken draft Drafts/team-phase-10: missing draft.md`.
- The message renders as a passive status-bar label.
- The user cannot click it to inspect, fix, dismiss, or delete the broken
  Draft metadata.
- This makes the notification non-actionable and leaves the recovery path
  implicit.

Target behavior:

- Persistent drive warnings use a typed status action, not only a string.
- Clicking or keyboard-activating the warning opens a dialog.
- The dialog lists each warning with path, reason, and available safe actions.
- Broken Draft warnings should offer at least:
  - inspect or copy the metadata path.
  - discard/delete the broken Draft metadata when the server can verify it is
    safe.
  - dismiss only for the current session if deletion is not desired.
- Repair/delete actions must route through chan-server and the existing Draft
  metadata boundary. Do not delete arbitrary filesystem paths from the
  frontend.
- Generic status messages remain passive unless they carry an explicit typed
  action.

Smoke:

- Create or seed a drive with a broken Draft metadata entry.
- Load the app and confirm the status bar shows the warning.
- Confirm the warning is reachable by pointer and keyboard.
- Open the warning dialog and verify path and reason text.
- Dismiss the dialog and confirm the warning remains or clears according to the
  chosen action.
- Delete or discard the broken Draft metadata through the dialog, reload, and
  confirm the warning is gone.

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
- File Browser transfer tests:
  - Drop-to-upload works in docked File Browser and File Browser tabs.
  - Upload progress appears in the status bar.
  - Upload cancel stops transfer and leaves no partial visible file.
  - Conflict handling does not silently overwrite existing files.
  - File Browser refresh preserves expanded/collapsed state after upload.
  - Browser drag-out either downloads the selected file or reports unsupported
    behavior cleanly.
  - Desktop drag-out coverage is verified through Track A.
- Persistent warning tests:
  - Broken Draft warning status is clickable and keyboard reachable.
  - Warning dialog lists path and reason.
  - Safe discard/delete clears the warning after reload.
  - Session dismiss does not delete metadata.
  - Generic transient status messages remain passive.

## Assumptions and non-goals

- Track C is web and embedded app work, not desktop-native shell work.
- Linux desktop launch failures live in Track A.
- Native desktop drag-out/download support for File Browser lives in Track A.
- Terminal renderer stability is tested visually in Browser/iab and, when
  useful, through automated DOM or canvas checks.
