# @@WebtestB idle

Status: idle after two rounds of parallel scenarios

Completed in [webtest-2](./webtest-2.md):

Round 1 (initial build at PID 67471):

* Scenario 3 - chan-drive `path_class` HTTP slice via
  `/api/files` + `/api/fs-graph`. PASS with observations
  `OBS-WT6-B/C/D`.
* Scenario 9 - Terminal-N enumeration (monotonic across
  close), `CHAN_TAB_NAME` at spawn, Shift+Enter bytes
  (`\x1b[13;2u`), Ctrl+D close-hint and tab close. All
  PASS. Extra finding `OBS-WT6-E` (close-tab confirm
  dialog for running tabs is new and not on the Phase 6
  checklist).
* Cross-cover for [frontend-1](./frontend-1.md) REVIEW:
  file-tree right-click "Copy Path" wired (`OBS-WT6-G`).

Round 2 (rebuilt service includes
[backsystacean-3](./backsystacean-3.md) +
[backsystacean-4](./backsystacean-4.md) at REVIEW; drive
reseeded with bob/jane contacts, unknown-kind.md,
note-with-tags.md, non-md-tags.txt, fifo.pipe,
socket.sock):

* Scenario 2 backend slice - `GET /api/inspector?path=...`
  for drive, dir, markdown (contact + non-contact),
  text, binary, media, symlink (internal + escaping),
  FIFO, Unix socket, read-only dir, missing path. All
  shapes match the design memo. PASS.
* Frontmatter kind ladder - `contact` recognised through
  `/api/files` `kind: "contact"`, `/api/graph`
  `node_kind: "contact"`, and `/api/contacts` filtering.
  Unknown chan.kind values fall back to plain markdown.
  PASS. Gap recorded: `OBS-WT6-J` (inspector payload
  doesn't carry frontmatter_kind despite
  [backsystacean-4](./backsystacean-4.md) saying it
  should) and `OBS-WT6-K` (fixture frontmatter shapes
  disagree).
* Tag / mention markdown-only scope - `note-with-tags.md`
  emits four edges (2 tags + 2 mentions);
  `non-md-tags.txt` emits zero edges even though its
  body has both. PASS.
* Hardlink double-count observed (`OBS-WT6-I`).
* Scenario 5 backend slice
  ([backsystacean-5](./backsystacean-5.md) REVIEW) - the
  fs-graph wire now emits `scope: "directory"` and
  node kind `"directory"`; the legacy `folder` value
  returns HTTP 400 (no shim). `rg [Ff]older crates/` is
  clean except the `rust-embed` macro attribute. PASS.
  Resolves the earlier `OBS-WT6-C`.
* [backsystacean-6](./backsystacean-6.md) - option (a)
  decision (spawn-time-only `CHAN_TAB_NAME` contract, no
  env mutation). Already exercised end-to-end in
  Scenario 9 (Terminal-3's shell read
  `CHAN_TAB_NAME=Terminal-3`); UI work for the inline
  restart prompt + stale-env badge tracked in
  [frontend-2](./frontend-2.md). No backend probing
  needed.
* [backsystacean-7](./backsystacean-7.md) BLOCKED on
  restart - the binary on disk includes the new
  `/api/health` `indexer` block (binary mtime
  `2026-05-18T00:53:17`), but the live listener
  (PID 78844, started `00:44:53`) is the pre-7 build.
  `GET /api/health` currently returns just
  `{"status":"ok"}`. Recorded as `OBS-WT6-L` for
  @@WebtestA to pick up on the next restart.

Round 3 (PID 4215, restarted at 01:12:03, includes
frontend-2 / 4 / 5 / 6 + backsystacean-7):

* [backsystacean-7](./backsystacean-7.md) PASS end-to-end.
  Idle baseline, single-file drop -> settling -> idle,
  20-file burst -> settling -> rebuilding -> idle.
  Resolves `OBS-WT6-L`.
* Scenario 6 PANE empty-pane right-click PASS (`Reload`,
  `Toggle Inspector` plus broader actions).
* Scenario 6 outside-overlay right-click PARTIAL
  (`OBS-WT6-M`) - shows the file-browser directory menu,
  not the literal PANE 2-button menu. Product call
  needed.
* Scenario 7 terminal bubble menu PASS for the
  size/copy/paste/find/restart/splits/search/settings
  move. CWD actions (`Copy path to CWD`, `Show Dir`,
  `Graph dir`) still pending per
  [frontend-2](./frontend-2.md) progress note
  (`OBS-WT6-N`).
* Scenario 1 drive-rooted graph from empty pane PASS;
  from active editor still scopes to that file
  (`OBS-WT6-O`).
* Scenario 2 inspector enrichment PASS (contact pill,
  rollup rows, `Graph from here`, `BACKLINKS` section).
* Scenario 4 royal-pink token PASS at the wiring level.
* Scenario 5 web PARTIAL - `directories` everywhere
  except the graph legend pill which still says
  `folder 0` (`OBS-WT6-P`).
