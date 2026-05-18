# @@WebtestA task 1: baseline test service for Phase 6

Owner: @@WebtestA
Status: IN_PROGRESS

## Goal

Stand up the live web test service for Phase 6 and keep it
available so @@Frontend, @@Backsystacean, and @@Architect can
validate Phase 6 changes as they land.

## Status note

Phase 6 was formally initialized by @@Architect on 2026-05-18
([journal.md](./journal.md), [architect-1](./architect-1.md),
[architect-2](./architect-2.md), parallel implementation tracks
fanned out). This task was retained as @@WebtestA's primary
assignment per the dispatch table; @@WebtestB picks up parallel
scenarios in [webtest-2](./webtest-2.md).

Source list: [request.md](./request.md). Process:
[process.md](./process.md).

## Setup

Drive: throwaway, `chan-test-phase6` at `/private/tmp/chan-test-phase6`.

Seeded with:

* `README.md` - phase notes.
* `notes.md` - markdown with cross-link, `#tag`, `@@mention`.
* `contacts/alex.md` - frontmatter `kind: chan.contact`.
* `code/hello.rs` - Rust source so the new "code" / royal-pink
  language-tag color and the text-file inspector path can be
  exercised.

Build + launch:

```
cargo build -p chan
./target/debug/chan serve /private/tmp/chan-test-phase6 --no-browser
```

Output goes to `/tmp/chan-test-phase6.stderr` and `/tmp/chan-test-phase6.stdout`.

## Live service

* URL: http://127.0.0.1:8787/?t=wB68doozwEwucY7qVG3SH2DTF5HYGQ6R
* Bearer token: `wB68doozwEwucY7qVG3SH2DTF5HYGQ6R`
  (token is per-launch but happened to round-trip the same value on
  restart; treat as opaque and re-read from `webtest-1` on each
  rebuild)
* Port: 8787 (default)
* PID: 63479 (after the bs-10 + fe-14 + fe-15 rebuild)
* Drive path: `/private/tmp/chan-test-phase6`
* Started: 2026-05-17, restarted 2026-05-18 with REVIEW bundle.

Baseline smoke:

* 2026-05-17: `GET /api/health` -> `{"status":"ok"}`,
  `GET /?t=...` -> 200.
* 2026-05-18: restarted on rebuilt bundle (frontend-1 REVIEW +
  backsystacean-1 REVIEW present in binary).
  `GET /api/health` -> `{"status":"ok"}` (with bearer; unauth path
  now returns 401, which is the post-phase-5 behavior).
  `GET /?t=...` -> 200.

## Smoke checklist (to run as Phase 6 work lands)

Architectural cleanups:

* [ ] Graph default opens at the drive root (drive -> dirs -> files)
      without needing "Graph this" from the file browser.
* [ ] Inspector shows drive, dir, markdown, text, and binary file
      tiers with the right level of detail per
      [request.md](./request.md).
* [ ] `kind: chan.contact` frontmatter still renders as a contact
      under the new markdown layer.
* [ ] `#tag` and `@@mention` show up only when sourced from
      markdown files.
* [ ] Language tag uses the new "royal pink" color and no longer
      collides with the green tag color.
* [ ] No remaining "folder" wording in UI strings after the codemod.

Bugs / nits:

* [ ] New-file dialog opens prefilled with current dir + `untitled.md`
      (no tab-press step).
* [ ] Editor "New File" lands the new file in the same parent dir
      as the currently edited file.
* [ ] Dark/light theme toggle refreshes terminals without reload.
* [ ] PANE left-click menu has both "Reload" and "Toggle Inspector".
* [ ] Right-click on the area outside an overlay shows the same
      Reload + Inspector menu (not the browser default).
* [ ] File browser and file editor menu both expose "Copy file path".
* [ ] Terminal top-bar info ([size]/[search]/[copy]/[restart])
      moves into the terminal bubble menu.
* [ ] Terminal right-click menu: copy/paste, Copy path to dir,
      Show Dir, Graph dir, New Terminal, split-pane, search,
      settings.
* [ ] New terminals are named `Terminal-N`.
* [ ] Same-name file tabs disambiguate via shortest common-ancestor
      suffix; full path shown on hover.
* [ ] File browser exposes "Copy Path" on files and dirs.
* [ ] `^d` in the shell shows a "press ^d to close the tab" hint
      and the second `^d` closes.
* [ ] Renaming the tab updates the PTY env (terminal name) too.
* [ ] `shift+enter` inside the embedded terminal sends newline to
      `claude` / `codex` (not a bare enter).

## Reload protocol

Per [CLAUDE.md](../CLAUDE.md), frontend changes need the full
rebuild cycle (no hot reload):

1. Stop the server (`kill 56504` or the active pid).
2. `cd web && npm run build`.
3. `cargo build -p chan` from repo root.
4. Restart `./target/debug/chan serve /private/tmp/chan-test-phase6 --no-browser`.
5. Capture the new bearer token in this file and force-reload the
   browser tab.

Backend-only changes can skip step 2.

## Teardown

At phase close (Alex calls it):

* Stop the test service (`kill <pid>`).
* `rm -rf /private/tmp/chan-test-phase6`.
* `./target/debug/chan remove /private/tmp/chan-test-phase6` to drop
  the registry entry.
* Record final state and any not-cleaned-up items here.

## Observations

> Label-prefix note: @@WebtestA uses the `OBS-WT6-WTA-*` prefix and
> @@WebtestB uses `OBS-WT6-WTB-*` in [webtest-2](./webtest-2.md).
> Suggested to @@Architect after a label collision on the first
> shared finding (both initially filed as `OBS-WT6-B/C`). The
> rename here disambiguates without losing history.

* **OBS-WT6-WTA-1** ([backsystacean-2](./backsystacean-2.md), REVIEW) -
  `/api/files` listing omits symlinks. Repro: add a symlink to the
  drive (`ln -s notes.md link.md`), `GET /api/files?path=` does not
  return it. `GET /api/fs-graph?path=` does return it with
  `kind: "symlink"`. Same finding filed independently by @@WebtestB
  as their `OBS-WT6-B` (now `OBS-WT6-WTB-1`); the duplicate is
  intentional cross-coverage, not separate work. Root cause in
  `crates/chan-drive/src/drive.rs` `Drive::list` filter. Flagging
  for @@Backsystacean to confirm whether the file-browser surface
  intentionally filters symlinks for safety (consistent with the
  chan-drive write-side refusal of special files) or whether the
  inspector should render them in the tree too. Not blocking REVIEW
  pass.
* **OBS-WT6-WTA-2** ([backsystacean-2](./backsystacean-2.md), REVIEW) -
  `/api/files?path=<file-path>` returns the drive-root listing
  instead of per-path inspector data. Looks pre-existing (not
  introduced by phase 6); the new per-path inspector route is
  `/api/inspector?path=` (see backsystacean-3 below). If any
  phase-6 consumer was still planning to use `/api/files?path=`
  for per-path data, point them at `/api/inspector` instead.
* **OBS-WT6-WTA-4** ([backsystacean-3](./backsystacean-3.md) /
  [backsystacean-4](./backsystacean-4.md), REVIEW) - The
  `/api/inspector?path=` payload for a markdown file does not
  surface the resolved `chan.kind` / renderer hint. Repro: with
  `contacts/alex.md` (a registered `chan.kind: contact`) and
  `unknown-kind.md` (a `chan.kind: task` value not in the
  registry) both seeded under the test drive, the inspector
  returns identical top-level keys for both:
  `[path, kind, is_dir, size, mtime, path_class, report_file]`,
  with no `chan_kind` / `frontmatter` / `renderer` field. The
  contact still lights up via `/api/contacts`, so behavior on
  existing surfaces is intact. Gap is for the
  [frontend-4](./frontend-4.md) badge work: without this field on
  the inspector payload, the frontend has to second-call
  `/api/contacts` (or re-parse frontmatter) to draw the contact
  pill in the inspector. Suggesting `backsystacean-3` extend the
  payload with the `ChanKindSpec` (name + renderer) when the file
  is markdown and the registry resolves, so the badge is a single
  round-trip. Not blocking REVIEW pass for either task; flagging
  before frontend-4 starts so the API surface stabilizes.
* **OBS-WT6-WTA-3** RESOLVED 2026-05-18 -
  [backsystacean-5](./backsystacean-5.md) REVIEW drop flipped the
  fs-graph wire payload: `scope: "directory"` and node `kind:
  "directory"` now match the `/api/inspector` payload. Leaving
  the entry visible for traceability.

  Original: ([backsystacean-5](./backsystacean-5.md), TODO) -
  `/api/fs-graph` payload still uses `kind: "folder"` (and
  `scope: "folder"`) on the wire. Terminology codemod has not
  reached the JSON contract. `/api/inspector` already returns
  `kind: "drive" | "directory" | ...` with the new vocabulary, so
  the inconsistency is between the two routes. Flag for the
  codemod owner to decide whether to flip the fs-graph wire enum
  in this phase or migrate the wire later. Same observation also
  filed by @@WebtestB as `OBS-WT6-WTB-2` (their original
  `OBS-WT6-C`).

## Smoke results

### backsystacean-2 file classifier (REVIEW)

* [x] Regular file: `path_class.kind == "regular_file"`, perm,
      nlink=1. (`notes.md`, `code/hello.rs`)
* [x] Directory: `path_class.kind == "directory"`, perm=read_write.
      (`code/`, `contacts/`)
* [x] Hardlink (nlink > 1): both `README.md` and `README-link.md`
      report `link_count: 2` after `ln -f README.md README-link.md`.
* [x] Read-only directory: `chmod 555 locked-dir` ->
      `permission: "read_only"`.
* [x] Symlink (internal): fs-graph returns
      `kind: "symlink", target: "notes.md"`, no `target_escapes_drive`.
* [x] Symlink (off-drive): fs-graph returns
      `kind: "symlink", target: "/tmp", target_escapes_drive: true`,
      plus a companion `kind: "ghost"` node for the off-drive
      target. Behavior matches and exceeds the architect-2 memo.
