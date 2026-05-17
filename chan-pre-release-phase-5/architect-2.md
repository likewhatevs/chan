# @@Architect task 2: documentation sweep + commit coordination

Owner: @@Architect
Status: IN_PROGRESS
Depends on: [backend-1](./backend-1.md), [frontend-1](./frontend-1.md),
[frontend-2](./frontend-2.md), [systacean-1](./systacean-1.md) all REVIEW.

## Goal

Two related close-out responsibilities the request flags as required:

1. "docs must be on point afterwards" — refresh every doc that still
   reflects the in-app Agent overlay or the pre-prune chan-llm /
   chan-drive shape.
2. Coordinate commits for the phase so wave-1, wave-2 enhancements,
   bug fixes, and the docs sweep land in coherent units (Alex pushes
   at phase close per the agreed decision).

## Docs to refresh

* `CLAUDE.md`: the layout section still describes `chan-drive`,
  `chan-llm`, and the tunnel crates as a sibling chan-core repo
  pulled in as path deps. They are workspace members in this repo.
  Update the layout + path-dep paragraph to match reality. Also note
  the MCP-only chan-llm boundary.
* `design.md`: any reference to the in-app Agent overlay,
  `LlmSession`, agent CLI backends, assistant conversation blobs,
  or the `assistant` overlay/URL hash must go. Add a one-paragraph
  note about the post-Phase-5 MCP-only boundary and the terminal
  PTY env vars used by external CLIs.
* `README.md`: same audit; in particular the feature list and any
  agent-related screenshots / wording. Mention the embedded terminal
  + MCP env exposure if appropriate.
* `crates/chan-llm/README.md` (already partly updated by
  [systacean-1](./systacean-1.md); spot-check for residual references).
* `crates/chan-drive/README.md` (already partly updated; spot-check).
* `crates/chan-server/README.md` if it exists; otherwise verify the
  module headers in `lib.rs`, `mcp_bridge.rs`, and `routes/sessions.rs`
  no longer mention `/api/llm/*`, `/api/assistant/*`, `/api/answers`.
* `desktop/` docs and any embedded help text in the `chan serve`
  `--help` output if it still mentions an Agent overlay.

## Commit coordination

### Practical reality

110 files modified/deleted across many lanes with heavy cross-lane
overlap. `web/src/state/store.svelte.ts` alone was touched by
frontend-2, -3, -4, -5, -6, -7. `crates/chan-server/src/lib.rs` was
touched by backend-1, backend-2, systacean-2, systacean-5.
Splitting cleanly per-lane needs `git add -p` hunk surgery on every
shared file. Going with three thematically-coherent
mega-commits instead — each is internally consistent, each builds
green, and each has a clear story. The lane attribution lives in
the body, not the commit boundary.

### Proposed three-commit plan

#### Commit 1 — `cleanup: remove in-app Agent overlay end to end`

Removes the in-app Agent overlay surface and pares chan-llm to its
MCP-only role. Closes wave-1 lanes:
[backend-1](./backend-1.md) (HTTP surface + terminal MCP env),
[frontend-1](./frontend-1.md) (overlay UI),
[frontend-2](./frontend-2.md) (store + API residue),
[frontend-5](./frontend-5.md) (URL hash key strip regression test),
[systacean-1](./systacean-1.md) (chan-llm + chan-drive deep prune),
[architect-2](./architect-2.md) docs portion (CLAUDE.md + design.md
+ README.md).

Files (excluding wave-2 enhancements + terminal persistence):

* `CLAUDE.md`, `README.md`, `design.md`
* `crates/chan-llm/Cargo.toml`, `README.md`, `design.md`,
  `src/lib.rs`, `src/error.rs`, `src/mcp.rs`, `src/prompts.rs`,
  `src/tools.rs`, `src/bin/chan-llm-mcp.rs`; deletes for
  `benches/end_to_end.rs`, `src/cli.rs`, `src/config.rs`,
  `src/session.rs`, all `src/backends/*` files
* `crates/chan-drive/README.md`, `design.md`, `src/blob.rs`,
  `src/drive.rs` (assistant helpers), `src/library.rs`,
  `src/lib.rs`, `src/paths.rs`, `src/registry.rs`,
  `src/graph.rs`, `tests/progress_events.rs`
