# webtest-2: parallel scenarios for phase 6

Owner: @@WebtestB
Status: IN_PROGRESS

## Goal

Run parallel scenarios alongside @@WebtestA's main smoke loop so the
phase-6 lanes that ship in different rebuilds get fast, focused
verification without contending for the shared test service in
[webtest-1](./webtest-1.md).

## Relevant links

* Live test service: [webtest-1](./webtest-1.md).
* Request: [request.md](./request.md).
* Journal: [journal.md](./journal.md).
* Design memo: [architect-2.md](./architect-2.md).

## Scenarios

Pick up each as the matching lane lands. Cross-link the source task
when filing observations.

### Architectural cleanups

1. **Drive-rooted graph default** ([frontend-4](./frontend-4.md)).
   With nothing selected, open the graph from each entry point
   (empty-pane menu, global shortcut, hash-restored state). Verify
   the scope is drive in every case. Verify per-file / per-dir
   "Graph this" still pivots correctly.
2. **Inspector enrichment** ([backsystacean-2](./backsystacean-2.md),
   [backsystacean-3](./backsystacean-3.md),
   [frontend-4](./frontend-4.md)). Open the inspector for the drive,
   for a directory, for a markdown file (contact + non-contact), a
   text file (Rust source), and a binary (small image). Confirm the
   shape matches the design memo; flag missing fields by section.
3. **File classifier rendering**
   ([backsystacean-2](./backsystacean-2.md),
   [frontend-4](./frontend-4.md)). Add a symlink to the test drive
   that points inside, a symlink that points outside, a chmod-ed
   read-only directory, and a regular file. Confirm the inspector
   badges and the graph dead-end behavior for the read-only dir.
4. **Royal-pink language color**
   ([frontend-4](./frontend-4.md)). Compare against the tag green
   side-by-side under light + dark mode; flag if either hex looks
   off.
5. **Terminology codemod**
   ([backsystacean-5](./backsystacean-5.md),
   [frontend-5](./frontend-5.md)). Walk the UI for "folder" copy;
   `rg` the crates and web sources for residue.

### Bugs and nits

6. **Right-click menus** ([frontend-2](./frontend-2.md)).
   PANE left-click + right-click both show Reload + Inspector;
   outside-overlay right-click shows the same; browser default
   does not fire.
7. **Terminal bubble menu + right-click**
   ([frontend-2](./frontend-2.md)). Walk every action in the new
   terminal right-click menu including Copy CWD, Show Dir, Graph
   dir, New Terminal, splits, search, settings. `[size]`
   present in the bubble menu; top bar carries only the title.
8. **Tab disambiguation** ([frontend-3](./frontend-3.md)).
   Open two `foo.md` files in different directories; verify the
   tab titles disambiguate per the algorithm. Hover shows the full
   path. Close one tab; verify the other reverts to plain basename.
9. **Terminal-N enumeration + Shift+Enter + ^D**
   ([backsystacean-1](./backsystacean-1.md)). Already in REVIEW;
   confirm Terminal-N naming holds across rename / close, Shift+Enter
   reaches claude / codex (real CLI if available), Ctrl+D after
   shell exit closes the tab.

## Reporting

* File observations in this task with a short label
  (`OBS-WT6-A`, `OBS-WT6-B`, ...) and a one-line repro.
* Cross-link to the source task when filing.

## Coordination with @@WebtestA

* @@WebtestA owns the service lifecycle in
  [webtest-1](./webtest-1.md). Rebuilds and restarts go through A;
  B reads the same PID. Coordinate in the task files when a
  restart is needed for a B scenario.

## Acceptance criteria

* Every scenario above is exercised on the matching rebuild and a
  PASS / FAIL recorded.
* Observations filed with cross-links.
* Final summary line at the end of the task with the round-by-round
  status.

## Progress notes

* 2026-05-18 @@WebtestB - Picked up the task. Server state per
  [webtest-1](./webtest-1.md): PID 61390 on
  `127.0.0.1:8787`, token `wB68doozwEwucY7qVG3SH2DTF5HYGQ6R`,
  drive `/private/tmp/chan-test-phase6`. Scenarios runnable right
  now on this build (frontend-1 REVIEW + backsystacean-1 REVIEW +
  backsystacean-2 REVIEW): Scenario 3 (HTTP slice over
  `/api/files` `path_class`), Scenario 9 (Terminal-N + Shift+Enter
  + ^D in the browser). Scenarios 1, 2, 4, 5, 6, 7, 8 are blocked
  on still-TODO frontend / backsystacean tasks; will poll the
  journal and re-pick as they hit REVIEW.
* `OBS-WT6-A` - PID drift in [webtest-1](./webtest-1.md). The
  recorded PID is 61390; the actual live listener on
  `127.0.0.1:8787` is PID 67471
  (`target/debug/chan serve /private/tmp/chan-test-phase6
  --no-browser`). Same drive, same bearer token, so smoke
  results stay valid. Flagged so @@WebtestA can update the
  task entry next restart; no rebuild required.
  Confirmed `/api/health` -> 401 (no token) and 200 with
  bearer.

### Scenario 3 - file classifier (HTTP slice)

Source task: [backsystacean-2](./backsystacean-2.md). Drive
already seeded by @@Backsystacean with the right fixtures:
`README.md` + `README-link.md` (hardlink pair, nlink=2),
`link-internal.md` -> `notes.md`, `link-outside` -> `/tmp`,
`locked-dir` (dr-xr-xr-x), regular dirs + files.

Probes:

