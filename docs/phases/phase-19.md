# Phase 19 - Linux desktop parity, in-tree Drafts, the mention lens (v0.26.1 - v0.28.1)

Status: closed
Span: 2026-06-04 - 2026-06-08. Opened right after phase-18 cut v0.26.0 and ran until v0.28.1. Based on git author dates and release-tag dates; the per-author journals/coordination bus for this round were not preserved, so the dating is git-derived (see Notes).
Versions: v0.26.1, v0.26.2 (2026-06-05), v0.27.0, v0.27.1, v0.28.0, v0.28.1 (2026-06-08)
Tags: #desktop #linux #drafts #graph #mentions #contacts #indexing #terminal #docs #release

> NOTE: This report was reconstructed after the fact from git history and the `dev/phase-19/brainstorm/` asks. The "what / when" is accurate to the commits; the "how the team ran it" (Team and coordination) and the Retrospective are TODO placeholders to be filled from the session's own record. Everything below the placeholders is git-grounded.

## Roadmap (the asks)

Phase-19 followed the phase-18 public-flip prep with a parity-and-polish stream: make chan-desktop work as well on Linux as on macOS, move Drafts out of the metadata layer and into the workspace tree, and turn `@@mentions` into a first-class graph lens. Three brainstorm notes seeded it (`dev/phase-19/brainstorm/`):

**Graph @@mention lens.** Expose `@@mention` as a first-class graph scope (like the `#tag` lens): click `@@Lead` and see every file that references it, anchored to its directory spine. The note called out three stacked gaps - no `mention:` scope kind in the graph UI, mention nodes only rendering as endpoints of already-visible file edges, and the graph index missing a mentioning file that was added while the server was down.

**Agent docs reorganization.** Consolidate scattered agent material (root CLAUDE.md/AGENTS.md, buried standards, `docs/agents/`, dispersed skills) into a single committed `.agents/` home: roster, playbook, orchestration, plus extracted standards (principles, writing rules, patterns), dropping the root CLAUDE.md/AGENTS.md auto-load in favor of an unmistakable `.agents/README.md` front door.

**Motivation and usage.** Document chan's design and the why behind it (composable local-first writing surface: editor + terminal + graph + dashboard; the workflow evolution from writing to a team of agents) as public-facing narrative.

Riding alongside, surfaced through use rather than a brainstorm: a backlog of Linux/WebKitGTK desktop defects from the first non-macOS builds, and the in-tree Drafts migration.

## Rounds and waves

The shipped work groups into the following threads (release tag in parens).

### Linux / WebKitGTK desktop parity (v0.26.1, v0.26.2)

The first Linux desktop builds exposed a stack of GUI-stack and keybind gaps, fixed across:
- `60311a76` prefer the host GTK/WebKit stack on the Linux AppImage, and `9434cf34` documenting that shim + the manifest endpoint.
- `3096b926` unstick the Hybrid flip under WebKitGTK; `7e7f761e` keep the terminal on the DOM renderer under WebKitGTK; `5112a1c9` keep Ctrl+E for readline in a focused terminal.
- `e8f5f57e` make the new-draft and show-source chords work off macOS; `70ae1713` build a File menu with About + Exit off macOS; `17bca9b0` render a working Quit in the Linux File menu.
- `066dbc1c` flatten the self-upgrade manifest endpoint; `90d0b2f8` fix the workspace-root inspector split-action button.

### In-tree Drafts model (v0.27.0, v0.27.1)

Drafts moved from the metadata layer to a configurable in-tree `.Drafts/` directory, addressed as ordinary in-root paths:
- `530b241a` store Drafts in-tree under a configurable `.Drafts/` dir; `535af8a2` address drafts as in-root paths + surface `drafts_dir`; `f3d65576` key the web draft-path logic off the surfaced `drafts_dir`.
- `effee182` / `5ab0cb3f` document the model and scrub stale Drafts-as-metadata wording.
- `5c30e88b` surface the drafts dir in the file tree on Cmd+N; `90f5ea92` let a moved/deleted draft tab close cleanly; `c6442e6f` persist file-browser expansion across reload + tab switch.

### Graph mention lens, contacts, offline reconcile (v0.28.0)

- `b4a69f0e` add the graph `@@mention` lens (the headline ask).
- `37622203` render cross-linked contacts as contact nodes in the graph; `5c4cf7b7` standardize the contact frontmatter on `chan.kind`.
- `1f4e6b03` reconcile offline-added files on startup (the index gap the mention-lens note surfaced: a file created while the server was down now gets re-walked on next boot).

### Docs + repo fold (v0.28.0)

- `dadac77b` move the agent docs into a committed `.agents/` home (root CLAUDE.md/AGENTS.md dropped).
- `9a61db6e` add a chan story page to the marketing site from the motivation-and-usage narrative.
- Post-phase-18 cleanup: `695aa27e` remove the consolidated journals + cut agent cards + archive, `21273378` gitignore the `dev/` scratch area, `1c1e8f2f` remove the tracked `.codex/config.toml`.

### Terminal paste (v0.28.1)

- `2b9a8857` paste in the terminal without tripping WKWebView's blue "Paste" permission button (macOS desktop): Cmd+V uses xterm's native gesture-paste, the right-click menu reads natively via the arboard Tauri command.

## Team and coordination

TODO: how phase-19 was structured (single round / multiple rounds, lanes vs solo, whether an architect/lead ran it), who worked which thread, the coordination scheme (journals / task files / pokes / gates / surveys), and the gate model. Git records the commits but not the team mechanics; fill from the session's own record.

## What shipped, tried, and undone

**Shipped.** Linux/WebKitGTK desktop parity (GUI stack, menus, chords, terminal renderer); the in-tree `.Drafts/` model end to end (server addressing, web path logic, file-tree surfacing, tab lifecycle); the graph `@@mention` lens; contacts as first-class graph nodes; offline-added-file reconciliation on startup; the `.agents/` docs fold; the marketing chan-story page; the WKWebView terminal-paste fix. Six releases, v0.26.1 through v0.28.1.

**Tried then corrected / Deliberately deferred.** TODO: fill from the session record (recons that changed direction, anything scoped out and why).

## Retrospective

TODO: highlights, lowlights/contention, and lessons worth carrying forward, plus per-role feedback. Fill from the session's own record.

## Notes

The per-author journals and coordination bus that backed earlier phases were not preserved for phase-19 (the phase-18 consolidation removed the `docs/journals/` tree and phase-19 ran after that). The brainstorm asks live in `dev/phase-19/brainstorm/` (gitignored scratch). The CHANGELOG was synced to phases 14-18 in `342da10d`; the v0.27.x/v0.28.x compare-links were added in `2cb0235e`.
