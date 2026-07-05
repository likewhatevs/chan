# v0.64.0 Round 1 - command launcher

Round 1 of v0.64.0 introduces the Cmd+K command launcher, its full context-filtered catalog, New diagram, and removes the hanging Export-to-PDF. Commit range `dc61d584..5f3c123a` on `command-launcher-v0640` (worktree `../chan-command-launcher`). Coordination tree `dev/v0.64.0/`. This is a Round 1 delivery: no version bump and no `v*` tag (a tag auto-publishes `latest.json` to every client regardless of base, so it stays out of this round).

## The round

Five lanes on one shared worktree, coordinating through an append-only task/journal bus in the main tree:

- Lead: the Export-to-PDF frontend removal, the App.svelte `runCommand` + `shortcuts.ts` Cmd+K wiring and the `windowMode` gate de-dup, integration and sequencing, the CHANGELOG and CLI-help regen, the full gate, this report, and host-smoke.
- Launcher-Core: the `commands.ts` registry, registration API, and `CommandContext`; the single-source `windowMode` gate; the Spotlight overlay component (focus trap, listbox a11y, type-ahead); the `OverlayId` / overlay-stack extension; and the active graph/dashboard tab accessors.
- Commands-A: the Editor, Graph, and Dashboard catalog, plus the active-editor bridge (`mountedEditors`) for the view-state commands and the New diagram frontend wiring.
- Commands-B: the Terminal, Panes, Global, Workspace, and Search net-new catalog.
- Server: the New diagram endpoint (draft-primary generalized off the hardcoded `draft.md`) and the desktop Export-to-PDF removal.

Each lane own-gated scoped (svelte-check plus focused vitest for the frontend; `cargo fmt` / `clippy -D warnings` / `test` for the server) and reported a pathspec sha; the lead ran the full `make pre-push` from an isolated detached gate worktree so the committed state gated clean of any in-flight work. Cross-lane questions routed through the lead; the design forks that came up (New diagram as a draft vs a plain file, the swap / New-window / metadata scope calls) were resolved within the plan's descope envelope without a host survey. Every load-bearing worker claim was re-verified firsthand before a relay or a commit.

## What shipped

- **The Cmd+K command launcher and its catalog.** A Spotlight-style overlay lists every UI action per category, fuzzy-searchable, keyboard-driven, with each command's current chord shown read-only. Availability is window-mode AND active-surface: a standalone terminal window hides the workspace-only commands, and surface commands appear only when their tab kind is active. Chorded commands reuse their `SHORTCUTS` id and dispatch through the existing `runCommand` guard; chordless commands call verified store actions directly; the terminal live-PTY actions ride a `chan:command` listener on the active terminal tab. The launcher's availability and `runCommand`'s dispatch read one shared `windowMode` gate.
- **New diagram.** A new `POST /api/diagrams/new` seeds a valid Excalidraw scene as a single-file draft; the draft module's primary-file detection was generalized off the hardcoded `draft.md` so a `.excalidraw` draft inspects, promotes to `<name>.excalidraw`, and discards cleanly. A never-drawn board discards silently like a pristine markdown draft.
- **Export-to-PDF removed, everywhere.** The Inspector action and its print engine are gone on web and desktop (the native macOS path could hang the shell indefinitely). The PDF viewer stays.

## The cut

Full `make pre-push` in the isolated detached gate worktree at `5f3c123a`: fmt, `clippy -D warnings`, `test --all-targets`, `--no-default-features` build, the gateway workspace build, `web-check` (svelte-check plus vitest plus the production builds for both SPAs), and the marketing check, all green. A docs and comments review over everything the round wrote confirmed snapshot style with no em dashes and no plan/phase references in code or commit subjects. No `v*` tag this round. `SERVE_LONG_ABOUT` (the `chan open --help` table) was regenerated from the shortcut registry.

## Carryover

- Round 2: the configuration UI over a per-library TOML, plus the shortcut-assignment surface (the launcher shows chords read-only until then).
- Round 3: moving config content out of the back-of-pane into the config UI, and the no-defaults shortcut cleanup (with the config UI as the rebinding escape hatch).
- Accepted gaps this round: Swap (a no-op outside pane mode) and New window (no reachable target from a workspace window without new multi-window plumbing) are out of the launcher; Convert to contact / slides is descoped (a `chan.kind` frontmatter upsert with document-corruption risk, pending a frontmatter helper). All are documented in `dev/v0.64.0/host-smoke.md`.