* `crates/chan-server/src/lib.rs` (agent route wiring removal
  hunks only — terminal env hunks belong to commit 2),
  `src/bus.rs`, `src/config.rs`, `src/error.rs`,
  `src/preferences.rs`, `src/state.rs`, `src/tunnel_guard.rs`,
  `src/util.rs`, `src/routes/mod.rs`,
  `src/routes/preferences.rs`, `src/routes/sessions.rs`
  (assistant funcs), `src/routes/storage.rs`,
  `src/routes/drive.rs`, `src/routes/ws.rs`,
  `src/mcp_bridge.rs` (Agent-removal-related hunks only);
  deletes `src/routes/llm.rs`
* `crates/chan/src/main.rs` (agent CLI flags removed)
* `web/index.html`, `web/src/App.svelte`,
  `web/src/api/client.ts`, `web/src/api/types.ts`,
  `web/src/api/markdown.ts`, `web/src/api/transport.ts`,
  `web/src/state/store.svelte.ts` (agent removal +
  hash-strip hunks; persistent-terminal hunks belong to
  commit 3), `web/src/state/store.test.ts`,
  `web/src/state/shortcuts.ts`, `web/src/state/scope.svelte.ts`,
  `web/src/state/idle.svelte.ts`, `web/src/state/kinds.ts`,
  `web/src/state/pageWidth.svelte.ts`,
  `web/src/state/tabs.svelte.ts` (agent removal hunks only),
  `web/src/components/AppStatusBar.svelte`,
  `web/src/components/Bubble.svelte`,
  `web/src/components/FileBrowserOverlay.svelte`,
  `web/src/components/FileEditorTab.svelte`,
  `web/src/components/GraphPanel.svelte`,
  `web/src/components/HamburgerMenu.svelte`,
  `web/src/components/OverlayShell.svelte`,
  `web/src/components/Pane.svelte`,
  `web/src/components/SearchPanel.svelte`,
  `web/src/components/SettingsPanel.svelte`,
  `web/src/components/StyleToolbar.svelte`,
  `web/src/design.md`,
  `web/src/editor/Source.svelte`,
  `web/src/editor/Wysiwyg.svelte`,
  `web/src/editor/commands/{date_macros,format,format.test}.ts`,
  `web/src/editor/highlight.ts`,
  `web/src/editor/links.ts`,
  `web/src/editor/overlays/{date_popover,preview_popover}.ts`,
  `web/src/editor/widgets/image.ts`;
  deletes `web/src/components/{AccessoryPill,
  AssistantInspectorBody, BottomPill, EnsoIcon, InlineAssist,
  ScopeHistoryOverlay}.svelte`, `agentBanner.ts`, and
  `web/src/state/scope_history.test.ts`
* `Cargo.lock` (deps removed by the chan-llm prune)

Verification trailer: full pre-push gate.

#### Commit 2 — `enhancements: indexer hardening + terminal MCP env`

Adds indexer prioritisation, the search aggression knob, the
VCS-aware fs-change path, and the embedded terminal's MCP env
exposure. Closes wave-2 enhancement lanes:
[backend-1](./backend-1.md) terminal MCP env portion,
[systacean-2](./systacean-2.md) (watcher gate + scheduling),
[systacean-3](./systacean-3.md) (search aggression knob),
[systacean-4](./systacean-4.md) (git/hg correctness + benchmarks),
[systacean-6](./systacean-6.md) (BUG-WT5-A regression test).

Files (the indexer + VCS + terminal-env hunks only — leave the
session registry for commit 3):

* `crates/chan-server/src/config.rs` (aggression config),
  `src/indexer.rs`, `src/routes/terminal.rs` (env exposure hunks),
  `src/lib.rs` (terminal env wiring hunks);
  `src/preferences.rs` (aggression read)
* `crates/chan-drive/src/vcs.rs`, `src/watch.rs`,
  `src/index/facade.rs`, `src/index/mod.rs`
* `desktop/src-tauri/src/serve.rs` (only the env-related
  hunks; per-window state hunks belong to commit 3)

Verification trailer: full pre-push gate plus the manual
checkout-profile numbers from systacean-4 (80 files / 20 touched:
initial 11078ms, checkout settle 3138ms, staged resume 235ms).

