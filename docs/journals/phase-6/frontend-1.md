# @@Frontend task 1

Status: REVIEW

Goal: land low-coupling frontend refinements from [request.md](./request.md).

Scope:
- New-file dialogs should start on `untitled.md`, with the stem selected.
- Editor New File should default to the current file's parent directory.
- File editor and file browser context menus should expose Copy Path.
- Overlay backdrops should use the same context menu as their panel.
- Embedded terminals should repaint when the app theme changes.
- Terminal top-bar status/actions should move into the terminal bubble menu.
- File-browser filesystem graph entrypoints should default to the drive graph and preselect the clicked file/directory.
- Duplicate file basenames in a pane should be disambiguated in the tab strip.
- Terminal right-click should expose basic terminal and pane actions.
- Language graph nodes/edges should use a distinct royal-pink color instead of green.
- User-facing copy should prefer "directory" over "folder" in the touched frontend flows.
- Search overlay context menu should include Reload next to the Details toggle.

Progress:
- Updated `fileOps.createFile` to seed `untitled.md` via the shared path helper.
- Updated `PathPromptModal` to select only the `untitled` stem for create-file prompts.
- Added editor menu Copy File Path and parent-directory New File behavior.
- Added FileTree Copy Path for file and directory row context menus.
- Routed File Browser, Search, and Graph overlay backdrop context menus through their existing menu handlers.
- Added terminal theme refresh from the shared `ui.theme` state without reconnecting the PTY.
- Removed the persistent terminal header; status, missed-bytes, find, copy scrollback, restart, and resume now live in the terminal tab bubble. Find opens as a transient in-terminal search box.
- Changed filesystem graph entrypoints to use drive scope with pending selection, and taught filesystem graph mode to load drive/global scopes directly.
- Added pane-local tab label disambiguation using the shortest unique path suffix, with ellipsis when leading path segments are omitted.
- Added terminal right-click menu support for Copy, Paste, Find, Copy Scrollback, Restart, New Terminal, split pane, Search, and Settings.
- Added `--g-language` and routed language graph nodes/edges/chips through it, avoiding the green tag/accent collision.
- Updated visible labels in file browser, file tree, inspector, drive info, path prompt, delete confirmation, dashboard, and import contacts modal from folder to directory where applicable.
- Added Search overlay menu Reload action that reruns the current query or clears empty-query results.

Verification:
- `npm run check` in `web` passed with 0 errors and 0 warnings.
- `npm test` in `web` passed: 18 files, 160 tests.
- `npm test -- src/state/store.test.ts` in `web` passed: 1 file, 7 tests.
- `npm test -- src/state/tabs.test.ts` in `web` passed: 1 file, 11 tests.
- Latest `npm test` in `web` passed: 18 files, 170 tests.
- Latest `npm run check` in `web` passed with 0 errors and 0 warnings.
- Latest `npm run build` in `web` passed with existing chunk-size warnings.

Risks / follow-up:
- Clipboard writes require browser/WebView permission; failures are surfaced in the status line.
- Manual webtest coverage still needed for modal selection and backdrop right-click behavior.
- Worktree had pre-existing edits across frontend and backend files; I only worked with the frontend surfaces described above and left unrelated backend/report/drive changes untouched.
- Terminal CWD-specific context-menu actions and live `CHAN_TAB_NAME` updates still need backend/session support before they can be wired honestly.
