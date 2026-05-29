# @@LaneA round-1 closing - Phase 13

You are @@LaneA picking up the round-1 CLOSING tasks. The
round-1 roadmap landed end-to-end (your `b2ef3f3b`,
`ad184179`, `39fd3373`, `08b28da8` are all on main at
`5a241f0f`). The cumulative chan-desktop smoke surfaced
regressions; this file IS your task list.

You do NOT cut v0.17.0; @@LaneB does at round close. You DO
NOT merge yourself; @@LaneB merge-gates. Your job is to land
fixes on `phase-13-lane-a` and report merge-ready slices on
the bus.

## Recover context (read in order)

- `/Users/fiorix/dev/github.com/fiorix/chan/CLAUDE.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/design.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/roadmap-round-1.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/bootstrap.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/round-1-closing-tests.md`  THE SMOKE REPORT
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/README.md`
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/lane-a-request.md` (your original brief; channel + worktree convention)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/lane-a/journal.md` (tail of the prior @@LaneA's turns)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-alex-lane-a.md` (your inbox)
- `/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/coordination/event-lane-b-lane-a.md` (cross-lane from @@LaneB)

## Worktree + branch

You inherit `../chan-lane-a` on branch `phase-13-lane-a` from
the prior @@LaneA. The branch is already merged to main. First
action: rebase on the current main tip.

```
git -C /Users/fiorix/dev/github.com/fiorix/chan-lane-a fetch . main
git -C /Users/fiorix/dev/github.com/fiorix/chan-lane-a rebase main
```

Source code work happens ONLY in `../chan-lane-a`. Journals /
channels stay in the MAIN checkout at
`/Users/fiorix/dev/github.com/fiorix/chan/docs/journals/phase-13/`
edited by ABSOLUTE PATH.

## Scope (your round-1 closing items)

The smoke report (`round-1-closing-tests.md`) has the verbatim
user framing. Items below quote the exact concern + cite source.

### A1. WorkspaceInfoBody parity

User words: "The workspace root dir is missing the 'Graph from
here' and Download buttons. Plus, the 'NOTES DIRECTORIES'
should now only exist in the dashboard, not in this inspector
anymore... we are going to add the buttons for here and for
the Graph's inspector, making it more like any other directory:
Show in File Browser, Graph from here, Upload, Download."

The user also asks for "a separator between COCOMO and NOTES
DIRECTORIES" but is removing the NOTES DIRECTORIES section in
the same breath; the separator is moot.

Files:
- `web/src/components/WorkspaceInfoBody.svelte`
- `web/src/components/FileInfoBody.svelte` (folder render
  branch is the parity reference; look for `onSetAsScope`
  wiring + the Download / Upload buttons in the file/dir
  inspector body).

Acceptance:
- WorkspaceInfoBody no longer renders the "Notes directories"
  config section (lines ~36-103 per the smoke investigation).
- WorkspaceInfoBody renders the same button row as the folder
  inspector: Show in File Browser, Graph from here, Upload,
  Download. The icon stays different (workspace root icon).
- Pre-release per `feedback_pre_release_no_backcompat`: just
  drop the config section, no migration.

### A2. Inspector body dispatch for `kind: "directory"` (graph parent-dir click)

User words: "However, when I click on the parent dir, the
inspector for the directory is missing."

Files:
- `web/src/components/InspectorBody.svelte` (selection
  dispatcher - confirm there's a branch routing
  `kind: "directory"` to FileInfoBody's folder render path).
- `web/src/components/GraphPanel.svelte::selectFromList`
  (~line 1210; folder nodes get id prefix `directory:`,
  stripped at line 1356).

Acceptance:
- Clicking a folder node in the graph opens the inspector
  with the folder's body (stats, kinds, children, action
  buttons).
- Verify with smoke (Cmd+R is unavailable; you cannot relaunch
  the desktop yourself - leave the chan-desktop smoke to
  @@LaneB at merge-gate).

### A3. Inspector body for `kind: "language"` (language node click)

User words: "the language itself has no Inspector".

The graph emits language bubbles (`kind: "language"`, id
`language:<lang>`, `files`, `code`). Clicking one should
surface an inspector body with the language's
files / lines-of-code stats and possibly a "Graph from here"
button that calls `openGraphForLanguage(language)`.

Files:
- `web/src/components/InspectorBody.svelte` - add a `language`
  arm.