#### Commit 3 — `terminal + ux: persistent sessions, per-window state, bug fixes`

Adds the chan-native PTY session registry, the per-window
session-blob keys, the tab close confirmation, the caret-scroll
fix, and the BUG-WT5-C / OBS-WT5-D fixes. Closes:
[backend-2](./backend-2.md) (per-window state),
[frontend-3](./frontend-3.md) (close confirm + caret),
[frontend-4](./frontend-4.md) (client reattach),
[frontend-6](./frontend-6.md) (BUG-WT5-C),
[frontend-7](./frontend-7.md) (OBS-WT5-D),
[systacean-5](./systacean-5.md) (server registry).

Files (the persistence + per-window + ux-fix hunks only):

* `crates/chan-server/src/routes/terminal.rs` (registry wiring
  hunks), `src/state.rs` (registry on AppState),
  `src/lib.rs` (terminal_sessions wiring + `[terminal]`
  config), `src/config.rs` (`[terminal]` block),
  `src/preferences.rs` (read-side), `src/util.rs` (random id
  helper if added)
* New file: `crates/chan-server/src/terminal_sessions.rs`
  (in the untracked set; `git add` it explicitly)
* `desktop/src-tauri/src/serve.rs` (per-window `w=<label>` hunks)
* `web/src/api/client.ts` (session-key selection + tab-id
  WS query hunks)
* `web/src/state/store.svelte.ts` (bootstrap hydrate hunks +
  per-window session blob hunks)
* `web/src/state/tabs.svelte.ts`
  (`hydrateTerminalSessionsFromLayout` + `terminalSessionId`/
  `lastSeq` descriptor)
* `web/src/components/TerminalTab.svelte` (reattach + control
  frames + close-reason UI), `web/src/components/ConfirmModal.svelte`
  (close confirm)
* New test files in `web/src/` if any (the
  `hydrates terminal session ids...` regression test went into
  `tabs.test.ts`)

Verification trailer: full pre-push gate plus
[webtest-1](./webtest-1.md) end-to-end PTY-reattach result
once that round of smoke is reported PASS.

### Ordering rule

Commits 1 → 2 → 3 in order. Commit 1 is the cleanest
no-regression baseline (everything still builds because dead code
goes away atomically with its callers). Commit 2 adds the
indexer + env work on top. Commit 3 lands the persistence and ux
fixes. Each commit must pass the pre-push gate green.

The terminal-trio rebase from [architect-3](./architect-3.md)
runs **before** commit 1 so the chan-term commit history is in
its final shape before any new work piles on top.

### Concrete recipe

When the smoke is clean and Alex signs off on architect-3 drafts:

```
# 1. Rewrite the terminal trio messages (architect-3 recipe).
git rebase HEAD~3 --exec '<the case-by-subject block from
architect-3.md>'

# 2. Stage commit 1 (broad cleanup). Worktree currently has all
#    three commits' changes mixed in; need git add -p (or per-file
#    stash) to pull just the agent-removal hunks. Files listed
#    above are the candidates.
git add <commit-1 file list>
git commit -m "$(cat /tmp/msg-phase5-commit-1.txt)"

# 3. Stage commit 2 (enhancements).
git add <commit-2 file list>
git commit -m "$(cat /tmp/msg-phase5-commit-2.txt)"

# 4. Stage commit 3 (terminal + ux). Remaining files + the new
#    chan-server/src/terminal_sessions.rs file.
git add <commit-3 file list>
git commit -m "$(cat /tmp/msg-phase5-commit-3.txt)"

# 5. Verify.
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --no-default-features
cargo test
cd web && npm run check && npm test -- --run && npm run build
```

Commit-message bodies for the three phase-5 commits will be drafted
into `/tmp/msg-phase5-commit-{1,2,3}.txt` once Alex signs off on
shape; the bodies follow the canonical style from architect-3
(framed why, surface-bulleted body, "Closes" footer linking to the
task files, Verification trailer).

### Untracked files (need `git add` not just `git add -u`)

Six untracked entries at round-9:

* `chan-pre-release-phase-5/` — coordination dir (this lane); lands
  in the dedicated phase-close commit, not in 1/2/3.