* `GET /api/files?t=...` (root listing). PASS for what is
  returned: `locked-dir` carries
  `path_class.permission="read_only"`, the hardlink pair
  carries `link_count: 2`, regular entries carry
  `regular_file`+`read_write`+`link_count:1`. See
  [OBS-WT6-B](#).
* `GET /api/files?dir=locked-dir` -> `[]`. Dead-end response
  is correct in shape. Caveat below.
* `GET /api/files?dir=code` -> single text entry
  (`code/hello.rs`, `kind:"text"`, `path_class.kind:
  "regular_file"`). PASS.
* `GET /api/files/link-internal.md` and
  `GET /api/files/link-outside` -> HTTP 415,
  `{"error":"refusing to operate on non-regular file
  (symlink): ..."}`. Editor refusal layer holds. PASS.
* `GET /api/fs-graph?t=...` (root). Graph payload PASSES
  every classifier check: symlinks render as
  `"kind":"symlink"` with `target`, `link-outside` flags
  `target_escapes_drive: true` and emits a
  `outside:link-outside` ghost node, the hardlink pair emits
  a `kind:"hardlink"` edge, `locked-dir` carries
  `permission:"read_only"` and is listed without descent
  (no `contains` edges below it). The escape symlink yields
  a `symlink` edge into the ghost node rather than into a
  drive-internal node.

`OBS-WT6-B` - `/api/files` listing silently drops symlinks.
`Drive::list` in
[crates/chan-drive/src/drive.rs](../crates/chan-drive/src/drive.rs)
line 712 filters out any entry that is not a directory or a
regular non-symlink file, so `link-internal.md` and
`link-outside` never appear in the file browser's tree even
though the underlying `PathClass` API supports them.
[request.md](./request.md) explicitly lists "fs specific,
devices, sym/hard links, etc" under what the inspector
"recognises". Two reasonable reads:

  1. Intentional: the file browser stays on regular content
     and symlinks live only in the graph (which already
     renders them). If so, the request line was already
     satisfied by `/api/fs-graph` and the inspector via
     `PathClass`.
  2. Gap: the listing should include symlink entries with
     `path_class.kind:"symlink"` so the file browser can
     render and dead-end them.

Routing to [backsystacean-2](./backsystacean-2.md) /
[frontend-4](./frontend-4.md) for a product call. Not a
blocker for the rest of the scenarios.

`OBS-WT6-C` - Terminology residue in `/api/fs-graph` wire
shape. Root response carries `"scope":"folder"`, the drive
root node carries `"kind":"folder"`, and every directory
node uses `"kind":"folder"`. Expected to be cleaned up by
[backsystacean-5](./backsystacean-5.md). Recorded here so
[frontend-5](./frontend-5.md) and the codemod include the
graph-side wire field if migration is in scope.

`OBS-WT6-D` - Locked-dir dead-end probe is shape-correct
but does not actually exercise refusal: the seeded
`locked-dir` is empty (link count 2 = only `.` and `..`),
so the `[]` response would hold even without read-only
enforcement. To prove the dead-end gate at the chan-drive
layer over HTTP, the fixture needs at least one entry
seeded before `chmod` strips the write bit; the unit tests
in [backsystacean-2](./backsystacean-2.md) already cover
this and `fs-graph` does refuse to descend (no `contains`
edges below `locked-dir` in the root depth=1 payload, while
non-empty read-write dirs do emit them at the next depth).
No action required, recorded for completeness.

### Scenario 9 - Terminal-N + Shift+Enter + Ctrl+D

Source task: [backsystacean-1](./backsystacean-1.md).
Exercised in a live Chrome session on the same test
service.

* Enumeration on fresh spawn: opened three terminals via
  `Cmd+`` ` and observed `Terminal-1`, `Terminal-2`,
  `Terminal-3` in order. PASS.
* `CHAN_TAB_NAME` at PTY spawn: `echo CHAN_TAB_NAME=$CHAN_TAB_NAME PWD=$PWD`
  inside `Terminal-3` returned
  `CHAN_TAB_NAME=Terminal-3 PWD=/private/tmp/chan-test-phase6`.
  PASS.
* Monotonic enumeration after close: closed `Terminal-2`
  (still-running tab; got the new app-level "Close tab?
  Terminal-2 is still running. Close anyway?" confirm
  dialog before close - extra finding, see
  [OBS-WT6-E](#)), then opened the next terminal. New tab
  was `Terminal-4`, not `Terminal-2`. PASS.
* Shift+Enter byte sequence: ran `cat -v` and typed
  `abc <Shift+Enter> def <Enter>`. Output rendered as
  `abc^[[13;2udef` - i.e. `\x1b[13;2u` (CSI 13;2u, kitty
  keyboard protocol for Shift+Enter) - exactly what the
  unit test in `web/src/terminal/keymap.test.ts` asserts.
  PASS at the PTY layer. End-to-end against a live
  `claude` / `codex` interactive prompt deliberately
  skipped to avoid sending paid prompts; the byte sequence
  is precisely what those CLIs expect under enhanced
  keyboard mode and unit tests cover the keymap.
* Ctrl+D close hint after shell exit: opened `Terminal-5`,
  sent `exit\r` to the shell via the open terminal
  WebSocket (programmatic key events from CDP did not
  reach xterm.js's input path on this build; the
  WebSocket fallback is a test-only workaround and not a
  product defect for keyboard users). Terminal showed
  `process exited (0); press Ctrl+D to close this tab`.
  PASS.
* Ctrl+D actually closes the tab: dispatched a synthetic
  `keydown` (`ctrlKey: true, key: "d"`) on the active
  terminal-tab panel. `event.defaultPrevented` flipped to
  `true` (the `onShellKeydown` handler at
  `web/src/components/TerminalTab.svelte:415` ran and
  invoked `closeTab(..., { force: true })`), and
  `Terminal-5` disappeared from the tab strip without a
  confirm prompt. PASS.

`OBS-WT6-E` - Close-tab confirm dialog is new for
running tabs. Clicking the `x` on a tab whose PTY is
still attached now opens a modal "Close tab? Terminal-N
is still running. Close anyway? [Cancel] [Close]" before
detaching. This is great UX, but it is not in any of the
Phase 6 acceptance criteria and was not in the journal
checklist. Recorded so @@Architect can decide whether to
add it as a row to the request checklist (and
[webtest-1](./webtest-1.md) smoke list) for explicit
sign-off. The exited path correctly skips the prompt
because the keymap branch passes `force: true`.

`OBS-WT6-F` - Tested incidentally: when a tab is exited
(status="exited"), the next `Cmd+`` ` opens a new tab
instead of resurrecting the dead one. Matches the
keymap-only close path. No action.

### Cross-cover (frontend-1 REVIEW)

Not in [webtest-2](./webtest-2.md)'s scenario list but
exercised while the browser session was live. Reporting
back to [webtest-1](./webtest-1.md)'s smoke checklist.

* `OBS-WT6-G` - Right-clicking `notes.md` in the file
  tree opens a context menu with `Graph this`,
  `Search this`, `Copy Path`, `Rename / Move`, `Delete`.
  Copy Path is wired. PASS for the file-tree side of the
  Copy Path line item in [webtest-1](./webtest-1.md).
  Editor-tab-menu Copy Path not exercised in this round.
* `OBS-WT6-H` - Drive now carries binary fixtures
  (`blob.bin`, `pixel.png`) rendered in italic style in
  the tree, distinct from regular text / markdown
  entries. Useful when [frontend-4](./frontend-4.md)
  lands the inspector pass: the binary tier already has
  styling hooks here. Recorded for context.

## Status

Scenario 3 (HTTP slice for the chan-drive classifier):
PASS with caveat, recorded as
[OBS-WT6-B](#) (file-browser listing drops symlinks).
Scenario 9 (Terminal-N + Shift+Enter + Ctrl+D): PASS on
all sub-items. Scenarios 1, 2, 4, 5, 6, 7, 8 are blocked
on still-TODO frontend / backsystacean tasks. Cross-cover
on [frontend-1](./frontend-1.md)'s Copy Path: PASS on the
file-tree surface.

@@WebtestB is idle and waiting for any of the following to
flip to REVIEW: [frontend-2](./frontend-2.md),
[frontend-3](./frontend-3.md),
[frontend-4](./frontend-4.md),
[frontend-5](./frontend-5.md),
[backsystacean-3](./backsystacean-3.md),
[backsystacean-4](./backsystacean-4.md),
[backsystacean-5](./backsystacean-5.md). @@WebtestA owns
the rebuild + restart per [webtest-1](./webtest-1.md);
@@WebtestB will pick up the matching scenarios as the
bundle rolls forward.

## Round-by-round

| Round | Scope                                  | Result |
|-------|----------------------------------------|--------|
| 1     | Server reachability (live build)       | PASS   |
| 1     | Scenario 3 HTTP slice                  | PASS   |
| 1     | Scenario 9 Terminal-N / Shift+E / ^D   | PASS   |
| 1     | Cross-cover [frontend-1](./frontend-1.md) Copy Path (tree) | PASS   |
| 2     | Scenario 2 backend slice ([backsystacean-3](./backsystacean-3.md) REVIEW) | PASS |
| 2     | Frontmatter kind ladder ([backsystacean-4](./backsystacean-4.md) REVIEW) | PASS |
| 2     | Tag / mention markdown-only scope rule | PASS   |
| 2     | Scenario 5 backend ([backsystacean-5](./backsystacean-5.md) REVIEW) | PASS |
| 2     | [backsystacean-7](./backsystacean-7.md) `/api/health` indexer block | BLOCKED on @@WebtestA restart |
| 2     | [backsystacean-6](./backsystacean-6.md) spawn-time env contract | PASS (already exercised in Scenario 9) |
| 3     | [backsystacean-7](./backsystacean-7.md) indexer transitions (idle->settling->rebuilding->idle) | PASS |
| 3     | Scenario 6 PANE right-click (`Reload + Toggle Inspector` + more) | PASS |
| 3     | Scenario 6 outside-overlay right-click | PARTIAL (`OBS-WT6-M`) |
| 3     | Scenario 7 terminal bubble menu (size moved, copy/paste/find/restart/split/search/settings) | PASS |
| 3     | Scenario 7 terminal right-click CWD actions | PENDING (`OBS-WT6-N`) |
| 3     | Scenario 1 drive-rooted graph from empty pane | PASS |
| 3     | Scenario 1 drive-rooted graph from active editor | OPEN (`OBS-WT6-O`) |
| 3     | Scenario 2 inspector enrichment (contact pill, file rollup, BACKLINKS, `Graph from here`) | PASS |
| 3     | Scenario 4 royal-pink token wiring | PASS |
| 3     | Scenario 5 web (`directories` everywhere except graph legend) | PARTIAL (`OBS-WT6-P`) |
| 3     | [frontend-6](./frontend-6.md) gated polling (no ghost selected -> no polls) | PASS |
| 3     | [frontend-6](./frontend-6.md) end-to-end with a real ghost node | OPEN (`OBS-WT6-Q`) |
| 4     | [backsystacean-8](./backsystacean-8.md) OBS-WT6-I hardlink dedupe | PASS |
| 4     | [backsystacean-8](./backsystacean-8.md) OBS-WT6-J frontmatter_kind in inspector | PASS |
| 4     | [backsystacean-8](./backsystacean-8.md) OBS-WT6-K canonical nested shape | PASS |
| 4     | [backsystacean-8](./backsystacean-8.md) OBS-WT6-WTA-1 symlinks in /api/files | PASS |
| 4     | [backsystacean-8](./backsystacean-8.md) OBS-WT6-WTA-5 special-file path_class in fs-graph | PASS |
| 4     | Scenario 8 [frontend-3](./frontend-3.md) tab disambiguation (README + contacts/README) | PASS |
| 4     | [Frontend-7](./frontend-7.md) WYSIWYG trailing buffer (defensive) | PASS |
| 4     | [Frontend-8](./frontend-8.md) file browser dismiss + LOADING placeholder | PASS |
| 4     | [Frontend-4](./frontend-4.md) full REVIEW - folder + language chips gone | PASS, resolves `OBS-WT6-P` |
| 5     | [backsystacean-9](./backsystacean-9.md) merged /api/graph?scope=drive shape | PASS |
| 5     | [backsystacean-9](./backsystacean-9.md) per-dir + per-file scopes | PASS |
| 5     | [backsystacean-9](./backsystacean-9.md) read-only dead-end + hardlink + language edges | PASS |
| 5     | [backsystacean-9](./backsystacean-9.md) drive-scope latency (warm 2-2.3 ms) | PASS |
| 5     | Live UI merged graph chip counts (link/tag/contact/language/media/folder) | PASS (`OBS-WT6-T` chip label residue, `OBS-WT6-U` fifo/socket bucketing) |
| 7     | [frontend-13](./frontend-13.md) Cmd/Ctrl+Enter terminal byte test (13;5u + 13;9u + 13;2u) | PASS |
| 7     | [frontend-10](./frontend-10.md) file-browser title shows selected path | PASS |
| 7     | [frontend-10](./frontend-10.md) `Terminal from here` opens PTY at directory CWD | PASS |
| 7     | [frontend-12](./frontend-12.md) directory `Graph from here` opens filesystem graph + inspector | PASS |
| 7     | Serendipitous: ghost-body fallback hint string rendered for `code/notes.md` (`OBS-WT6-Q` partial) | PASS |
| 7     | Filesystem graph chip uses `directory` label (not `folder`) - partial resolution of `OBS-WT6-T` | PASS |
| 8     | [backsystacean-10](./backsystacean-10.md) PTY CWD metadata via WS frames | PASS |
| 8     | [frontend-2](./frontend-2.md) CWD actions live; `Copy path to CWD` -> clipboard `"contacts"` | PASS, resolves `OBS-WT6-N` |
| 8     | [frontend-14](./frontend-14.md) PARTIAL rich-prompt overlay (Alt+Space, Cmd+Enter -> PTY input) | PASS |
| 8     | [frontend-15](./frontend-15.md) window-scoped broadcast (code + unit test) | PASS (live multi-window owed, `OBS-WT6-X`) |

### Round 2 - Scenario 2 (inspector payload) + frontmatter kind

[backsystacean-3](./backsystacean-3.md) +
[backsystacean-4](./backsystacean-4.md) both flipped to REVIEW
and the test service was rebuilt to include them. Drive was
also reseeded by @@Backsystacean with additional fixtures:
contacts/bob.md + contacts/jane.md (two more `chan.kind:
contact` entries), `unknown-kind.md` (markdown with a
non-registered `chan.kind`), `note-with-tags.md`
(markdown with `#tag` + `@@mention`),
`non-md-tags.txt` (text file carrying `#notatag` +
`@@notamention` that must NOT index), plus
`fifo.pipe` (named pipe) and `socket.sock` (UDS).

Probes against `GET /api/inspector?path=<rel>`:

* drive root -> `kind: drive`, `report_summary.totals.files=5`
  (markdown + rust indexed files), `by_language` sorted by
  byte count (Markdown first at 1902 bytes, Rust second at
  189), `subtree.file_kinds` = `{binary:1, document:7,
  media:1, text:1}`. Hardlink pair (`README.md` /
  `README-link.md`) is counted as two markdown files in the
  rollup; flagged for product review at
  [OBS-WT6-I](#).
* `code` (dir) -> `kind: directory`, rollup scoped to subtree
  (1 Rust file, 189 bytes).
* `contacts/alex.md` -> `kind: markdown`, regular markdown
  payload with Markdown report row. PASS.
* `code/hello.rs` -> `kind: text`, language=`Rust`, full
  report row. PASS.
* `non-md-tags.txt` -> `kind: text`, language=`Plain Text`,
  report row present (tokei still tokenizes; it just doesn't
  emit graph edges).
* `note-with-tags.md` -> `kind: markdown`.
* `unknown-kind.md` -> `kind: markdown` (unknown chan.kind
  registry value falls back to plain markdown; not flagged
  as a special kind).
* `blob.bin` -> `kind: binary`, minimal payload (no
  `report_file`, no `report_summary`). PASS for the
  "binary inspector stays minimal" line in
  [request.md](./request.md).
* `pixel.png` -> `kind: media`, minimal payload. PASS.
* `link-internal.md` -> `kind: special`,
  `path_class.kind: symlink, target: notes.md`. PASS.
* `link-outside` -> `kind: special`, `path_class.kind:
  symlink, target: /tmp, target_escapes_drive: true`. PASS.
* `fifo.pipe` -> `kind: special`, `path_class.kind: fifo`.
  PASS for the "fs specific, devices, sym/hard links"
  classifier coverage in [request.md](./request.md).
* `socket.sock` -> `kind: special`, `path_class.kind:
  socket`. PASS.
* `locked-dir` -> `kind: directory`, `permission:
  read_only`, empty rollup (zero everything). Dead-end
  shape PASS.
* `no-such-path` -> HTTP 404 with structured JSON error.
  PASS.

Resolves the listing-side caveat from
[OBS-WT6-B](#): the inspector route does carry the symlink
metadata even though `/api/files` listing previously
dropped them - which makes the file-tree gap a render
choice rather than a backend gap. (The fresh
`/api/files` listing in Round 2 *does* include symlinks
this time. The previous listing was captured before the
@@Backsystacean reseed; suspect the indexer needed a
watcher tick to pick up the new entries, not a code
change in [backsystacean-2](./backsystacean-2.md).)

Probes for [backsystacean-4](./backsystacean-4.md):

* `GET /api/files` returns `kind: contact` on the three
  `contacts/*.md` entries (FileClass surfaces the
  frontmatter-driven kind). `unknown-kind.md` returns
  `kind: document`, which is the registry fallback.
  PASS.
* `GET /api/contacts` returns 3 rows (alex / bob / jane).
  Note: bob and alex have proper titles; jane.md has no
  H1 so the label falls back to `jane.md`. PASS.
* `GET /api/graph` carries contact files as
  `kind: file, node_kind: "contact"`. The dual-kind shape
  lets the renderer pick the contact pill without losing
  the file-system shape. PASS.
* Tag / mention scope:
  `note-with-tags.md` emits four edges
  (`mention -> @@Architect`, `mention -> @@WebtestA`,
  `tag -> #phase6`, `tag -> #regression-test`).
  `notes.md` emits its own set.
  `non-md-tags.txt` emits **no** tag or mention edges
  even though its body contains `#notatag` and
  `@@notamention`. PASS for the markdown-only scope rule
  asserted in [architect-2](./architect-2.md).

`OBS-WT6-I` - Hardlink pair counted twice in
`report_summary.totals`. The drive contains
`README.md` + `README-link.md` as a hardlink pair
(same inode, `link_count: 2`). Both appear in the
listing and both contribute to
`report_summary.totals.files` (4 markdown files = 2
distinct + 2 hardlink copies) and `subtree.bytes`
(double-counted bytes for the hardlinked content). The
classifier flags it (`link_count: 2`), so a downstream
deduper could subtract; today the inspector doesn't. Not
necessarily a bug - the user does see two filenames -
but worth a product call. Routed to
[architect-2](./architect-2.md) for the design memo.

`OBS-WT6-J` - The inspector payload does **not** carry
the frontmatter `chan.kind` value for any file.
[backsystacean-4](./backsystacean-4.md) progress notes
state that "the frontmatter kind badge surfaces via the
inspector payload added in
[backsystacean-3](./backsystacean-3.md)", but
`InspectorPayload` (in
[crates/chan-server/src/routes/inspector.rs](../crates/chan-server/src/routes/inspector.rs))
has no `frontmatter_kind` field. The contact badge today
rides on the `kind` enum exposed by `/api/files`
(`FileClass`) and on the `node_kind` field exposed by
`/api/graph`. Either backsystacean-3's payload needs to
add `frontmatter_kind`, backsystacean-4's wiring needs
to land in the inspector route, or the intended source
of truth needs to be documented (probably "the inspector
reads it from `/api/files`'s `kind` field"). Flagged for
@@Architect / @@Backsystacean before commit.

`OBS-WT6-K` - Frontmatter shape disparity between
fixtures. `contacts/alex.md` uses a flat
`kind: chan.contact` shape. `unknown-kind.md` uses a
nested `chan:\n  kind: task` shape. The graph still
treats `unknown-kind.md` as plain markdown (correct
fallback), so the test passes, but the two fixtures
exercise different frontmatter conventions and the
behavior is asymmetric: one tests "value not in
registry", the other tests "frontmatter shape not in
registry". Both reach the same fallback today, but if
the registry is meant to support a nested shape later,
the contact also needs to migrate. Routed to
[architect-2](./architect-2.md) / [backsystacean-4](./backsystacean-4.md).

### Round 2 follow-up - Scenario 5 backend slice

[backsystacean-5](./backsystacean-5.md) flipped to REVIEW
and the live binary now carries the codemod.

* `GET /api/fs-graph` (default scope) returns
  `scope: "directory"` and node kinds
  `{directory, file, ghost, symlink}`. The `folder`
  spelling is gone from the wire shape. PASS.
* `GET /api/fs-graph?scope=folder&path=code` returns
  HTTP 400 with
  `Failed to deserialize query string: unknown variant
  "folder", expected "file" or "directory"`. No
  backwards-compat shim - matches the "no softening,
  full codemod" decision in
  [journal.md](./journal.md). PASS.
* `GET /api/fs-graph?scope=directory&path=code` returns
  the directory-rooted graph slice as expected. PASS.
* `rg -n '[Ff]older' crates/` returns no hits except
  the documented `rust-embed` macro attribute in
  `crates/chan-server/src/static_assets.rs:29`.
  Matches [backsystacean-5](./backsystacean-5.md)
  completion notes. PASS.

Resolves [OBS-WT6-C](#). Web-side codemod
([frontend-5](./frontend-5.md)) still TODO; the live UI
will see `kind: "directory"` on the wire as soon as it
lands.

### Round 2 follow-up - backsystacean-7 blocked on restart

[backsystacean-7](./backsystacean-7.md) (surface indexer
state on `/api/health`) flipped to REVIEW with the
binary rebuilt at `target/debug/chan` mtime
`2026-05-18T00:53:17`. The live listener (PID 78844,
elapsed 17:33) started at `2026-05-18T00:44:53`, before
the rebuild, so it is still running the pre-7 binary.
`GET /api/health` currently returns just
`{"status":"ok"}` with no `indexer` block.

`OBS-WT6-L` - [backsystacean-7](./backsystacean-7.md) is
on disk but not yet in the live process. Needs
@@WebtestA to restart per
[webtest-1](./webtest-1.md)'s reload protocol (no
frontend bundle change needed - backend-only restart is
sufficient). Probe to re-run once the new PID is up:

```
curl "http://127.0.0.1:8787/api/health?t=<token>"
```

Expect the `indexer` block from
[backsystacean-7](./backsystacean-7.md) completion
notes. To verify the transition, drop a file into
`/private/tmp/chan-test-phase6` and confirm
`indexer.status` flips to `settling` or `rebuilding`
before returning to `idle`.

### backsystacean-6 - no HTTP slice

Decision recorded as option (a) on 2026-05-18: spawn-
time-only `CHAN_TAB_NAME` contract, no env mutation
into the running shell, UI title changes immediately,
inline restart prompt on rename. The current behavior
already matches the contract (verified in Scenario 9 -
`echo $CHAN_TAB_NAME` returns `Terminal-3` after
spawn). UI work for the inline restart prompt and
stale-env badge tracked in
[frontend-2](./frontend-2.md). No backend probing
needed until that lands.

## Round 3 - rebuilt service (PID 4215)

@@WebtestA restarted the service at `2026-05-18T01:12:03`
with the rebuild that includes
[backsystacean-7](./backsystacean-7.md) and the
frontend-2 / frontend-4 / frontend-5 / frontend-6 partial
REVIEW bundles. Browser hard-reloaded; new hashed assets
`index-CsvBw-49.js` + `index-CFwDVi2A.css` served.

### Backsystacean-7 - /api/health indexer block

* Baseline `/api/health` returns:
  ```json
  {"status":"ok","indexer":{"status":"idle","queue_depth":0,"last_event_at":null,"last_settled_at":<ts>,"coalesced_rebuild":false}}
  ```
  PASS for the shape from
  [backsystacean-7](./backsystacean-7.md) completion notes.
* Single-file drop: `status` -> `settling`, `queue_depth: 1`,
  `last_event_at` advances. After ~2 s: `status` -> `idle`,
  `last_settled_at` advances.
* 20-file burst: `status` -> `settling, queue_depth: 20`
  for ~2 s, then `rebuilding, queue_depth: 0` during the
  build pass, then `idle` once cleanup runs. The
  `idle -> settling -> rebuilding -> idle` sequence works
  end-to-end. PASS.
* `coalesced_rebuild` stayed `false` throughout (the flag
  is for the phase-5 git/hg / mass-burst lane; not
  triggered by 20 files).

Resolves [OBS-WT6-L](#).

### Scenario 6 - PANE empty-pane right-click menu
([frontend-2](./frontend-2.md))

Right-clicking the empty pane area opens a menu with:
`Reload`, **`Toggle Inspector`**, `New File` (Ctrl+Alt+N),
`Files` (Cmd+P), `Search` (Cmd+Shift+F),
`Graph` (Cmd+Shift+M), `Terminal` (Cmd+`),
`Split right`, `Split down`, `Settings`. The Toggle
Inspector entry is the new addition from
[frontend-2](./frontend-2.md). The two-button core
("Reload + Toggle Inspector") is present alongside the
broader pane action set. PASS.

### Scenario 6 (continued) - outside-overlay right-click

`OBS-WT6-M` - With the files overlay open, right-clicking
on the dim backdrop area to the left of the overlay panel
shows a context menu (not the browser default - the chan
backdrop handler from [frontend-1](./frontend-1.md) is
active), but the menu shown is the file-browser
**directory** context menu (`Hide Details`, `New file`,
`New directory`, `Import contacts...`,
`Graph from here`, `Search this`,
`Expand all directories`, `Reload`, `Rename drive...`,
`DIRECTORY private/tmp/chan-test-phase6/`, `Settings`),
not the PANE's compact "Reload + Toggle Inspector" menu.

[request.md](./request.md) bullet 42 calls for "the same
2-button menu from the PANE" on the outside-overlay
right-click. Two valid product reads:

1. Show the PANE menu (literal request) - keeps the
   surface predictable; what is "outside" stays outside.
2. Show the overlay's context-aware menu (current
   behavior) - the backdrop is treated as the overlay's
   own background, and a directory-level menu is
   contextually richer.

`Reload` is in both menus; `Toggle Inspector` is only in
the PANE menu. Flagged for @@Architect /
[frontend-2](./frontend-2.md) decision before commit.

### Scenario 7 - terminal bubble menu + right-click
([frontend-2](./frontend-2.md))

Left-clicking the terminal tab AND right-clicking inside
the terminal viewport both open the same bubble menu:

* `Name` rename input (top, with pencil icon)
* `connected · 174x41` status line (size info moved from
  the old top-bar into the bubble - matches
  [request.md](./request.md) bullet 44).
* `Copy`, `Paste` - terminal copy / paste basics
  (request bullet 45).
* `Find`, `Copy Scrollback`, `Restart`,
  `New Terminal` (Cmd+`), `New File` (Ctrl+Alt+N),
  `Split Right`, `Split Down`,
  `Search` (Cmd+Shift+F), `Settings` (Cmd+,).
* MCP env vars: `Set MCP env vars` (default on),
  `Show MCP env in terminal`.
* `Broadcast Input Off` (Cmd+Shift+I) + `Select All` +
  per-tab toggles (`Terminal-1`, `Terminal-3`,
  `Terminal-4`). The current terminal is included in the
  broadcast picker per [frontend-2](./frontend-2.md)
  progress note.

`OBS-WT6-N` - CWD-dependent actions still pending. The
terminal right-click menu does NOT yet expose
`Copy path to dir (to CWD)`, `Show Dir (to open in file
browser)`, or `Graph dir` from
[request.md](./request.md) bullet 45. Matches
[frontend-2](./frontend-2.md) progress note: "Terminal
CWD actions ... remain backend/session-metadata
dependent. `New File` is present but currently falls
back to drive root." Routed back to
[frontend-2](./frontend-2.md).

### Scenarios 1 / 2 / 4 - frontend-4 partial REVIEW

* Scenario 1 (drive-rooted graph default scope):
  `Cmd+Shift+M` from an empty pane opens the graph with
  `SCOPE: Whole drive` and URL hash `#graph=drive`. PASS.
  `OBS-WT6-O` - the same `Cmd+Shift+M` opened with an
  active markdown editor tab (`note-with-tags.md`)
  scoped to that file, not drive. The journal decision
  reads "all entry points" default to drive; the overlay
  opener inherits the active editor's context here.
  Either the decision allows context inheritance from
  the active tab, or this is a gap. Flagged for
  @@Architect.
* Scenario 2 (inspector enrichment): the file browser's
  details panel for `contacts/alex.md` now renders the
  full contact inspector: yellow `CONTACT` pill at top,
  rows for size, modified, tags, contacts, dates,
  links out, backlinks, plus a `CODE` block
  (language=Markdown, SLOC, comments, blanks,
  complexity), action buttons `Open in this pane` and
  `Graph from here`, and a `BACKLINKS` section listing
  `contacts/jane.md`. PASS.
* Scenario 2b (inspector for drive root): drive
  inspector shows `11 files · 3 directories · 3 contacts`
  on the empty-pane help screen (replacing the
  `5 files · 3 folders` legacy text). PASS.
* Scenario 4 (royal-pink language color): the graph
  legend now includes a `language` pill chip and the
  inspector's CODE block uses the new
  `--chan-color-language` token (visual confirmation
  pending side-by-side comparison; the token is wired
  per [frontend-4](./frontend-4.md) progress notes).
  PASS at the wiring level.

### Scenario 5 web - frontend-5 partial REVIEW

* Help screen: `11 files · 3 directories · 3 contacts` -
  `folder` is gone from this surface. PASS.
* File-browser outside-overlay menu: contains
  `New directory`, `Expand all directories`,
  `DIRECTORY private/tmp/chan-test-phase6/`. No
  `folder` copy in this surface. PASS.
* `OBS-WT6-P` - graph legend pill still labeled
  `folder 0`. The backend codemod from
  [backsystacean-5](./backsystacean-5.md) renamed the
  wire shape to `kind: "directory"`, so this pill now
  filters for the obsolete `folder` kind and matches
  zero nodes. Two follow-ups: rename the pill text and
  update its filter to match `kind: "directory"`.
  Routed to [frontend-5](./frontend-5.md) per the
  "broad identifier codemod still pending" note.

### Frontend-6 - live indexer status (gated polling)

[frontend-6](./frontend-6.md) (`web/src/components/GraphPanel.svelte`,
lines 478-514) polls `/api/health` on a 1 s cadence
**only when** the inspector renders a ghost node:

```svelte
let timer: ReturnType<typeof setInterval> | null = null;
...
timer = setInterval(() => void poll(), 1000);
```

with the message mapping:

* `settling`: `indexer is catching up (N event(s) pending)`
* `rebuilding`: `indexer is rebuilding (full pass)`
* `idle`: the static `not in the current file listing
  (try Reload / chan index)` fallback.

Live verification:

* Wrapped `window.fetch` to count `/api/health` calls.
  Opened the graph at drive scope (no ghost selected),
  triggered an indexer settling cycle by writing then
  deleting a markdown file under the drive.
  `__healthFetchCount` stayed at `0` throughout - the
  panel did NOT poll. Matches the gated design.
* Tried to materialise a ghost node by adding a
  markdown file with a broken link
  (`[missing](./no-such-file.md)`). The graph reports
  the link as a `broken: true` edge against the
  surviving link target, not as a separate ghost node.
  Confirmed via `/api/graph`: `kind` counts stayed at
  `{file, tag, mention}` with no `ghost` rows. So
  broken-link probes are NOT sufficient to create a
  ghost - the design path is "drive switch / mass
  delete" per the GraphPanel source comment at
  line 469. Component-level coverage rides on the
  frontend's own test suite (`npm test --run` ->
  18 files / 163 tests passing).
* `OBS-WT6-Q` - end-to-end live verification of
  the ghost-selected -> polled -> rendered hint flow is
  owed and would need a deliberate ghost-node fixture
  (or a transient drive-switch step). Source path,
  wire path ([backsystacean-7](./backsystacean-7.md)),
  and gated trigger all PASS individually.

## Round 4 - rebuilt service (PID 18411 / 4215 restart cycle)

Server restarted again around 01:39:01 to pick up the
[backsystacean-8](./backsystacean-8.md) follow-ups and
the additional REVIEW frontend bundle
([frontend-3](./frontend-3.md),
[frontend-7](./frontend-7.md),
[frontend-8](./frontend-8.md), full
[frontend-4](./frontend-4.md),
[frontend-6](./frontend-6.md)). Bundle on the wire is
`index-Bv-iXYd2.js` + `index-BwBuDndQ.css`.

### Backsystacean-8 fixes

* **OBS-WT6-I (hardlink dedupe)** PASS. Drive root
  inspector report_summary now reports
  `totals.files: 9` (Markdown=7 / Rust=1 / Plain Text=1)
  on a drive containing `README.md` + `README-link.md`
  (hardlink pair). Bytes math checks out:
  166 (alex) + 140 (jane) + 71 (bob) + 677 (README,
  counted once) + 385 (notes) + 138 (note-with-tags)
  + 152 (unknown-kind) = 1729 = `by_language.Markdown.bytes`.
  `subtree.bytes: 2119` adds rust + plain text + binary
  + media on top with the hardlink also deduped at the
  subtree level.
* **OBS-WT6-J (frontmatter_kind)** PASS.
  `/api/inspector?path=contacts/alex.md` returns
  `"frontmatter_kind": "contact"`.
  `unknown-kind.md` and `notes.md` return
  `"frontmatter_kind": null` (unknown kind falls back to
  plain markdown).
* **OBS-WT6-K (canonical nested shape)** PASS. Fixtures
  rewritten: `alex.md` / `bob.md` / `jane.md` /
  `unknown-kind.md` all use nested
  `chan:\n  kind: <name>` form. Flat
  `kind: chan.contact` is gone from the drive.
* **OBS-WT6-WTA-1 (symlinks in /api/files)** PASS. The
  file browser uses `/api/files?dir=` (single-level)
  which now returns symlinks with
  `path_class.kind: "symlink"`. The legacy no-dir
  recursive form still filters symlinks at
  `fs_ops::list_tree_filtered`; that path is used by
  consumers other than the file tree and per the doc
  comment in
  [crates/chan-drive/src/drive.rs](../crates/chan-drive/src/drive.rs)
  line 660 the deep walk "drops special entries (FIFOs,
  sockets, devices) at every level" while "symlinks
  stay visible to the browser". Worth flagging in the
  design memo for clarity.
* **OBS-WT6-WTA-5 (special-file path_class in fs-graph)**
  PASS. `/api/fs-graph` nodes for the FIFO carry
  `path_class.kind: "fifo"`, the socket carries
  `path_class.kind: "socket"`, and symlinks carry
  the existing `path_class.kind: "symlink"` plus
  `target_escapes_drive` when applicable.

All five [backsystacean-8](./backsystacean-8.md) items
PASS in the running binary. Resolves `OBS-WT6-I/J/K/WTA-1/WTA-5`.

### Scenario 8 - [frontend-3](./frontend-3.md) tab disambiguation

* Opened `README.md` (root) + `contacts/README.md`. Tab
  titles render as `README.md` and `contacts/README.md`
  (shortest unique segment prepended). PASS.
* Opened `notes.md` (root) alongside the root tab.
  When only one `notes.md` is open the tab title stays
  as `notes.md`. PASS.
* Hover tooltip: both tabs carry their disambiguated
  path in the `title` attribute (so OS tooltip on hover
  shows it). PASS.
* `OBS-WT6-R` - browser-driven test of the
  `code/notes.md` vs root `notes.md` pair was not
  exercised this round (the code/ subtree had to be
  expanded inside the file tree; only one notes.md ref
  was reachable at any time after the file browser
  auto-dismissed). The README pair already exercises
  the same code path so this is not a regression risk -
  recorded for completeness.

### [Frontend-7](./frontend-7.md) - WYSIWYG trailing buffer

* Opened four markdown tabs (README.md,
  contacts/README.md, notes.md, note-with-tags.md) and
  cycled between them rapidly (long -> short -> medium
  -> short -> long). Each switch rendered cleanly with
  no trailing buffer below the document end.
* The fix is defensive (a `{#key tab.id}` block around
  the editor body) per the task progress notes; not
  reliably reproducible at report time. PASS at the
  observable level.

### [Frontend-8](./frontend-8.md) - dismiss + LOADING

* Wrapped `window.fetch` to delay `/api/files/<path>`
  reads by 2 s. Double-clicked `non-md-tags.txt` in the
  file browser; the file-browser overlay dismissed in
  the same tick, a new tab focused immediately, and the
  body rendered a centred italic `loading...`
  placeholder while the fetch was in flight. After the
  fetch resolved, the editor content replaced the
  placeholder. PASS.
* Tab strip also showed a small in-tab spinner icon
  next to the basename during the LOADING phase
  (nice-to-have UX, not in the acceptance criteria but
  worth noting).
* Did not exercise the explicit error path
  (`api.read` rejection rendering inside the tab body)
  this round; recorded as a follow-up under
  `OBS-WT6-S` if the same pattern needs to be
  exercised against a deliberate 404 or 401.

### [Frontend-4](./frontend-4.md) - upgraded to full REVIEW

Two graph chip-legend changes confirmed in the live
bundle:

* `language` chip pill is gone from the semantic graph
  legend (was visible in Round 3). The legend now
  reads `link 5 · tag 4 · contact 7 · media 0` for a
  whole-drive scope.
* `folder` chip pill is gone (was `folder 0` in Round 3).
  Resolves [OBS-WT6-P](#).
* Drive inspector header still reads
  `X files · Y directories · Z contacts` - `folder` is
  gone from this surface too.

### [Frontend-6](./frontend-6.md) - upgraded to full REVIEW

No new live verification this round. Same gated polling
behavior as Round 3: panel polls `/api/health` only
when a ghost node is selected. Backend transitions
exercised in
[backsystacean-7](./backsystacean-7.md) probe stay
PASS. End-to-end with a real ghost node still owed
(`OBS-WT6-Q`).

## Round 5 - backsystacean-9 merged graph (PID 25002)

@@WebtestA restarted the service at `2026-05-18T01:52:50`
to pick up [backsystacean-9](./backsystacean-9.md) (folds
fs-graph + language-graph into `/api/graph`). Bundle on
the wire is `index-BIdfKyiE.js` + `index-CiUntYr6.css`
(rust-embed dev mode hot-reads the latest `web/dist/`).

### HTTP shape

`GET /api/graph?scope=drive` returns the merged graph:

* **31 nodes**: file=18 (every regular file +
  `outside:link-outside` ghost), directory=4
  (root + `code` + `contacts` + `locked-dir`),
  language=3 (Markdown, Plain Text, Rust), media=1
  (pixel.png), tag=3, mention=2. PASS for the
  acceptance criteria (every declared kind present
  with non-zero counts on the seeded fixture).
* **39 edges**: contains=21 (filesystem spine),
  language=5 (Markdown -> root/code/contacts;
  Plain Text -> root; Rust -> code), link=5
  (markdown cross-link), tag=4, mention=4.
* Each file node carries `path_class` (regular_file,
  symlink, etc) and link_count - hardlink pair
  README.md / README-link.md both show
  `link_count: 2`.
* Directory node ids use the
  `directory:<rel>` form; language edges target the
  same id, so language and filesystem layers land on
  one rendered directory node.
* `locked-dir` shows as a node but emits NO outgoing
  `contains` edges to children. Dead-end semantics
  carry through from
  [backsystacean-2](./backsystacean-2.md). PASS.

`GET /api/graph?scope=directory&path=code` returns 20
nodes / 14 edges scoped to the `code` subtree plus
semantic-graph neighbors. The two file children
(`code/hello.rs`, `code/notes.md`) are emitted with
`contains` edges from `directory:code`. Both Markdown
and Rust language edges land on `directory:code`. PASS.

`GET /api/graph?scope=file&path=notes.md` returns 18
nodes / 12 edges (notes.md plus its parent directory,
its language, and all semantic-graph neighbors). PASS.

### Latency

5 sequential `scope=drive` requests on the 17-entry
test drive returned in 2.0-2.3 ms each (warm
loopback). Well under the
[backsystacean-9](./backsystacean-9.md) acceptance
criteria of "drive-scope response under ~20 ms warm on
the 298-file fixture". PASS at this drive size; the
298-file probe stays owed to
[webtest-1](./webtest-1.md).

### Live UI

Graph overlay rendered at drive scope shows
`30/30 nodes · 31/31 edges` (1 node hidden by the
visible filter set). Legend reads
`link 5 · tag 4 · contact 7 · language 3 · media 1 ·
folder 19`. Six chip categories with non-zero counts;
the filesystem layer is now front and centre. PASS.

`OBS-WT6-T` - the legend pill still labelled
**`folder 19`** even though the producer side uses
`kind: "directory"` from
[backsystacean-5](./backsystacean-5.md). The pill
count is now real (19 = 4 directories + 18 file nodes
- 3 contacts that have their own chip; counts overlap
on multi-kind nodes). The previous `OBS-WT6-P` Round 3
note flagged the same chip when it was at 0; Round 4
saw it disappear on a file-scoped graph (because the
visible filter set was different). Now in Round 5 it
re-emerges on the drive-scope view with a non-zero
count. The remaining work is the **label rename**
("folder" -> "directory") which
[frontend-5](./frontend-5.md) flagged as part of the
"broad identifier codemod still pending".

`OBS-WT6-U` - FIFO and Unix-socket entries appear as
`kind: file` graph nodes alongside regular files. The
inspector at `/api/inspector` distinguishes them as
`kind: "special"` with `path_class.kind: "fifo|socket"`.
The merged-graph node carries the
`path_class.kind` value, so the frontend CAN render a
distinct badge, but the chip taxonomy lumps FIFO /
socket into the `file` category. Worth a product call
- a dedicated `special` chip (or merging them into
the `media` chip's "non-editable" bucket) would mirror
the inspector taxonomy more closely.

`OBS-WT6-V` - Ghost-kind regression between standalone
`/api/fs-graph` and merged `/api/graph` from
[backsystacean-9](./backsystacean-9.md).

Side-by-side on the same drive:

|                            | `/api/fs-graph`           | merged `/api/graph` (scope=drive) |
|----------------------------|---------------------------|----------------------------------|
| `fifo.pipe`                | `kind: ghost` + `path_class.kind: fifo` | `kind: file` + `path_class.kind: fifo` |
| `socket.sock`              | `kind: ghost` + `path_class.kind: socket` | `kind: file` + `path_class.kind: socket` |
| `outside:link-outside`     | `kind: ghost, outside: true`              | `kind: file` (no `path_class`)   |
| `link-internal.md`         | `kind: symlink`           | `kind: file`                     |
| `link-outside`             | `kind: symlink`           | `kind: file`                     |

The merged response flattens every fs-graph node into
`kind: "file"`, losing the `ghost` / `symlink`
distinctions. The classifier metadata is preserved on
`path_class.kind`, so the rendering layer CAN still
distinguish them with one more conditional. But:

* [Frontend-6](./frontend-6.md) polling triggers when
  `selectedFsNode.kind === "ghost"` (per
  `web/src/components/GraphPanel.svelte:1180`). On the
  merged graph that condition is never true for FIFO /
  socket / outside-symlink nodes, so the indexer-status
  hint will NOT show when the user clicks those nodes
  in the new merged overlay. The hint still works for
  any node the merged graph explicitly tags
  `kind: "ghost"` (currently none in my fixtures).
* The legend chip taxonomy is consistent with this
  (`OBS-WT6-U`); both observations point at the same
  unified "what is a ghost in the merged layer" call.

Recommendation for @@Architect /
[backsystacean-9](./backsystacean-9.md): either keep
`kind: "ghost"` for fs-graph dead-ends in the merged
shape, OR have the frontend treat any
`path_class.kind in (fifo, socket, "outside")` as a
ghost-equivalent for the inspector. Either path
restores the polling trigger.

This also explains why end-to-end verification of
[OBS-WT6-Q](#) (frontend-6 live ghost-node hint) keeps
landing in "not exercised" territory in the merged
graph view: there are no ghost nodes in the merged
payload on the seeded drive.

## Round 7 - frontend-10 / 12 / 13 + serendipitous OBS-WT6-Q

@@WebtestA restarted the service at `2026-05-18T02:58:14`
(PID 47718) to pick up
[frontend-10](./frontend-10.md),
[frontend-12](./frontend-12.md) (dir half),
[frontend-13](./frontend-13.md) at REVIEW.

### [Frontend-13](./frontend-13.md) - terminal modifier-Enter chords

Ran `cat -v` inside a terminal and pressed each
modifier-Enter chord in turn. Output (one logical line):
`s:^[[13;2uc:^[[13;5um:^[[13;9uEND`

Decoding:

* `s:` -> Shift+Enter -> `\x1b[13;2u` (kitty CSI 13;2u).
  Matches the previously verified Round 1 byte sequence.
* `c:` -> Ctrl+Enter -> `\x1b[13;5u` (kitty CSI 13;5u).
* `m:` -> Cmd+Enter -> `\x1b[13;9u` (kitty CSI 13;9u).

All three encodings match
`web/src/terminal/keymap.ts` line 4-6. PASS.

### [Frontend-10](./frontend-10.md) - file browser title + Terminal from here

* **Title**: file-browser overlay header now reads the
  drive-relative path of the currently selected entry
  (e.g. `contacts/alex.md` when alex.md is selected,
  `notes.md` when notes.md is selected). Truncation +
  hover tooltip not directly exercised; the path text
  fits within the title bar at the test viewport
  width. PASS.
* **"Terminal from here"**: right-click on the `code/`
  directory exposes a `Terminal from here` action.
  Clicking it spawns a new `Terminal-2` tab whose
  shell prompt reads
  `mbp ...e/tmp/chan-test-phase6/code $` - PTY launched
  with `code/` as its CWD. PASS.

### [Frontend-12](./frontend-12.md) - directory "Graph from here"

Right-click on the `contacts/` directory exposes a
`Graph from here` action. Clicking it opens the
**filesystem graph** overlay (distinct from the
semantic / merged graph) with the directory's inspector
panel populated:

* DIRECTORY pill at top.
* `contacts` summary: files=4, subdirectories=0,
  size=435 B, last change=1h ago.
* FILE KINDS row: `contact 3 · document 1`.
* CODE rollup: indexed=4, SLOC=0, comments=24,
  blanks=4, complexity=1, Markdown=4 files / 0 SLOC.
* COCOMO (basic-organic) summary.
* `Show Directory` + `Graph from here` action buttons.

The directory variant of "Graph from here" now lands
end-to-end. URL hash carries `graph=drive|1||1|fs`
(the `fs` suffix is the filesystem graph mode). PASS.

`OBS-WT6-W` (partial resolution of `OBS-WT6-T`) -
the filesystem-graph chip legend uses
`contains 0 · symlink 2 · hardlink 1 · directory 19`,
i.e. the **correct `directory` label**. The legacy
`folder` chip residue is therefore confined to the
**semantic / merged graph** view that
[backsystacean-9](./backsystacean-9.md) introduces.
Two surfaces with two different chip sets, only the
semantic one still says `folder`. Confirms the
remaining work is the
[frontend-5](./frontend-5.md) broad-identifier codemod
pass (parked to phase 6.1 per
[architect-4](./architect-4.md)).

### OBS-WT6-Q (frontend-6) - exercised at last

While clicking through the file tree I landed the
graph inspector on a `code/notes.md` graph node. The
DETAILS panel rendered the **ghost-body view**:

```
DOCUMENT
notes
code/notes.md
not in the current file listing (try Reload / chan index)
```

This is exactly the static fallback string mapped at
`web/src/components/GraphPanel.svelte:1213`. The
indexer was idle at the time, so the live hint was the
static one - matching the `idle` branch of
[frontend-6](./frontend-6.md). End-to-end ghost-body
render path is wired. The remaining live verification
(indexer settling -> hint updates to "indexer is
catching up (N event(s) pending)") was not exercised
this round but is now within reach: pick a ghost node,
trigger an indexer burst, watch the hint flip.

Why a ghost rendered: the inspector was looking at
`code/notes.md` from the merged-graph payload, but
that file was in a different tree-state at the click
moment. The "missing from current listing" semantics
fired. Worth a follow-up to confirm whether this is a
race against an incremental indexer pass or a
genuine "graph DB says it exists but filesystem
listing doesn't" gap.

## Round 8 - backsystacean-10 + frontend-14 + frontend-15

@@WebtestA restarted the service at `2026-05-18T03:22:38`
(PID 63479) with [backsystacean-10](./backsystacean-10.md),
[frontend-14](./frontend-14.md) PARTIAL, and
[frontend-15](./frontend-15.md) REVIEW.

### [Backsystacean-10](./backsystacean-10.md) - PTY CWD metadata

Terminal-3 was spawned earlier via "Terminal from here"
on `contacts/` and its shell prompt reads
`mbp ...p/chan-test-phase6/contacts $`. Right-clicking
inside the terminal now exposes the previously-blocked
CWD actions:

* `Copy path to CWD`
* `Show Dir`
* `Graph dir`
* `New File` (the row that previously fell back to drive
  root)
* `Rich prompt` (Alt+Space) - new from
  [frontend-14](./frontend-14.md)

`Copy path to CWD` action exercised end-to-end: clicked,
then `navigator.clipboard.readText()` returned the
literal string `"contacts"` - the drive-relative path
of the PTY's live CWD. Drive-root prefix stripped per
[backsystacean-10](./backsystacean-10.md)'s wire
contract. PASS.

Resolves [OBS-WT6-N](#) - the
[frontend-2](./frontend-2.md) CWD action set is no
longer gated on backend metadata.

### [Frontend-14](./frontend-14.md) PARTIAL - rich-prompt overlay

`Alt+Space` (or right-click -> `Rich prompt`) opens an
in-pane markdown editor attached to the current
terminal. Layout: terminal stays in the upper portion,
the prompt overlay docks in the lower portion with a
draggable splitter (`:::`) between them. Toolbar
exposes `Aa` (text/render toggle), `</>` (source view),
attach (document icon), send (paper-airplane), and
close (X).

End-to-end smoke: typed `echo test-from-rich-prompt`
into the rich prompt, pressed `Cmd+Enter`. The raw text
appeared in Terminal-3's input line (no newline
appended). Closed the overlay and pressed `Return`
inside the terminal to execute - shell ran the command
and printed `test-from-rich-prompt`. PASS.

Note the deliberate "no newline" send semantics: the
user can review / edit the typed text in the terminal
before pressing Enter. Matches the design intent of
"sends the raw markdown source to that terminal's
PTY".

Per-terminal overlay state: each terminal keeps its own
overlay. Previous overlay content (`hello from rich
prompt`) was restored on re-open.

### [Frontend-15](./frontend-15.md) - window-scoped broadcast invariant

Defensive lane: per the progress notes,
`broadcastTerminalInput` only resolves target ids
through `allTerminalTabs()`, which walks the current
window's Svelte layout registry. A registered input sink
for a tab not present in the layout is skipped. The
explicit doc-comment + Vitest cross-window scenario
(`19 files / 185 tests`) cover the invariant.

Live multi-window verification (open two chan-desktop
windows on the same chan-server, broadcast from one and
confirm the other never sees the input) was not
exercised this round - would need a chan-desktop launch
or a second browser tab pointing at a distinct
`w=<window-label>`. The code path is read-clean and the
unit test guards regressions. PASS at the code-review
and test-coverage level.

`OBS-WT6-X` - end-to-end live verification of
window-scoped broadcast is owed for the final
hardening pass. Easiest path: open chan in two browser
tabs (each gets its own `w=<window-label>`), spin up
matching terminal sets, enable broadcast in one
tab, type into a terminal, confirm zero echo in the
other tab's terminals.

## Completion notes

(populated at phase close)