- Decide: is `LanguageInfoBody.svelte` warranted, or can the
  language case render inline? Lane A's previous round added
  `KindChip`'s clickable variant; mirror that aesthetic.

Acceptance:
- Clicking a language bubble opens an inspector body with
  the language name + file count + code lines + a
  "Graph from here" button wired to
  `openGraphForLanguage(language)`.

### A4. Editor `@{name}` autocomplete missing mentions

User words: "I also realised the @@{mentions} aren't
available for searching when I type @{name} in the editor".
Screenshot: `image-6.png`.

Files:
- `crates/chan-server/src/routes/mentions.rs` (the
  `/api/mentions?q=` endpoint Lane A wired in slice 4
  earlier).
- Editor's @-mention completion provider in `web/src/editor/`
  (look for `mentionCompletions` or similar).

Acceptance:
- Typing `@{name}` (without the second @) surfaces all
  `@@mention` instances across the markdown corpus, not just
  the `kind: contact` frontmatter files. Lane A already
  shipped the prefix-match endpoint; verify it's actually
  feeding the editor completion provider.

### A5. Mention resolver: `@@mention` surfaces in the graph

User words: "despite having multiple mentions across various
.md files, I do not see any of them in the graph, at all.
Perhaps our graph is only considering contacts the .md files
with frontmatter kind: contact".

Files:
- `crates/chan-server/src/routes/graph.rs::api_graph` (look
  for mention-edge emission near line ~1664 where tag edges
  emit; mentions should emit a `kind: "mention"` edge from
  file -> mention node).
- `crates/chan-server/src/routes/contacts.rs` if the
  contact-kind frontmatter rewrite (mention -> contact-file
  collapse at `graph.rs:1491-1499`) is masking the bare
  mention nodes.
- The frontend GraphView already supports `kind: "mention"`
  nodes (`web/src/api/types.ts`).

Acceptance:
- The workspace graph response (`/api/graph?scope=workspace`)
  includes a node per `@@mention` actually used in the
  corpus + edges from each referencing file to that mention.
- Lane B's contact lens BFS (`web/src/components/GraphPanel.svelte`,
  `currentScope.kind === "contact"`) is already bidirectional
  and seeds from the contact file's path. If the contact
  file exists AND the mention edges exist, the lens populates.
  Lane B's BFS does NOT need a fix; the data source does.

## Cross-lane

- A1 (WorkspaceInfoBody parity) and A2 (directory inspector
  dispatch) make Lane B's tag / language tab title and
  scope-header dropdown row feel complete; Lane B reads what
  you ship for the dashboard's Workspace-info widget reuse.
- A5 unblocks Lane B's contact lens visibly populating.
- Cross-lane channel for hand-offs: append to
  `event-lane-a-lane-b.md`.

## Per-slice gate (mandatory before any "ready to merge")

Run in `../chan-lane-a`:

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --no-default-features
(in web/)   npm run check
(in web/)   npm run build
(in web/)   npm test     (vitest)
```

`feedback_svelte_static_gate_misses_runtime`: browser-smoke any
reactive change; flag what wants a Chrome walk so Lane B's
combined-tree smoke can pick it up.
`feedback_terminal_webgl_wkwebview`: changes that touch the
terminal-rendering path want a chan-desktop smoke too.

Append merge-ready entries to `event-lane-a-alex.md`:

```
ready to merge: phase-13-lane-a@<sha>  -  <one-line slice summary>
```

## Coordination rules

- Append-only channels; never edit another agent's entries.
- Each turn, before acting, read:
  - `event-alex-lane-a.md` (your inbox)
  - `event-lane-b-lane-a.md` (cross-lane from @@LaneB)
  - `event-lane-b-alex.md` (visibility into Lane B's merge gate)
- Self-document in `lane-a/journal.md`.
- DO NOT push to origin (`feedback_merge_is_not_push`).
- DO NOT amend commits or rewrite history.

## Out of scope

- Anything not in the smoke report
  (`round-1-closing-tests.md`). Escalate scope creep to
  @@Alex on `event-lane-a-alex.md`.
- v0.17.0 release cut. Lane B owns it.

## First-turn checklist

1. Rebase `../chan-lane-a` on `main`.
2. Read the recovery files.
3. Append a turn-1 opening entry to `lane-a/journal.md`.
4. Pick the smallest item (A2 or A4 is the smallest);
   work it to the per-slice gate.
5. Report on `event-lane-a-alex.md`.