* [frontend-6](./frontend-6.md) gated polling
  negative-verified (no ghost selected -> 0 fetches)
  PASS. End-to-end with a real ghost node not exercised
  (`OBS-WT6-Q`); would need a deliberate ghost fixture.

Round 4 (rebuilt service: PID 18411 from 01:39:01,
includes [backsystacean-8](./backsystacean-8.md) +
[frontend-3](./frontend-3.md) +
[frontend-7](./frontend-7.md) +
[frontend-8](./frontend-8.md) + full
[frontend-4](./frontend-4.md) +
[frontend-6](./frontend-6.md)):

* [backsystacean-8](./backsystacean-8.md) - all five
  items PASS at the HTTP layer. Resolves
  `OBS-WT6-I/J/K/WTA-1/WTA-5`.
* Scenario 8 (tab disambiguation): README.md (root) +
  contacts/README.md disambiguate as expected; full
  path in hover tooltip. PASS.
* [frontend-7](./frontend-7.md) WYSIWYG trailing buffer:
  not reproduced across a four-tab cycle. PASS at the
  observable level.
* [frontend-8](./frontend-8.md): with `/api/files/<path>`
  delayed 2 s, the overlay dismissed instantly and the
  destination tab showed a centred `loading...`
  placeholder; content rendered after the fetch
  resolved. PASS.
* [frontend-4](./frontend-4.md) graph legend: `language`
  and `folder` chips both removed. Resolves
  `OBS-WT6-P`.

Round 5 ([backsystacean-9](./backsystacean-9.md) merged
graph, PID 25002 from 01:52:50):

* `/api/graph?scope=drive` returns 31 nodes / 39 edges
  with file / directory / language / media / tag /
  mention kinds and contains / language / link / tag /
  mention edges. PASS for the merged-shape acceptance
  criteria.
* Per-directory and per-file scopes return correct
  subtree + neighbor sets. PASS.
* `locked-dir` emits no outgoing `contains` edges
  (dead-end carries through). Hardlink pair shows
  `link_count: 2` on both file nodes. Language edges
  target the same `directory:<rel>` node ids that the
  filesystem spine uses. PASS.
* Drive-scope latency 2-2.3 ms warm (well under the
  20 ms target on the 298-file probe, which is still
  owed by [webtest-1](./webtest-1.md)).
* Live UI: legend chip counts
  `link 5 · tag 4 · contact 7 · language 3 · media 1 ·
  folder 19`. PASS.

* `OBS-WT6-T` - legend chip still labelled
  **`folder`** in the live UI (count is now real).
  The label rename to `directory` belongs to the
  remaining [frontend-5](./frontend-5.md) "broad
  identifier codemod" pass.
* `OBS-WT6-U` - FIFO + Unix-socket file nodes appear
  in the `file` chip bucket while the inspector
  surfaces them as `kind: "special"`. Worth a product
  call on whether the chip taxonomy should grow a
  `special` bucket.
* `OBS-WT6-V` - Ghost-kind regression: the merged
  `/api/graph` emits FIFO / socket / outside-symlink
  as `kind: "file"` while standalone `/api/fs-graph`
  emits them as `kind: "ghost"`. The classifier still
  rides on `path_class.kind`, but
  [frontend-6](./frontend-6.md) polling is gated on
  `kind === "ghost"` and never fires on these nodes
  in the merged overlay. Routes back to
  [backsystacean-9](./backsystacean-9.md) /
  [architect-2](./architect-2.md) for a product call.
  Also explains why `OBS-WT6-Q` keeps landing in
  "not exercised" - there are no ghost nodes in the
  merged payload on the seeded drive.

Round 7 (PID 47718 from 02:58:14, includes
[frontend-10](./frontend-10.md),
[frontend-12](./frontend-12.md) dir half,
[frontend-13](./frontend-13.md)):

* [frontend-13](./frontend-13.md) terminal chord bytes:
  Shift+Enter -> 13;2u, Ctrl+Enter -> 13;5u,
  Cmd+Enter -> 13;9u. All PASS via `cat -v` byte
  capture.
* [frontend-10](./frontend-10.md) file-browser title
  shows selected entry's drive-relative path.
  `Terminal from here` spawns a PTY whose CWD is the
  selected directory (verified via the prompt path).
  PASS.
* [frontend-12](./frontend-12.md) directory
  `Graph from here` opens the **filesystem graph**
  overlay (not the semantic / merged graph) with a
  rich DIRECTORY inspector for the selected dir
  (file counts, size, FILE KINDS, CODE rollup,
  COCOMO). PASS.
* `OBS-WT6-W` (partial resolve of `OBS-WT6-T`) - the
  filesystem graph's chip legend uses
  `contains 0 · symlink 2 · hardlink 1 ·
  directory 19`. So the legacy `folder` chip label is
  confined to the semantic / merged graph view. Two
  graph views, two chip sets; the merged one is the
  remaining bit of `frontend-5` broad codemod.
* `OBS-WT6-Q` (partial) - serendipitously rendered the
  ghost-body view on `code/notes.md` while exercising
  frontend-10's row clicks. The static idle hint
  string fires correctly; the live `settling` /
  `rebuilding` swap is now within reach with a
  deliberate ghost + burst.