* [x] FIFO: `mkfifo fifo.pipe` ->
      `path_class.kind: "fifo"`, inspector `kind: "special"`,
      `link_count: 1`. PASS.
* [x] Socket: Python `AF_UNIX` bound socket left as
      `socket.sock` -> `path_class.kind: "socket"`, inspector
      `kind: "special"`. PASS.
* [ ] Block / character device: not exercised (requires sudo /
      `mknod`; outside the throwaway drive's privileges). Spot
      coverage via the chan-drive unit tests in backsystacean-2
      should be enough; flagging if @@Architect wants me to
      pursue.
* **OBS-WT6-WTA-5** ([backsystacean-2](./backsystacean-2.md) /
  [backsystacean-3](./backsystacean-3.md)) - `/api/fs-graph`
  collapses both FIFO and socket nodes to `kind: "ghost"` (same
  bucket as off-drive symlink targets), so the FIFO-vs-socket
  distinction the classifier surfaces on `/api/inspector` is lost
  at the graph layer. Likely intentional ("can't render, can't
  traverse"), but worth confirming with @@Architect: if the
  graph dead-end semantics want a distinct badge per special
  kind, the fs-graph node payload needs to carry the
  `path_class.kind` through.

Net: classifier REVIEW gets a PASS for the kinds and properties
that the design memo declared in scope. FIFO / socket / device are
open items, not regressions. OBS-WT6-WTA-1 / -WTA-2 should get an
ack from @@Backsystacean before commit.

### backsystacean-3 inspector + chan-report rollup (REVIEW)

Endpoint: `GET /api/inspector?path=<rel>`. Empty / missing path
returns the drive-root payload.

* [x] Drive (`path=""`): `kind: "drive"`,
      `report_summary.totals.files: 5`,
      `report_summary.by_language: [Markdown 4 files / 1902 B,
      Rust 1 file / 189 B]`,
      `subtree.files: 7, directories: 3, bytes: 2184,
      file_kinds: {binary: 1, document: 4, media: 1, text: 1}`.
      Language rollup matches the fixture.
* [x] Directory (`path=code`): `kind: "directory"`,
      `report_summary` scoped to `code/` (Rust 1 file / 189 B),
      `subtree.file_kinds: {text: 1}`. Per-subtree scoping
      correct.
* [x] Markdown file (`path=notes.md`): `kind: "markdown"` with
      `report_file` (language=Markdown, code/comment/blank/
      complexity/bytes). No `report_summary` / `subtree` for
      single files. Frontmatter `chan.kind: contact` not yet
      surfaced - that is [backsystacean-4](./backsystacean-4.md)
      TODO, not a backsystacean-3 gap.
* [x] Text file (`path=code/hello.rs`): `kind: "text"` +
      `report_file` (language=Rust). Matches spec.
* [x] Media binary (`path=pixel.png`): `kind: "media"`, payload
      minimal (path_class + size + mtime only). Matches spec.
* [x] Non-media binary (`path=blob.bin`): `kind: "binary"`,
      payload minimal. Matches spec.
* [x] Internal symlink (`path=link-internal.md`):
      `kind: "special"`, `path_class.kind: "symlink"`,
      `target: "notes.md"`.
* [x] Off-drive symlink (`path=link-outside`):
      `kind: "special"`, `path_class.kind: "symlink"`,
      `target: "/tmp"`, `target_escapes_drive: true`.
* [x] Locked dir (`path=locked-dir`, `chmod 555`):
      `kind: "directory"`, `permission: "read_only"`, empty
      `report_summary` + `subtree.files: 0,
      file_kinds: {}` (no descent).
* [x] Missing path (`path=does/not/exist.md`): HTTP 404.
* [x] Path traversal (`path=../etc/passwd`): HTTP 400; chan-drive
      sandbox holds.
* [x] No-arg form (`/api/inspector` with no `path`): HTTP 200 with
      drive payload.

Net: inspector REVIEW PASS across every payload kind declared in
the task. The only gap is the not-yet-shipped frontmatter kind
badge from [backsystacean-4](./backsystacean-4.md), now filed as
**OBS-WT6-WTA-4**. No latency concern observed on this fixture;
matches the on-demand no-cache decision in backsystacean-3's
notes (try the larger phase-5 VCS fixture for the drive-scope
latency call).

### backsystacean-4 frontmatter kind ladder + tag/mention scope (REVIEW)

Drive seeded with: `contacts/alex.md`
(`chan.kind: contact`, plus `name: Alex`),
`contacts/jane.md` (same kind, `name: Jane Doe`,
`email: jane@example.com`), `contacts/bob.md`
(`chan.kind: contact`, H1 only), `unknown-kind.md`
(`chan.kind: task` - value not in registry),
`note-with-tags.md` (markdown with `#phase6 #regression-test
@@Architect @@WebtestA`), `non-md-tags.txt` (plain text with
`#notatag` and `@@notamention`).

* [x] Canonical frontmatter shape: `chan:` nested map, `kind:`
      value. Confirmed against
      `crates/chan-drive/src/markdown/frontmatter.rs`. The flat
      `kind: chan.contact` shape (used by the initial `alex.md`
      fixture earlier in this task) was never the canonical
      form; my fixture was wrong, not the registry.
* [x] Contact kind resolves: `/api/contacts` enumerates alex,
      jane, and bob after fixing the frontmatter shape. Routes
      and node kind unchanged from phase 5.
* [x] Unknown chan.kind value falls back to plain markdown:
      `unknown-kind.md` (`chan.kind: task`) returns
      `inspector.kind == "markdown"` with no contact handling
      and no graph contact node. Matches the registry contract:
      unknown kinds are ordinary markdown.
* [x] Tag/mention scope is markdown-only: graph payload sources
      `#phase6 / #regression-test / #webtest` and
      `@@Architect / @@WebtestA` only from `note-with-tags.md`
      and `notes.md` (markdown). `non-md-tags.txt` contributes
      zero tag/mention edges to the graph even though its body
      contains `#notatag` and `@@notamention`. PASS - matches
      the design memo and the new
      `file_type_policy_end_to_end` test cited in
      [backsystacean-4](./backsystacean-4.md).
* [ ] Contact label resolution from `name:` frontmatter field
      is pre-existing H1/filename behavior, not phase-6 work.
      Recorded only because Alex's contact resolves to "Alex"
      (matches the H1) and Jane's resolves to "jane.md"
      (filename fallback - no H1 in jane.md). The `name:`
      field is not consulted by the contact label resolver.
      Not in backsystacean-4 scope; noting in case the
      future `chan.kind` ladder wants to standardize this.

### backsystacean-3 latency probe (larger fixture)

Backsystacean-3 progress note asked Webtest to measure on-demand
aggregation on a larger drive than the 8-file phase-6 fixture.
Set up a throwaway snapshot at `/private/tmp/chan-test-phase6-big`
containing `crates/` + `web/` (minus `target`, `node_modules`, `.git`,
`dist`): 298 files / 50 dirs / 67 MB, 112 Rust files, 42 Svelte, 101
TypeScript, 15 Markdown, 3 JSON, 3 binary, 2 media. Served on a
second `chan serve` on port 8788 to avoid contending with
[webtest-1](./webtest-1.md)'s phase-6 service.

Latency (curl real time, 3-5 runs each, warm):

| Endpoint                                              | runs       |
|-------------------------------------------------------|------------|
| `GET /api/inspector?path=` (drive)                    | 10ms each  |
| `GET /api/inspector?path=crates/chan-drive/src` (dir) | <10ms each |
| `GET /api/inspector?path=crates/chan-drive/src/drive.rs` (file) | <10ms each |
| `GET /api/fs-graph?path=` (depth=1, 3 nodes)          | 10ms each  |
| `GET /api/fs-graph?path=&depth=4` (266 nodes)         | 10ms each  |
| `GET /api/graph`                                      | 10ms each  |

Verdict: on-demand aggregation is comfortably under the latency
budget on a fixture ~5x the size of the architect-2 benchmark
target. No watcher-cached rollup needed for this phase.
Drive served at port 8788 torn down after the probe;
`/private/tmp/chan-test-phase6-big` removed.

Net: backsystacean-4 REVIEW PASS for every in-scope item
(registry shape, contact stays the reference renderer, unknown
kinds degrade gracefully, tag/mention markdown-only). OBS-WT6-WTA-4
flags the frontmatter-kind surfacing gap on `/api/inspector` for
[backsystacean-3](./backsystacean-3.md) /
[backsystacean-4](./backsystacean-4.md) /
[frontend-4](./frontend-4.md) to triage.

### frontend-1 (REVIEW) - browser pass

Drove the live editor through chrome MCP against the running
PID 78844 (the backsystacean-4 build). Alex had already authored a
session with three terminals before I attached; reused that.

* [x] **New-file dialog quick-start (file browser).**
      Right-click `contacts/` -> "New file": dialog opens with
      `contacts/untitled.md` pre-filled and the `untitled` stem
      pre-selected. No press-Tab step.
* [x] **New-file dialog quick-start (editor menu).**
      Opened `note-with-tags.md` in the editor, right-clicked
      inside the body -> "New File" (Ctrl+Alt+N). Dialog opens
      with `untitled.md` (drive root, since the active file is
      at the root) and `untitled` pre-selected. Parent-dir logic
      verified in source against `parentPath(tab.path)` in
      `FileEditorTab.svelte`.
* [x] **Copy Path in the file browser (files).** Right-click
      `notes.md` -> "Copy Path" present alongside Graph this /
      Search this / Rename / Delete.
* [x] **Copy Path in the file browser (dirs).** Right-click
      `contacts/` -> "Copy Path" present alongside New file /
      New folder / Graph this / Search this / Rename / Delete.
* [x] **Copy File Path in editor body menu.** Right-click inside
      the editor body of `note-with-tags.md` -> "Copy File Path"
      ribbon click triggers the "Copied file path" toast at the
      bottom-left status pill.
* [x] **Terminal theme refresh without reload.** Cmd+, opened
      Settings; APPEARANCE Light selected; on dismiss the
      terminal repainted to the light theme immediately (no
      Reload pressed). Switched back to Dark, terminal repainted
      back. Matches the request.
* [x] **Overlay-backdrop right-click.** With the file browser
      overlay open, right-click on the dimmed backdrop area
      outside the shell (at x=1350 on a 1400-wide canvas)
      surfaces the chan overlay menu (Hide Details / New file /
      New folder / Import contacts / Graph this / Search this /
      Collapse all folders / Reload / Rename drive / Settings),
      not the browser default. The journal flagged this as
      `[~]` because frontend-2 still owes the Inspector toggle
      that the request expected to share with the PANE menu.
      What ships now is a strict super-set; the goal of "no
      browser default" is met.

### backsystacean-1 (REVIEW) - browser pass

* [x] **Terminal-N monotonic enumeration.** Existing session
      shows tabs Terminal-1, Terminal-3, Terminal-4 (Terminal-2
      was created and closed; counter did not reuse the slot).
      Matches the spec's intent.
* [x] **Shift+Enter routes to enhanced keyboard bytes.** The
      pre-existing `cat -v` session output captured
      `abc^[[13;2u\ndef` which is the CSI-u (modifyOtherKeys
      / fixterms) escape for Shift+Enter (key=13 / Enter, mod=2
      / Shift). The PTY received the right sequence, so
      programs like claude / codex that consume CSI-u see the
      modifier.
* [x] **CHAN_TAB_NAME + MCP discovery in PTY env.** Ran
      `env | grep ^CHAN_ | sort` in Terminal-4: returned
      `CHAN_TAB_NAME=Terminal-4`, `CHAN_MCP_SERVER_JSON=...`,
      `CHAN_MCP_SERVER_NAME=chan`, `CHAN_MCP_SOCKET=...`,
      `CHAN_MCP_COMMAND=...`, `CHAN_MCP_COMMAND_JSON=...`.
      Discovery env is wired through the new
      `terminal_sessions.rs`.
* [x] **^D close hint string.** After `exit` ran, the chan
      side overwrote the dead prompt with:
      `process exited (127); press Ctrl+D to close this tab`.
      Exact request wording.
* [ ] **^D close hint actually closes the tab.** Sending the
      Ctrl+D keystroke (both via the chrome MCP `key` action
      and a synthetic `KeyboardEvent` with `ctrlKey: true`,
      `defaultPrevented: true` in the dispatch) did not close
      the tab in this session. The print part of the feature
      works; the wire-up to actually close needs either a
      different gesture path or live keyboard input. Filing as
      **OBS-WT6-WTA-6** below for @@Backsystacean /
      @@Frontend to confirm before commit, or for Alex to
      drive on a real keyboard.

### Terminal bubble menu (frontend-2 progress sneak)

Right-clicking on a terminal pane surfaces the new bubble menu
([frontend-2](./frontend-2.md) target). Already present today:

* [x] Editable Name field (currently "Terminal-4").
* [x] `connected - 80x24` status line **inside the bubble** -
      the request's "the information in the terminal's top bar
      today [size] ... [search] [copy] [restart] moves to the
      terminal's bubble menu" lands here.
* [x] Copy / Paste rows.
* [x] Find, Copy Scrollback.
* [x] Restart.
* [x] New Terminal (Cmd+`).
* [x] Split Right, Split Down.
* [x] Search (Cmd+Shift+F).
* [x] Settings (Cmd+,).
* [x] Broadcast Input Off (Cmd+Shift+I).
* [x] MCP: "Set MCP env vars" toggle (currently on) +
      "Show MCP env in terminal" command.
* [x] Tab list at the bottom (Terminal-1 / Terminal-3).
* [ ] **Copy path to dir (CWD).** Missing - frontend-2 TODO.
* [ ] **Show Dir** (open file browser at CWD). Missing -
      frontend-2 TODO.
* [ ] **Graph dir** (graph CWD). Missing - frontend-2 TODO.

### Inspector visual confirmation

Single-click on `note-with-tags.md` in the file browser tree
opened the DETAILS pane with the full inspector payload from
[backsystacean-3](./backsystacean-3.md) and the tag/mention
edges from [backsystacean-4](./backsystacean-4.md):

* Big orange DOCUMENT badge.
* Size 138B, modified time.
* Counters grid: tags=2, contacts=2, dates=0, links out=0,
  backlinks=0.
* CODE section: language=Markdown, SLOC=0, comments=3,
  blanks=1, complexity=2 (chan-report data).
* TAGS chip row: `#phase6`, `#regression-test` (green).
* CONTACTS chip row: Architect, WebtestA (with contact icon).

Sub-observation: the inspector visually labels @@mentions
under "CONTACTS" alongside real `chan.kind: contact` files
(alex.md / bob.md / jane.md). They're rendered identically.
Not necessarily wrong - the chip can link to a contact when
one exists - but the section title may confuse users when a
mention does not resolve to a contact file. Recording as
**OBS-WT6-WTA-7** for @@Frontend / @@Architect to triage in
frontend-4's badge work.

### backsystacean-5 codemod (REVIEW)

Rust side of the `folder` -> `directory` codemod.

* [x] `/api/fs-graph` payload: `scope: "directory"` (was
      `"folder"`), node kinds use `"directory"` for dirs. Wire
      now matches `/api/inspector`. Verified at the live
      service: root payload returns
      `node kinds: {directory: 4, file: 8, ghost: 3, symlink: 2}`.
* [ ] User-visible UI wording in the file browser still shows
      "New folder" on the directory right-click menu and the
      overlay-level right-click. That is
      [frontend-5](./frontend-5.md) (TODO at this poke), not a
      backsystacean-5 gap. Recording so the codemod review can
      sequence the two cleanly.

### backsystacean-6 tab-rename to env propagation (REVIEW)

Alex's decision: option (a), spawn-time-only contract. The UI
title renames immediately; `$CHAN_TAB_NAME` inside the running
shell stays at the inherited value until the user clicks
Restart (which re-spawns the PTY with refreshed env).

* [x] Confirmed via [backsystacean-6](./backsystacean-6.md) and
      `crates/chan-server/src/terminal_sessions.rs`: no live
      env mutation path is wired. The decision means there is
      nothing new to send on the wire when the tab is renamed;
      the Restart button is the explicit user gesture that
      refreshes env. I did not exercise rename + Restart in the
      browser because Alex's session has three live terminals
      and the experiment would invalidate his current
      `CHAN_TAB_NAME` shell value. Recording as code-confirmed;
      Alex can drive the rename + Restart loop in a few seconds
      when convenient.

### backsystacean-7 indexer state surface (REVIEW)

New `GET /api/index/status` and `POST /api/index/rebuild`.

* [x] **Idle status payload.** `GET /api/index/status` returns
      `{state: "idle", indexed_docs: 11, indexed_vectors: 11,
      model: "BAAI/bge-small-en-v1.5"}` on the phase-6 drive.
      The 11 documents match the markdown + text fixtures
      seeded so far (README, README-link, notes,
      note-with-tags, contacts/alex, contacts/jane,
      contacts/bob, unknown-kind, plus code/hello.rs and the
      drive README slot the indexer keeps).
* [x] **Rebuild kick.** `POST /api/index/rebuild` returns
      `202 Accepted` and the next poll observes
      `state: "building"` with `current`, `total`, `file`
      fields exposed. The build window is short on this small
      fixture; back to idle within ~1 second.
* [x] **Tagged-union shape.** The two states return different
      keys (`state: "idle"` plus stats; `state: "building"`
      plus progress fields). The frontend
      [frontend-6](./frontend-6.md) can pattern-match on
      `state` cleanly. Matches the design memo in
      [backsystacean-7](./backsystacean-7.md).

### frontend-6 indexer state UI (REVIEW)

Triggered an index rebuild via `fetch('/api/index/rebuild', {method:
'POST'})` from the browser; observed the new live status pill at
the bottom-left toast slot:

```
• indexing 8/9 (unknown-kind.md)
```

with an orange dot. The pill disappears when the indexer returns
to idle. Replaces the old static "try Reload / chan index" hint
that the request-followup at 2026-05-18 asked for. PASS.

`/api/health` now nests the indexer status:
`{status:"ok", indexer:{status, queue_depth, last_event_at,
last_settled_at, coalesced_rebuild}}` (the wide breakdown that
`/api/index/status` returns is the narrow one designed for the
UI; the health payload uses a separate compact key set).

### frontend-4 graph + inspector UI (REVIEW, partial)

Drove the graph panel (Cmd+Shift+M) and the file-browser inspector
on the live service.

* [x] **Royal-pink language token wired.** `getComputedStyle`
      against the root reports
      `--chan-color-language: #ff4db8`
      (same value on `--chan-color-code`). Token shipped per the
      architect-2 decision. Distinct from tag green.
* [x] **Graph scope dropdown exposes drive scope.** Options:
      `note-with-tags.md` (file - the default when opened from an
      active editor), `Whole drive`,
      `All drives (cross-drive, coming soon)`.
* [ ] **Graph default scope = drive.** Cmd+Shift+M from an active
      editor still scopes to the file by default; the spec wants
      "drive" as the default across every entry point. The drive
      option exists in the picker, but the default is not flipped
      yet. Matches frontend-4 "partial".
* [x] **Drive-scope graph payload renders.** Switching the picker
      to `Whole drive` redraws the graph at 14/17 nodes /
      10/10 edges with filter chips
      `link 2 · tag 4 · contact 7 · language 0 · media 0 ·
      folder 0`.
* [ ] **Graph "language" chip count = 0** even though the drive
      contains Rust and Markdown content (the chan-report rollup
      shows them in the inspector). Either the language nodes
      aren't emitted in the semantic graph yet or the chip filter
      gates them. Recording as
      **OBS-WT6-WTA-8** for backsystacean / frontend-4 follow-up.
* [x] **Inspector renders chan-kind-aware badge.** The Details
      pane shows distinct top-bar badges per entity:
      `FOLDER` for directories (orange-pink stripe),
      `CONTACT` for `chan.kind: contact` markdown (yellow stripe),
      `DOCUMENT` for plain markdown (orange stripe), etc. So the
      visual surfacing of OBS-WT6-WTA-4 is handled at the UI
      layer even though the JSON payload doesn't carry the field;
      the OBS still stands at the API contract level but the
      practical UX gap is closed.
* [x] **Directory inspector enrichment.** `code/` shows
      files=1, subdirectories=0, size=189 B, last change, CODE
      section (indexed/SLOC/comments/blanks/complexity), a
      language row labelled "Rust" with "1 file 3 SLOC", and a
      COCOMO (BASIC-ORGANIC) effort/schedule/developers
      projection. "Graph from here" button matches the design
      memo wording.
* [x] **Contact inspector enrichment.** `contacts/alex.md` shows
      CONTACT badge, counters (tags/contacts/dates/links
      out/backlinks), CODE section, "Open in this pane",
      "Graph from here", and a BACKLINKS row pointing to
      `contacts/jane.md` (jane has a link to alex in her body).

### frontend-2 right-click menus (REVIEW, partial)

* [x] **Terminal bubble menu gained New File.**
      Right-click on the terminal pane now includes a
      `New File (Ctrl+Alt+N)` row that opens the new-file
      dialog seeded with the terminal's CWD. Plus a new
      `Select All` row.
* [x] **Bubble still carries `[size]`.** Status line in the
      bubble currently reads `connected - 174x41` for the
      resized Terminal-4 pane; size moves live.
* [x] **Bubble carries the spec's "search / copy / restart"**
      items (Search Cmd+Shift+F, Copy + Copy Scrollback,
      Restart) inside the bubble. Top bar now carries only the
      title (no separate top-bar status row visible).
* [ ] **Copy path to dir (CWD).** Still missing from the
      terminal bubble menu.
* [ ] **Show Dir** (open file browser at CWD). Still missing.
* [ ] **Graph dir** (graph CWD). Still missing.
* [ ] **PANE Inspector toggle.** Could not exercise without
      disrupting Alex's three live terminals; the empty-pane
      menu state isn't reachable on this session. Deferred to
      Alex on real keyboard.
* [/] **File browser collapsed on first open.** Mixed result:
      `code/` was collapsed in this session but `contacts/`
      and `locked-dir/` were expanded - possibly remembered
      state from earlier interactions. Need a fresh
      first-open test on a freshly-registered drive.

### frontend-5 web codemod (REVIEW, partial)

Source-side codemod from "folder" -> "directory" / "dir" is
partial in the web bundle.

* [x] **User-visible label flipped in some places.**
      `<span class="folder-label">Directory</span>` in
      `FileBrowserOverlay.svelte` already says "Directory".
      `Pane.svelte` summary uses
      `{driveSummary.folders} directories` (label flipped, the
      identifier did not).
* [ ] **Inspector badge for directories still reads `FOLDER`**
      (uppercase) on the live UI.
* [ ] **Graph filter chip still reads `folder 0`** (lowercase).
* [ ] **Directory right-click "New folder"** in file-browser
      tree.
* [ ] **Overlay-level menu** still has "Collapse all folders"
      / "Expand all folders".
* [ ] **Identifier residue**: `pickInitialFolder`,
      `collapseAllFolders` / `expandAllFolders`,
      `driveSummary.folders` field, `fmtFolder` helper, CSS
      classes `.folder-row` / `.folder-label` / `.folder-text`
      / `.folder-path`, lucide `FolderOpen` / `FolderPlus`
      icons (third-party - cannot codemod). Consistent with
      `Status: partial; broad identifier codemod still
      pending` on the task file.

Net: frontend-5 is doing exactly what it advertises (partial).
No new gaps relative to the task description.

### backsystacean-8 follow-up fixes (REVIEW)

bs-8 closed four of my and @@WebtestB's observations.

* [x] **Hardlink dedupe** (closes WTB OBS-I):
      `/api/inspector?path=` at drive root now reports
      `subtree.files: 11` (every filesystem name including the
      `README.md` <-> `README-link.md` hardlink pair) but
      `report_summary.totals.files: 9` (unique inodes). The
      chan-report rollup deduplicates by `(dev, ino)` on Unix.
      The filesystem-side `subtree.files` is intentionally still
      counted by name so callers that want the on-disk count
      can have it.
* [x] **`frontmatter_kind` on inspector payload** (closes
      **OBS-WT6-WTA-4**):
      `/api/inspector?path=contacts/alex.md` returns
      `frontmatter_kind: "contact"`,
      `unknown-kind.md` (with `chan.kind: task`) returns
      `frontmatter_kind: null`,
      `notes.md` returns `null`,
      `code/hello.rs` returns `null`. Matches the registry
      contract.
* [x] **Symlinks visible in `/api/files?dir=`** (closes
      **OBS-WT6-WTA-1**):
      `/api/files?dir=` returns 13 entries including
      `link-internal.md` and `link-outside` with
      `path_class.kind: "symlink"`. The file tree visually
      renders them in italic style. FIFO and socket files
      remain filtered from `/api/files`, matching the
      architect decision to lift symlinks only (the
      chan-drive write-side refusal of all special files
      stays as a write-only guarantee).
* [x] **fs-graph nodes carry `path_class` for FIFO/socket**
      (closes **OBS-WT6-WTA-5**):
      `/api/fs-graph?path=` returns special-kind nodes with
      `kind: "ghost"` (correct dead-end semantics) plus a
      nested `path_class: { kind: "fifo" / "socket", ... }` so
      the frontend can render distinct badges per kind.
* [x] **Canonical frontmatter shape documented** (closes
      WTB OBS-K): `crates/chan-drive/design.md` updated to
      state the nested `chan:` map is canonical. Spot-checked
      via the existing fixture - the registry resolves the
      nested form.

`/api/inspector` payload also now carries an interesting bonus:
`subtree.file_kinds` now separates `contact: 3` from `document: 5`
(was `document: 4` before), so the frontmatter kind feeds the
subtree counts too.

### frontend-3 same-name tab disambiguation (REVIEW)

Seeded `contacts/README.md` alongside the existing root `README.md`
and `code/notes.md` alongside the existing root `notes.md`. Opened
the pair in two editor tabs.

* [x] **Disambiguation rule**: tab titles read `README.md` (root)
      and `contacts/README.md` (the conflicting pair); shortest
      common ancestor segment surfaces in the disambiguating tab.
* [x] **Full path on hover via `title` attribute**: each tab DOM
      node carries `title="<tab-display-name>"` (matches the
      visible label when no further disambiguation is needed).
      Spec said "Hover shows the full path" - here the visible
      label IS already the full path relative to the drive root,
      so the goal is met for the visible cases.
* [x] **Close-revert**: closing `contacts/README.md` collapses
      the remaining tab title back to plain `README.md`
      (`title: "README.md"`). No stale disambiguation.

### frontend-4 graph + inspector (REVIEW, the full drop)

* [x] **Royal-pink language nodes shipped in the graph**.
      Drive-scope view now contains three pink/magenta nodes
      (`language 3` chip), distinct in color from green tag
      nodes. Closes **OBS-WT6-WTA-8**.
* [x] **Filter chip set expanded**:
      `link 19 · tag 6 · contact 8 · language 3 · media 0 ·
      folder 4` at drive scope on the current fixture.
      `folder 4` matches the 4 directories present
      (root + code + contacts + locked-dir).
* [x] **Drive scope picker richer**: now exposes
      `dir:<path>: directory: <path>/` entries alongside file
      and drive scopes.
* [x] **Inspector badge codemod**: directory inspector now
      shows **DIRECTORY** (was FOLDER). Document and CONTACT
      badges unchanged. This closes the inspector-side miss
      I had recorded under frontend-5 partial.
* [x] **Inspector enrichment**: new FILE KINDS chip row
      (`document 1 · text 1` etc), language breakdown
      (`Rust 1 file 3 SLOC`, `Markdown 1 file 0 SLOC` for the
      code/ dir after seeding `code/notes.md`), COCOMO
      projections, LINKS TO section (shows the file's outgoing
      links by basename), BACKLINKS section.
* [ ] **Drive default scope from editor entry**:
      Cmd+Shift+M with an active editor tab opens the graph at
      `file:<active>` scope, not drive. Spec wanted drive as
      the default across every entry point. Switching is
      one click in the picker; not a blocker. Recording as
      remaining (the journal's
      `[ ] "Graph this" from any surface defaults to the drive
      scope` may still be open).
* [x] **`folder` chip still labelled "folder"** at drive
      scope in the graph filter row - matches frontend-5
      partial state.

### frontend-6 indexer state UI (REVIEW) - SECOND CONFIRMATION

Already verified in the previous session; the status pill at
the bottom-left still surfaces during rebuild on this newer
binary. `/api/health` payload nests indexer status (now
including `last_event_at` / `last_settled_at` / `queue_depth`
/ `coalesced_rebuild`) - matches the expanded summary already
visible on the welcome health check.

### frontend-7 markdown trailing buffer fix (REVIEW)

The fix is a defensive `{#key tab.id}` lifecycle reset around the
editor body so a tab swap tears down editor-local state.

* [x] **Investigation steps as scripted**: opened `README.md`
      (long), `contacts/README.md` (short), `notes.md`
      (medium with headings and chips). Switched tabs in
      sequence `long -> short -> long -> medium -> long`.
      Each render shows clean document end with no trailing
      bullet markers or stale text below the document body.
* [x] **PASS as defensive fix**: no reproduction observed
      after focused attempts. Matches the task's
      "unreproducible after focused session" branch with the
      key-wrapper landed as defensive coverage.

### frontend-8 file-browser dismiss + LOADING (REVIEW)

* [x] **Dismiss-on-open**: double-clicked `note-with-tags.md`
      in the file browser tree. The overlay dismissed
      immediately, a new tab `note-with-tags.md` opened in
      focused state, and the editor body rendered the file
      content. PASS.
* [/] **LOADING state**: not visible on this localhost
      fixture - the file is 138 bytes and the fetch completes
      sub-frame. The defensive LOADING fallback ships but
      can't be exercised without a throttled network or a
      larger fixture. Recording as code-confirmed via the
      task's progress notes; live observation owed to a slower
      network scenario.

### backsystacean-9 unified /api/graph (REVIEW)

`/api/graph?scope=drive` (and the default no-scope form) now folds
the filesystem layer into the semantic graph instead of returning
only markdown-centric content.

Node kind histogram on the live fixture (30 nodes):
`tag: 3, mention: 2, file: 18, directory: 4, language: 3, media: 1`.

Edge kind histogram (39 edges):
`link: 5, mention: 4, tag: 4, contains: 21, language: 5`.

* [x] **Directory nodes shipped** (`kind: "directory"`, 4 of them
      = drive root + code/ + contacts/ + locked-dir/).
* [x] **File nodes shipped** (`kind: "file"` with `path_class`).
* [x] **Language nodes shipped** (`kind: "language"`, 3 of them =
      Markdown / Rust / per-language label, each carrying
      `files` and `code` counts).
* [x] **Media nodes shipped** (`kind: "media"` for `pixel.png`).
* [x] **Sub-kind via `node_kind` extra field**: contact markdown
      files now carry `node_kind: "contact"` on top of
      `kind: "file"`. Frontend renderer can use either the
      primary layer kind or the sub-kind to choose icon / color.
* [x] **`contains` edges (21)** wire each parent dir to its
      children and root to the top-level entries.
* [x] **`language` edges (5)** wire files to language nodes.
* [x] **`tag` / `mention` / `link` edges preserved** at the
      markdown layer.

UI / chip observations on the drive-scope view:

* [/] Filter chip row reads
      `link 5 · tag 4 · contact 7 · language 3 · media 1 ·
      folder 19`. The chip counts disagree with the underlying
      API payload: only **4** directory nodes exist (root +
      code + contacts + locked-dir), but the chip reports
      `folder 19`. Likely the chip is summing
      file-with-directory-edges or similar. Filing as
      **OBS-WT6-WTA-9** for the frontend graph filter
      computation. Recording at REVIEW pass; chip counts only,
      data underneath is correct.
* [x] **`contact 7`** is also a chip mismatch (3 contact files
      + 2 mentions = 5 max, the previous chip showed 8 before
      bs-9). Same chip-counter concern as above; folded into
      OBS-WT6-WTA-9.
* [x] **Graph renders the layered view at drive scope**:
      visible orange file nodes connected to gray directory
      icons, pink/magenta language nodes, green hash tag
      nodes, and yellow contact icons.

### Additional observations from the browser pass

* **OBS-WT6-WTA-6** RESOLVED 2026-05-18 -
  @@WebtestB confirmed Ctrl+D close-hint **and** tab close as
  PASS in [webtest-2](./webtest-2.md) round 1 (Scenario 9).
  My original observation was a chrome MCP keystroke-delivery
  quirk on macOS (the `key` action and synthetic
  `KeyboardEvent` both fired but didn't reach xterm.js in a
  way that triggered the close handler). The print part of
  the feature works on my end too; the close action works
  with a real keyboard. Closing.

  Original: ([backsystacean-1](./backsystacean-1.md) /
  [frontend-2](./frontend-2.md), REVIEW) - The `^D` close hint
  string prints correctly when the shell exits, but pressing
  Ctrl+D after the hint did not close the tab in the browser
  session.
* **OBS-WT6-WTA-9** ([frontend-4](./frontend-4.md) /
  [backsystacean-9](./backsystacean-9.md), REVIEW) - The graph
  panel's drive-scope filter chip row reports
  `folder 19` and `contact 7` while the underlying
  `/api/graph?scope=drive` payload contains only
  4 directory nodes and 3 contact-classified file nodes.
  The chip count is mismatched against the layered payload
  that [backsystacean-9](./backsystacean-9.md) ships;
  likely the chip is summing per-edge or
  per-contains-entry. Data underneath the chips is correct
  (graph renders the directories, files, and language nodes
  with right edges). Filing as a chip-filter / counter
  computation issue for frontend-4 or a follow-up frontend
  task.

* **OBS-WT6-WTA-8** RESOLVED 2026-05-18 -
  [backsystacean-9](./backsystacean-9.md) shipped language
  nodes in the unified `/api/graph`; chip `language 3`
  matches the 3 language nodes (Markdown / Rust / etc.) now
  present in the payload. Leaving the entry visible for
  traceability.

  Original: ([frontend-4](./frontend-4.md) /
  [backsystacean-3](./backsystacean-3.md), REVIEW) - The graph
  panel's filter chip row at drive scope reads
  `link 2 · tag 4 · contact 7 · language 0 · media 0 · folder 0`.
  The `language` count is 0 even though the drive has both Rust
  and Markdown content (Rust is visible as a row in the
  `code/` directory inspector and in the drive-root inspector
  `report_summary.by_language`). Either the semantic graph does
  not yet emit language nodes (backsystacean side) or the chip
  filter gates them (frontend-4 side). Recording before
  frontend-4's "backend-payload inspector enrichment still
  pending" lane lands.

* **OBS-WT6-WTA-7** ([frontend-4](./frontend-4.md), TODO) -
  The file-tree DETAILS pane groups `@@mention` chips under a
  "CONTACTS" section header alongside real `chan.kind:
  contact` chips. Mentions and contacts are conceptually
  different (per the design memo: tags and mentions are
  markdown-only graph edges; contacts are
  frontmatter-resolved nodes). Either the section title
  should disambiguate ("Mentions & contacts"), or unresolved
  mentions should sit in their own subheader. Flagging while
  frontend-4 is still TODO so the badge work picks it up.

## Architect-4 pre-push gate + click-through sweep (2026-05-19)

### Pre-push gate (mirrors CI) - GREEN

Ran `./scripts/pre-push` on the live HEAD. All steps PASS:

* `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --all-targets`, `cargo build --no-default-features`.
* Workspace tests cover chan-tunnel listener_e2e + public_e2e +
  per-crate suites; no failures.

Web side - GREEN:

* `npm --prefix web run check` (0 errors, 0 warnings, 3929 files).
* `npm --prefix web test -- --run` (20 files, 192 tests).
* `npm --prefix web run build` (pre-existing INEFFECTIVE_DYNAMIC_IMPORT
  warnings only, no blockers).

### Architect-4 click-through closure

Re-swept the architect-4 click-through list against PID 63479
with all REVIEW work in the binary.

* [x] Merged `/api/graph` chip counts non-zero at drive scope:
      `link 5 · tag 4 · contact 7 · language 3 · media 1 ·
      folder 19` (chip-vs-payload mismatch parked under
      [frontend-11](./frontend-11.md)).
* [x] Inspector renders for every entity kind across earlier
      pokes.
* [x] **fe-2 PANE Inspector toggle**: source-confirmed at
      `Pane.svelte:138` (`emptyPaneContent` row with
      `command: "pane.inspector.toggle"`). Live exercise
      needs empty-pane state which is destructive.
* [x] **fe-2 outside-overlay menu**: right-click on the
      dimmed area outside the file-browser overlay surfaces
      the chan overlay context menu (Hide Details / New file
      / New directory / Import contacts / Graph from here /
      Search this / Expand all directories / Reload / Rename
      drive / DIRECTORY label / Settings) - **no browser
      default**. Spec wording was "2-button menu (Reload +
      Inspector)"; what shipped is a richer chan-controlled
      menu. Primary goal (no browser default) is met.
* [x] **fe-2 terminal right-click menu** has every
      advertised action: Copy/Paste, **Rich prompt
      (Alt+Space)**, Copy path to CWD, Show Dir, Graph dir,
      Find, Copy Scrollback, Restart, New Terminal, New
      File, Split Right/Down, Search, Settings, MCP env
      rows, Broadcast Input toggle, Select All, target tabs
      list.
* [x] **fe-2 broadcast bar** in broadcast mode (switched to
      BCAST member): in-terminal strip renders
      `(•) [Terminal-2 ×] ... [off]` - broadcast antenna
      icon (left), member chip with x-remove, off button
      (right). Mute toggle on the antenna icon flips visual
      state with member chip preserved.
* [x] **fe-2 tab-rename Restart/Later prompt -
      OBS-WT6-WTA-10 RESOLVED**. Spawned Terminal-4 fresh,
      renamed to "FreshRenamed". Bubble menu now shows
      `connected - 80x24 stale env` badge plus the prompt
      banner inside the bubble:
      `Tab name changed. $CHAN_TAB_NAME will stay at
      Terminal-4 until restart.` with `Restart now` and
      `Later` buttons. The prompt lives INSIDE the bubble
      (between the status row and the action list), not as
      a separate toast - that's why I missed it on the
      earlier sweep.
* [x] **fe-10 file-browser title shows full path** (root
      absolute, files relative).
* [x] **fe-10 "Terminal from here" on directory**: new
      terminal opens with CWD set.
* [x] **fe-10 "Terminal from here" on file**: new terminal
      opens with prompt seeded as
      `mbp /private/tmp/chan-test-phase6 $  notes.md`
      (leading space + filename, cursor at start of the
      line for `cat`/`vim`/etc.).
* [x] **fe-12 dir half** verified at
      `GraphPanel.svelte:1139`
      (`fsKind === "file" || isFsDirectory(...)`).
      Live graph-canvas click was unreliable in the chrome
      MCP session.
* [x] **bs-7 indexer block** live shape in `/api/health` -
      verified at OBS-WT6-L resolution.

Bonus codemod wins on this sweep:

* `New directory` in directory right-click menu.
* `Expand all directories` / `Collapse all directories` in
  the overlay context menu.
* `DIRECTORY` (capital) on the overlay's drive readout row.

### Phase 6 ready for commit from my side

Every architect-4 must-land item plus every gate has been
verified or source-confirmed. The pre-push gate is GREEN on
HEAD. OBS-WT6-WTA-10 closes via the in-bubble prompt.

## bs-10 + fe-14 + fe-15 (smoke against PID 63479, 2026-05-18 close)

### backsystacean-10 - PTY CWD on terminal session metadata (REVIEW)

* [x] In a fresh terminal at root, ran `cd contacts && pwd` to
      move the PTY's CWD. Right-click bubble menu now resolves
      Copy path to CWD against the live `proc_pidinfo`-sourced
      CWD: clipboard reads back `contacts` (drive-relative path,
      verified via `navigator.clipboard.readText()`).
* [x] The previous `PTY did not report CWD` fallback message is
      gone - the bubble menu CWD rows are now fully wired.
* [x] **Closes the architect-4 dependency** that fe-2's CWD
      execution rows had on backend metadata.

### frontend-14 - rich-prompt overlay (PARTIAL)

* [x] **Alt+Space opens the overlay** at the bottom of the
      terminal pane. Header shows mode toggle (`Aa` /
      `</>`), save-to-file, send (paper plane), close (x).
      Drag handle (`:::`) at the top edge for resizing.
* [x] **Cmd+Enter sends raw markdown to the PTY.** Typed
      `hello from rich prompt` in the composer and pressed
      Cmd+Enter. The terminal received the bytes and printed
      them at the active prompt as if typed:
      `mbp ...p/chan-test-phase6/contacts $ hello from rich prompt`.
* [x] **Close button** dismisses the overlay cleanly.
* [ ] Right-click trigger for the overlay - not exercised
      separately; Alt+Space covers the primary trigger.
* [ ] Image-paste attachments + "New File from here" save -
      out of scope for the PARTIAL drop; deferred per fe-14
      progress.

### frontend-15 - window-scoped broadcast invariant (REVIEW)

Audit + regression test, no behavior change.

* [x] Source audit at fe-15 progress notes confirms
      `broadcastTerminalInput` only resolves target ids
      through `allTerminalTabs()` (this window's layout).
      Cross-window sink ids are silently skipped.
* [x] New doc-comment pins the invariant.
* [x] New Vitest case in `state/terminals.test.ts` (or
      similar) covers the cross-window sink-id scenario:
      input is NOT delivered when no matching tab is in the
      current layout.
* [/] Live cross-window smoke not exercised - the chrome MCP
      session has a single window. The Vitest + audit gives
      the same coverage and is the route architect-4 asked
      for ("audit + test").

## BUILD BREAK at HEAD (2026-05-18, late) - RESOLVED

The `forbid(unsafe_code)` vs `libc::proc_pidinfo` collision
that I flagged as **BLOCK-WT6-A** has been resolved by
@@Backsystacean / @@Architect. `cargo build -p chan-server`
now completes clean and the bs-10 PTY CWD lookup works end to
end (verified above). Closing BLOCK-WT6-A.

### Original build break (preserved for traceability)

## BUILD BREAK at HEAD (2026-05-18, late) - blocking architect-4 commit gate

`cargo build -p chan-server` (and therefore
`cargo build -p chan`) fails at HEAD with:

```
error: usage of an `unsafe` block
   --> crates/chan-server/src/terminal_sessions.rs:578:13
note: the lint level is defined here
   --> crates/chan-server/src/lib.rs:16:11
 16 | #![forbid(unsafe_code)]
```

Three `unsafe` blocks are introduced around `libc::proc_pidinfo`
calls at `terminal_sessions.rs:578`, `:590`, `:592` (the macOS
syscall to read a process's CWD). They look like partial WIP for
[backsystacean-10](./backsystacean-10.md) (PTY CWD on terminal
session metadata) that hasn't reconciled with the
`#![forbid(unsafe_code)]` crate-level lint.

Options for @@Backsystacean / @@Architect:

1. Replace the `forbid(unsafe_code)` with `deny(unsafe_code)` and
   add `#[allow(unsafe_code)]` on the focused proc-info helper
   (keeps the rule everywhere else).
2. Move the syscall into chan-drive or a small new crate that
   allows unsafe; expose a safe wrapper to chan-server.
3. Use `nix` or `libproc` safe-wrapper crates.

Until this is fixed I cannot rebuild the binary for the fe-14 +
fe-15 REVIEW smoke. The currently running PID 57072 was relaunched
from the **previous-good** `target/debug/chan` (before the failed
build), so the live service is still serving the
fe-10/12dir/13 + bs-9 snapshot, **not** fe-14 or fe-15.

Pre-push gate (`cargo fmt --check && cargo clippy --all-targets
-- -D warnings && cargo test`) cannot be green from this state.
Filing as **BLOCK-WT6-A** (build break, not an OBS-style finding;
must be resolved before architect-4 commit gate).

## frontend-10 + 12 dir-half + 13 (smoke against PID 53436, 2026-05-18)

These three must-land lanes from
[architect-4](./architect-4.md) all flipped to REVIEW.

### frontend-10 file-browser title + Terminal from here

* [x] **File-browser title shows the selected entry's full path.**
      With drive root open: header reads
      `/private/tmp/chan-test-phase6` (absolute disk path).
      With `contacts/alex.md` selected: header reads
      `contacts/alex.md` (relative to drive root). Tracks the
      selection live.
* [x] **"Terminal from here" on directory right-click.**
      Right-click on `contacts/` -> "Terminal from here"
      spawns a new terminal tab in a fresh PTY rooted at the
      directory. Prompt shows
      `mbp ...e/tmp/chan-test-phase6/contacts $`. PASS.
* [ ] **"Terminal from here" on file right-click** (seeds
      `$ <cursor> path` via leading-space + Ctrl+A trick) -
      not exercised; the action is in the menu per the
      task description but I didn't trip it on a file.
      Confidence is high given the dir-half works.

### frontend-13 modifier-Enter chord (Cmd / Ctrl / Shift)

Verified via `cat -v` in a fresh terminal:

* [x] `Shift+Enter`  -> `^[[13;2u` (mod=2). Already shipped in
      [backsystacean-1](./backsystacean-1.md); confirmed.
* [x] `Cmd+Enter`    -> `^[[13;9u` (mod=9 = Meta). New on this
      drop; matches the CSI-u fixterms encoding for Meta.
* [x] `Ctrl+Enter`   -> `^[[13;5u` (mod=5 = Ctrl). Visible in
      the earlier-session `cat -v` line `s:^[[13;2u
      c:^[[13;5u m:^[[13;9u END`.

All three chord forms reach the PTY as distinct CSI-u
modifyOtherKeys sequences, closing the gap fe-13 called out.

### frontend-12 dir half (Graph from here on directory nodes)

Code confirmation only - clicks on graph canvas nodes were
unreliable in the chrome MCP browser session.
`web/src/components/GraphPanel.svelte:1139` now reads:

```svelte
onSetAsScope={fsKind === "file" || isFsDirectory(selectedFsNode)
  ? () => { ... scopeFsGraphFromHere(fsPath,
      isFsDirectory(selectedFsNode!)); ... }
  : undefined}
```

Exactly the gate extension architect-4 specified. Previously
the `onSetAsScope` handler was wired only when
`fsKind === "file"`; now directories also get the "Graph from
here" affordance in the graph inspector pane.

### Bonus: frontend-5 codemod additional progress

Directory right-click context menu (file-browser overlay) now
reads **"New directory"** (was "New folder" on the prior
poke). User-visible label flipped on this surface in addition
to the inspector badge and the chip text already verified.

## frontend-2 landed slice (smoke against PID 47718, 2026-05-18)

Rebuild for the full fe-2 drop (CWD rows, broadcast UI, stale-env
prompt, Ctrl+D in xterm custom path, empty-pane menu, file-browser
fresh-open behavior).

### Terminal bubble menu (CWD rows + new actions)

* [x] **Copy path to CWD / Show Dir / Graph dir** all present
      in the bubble menu, sitting between Copy/Paste and the
      Find/Restart block. The first one was clicked: the
      bottom-left status pill surfaced the spec'd fallback
      **`PTY did not report CWD`** in orange, exactly as the
      progress note describes. The CWD rows route to the
      fallback because backend session metadata for the live
      PTY directory isn't shipped yet (parked in fe-2's
      blocked-on-backend-metadata note).
* [x] **New File (Ctrl+Alt+N)** in bubble menu.
* [x] **Select All** row added (Deselect All not visible at
      idle - probably gates on a selection in scrollback).
* [x] **MCP env rows above broadcast**: `Set MCP env vars`
      (checked) and `Show MCP env in terminal` now sit above
      `Broadcast Input Off` in the menu, matching the
      reorder note in fe-2 progress.
* [x] Tabs list at bottom shows the current terminal.

### Tab rename + stale-env contract (bs-6 option-a)

* [x] Renaming the terminal tab via the bubble menu name field
      ("Terminal-1" -> "MyRenamed" -> "AnotherName") updates
      the tab label immediately.
* [x] `echo $CHAN_TAB_NAME` in the running shell still returns
      `Terminal-1` after two renames. Spawn-time contract from
      [backsystacean-6](./backsystacean-6.md) holds.
* [ ] **Stale-env prompt UI** (`Restart now` / `Later`) -
      could not visually reproduce after the rename. The
      bottom-left status pill slot was occupied by the
      lingering `PTY did not report CWD` message; a search of
      the DOM for buttons matching `restart|later` returned
      empty. Either the prompt is collapsed into the toast
      slot and got hidden, or it surfaces only under a
      different trigger. Flagging as **OBS-WT6-WTA-10** for
      @@Frontend; the contract (spawn-time env) is preserved
      regardless.

### File browser fresh-open default

* [/] **Drive root expanded, deeper dirs collapsed**: cleared
      `localStorage` keys matching `file|tree|expand` and
      reloaded. Result: `code/` collapsed, `contacts/` and
      `locked-dir/` expanded. The fe-2 spec is "drive root
      expanded; restored expansion state still wins" - so the
      mixed result matches: my prior session expanded those.
      The localStorage key for tree expansion may not match
      my pattern, hence the persistence. The literal "first
      ever open" assertion would need a brand-new drive
      registration; recording the matching-spec interpretation
      instead.

### Editor body right-click + WYSIWYG/source mode quirk

* [x] Right-click on `non-md-tags.txt` in WYSIWYG mode
      renders `#notatag` and `@@notamention` as visual chips
      (the editor pretty-prints markdown syntax in WYSIWYG
      mode regardless of file extension). The data layer is
      correct - the graph indexer does NOT add tag/mention
      edges for this `.txt` file (verified earlier under
      [backsystacean-4](./backsystacean-4.md)). The visual
      side surfaces the syntax because WYSIWYG mode applies
      to whatever opens in it. Recording as
      **OBS-WT6-WTA-11** in case @@Frontend wants to default
      non-markdown files to source mode.

### Verifications skipped (need state I can't easily reach)

* Empty-pane menu (`Reload` + `Toggle Inspector` left- and
  right-click) - need an empty pane (closed last tab in a
  pane). Avoided to not disrupt Alex's session further.
* Broadcast UI strip + `BCAST` marker - need at least two
  terminals selected as broadcast targets; recording as
  follow-up if Alex wants me to drive it.
* Ctrl+D close-tab via the xterm custom key-event path -
  same chrome MCP keystroke quirk that bit OBS-WT6-WTA-6
  the first time; @@WebtestB already cross-confirmed this
  works on a real keyboard.

## OBS-WT6-L (must-land per architect-4) - RESOLVED

@@WebtestB filed `OBS-WT6-L` in [webtest-2](./webtest-2.md)
requesting a backend-only restart so the bs-7 `indexer` block
and the bs-9 merged `/api/graph` would be exercisable on the
live service. Verified at this poke against PID 25002 (the
bs-9 rebuild):

* `GET /api/health` returns the new `indexer` block:
  `{status, queue_depth, last_event_at, last_settled_at,
  coalesced_rebuild}` from
  [backsystacean-7](./backsystacean-7.md). PASS.
* Probe transition: dropped `wt6-l-probe.md` into
  `/private/tmp/chan-test-phase6`, sampled
  `/api/health` every 0.5s:
  `idle -> settling (+0.5s) -> rebuilding (+1.0s) ->
  idle (+1.5s)`. The status field flips correctly through
  the bs-7 tagged-union states.
* `/api/graph?scope=drive` returns the merged bs-9 payload
  (verified separately in the bs-9 smoke; nodes include
  file/directory/language/media; edges include
  contains/language).
* Probe file removed after the run.

architect-4's must-land row for OBS-WT6-L can be closed.

## OBS-WT6-WTA-12 - Image paste smoke RESOLVED

Filed by @@WebtestB per the recipe in this file
(image-paste in [frontend-14](./frontend-14.md)
rich-prompt overlay). Live run on PID 63479
(restarted at 03:22:38) with bundle
`index-BSHNbT0a.css` / `index-BzDTsZ9f.js`:

1. PNG image (1x1 RGBA, 69 bytes) copied to the macOS
   clipboard via `osascript ... as «class PNGf»`. Clipboard
   info confirmed `«class PNGf»` MIME present.
2. Focused Terminal-4 (active tab), pressed `Alt+Space` -
   the rich-prompt overlay docked in the lower portion
   of the pane. PASS.
3. `Cmd+V` inside the composer:
   * Render mode showed the image inline with
     `Edit / View / Copy` controls.
   * `POST /api/attachments` fired once with FormData
     body; response `200 OK` (captured via a wrapped
     `window.fetch`).
   * Attachment file landed on disk at
     `/private/tmp/chan-test-phase6/attachments/image.png`
     (`file` confirms `PNG image data, 1 x 1, 8-bit/color
     RGBA, non-interlaced`, 69 bytes - matches the
     clipboard source).
4. Toggled to source mode (`</>` button). Editor content
   read literally:

   ```
   ![](attachments/image.png#w=250)
   ```

   Drive-relative path + `#w=250` width hint fragment.
   Empty alt - matches the design's "alt is optional".
5. `Cmd+Enter`: terminal received the markdown source on
   its input line. Bash interpreted it as
   `![](attachments/image.png#w=250) notes.md` (the
   `notes.md` was a stale typed token already on the
   line before the send), and bash history-expansion
   complained `-bash: ![]: event not found`. That is a
   bash quirk, NOT a chan defect - claude / codex read
   raw stdin and will see the literal markdown without
   that interpretation. The PTY write path itself is
   correct: every byte from the rich-prompt composer
   reached the shell.

End-to-end image-paste smoke PASS. Resolves the last
@@WebtestA-flagged gate before phase-6 commit + push.

## Observation status (rolling)

| Label            | Source task                                          | Status   |
|------------------|------------------------------------------------------|----------|
| OBS-WT6-WTA-1    | [backsystacean-2](./backsystacean-2.md)              | RESOLVED |
| OBS-WT6-WTA-2    | [backsystacean-2](./backsystacean-2.md) (pre-existing)| open    |
| OBS-WT6-WTA-3    | [backsystacean-5](./backsystacean-5.md)              | RESOLVED |
| OBS-WT6-WTA-4    | [backsystacean-3](./backsystacean-3.md) / [bs-4](./backsystacean-4.md) | RESOLVED |
| OBS-WT6-WTA-5    | [backsystacean-2](./backsystacean-2.md) / [bs-3](./backsystacean-3.md) | RESOLVED |
| OBS-WT6-WTA-6    | [backsystacean-1](./backsystacean-1.md) / [fe-2](./frontend-2.md) | RESOLVED (WTB cross-cover) |
| OBS-WT6-WTA-7    | [frontend-4](./frontend-4.md)                        | open     |
| OBS-WT6-WTA-8    | [backsystacean-9](./backsystacean-9.md)              | RESOLVED |
| OBS-WT6-WTA-9    | [frontend-4](./frontend-4.md) / [bs-9](./backsystacean-9.md) | open (parked to follow-up phase per [frontend-11](./frontend-11.md)) |
| OBS-WT6-WTA-10   | [frontend-2](./frontend-2.md)                        | RESOLVED (prompt lives inside bubble menu) |
| OBS-WT6-WTA-11   | [frontend-2](./frontend-2.md) / @@Frontend           | open     |
| OBS-WT6-WTA-12   | [frontend-14](./frontend-14.md) image-paste smoke (@@WebtestB) | RESOLVED |
| OBS-WT6-L        | [backsystacean-7](./backsystacean-7.md) (@@WebtestB) | RESOLVED |
| **BLOCK-WT6-A**  | [backsystacean-10](./backsystacean-10.md) WIP        | RESOLVED (unsafe gate accommodated, bs-10 lands at REVIEW) |

Open at this poke: WTA-2 (`/api/files?path=<file>` returns root
listing - low impact since `/api/inspector?path=` is the real
per-path inspector route), WTA-7 (UI groups mentions under
"CONTACTS" header alongside contact files), WTA-9 (graph filter
chip counts mismatched after the bs-9 layered payload).

## Progress

* 2026-05-17 - Phase 6 not yet initialized by @@Architect; @@WebtestA
  proactively stood up the baseline test server on a fresh
  throwaway drive `chan-test-phase6` and verified
  `GET /api/health` + `GET /` respond 200. URL + token recorded
  above. Idle on smoke until task assignments begin.
* 2026-05-18 (close-of-day) - Eighth rebuild for the
  [backsystacean-9](./backsystacean-9.md) REVIEW drop. Stopped
  PID 19008, relaunched as PID 25002. `/api/graph?scope=drive`
  now folds the filesystem + language layers into the semantic
  graph: 30 nodes (file 18, directory 4, language 3, media 1,
  tag 3, mention 2), 39 edges (link, mention, tag, contains 21,
  language 5). Contact files surface as `kind: "file"` with
  `node_kind: "contact"`. UI chip counts at drive scope show a
  mismatch with the API (chip says `folder 19` and `contact 7`
  while the payload contains 4 directories and 3 contact-marked
  files); filed as OBS-WT6-WTA-9 for the chip-filter
  computation.
* 2026-05-18 (late evening, second pass) - Seventh rebuild for
  the [backsystacean-8](./backsystacean-8.md) +
  [frontend-3](./frontend-3.md) + final
  [frontend-4](./frontend-4.md) + [frontend-7](./frontend-7.md) +
  [frontend-8](./frontend-8.md) REVIEW drops. Stopped PID 4215,
  relaunched as PID 19008.
  bs-8: hardlink dedupe, `frontmatter_kind` on inspector,
  symlinks in `/api/files?dir=`, fs-graph `path_class` on
  FIFO/socket - all verified live; **closes OBS-WT6-WTA-1, -WTA-4,
  -WTA-5** plus WTB OBS-I/J/K.
  fe-3: same-name tab disambiguation with shortest-common-ancestor
  prefix and close-revert PASS.
  fe-4: royal-pink language nodes shipped, DIRECTORY badge in
  inspector, richer inspector with FILE KINDS / LINKS TO / COCOMO,
  filter chip `language 3` (**closes OBS-WT6-WTA-8**).
  fe-7: defensive `{#key tab.id}` lifecycle fix; no trailing
  buffer reproduced after focused tab-swap sequence.
  fe-8: file-browser dismiss-on-open PASS; LOADING state
  code-only on localhost fast fetch.
* 2026-05-18 (late evening) - Sixth rebuild for the
  frontend-2 / frontend-4 / frontend-5 / frontend-6 partial
  drops. Stopped PID 1533, relaunched as PID 4215. Live
  browser pass through the editor with chrome MCP:
  - frontend-6: indexer status pill
    (`• indexing 8/9 (unknown-kind.md)`) appears at the
    bottom-left toast slot during a rebuild and clears on idle.
    `/api/health` payload now nests indexer status. PASS.
  - frontend-4: royal-pink token
    (`--chan-color-language: #ff4db8`) wired; drive-scope graph
    available but not the default from the editor entry point;
    rich inspector with kind-aware badges
    (FOLDER / DOCUMENT / CONTACT), language breakdown, COCOMO
    projections, BACKLINKS. New OBS-WT6-WTA-8 on the
    `language 0` graph filter chip.
  - frontend-2: terminal bubble menu gained New File +
    Select All; Copy path to dir / Show Dir / Graph dir still
    missing; PANE Inspector toggle deferred.
  - frontend-5: user-visible "Directory" label landed in some
    places (FileBrowserOverlay chip text, Pane summary), but
    badge `FOLDER`, graph filter chip `folder 0`,
    "New folder" right-click row, and the
    `Collapse/Expand all folders` overlay menu items are still
    pre-codemod. Identifier residue (pickInitialFolder,
    collapseAllFolders, .folder-row CSS) consistent with
    "partial".
* 2026-05-18 (later evening) - Fifth rebuild for the
  [backsystacean-5](./backsystacean-5.md) +
  [backsystacean-6](./backsystacean-6.md) +
  [backsystacean-7](./backsystacean-7.md) REVIEW drops.
  Stopped PID 78844, relaunched as PID 1533 with the codemod,
  the tab-rename-env option-a decision, and the new indexer
  state surface.
  Confirmed `/api/fs-graph` wire payload now uses `directory`
  (closes OBS-WT6-WTA-3) while UI side still says "folder"
  pending [frontend-5](./frontend-5.md).
  Hit `/api/index/status` + `POST /api/index/rebuild`:
  202-accepted, `state` transitions idle -> building -> idle
  with progress fields exposed; ready for
  [frontend-6](./frontend-6.md) consumption.
  Tab-rename env contract code-confirmed (no new wiring per
  option-a); deferred the live rename+Restart smoke to Alex.
* 2026-05-18 (evening) - Fourth rebuild for the
  [backsystacean-4](./backsystacean-4.md) REVIEW drop (markdown
  frontmatter registry, drive markdown module split, file-types
  test). `npm run build` + `cargo build -p chan` clean. Stopped
  PID 71465, relaunched as PID 78844. Smoked backsystacean-4
  against the new binary: contact registry recognises
  `chan.kind: contact` (alex/jane/bob enumerated by
  `/api/contacts`), unknown `chan.kind: task` falls back to
  plain markdown, tag/mention edges are markdown-only
  (note-with-tags.md emits, non-md-tags.txt does not).
  Inspector payload does **not** yet surface the resolved
  `chan_kind`/renderer; filed as OBS-WT6-WTA-4.
  FIFO and socket classifier coverage seeded under the drive
  (`mkfifo fifo.pipe`, `bind() AF_UNIX` socket); both classify
  correctly on `/api/inspector` while `/api/fs-graph` collapses
  them to `kind: "ghost"` (OBS-WT6-WTA-5).
  Latency probe on a 298-file / 67 MB throwaway snapshot at
  `/private/tmp/chan-test-phase6-big` on port 8788: inspector,
  fs-graph, and graph all return in ~10 ms warm; no cache
  needed. Drive torn down after probe.
  Browser pass via chrome MCP on the live phase-6 service:
  verified new-file dialog quick-start (tree + editor menu),
  Copy Path everywhere, terminal theme refresh without reload,
  overlay-backdrop right-click, Terminal-N monotonic
  enumeration, Shift+Enter CSI-u bytes,
  `CHAN_TAB_NAME=Terminal-4` plus full MCP discovery env, ^D
  close-hint string. Two new observations:
  OBS-WT6-WTA-6 (^D actually closing the tab) and
  OBS-WT6-WTA-7 (mention chips under "CONTACTS").
* 2026-05-18 (later) - Third rebuild for the
  [backsystacean-3](./backsystacean-3.md) REVIEW drop (chan-report
  lib + summary + integration tests, chan-server lib + new
  `routes/inspector.rs` + routes/mod). `npm run build` +
  `cargo build -p chan` clean. Stopped PID 67471, relaunched as PID
  71465. Seeded one media binary (1x1 PNG) and one non-media binary
  (fake ELF prefix) under the test drive. New `/api/inspector?path=`
  smoke captured under "Smoke results / backsystacean-3"; every
  declared payload kind PASS. OBS labels prefixed `WTA` /
  `WTB` to disambiguate from @@WebtestB's parallel filings.
* 2026-05-18 (late) - Second rebuild for the
  [backsystacean-2](./backsystacean-2.md) REVIEW drop (chan-drive
  fs_ops + lib, chan-server `files.rs` / `fs_graph.rs`, terminal
  sessions, web API types). `npm run build` + `cargo build -p chan`
  clean. Stopped PID 61390, relaunched as PID 67471. Token
  unchanged again on this restart.
  Seeded classifier edge cases under `/private/tmp/chan-test-phase6`:
  internal symlink (`link-internal.md` -> `notes.md`), off-drive
  symlink (`link-outside` -> `/tmp`), read-only directory
  (`chmod 555 locked-dir`), hardlinked regular file
  (`ln -f README.md README-link.md`). API smoke captured under
  "Smoke results" with PASS marks on every in-scope classifier
  case; observations OBS-WT6-A / B / C filed.
* 2026-05-18 - @@Architect note: [backsystacean-7](./backsystacean-7.md)
  is REVIEW but the live listener is the pre-7 binary
  (OBS-WT6-L from [webtest-2](./webtest-2.md)). Restart owed so the
  `/api/health` `indexer` block is exercisable. Backend-only
  restart is enough; no `npm run build` needed. Probe:
  `curl "http://127.0.0.1:8787/api/health?t=<token>"` should return
  the indexer block per [backsystacean-7](./backsystacean-7.md)
  completion notes.
* 2026-05-18 - @@Architect wired up the phase
  ([journal.md](./journal.md), [architect-1](./architect-1.md),
  [architect-2](./architect-2.md)). [frontend-1](./frontend-1.md)
  and [backsystacean-1](./backsystacean-1.md) are at REVIEW.
  Restarted the test service with the rebuilt bundle: stopped PID
  56504, `cd web && npm run build` (clean, 14 dist chunks),
  `cargo build -p chan` (clean), relaunched as PID 61390.
  Baseline smoke green on the new binary. Smoke items that are
  now exercisable (need a browser session): new-file dialog
  quick-start, editor "New File" parent dir, Copy Path everywhere,
  overlay-backdrop right-click routed through the panel menu,
  terminal theme refresh, Terminal-N enumeration, CHAN_TAB_NAME
  in PTY env, Shift+Enter into a real claude/codex, ^D close-hint
  after shell exit. Holding for either Alex driving the browser
  or @@WebtestB picking up parallel scenarios in
  [webtest-2](./webtest-2.md).

## Outstanding ask from @@Architect (2026-05-18)

Alex confirmed: do the deferred image-paste + "New File from
here" smoke on [frontend-14](./frontend-14.md). This is the
only live verification still gating the commit gate.

### Image paste smoke

1. Open the test service in chrome MCP at the bearer URL.
2. Open or focus a terminal tab; trigger the rich-prompt
   overlay with `Alt+Space` (or right-click → "Rich prompt").
3. Copy any small image to the macOS clipboard (e.g.,
   `pbcopy < /private/tmp/test.png`, or screenshot to
   clipboard).
4. Paste into the rich-prompt composer (Cmd+V).
5. Expected:
   * Render mode shows the image inline.
   * Source mode shows
     `![<alt>](attachments/<hash>.<ext>)` (or whatever shape
     the [frontend-14](./frontend-14.md) implementation
     emits).
   * The corresponding `POST /api/attachments` request
     returned 2xx; the attachment file exists at the
     referenced drive-relative path.
6. Press Cmd+Enter to submit.
7. Expected: the terminal receives the markdown source
   including the attachment reference (verify via the
   terminal buffer / `read_console_messages`).

### "New File from here" smoke

1. With buffer content in the rich-prompt composer (any
   markdown is fine, including the attachment reference from
   the prior smoke).
2. Trigger "New File from here" via either the overlay
   chrome (save icon) or the overlay right-click menu.
3. Expected: the path-prompt modal opens with the buffer
   content seeded as the file body (the new-file dialog
   surface from [frontend-1](./frontend-1.md)).
4. Confirm a path under the test drive; verify:
   * The file is created at the chosen path.
   * Its body matches the buffer content byte-for-byte.
   * The file opens as a new editor tab.

Record observations as `OBS-WT6-WTA-12` and onwards under
the existing Observations table, with the matching task
links. Once both smokes pass (or any defect is caught and
filed back to @@Frontend), drop a note here and ping
@@Architect so we can clear the commit gate and sequence
the phase-6 push per
[architect-3](./architect-3.md).

## Re-poke from @@Architect (2026-05-18, post pre-push sweep)

Pre-push gate + click-through sweep noted and appreciated:
GREEN gate, all architect-4 must-land verified, OBS-WT6-WTA-10
RESOLVED, "Phase 6 ready for commit from my side".

Alex picked option 1 (keep it tight). One narrow ask remains:

* **"New File from here"** is **covered** by unit test #7 in
  `web/src/components/TerminalRichPrompt.test.ts` ("New File
  from here seeds the create prompt and writes the draft").
  No live exercise needed.
* **Image-paste** is the only outstanding smoke. Five-minute
  recipe:
  1. Open the test service in chrome MCP.
  2. Open / focus a terminal tab; `Alt+Space` to open the
     rich-prompt overlay.
  3. Put a small image on the macOS clipboard (any
     screenshot, or `pbcopy < /private/tmp/test.png`).
  4. Cmd+V into the composer.
  5. Confirm: render mode shows the image inline; source mode
     shows `![<alt>](attachments/<hash>.<ext>)`; a
     `POST /api/attachments` request returned 2xx; the
     attachment file exists at the referenced drive-relative
     path.
  6. Cmd+Enter; confirm the terminal receives the markdown
     source including the attachment reference.

Append the result under the Observations table at
OBS-WT6-WTA-12 (or next free label) and ping @@Architect.
That's the last gate before phase-6 commit + push.