* `crates/chan-server/src/terminal_sessions.rs` — commit 3 (registry).
* `web/src/api/client.test.ts` — commit 3 (backend-2 + frontend-7
  client tests).
* `web/src/state/confirm.svelte.ts` — commit 3 (close-confirm state
  shared with frontend-3 + frontend-4).
* `web/src/state/tabs.test.ts` — commit 3 (frontend-6's hydration
  regression test).
* `web/src/terminal/` — commit 3 (likely the terminal helpers
  module; confirm scope before staging).

### Commit 4 — `release: close phase 5 tasks`

Lands the `chan-pre-release-phase-5/` coordination directory once
everything else is committed and the post-commit lanes
([systacean-7](./systacean-7.md), [webtest-3](./webtest-3.md))
have written their completion notes. Mirrors the
`release: close phase 3 tasks` and `release: close phase 2 tasks`
precedent in this repo's history.

### Pre-flight on current HEAD (round-9)

* `cargo fmt --check` ✓
* `cargo clippy --all-targets -- -D warnings` ✓
* `cargo test --workspace` ✓ (exit 0)
* `npm --prefix web run check` ✓
* `npm --prefix web test -- --run` ✓ (16 files / 144 tests)
* `npm --prefix web run build` ✓

Working tree at round-9: 110 files (84 modified, 20 deleted,
6 untracked including `chan-pre-release-phase-5/` and the new
`crates/chan-server/src/terminal_sessions.rs`).

## Acceptance criteria

* Every grep against `Agent` / `agent` / `assistant` / `LlmSession`
  in checked-in docs returns only intentional historical mentions
  (e.g. "phase 3 introduced the Agent overlay; phase 5 removed it"
  is fine; live "the Agent overlay lets you..." is not).
* Commit messages follow the existing repo style (subject line
  imperative, ~70 chars, body explains why if non-obvious, listed
  verification at the end).
* Each commit is signed off by its lane owner via the task file
  (recorded in the task's "Commit readiness" section).
* Pre-push gate green on the final HEAD before push.

## Progress

* 2026-05-17 @@Architect: refreshed `CLAUDE.md`. Layout block now
  reflects the in-tree chan-drive / chan-llm / chan-report / chan-
  tunnel-* / desktop layout (no more "sibling chan-core" path-dep
  framing). Tunnel + chan-drive design-doc paths updated. Added
  the "MCP server only, no in-app agent" principle. Rewrote the
  "LLM lives in chan-llm" contributor bullet as the MCP-only
  contract.
* 2026-05-17 @@Architect: refreshed `design.md`. Workspace layout
  section now lists every workspace crate plus `desktop/` and the
  per-window state contract. chan-server module + routes table is
  current (no `llm.rs`, sessions narrowed to the per-window blob,
  added `terminal.rs`, `fs_graph.rs`, `report.rs`, `tunnel_guard.rs`).
  `chan-llm` section rewritten as MCP-only with the LlmSession /
  CLI-backend removal called out explicitly. Feature ownership
  table dropped the assistant-chat row and the dead LLM-backends
  row; added the embedded-terminal row.
* 2026-05-17 @@Architect: grep on `agent overlay`, `/api/llm`,
  `/api/assistant`, `LlmSession`, `in-app agent` across README,
  design.md, and every crates/*/README.md returns only the
  intentional historical references in `design.md`'s explanation
  paragraph and `crates/chan-llm/README.md`'s MCP-only statement.
* 2026-05-17 @@Architect: README.md spot audit run. The Layout
  block already reads chan-llm as "MCP server/tool sandbox" with
  no in-app-agent framing; the embedded-terminal section now
  documents the CHAN-only MCP env contract. Grep for
  `assistant|agent overlay|LlmSession|in-app agent` against
  `README.md` returns zero matches. Docs sweep portion is
  complete; commit coordination is the remaining piece.

Commit coordination notes will land here as wave-2 lanes
(frontend-3, backend-2, frontend-4, frontend-5, systacean-2/3/4/5/6)
reach REVIEW or DONE and we agree on commit groupings. The
[architect-3](./architect-3.md) terminal-trio rebase folds into
those groupings.

## Completion notes

(populated by @@Architect after the commit groupings land)