Round 8 (PID 63479 from 03:22:38, includes
[backsystacean-10](./backsystacean-10.md),
[frontend-14](./frontend-14.md) PARTIAL,
[frontend-15](./frontend-15.md) REVIEW):

* [backsystacean-10](./backsystacean-10.md) PTY CWD
  metadata is on the wire. The terminal right-click now
  exposes `Copy path to CWD` / `Show Dir` / `Graph
  dir` / `New File` (CWD-seeded) and `Rich prompt`.
  Exercised `Copy path to CWD` end-to-end: clipboard
  received the literal drive-relative path `contacts`.
  PASS. Resolves [OBS-WT6-N](#).
* [frontend-14](./frontend-14.md) PARTIAL rich-prompt
  overlay: Alt+Space opens an in-pane markdown editor;
  Cmd+Enter sends the raw text to the PTY without
  appending a newline (user reviews + Enters). Per-
  terminal overlay state is preserved. PASS.
* [frontend-15](./frontend-15.md) window-scoped
  broadcast: code is read-clean
  (`broadcastTerminalInput` resolves only through
  `allTerminalTabs()`) and the Vitest cross-window
  scenario covers regressions
  (`19 files / 185 tests`). Live multi-window
  verification still owed (`OBS-WT6-X`).

@@WebtestB is now idle on every Phase 6 task that has
flipped to REVIEW or partial-REVIEW. Remaining work:

* [frontend-3](./frontend-3.md) extra pair check
  (code/notes.md vs root notes.md). Same code path as
  README pair already covered (`OBS-WT6-R`).
* [frontend-6](./frontend-6.md) end-to-end ghost-node
  + live indexer hint (`OBS-WT6-Q`).
* [frontend-8](./frontend-8.md) explicit error path
  rendering in tab body (`OBS-WT6-S`).
* Remaining open scenarios in
  [webtest-2.md](./webtest-2.md) round-by-round.

Operational notes:

* `OBS-WT6-A` - The PID recorded in
  [webtest-1](./webtest-1.md) was 61390 at the start of
  this round; the live listener was actually a later
  rebuild. Same drive, same bearer token. Heads-up for
  @@WebtestA on every restart pass.

Observations needing a product call before commit:

* `OBS-WT6-B` - file-browser listing initially dropped
  symlinks. Round-2 listing includes them; suspect
  watcher tick rather than a code change. Worth a
  follow-up confirmation.
* `OBS-WT6-E` - new app-level "Close tab? Terminal-N is
  still running" confirm dialog is not on the Phase 6
  checklist.
* `OBS-WT6-I` - hardlink pair counted twice in
  inspector report_summary totals + subtree.bytes.
* `OBS-WT6-J` - inspector payload missing
  `frontmatter_kind`.
* `OBS-WT6-K` - frontmatter fixture shapes
  (`kind: chan.contact` flat vs `chan:\n  kind: task`
  nested) are asymmetric.

Blocked / ready for the next rebuild:

* Scenario 1 (drive-rooted graph default scope) needs
  [frontend-4](./frontend-4.md).
* Scenario 2 web side (inspector UI rendering) needs
  [frontend-4](./frontend-4.md). Backend side cleared
  in Round 2.
* Scenario 4 (royal-pink language color) needs
  [frontend-4](./frontend-4.md).
* Scenario 5 (terminology codemod) web side needs
  [frontend-5](./frontend-5.md). Crates side cleared.
* Scenario 6 (right-click menus) needs
  [frontend-2](./frontend-2.md).
* Scenario 7 (terminal bubble menu + right-click) needs
  [frontend-2](./frontend-2.md).
* Scenario 8 (tab disambiguation) needs
  [frontend-3](./frontend-3.md).

@@WebtestB will pick up the matching scenarios as those
flip to REVIEW and @@WebtestA restarts the service per
[webtest-1](./webtest-1.md)'s reload protocol.

## 2026-05-18 phase-close ping

@@Architect: the final image-paste smoke that
@@WebtestA flagged as the last gate before commit is
**RESOLVED**. Recipe ran end-to-end on the live
PID 63479 service:

1. PNG copied to clipboard with `osascript ... PNGf`.
2. `Alt+Space` opened the rich-prompt overlay on the
   active terminal.
3. `Cmd+V` -> image rendered inline, `POST
   /api/attachments` returned `200`, attachment file
   created at
   `/private/tmp/chan-test-phase6/attachments/image.png`
   (69 bytes, 1x1 PNG, matches source).
4. Source mode shows `![](attachments/image.png#w=250)`.
5. `Cmd+Enter` -> terminal PTY received the literal
   markdown source on its input line. Bash history-
   expansion complains about `![`, but that is a shell
   quirk, not a chan defect; claude / codex read raw
   stdin without expansion.

Full detail under
[OBS-WT6-WTA-12](./webtest-1.md#obs-wt6-wta-12---image-paste-smoke-resolved)
in [webtest-1](./webtest-1.md). Phase 6 commit gates
from my side are now all green.
