# v0.64.0

The command launcher: a Cmd+K palette that lists, filters, and runs every UI action, plus New diagram, the tab right-click menus trimmed to what the launcher now owns, a workspace held open by another machine surfaced in the launcher, and the hanging Export-to-PDF removed. Commit range `dc61d584..d53467cd` on `main` (delivered on `command-launcher-v0640`, worktree `../chan-command-launcher`, fast-forwarded into `main`). Coordination tree `dev/v0.64.0/`. GA cut 2026-07-05.

## The round

The launcher itself landed as a five-lane delivery on one shared worktree, coordinating through an append-only task/journal bus in `dev/v0.64.0/`:

- Lead: the Export-to-PDF frontend removal, the App.svelte `runCommand` + `shortcuts.ts` Cmd+K wiring and the `windowMode` gate de-dup, integration, the CHANGELOG and CLI-help regen, the gate, and the report.
- Launcher-Core: the `commands.ts` registry and registration API, the single-source `windowMode` gate, the Spotlight overlay (focus trap, listbox a11y, type-ahead), the `OverlayId` / overlay-stack extension, and the active graph/dashboard tab accessors.
- Commands-A: the Editor, Graph, and Dashboard catalog, the active-editor bridge (`mountedEditors`), and the New diagram frontend wiring.
- Commands-B: the Terminal, Panes, Global, Workspace, and Search net-new catalog.
- Server: the New diagram endpoint (draft-primary generalized off the hardcoded `draft.md`) and the desktop Export-to-PDF removal.

The host then landed a follow-on pass on the same branch: the launcher chord wired through the SPA keymap and desktop key bridge (Cmd+K on macOS, Ctrl+Alt+K on the web and Linux / Windows so a focused terminal keeps plain Ctrl+K); alphabetical launcher ordering with the active tab's surface pinned first plus a File Browser command category; the terminal, editor, and graph tab right-click menus trimmed to their surface controls with the discoverability moved into the launcher; the file browser tab menu trimmed to match; the "Computers" / "This machine" launcher labels; and two launcher/library fixes (a workspace held open by a foreign holder shown as locked, and library state kept in sync with devserver state). A mandatory full `make pre-push` before the cut caught a regression the follow-on introduced: the churn test `close_then_reopen_under_pressure` failed because a lagging file-watch reload could clobber a same-handle registry write; `ee535a1e` serializes `reload_registry` under the registry mutex, and the re-gate came back green.

## What shipped

- **The Cmd+K command launcher and its catalog.** A Spotlight-style overlay lists every UI action per category, fuzzy-searchable, keyboard-driven, with each command's current chord shown read-only. Availability is window-mode AND active-surface: a standalone terminal window hides the workspace-only commands, and surface commands appear only when their tab kind is active. Sections and rows sort alphabetically with the active tab's surface pinned first. Cmd+K on macOS; Ctrl+Alt+K on the web and Linux / Windows.
- **New diagram.** A new `POST /api/diagrams/new` seeds a valid Excalidraw scene as a single-file draft; the draft module's primary-file detection was generalized off the hardcoded `draft.md` so a `.excalidraw` draft inspects, promotes to `<name>.excalidraw`, and discards cleanly.
- **Tab right-click menus trimmed to the surface.** The terminal, editor, graph, and file browser tab menus keep only their surface controls (broadcast, page width, graph depth and filters, the file browser dock toggles) plus Close; everything else they used to list is reachable from the command launcher.
- **A workspace held open by another machine shows as locked in the launcher.** A writer-lock probe marks a foreign-held workspace with a lock icon and a disabled toggle with the reason on hover, instead of a control that can only fail; the library view stays in sync with live devserver state.
- **Export-to-PDF removed, everywhere.** The Inspector action and its print engine are gone on web and desktop (the native macOS path could hang the shell indefinitely). The PDF viewer stays.

## The cut

One GA commit `d53467cd` "chore(release): bump 0.64.0" moved every version pin from 0.63.0 to 0.64.0 (root `Cargo.toml` workspace and path-dep pins, `gateway/Cargo.toml`, `desktop/src-tauri/tauri.conf.json`, the six web `package.json` files and the marketing `@chan/*` pins), regenerated the root, gateway, and web lockfiles, and dated the CHANGELOG `## [v0.64.0]` section. It sits on top of the regression fix `ee535a1e`. The full `make pre-push` ran green (fmt, `clippy -D warnings`, `test --all-targets`, `--no-default-features` build, the gateway workspace build, `web-check` with svelte-check and both vitest suites and the production builds, and the marketing check). A `publish=false` dispatch of `release.yml` on `main` (run 28731218077) exercised the cross-platform validate and the macOS sign / notarize path green, then the annotated `v0.64.0` tag on the GA commit ran `release.yml` with `publish=true` (run 28732530176), shipping the 26-asset GitHub Release, regenerating the `/dl` `latest.json` for CLI and desktop to 0.64.0, deploying Pages, and pushing the four Docker manifests. The `command-launcher-v0640` branch was deleted local and remote.

## Carryover

- v0.65.0: the configuration UI over a per-library TOML with the shortcut-assignment surface, the workspace-UX polish, and the v0.64.0 bug fixes, then moving config content out of the back-of-pane into the config UI and the no-defaults shortcut cleanup (briefs and team scaffold in `dev/v0.65.0/`).
- Open bug reports in `dev/v0.65.0/bug-reports.md`: the terminal cwd-unavailable pill being un-dismissible (root-caused: `terminalCwdUnavailable` omits `ui.statusKind = "persistent"`), the editor word-selection highlight, list image-paste losing the list on Enter, editor image copy, and reopen-last-tab recovering a just-deleted draft.
- Accepted gaps from the launcher round: Swap and New window are out of the launcher, and Convert to contact / slides is descoped pending a frontmatter helper (see `dev/v0.64.0/host-smoke.md`).
