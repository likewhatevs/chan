# frontend-10: file-browser title + "Terminal from here"

Owner: @@Frontend
Status: REVIEW

## Goal

Two small file-browser-adjacent UX additions.

## Scope

### File browser title shows the selected item's full path

* Today: file browser overlay header reads a static label.
* Change: the header reads the **full drive-relative path** of the
  currently selected entry. Empty selection falls back to the
  drive root path (or a sensible default like "drive").
* The path text should truncate with ellipsis on overflow and
  carry the full path as a `title` attribute (hover).
* Source: `web/src/components/FileBrowserOverlay.svelte`.

### "Terminal from here" action

* New context-menu action on file browser rows (and ideally on
  the editor tab menu too, for parity with "New File from current
  file's parent").
* Behavior:
  * On a **directory** row: open a new terminal tab with CWD set
    to that directory. Empty prompt; user types whatever they
    want.
  * On a **file** row: open a new terminal tab with CWD set to
    the file's **parent** directory, **and** seed the prompt
    with the file path so the user can type a command in front
    of it. Final visible state:
    ```
    $ <cursor> path/of/browser/selection
    ```
    (one space between cursor and path; cursor at the start of
    the input). Typing `vim`, `cat`, `less` etc. then completes
    to e.g. `vim path/of/browser/selection` ready to Enter.
* The new terminal follows the existing Terminal-N naming
  enumeration from [backsystacean-1](./backsystacean-1.md).
* CWD plumbing: PTY spawn already accepts a working directory.
  Check the terminal session creation path
  (`/api/terminal/ws` query / chan-server terminal_sessions); add
  a `cwd=<rel>` param if not already present.
* Sandbox: the CWD must resolve under the drive root. Reject
  requests with paths outside the drive (consistent with the
  rest of chan-drive's path sandbox).

#### File-prompt seeding mechanism

* Path is drive-relative; if the file's parent is the CWD, the
  bare basename is enough (looks cleaner). Otherwise use the
  path relative to CWD.
* Shell-quoting: if the path contains a character outside
  `[A-Za-z0-9/_.-]`, single-quote it (and escape embedded
  single quotes the standard way: `'\''`). Otherwise raw.
* Byte sequence to inject once the PTY is attached and the first
  prompt has rendered:
  1. Write `" " + <path>` (leading space + path).
  2. Write `\x01` (Ctrl+A) to move the readline cursor to the
     start of the input. The leading space then separates the
     cursor from the path.
* Timing: simplest workable path is to send the seed bytes
  after the first PTY output chunk (which typically contains
  the prompt). A small delay (e.g., 150ms) on top is fine if
  needed. Do not block the spawn on it.
* The seed runs only for **file** rows. Directory rows skip it.
* If the user types a path-altering command (Ctrl+E to end then
  edits, etc.), no special handling needed; readline owns the
  buffer from that point on.

## Out of scope

* Spawning the terminal in a specific pane (defaults to the same
  pane logic as today's "New Terminal" action).
* Configuring the shell or initial env beyond what `CHAN_TAB_NAME`
  and the spawn-time MCP discovery vars already provide.

## Relevant links

* Request: [request.md](./request.md)
* Journal: [journal.md](./journal.md)
* Terminal sessions: `crates/chan-server/src/terminal_sessions.rs`,
  `crates/chan-server/src/routes/terminal.rs`.
* File browser: `web/src/components/FileBrowserOverlay.svelte`,
  `web/src/components/FileTree.svelte`.

## Acceptance criteria

* File browser overlay header renders the full drive-relative
  path of the selected entry; ellipsis on overflow; full path on
  hover via `title`.
* "Terminal from here" appears in the file browser row context
  menu for both files and directories.
* Clicking opens a new terminal tab with CWD set correctly
  (directory itself for directories, parent for files).
* CWD path is sandboxed under the drive root.

## Tests

* Vitest coverage for the title path resolution + the menu
  action's CWD computation (file -> parent dir).
* Server-side `cargo test -p chan-server terminal` if the CWD
  param needs new server work; assert paths outside the drive
  are rejected.
* `npm --prefix web run check`, `npm --prefix web test -- --run`,
  `npm --prefix web run build` green.
* Pre-push gate green.

## Review and hardening

* @@Frontend self-review for the path-sandbox plumbing on the
  client (the server is the trust boundary; the client should
  still avoid sending obviously-bad paths).
* @@Webtest live verification: open a terminal from a deep
  directory, from a file inside it, confirm `pwd` reads back the
  expected path.

## Progress notes

* 2026-05-18: File browser overlay header now derives from the
  current `browserSelection.path`, falling back to the drive root/name
  label, with existing ellipsis styling and `title` hover intact.
* 2026-05-18: Added shared terminal-from-here path helper:
  directories use themselves as `cwd`; files use parent directory
  and seed the prompt with a shell-quoted basename.
* 2026-05-18: File browser row context menu and file editor tab menu
  now include "Terminal from here".
* 2026-05-18: `/api/terminal/ws` accepts `cwd=` for fresh sessions
  only. Server resolves it through `resolve_safe_strict`, requires a
  directory under the drive root, and passes it to PTY spawn.

## Completion notes

Ready for review. Validation:

* `cargo test -p chan-server terminal`
* `npm --prefix web run check`
* `npm --prefix web test -- --run`
* `npm --prefix web run build` (passes with existing Vite chunk-size
  and ineffective dynamic import warnings)
