# Chan Pre-Release Phase 6 Summary

Status: **FINAL** — closed by @@Architect on Alex's "yes" to wrap
after the full must-land queue cleared and the live test service
verified end-to-end.

Source request: [request.md](./request.md). Process:
[process.md](./process.md). Journal: [journal.md](./journal.md).
Commit-grouping plan: [architect-3](./architect-3.md). Wrap plan +
parking decisions: [architect-4](./architect-4.md). Phase-7
process scaffold (new append-only per-author tree):
[../chan-pre-release-phase-7/process.md](../chan-pre-release-phase-7/process.md).

## Outcome and completion status

Phase 6 closed cleanly. Workspace is now at **0.10.0** with the
embedded chan binary inside Chan.app matching. Every architectural
cleanup item plus every bug / nit Alex named in
[request.md](./request.md) is landed and live-verified. Three
mid-phase additions parked to **phase 6.1**, explicitly carried
in the manifest below.

### Request checklist outcome

Architectural cleanups:

* Filesystem as the primary graph layer:
  [backsystacean-2](./backsystacean-2.md) classifier +
  [backsystacean-3](./backsystacean-3.md) inspector +
  [backsystacean-9](./backsystacean-9.md) merged `/api/graph` +
  [frontend-4](./frontend-4.md) wiring. Default scope is drive
  across every entry point; "Graph this" renames to "Graph from
  here" everywhere. Live chip counts on the seeded drive:
  `link 5 . tag 4 . contact 7 . language 3 . media 1 . folder 4`.
* File classifier (sym/hard/FIFO/socket/device + read-only):
  [backsystacean-2](./backsystacean-2.md). Read-only directories
  are graph dead-ends; symlinks pointing outside the drive
  render but do not traverse.
* Markdown layer + frontmatter kind ladder:
  [backsystacean-4](./backsystacean-4.md). Canonical YAML shape
  is the nested `chan:` map; `contact` is the only entry. Tag
  and mention edges are markdown-only (pinned by test +
  design.md).
* Text + binary inspector + chan-report rollups:
  [backsystacean-3](./backsystacean-3.md) +
  [frontend-4](./frontend-4.md).
* Language binds to directory + drive breakdown:
  [backsystacean-3](./backsystacean-3.md) inspector +
  [backsystacean-9](./backsystacean-9.md) graph +
  [frontend-4](./frontend-4.md) chip set.
* Royal-pink color slot:
  `--chan-color-language` / `--chan-color-code`,
  `#C71585` light / `#FF4DB8` dark, wired through
  `--g-language`. Locked by Alex 2026-05-18.
* Terminology codemod folder -> directory:
  [backsystacean-5](./backsystacean-5.md) crates-side complete;
  [frontend-5](./frontend-5.md) shipped user-visible copy + wire
  vocabulary. Broad compat-sensitive identifier sweep parks to
  6.1.
* Ghost-node indexer-progress:
  [backsystacean-7](./backsystacean-7.md) `/api/health` indexer
  block + [frontend-6](./frontend-6.md) live 1s poll while the
  ghost inspector is open.

Bugs and nits:

* New-file dialog quick-start +
  editor "New File" parent directory +
  Copy Path everywhere +
  terminal theme refresh: [frontend-1](./frontend-1.md).
* Terminal-N enumeration + Shift+Enter +
  Ctrl+D close-hint + CHAN_TAB_NAME at spawn:
  [backsystacean-1](./backsystacean-1.md).
* Same-name tab disambiguation
  (shortest divergent segment, deep tails collapse as
  `x/[...]/foo.md`, full path on hover):
  [frontend-3](./frontend-3.md).
* PANE Inspector toggle + outside-overlay menu +
  terminal right-click expansion +
  collapsed file browser on first open +
  tab-rename stale-env prompt + bubble menu reorder +
  broadcast peer-group with mute icon + member `[x]` +
  Select All + tab-strip BCAST marker:
  [frontend-2](./frontend-2.md).
* Tab-rename to env (spawn-time-only contract; Restart prompt):
  [backsystacean-6](./backsystacean-6.md) memo
  (Alex picked option a) + [frontend-2](./frontend-2.md) UI.
* Modifier-Enter chord gap
  (Ctrl+Enter `\x1b[13;5u`, Cmd/Meta+Enter `\x1b[13;9u`):
  [frontend-13](./frontend-13.md).

Mid-phase additions (each tracked in the journal Extended
Requests table):

* File-browser dismiss-then-load + graph "Open in this pane"
  pattern: [frontend-8](./frontend-8.md).
* WYSIWYG trailing-buffer investigation
  (defensive `{#key tab.id}`; not reproducible):
  [frontend-7](./frontend-7.md).
* Window-scoped broadcast invariant (audit + Vitest):
  [frontend-15](./frontend-15.md).
* File-browser title shows selected entry's full path +
  "Terminal from here" (dir = CWD; file = parent + prompt
  seed via leading-space + Ctrl+A): [frontend-10](./frontend-10.md).
* Directory "Graph from here" in graph inspector:
  [frontend-12](./frontend-12.md) dir half.
* PTY CWD on terminal session metadata
  (Linux `/proc/<pid>/cwd`, macOS `/usr/sbin/lsof` subprocess
  to honor `#![forbid(unsafe_code)]`):
  [backsystacean-10](./backsystacean-10.md).
* Inspector inode dedupe + `frontmatter_kind` field +
  `Drive::list` symlink visibility +
  fs-graph special-file `path_class`:
  [backsystacean-8](./backsystacean-8.md) (Webtest defect
  bundle, 5 OBS items).
* Rich-prompt overlay per terminal (markdown composer,
  Alt+Space / right-click, Cmd+Enter to PTY, image paste via
  `/api/attachments`, "New File from here" save,
  per-window session persistence):
  [frontend-14](./frontend-14.md).

## Highlights

* The headline architectural ask landed end-to-end. Alex spotted
  the gap live ("language 0 / folder 0" chip counts on the
  seeded drive) and named the producer fix. Server merge in
  [backsystacean-9](./backsystacean-9.md) was filed, contract-
  reviewed, and shipped inside one work session. Final chip
  counts on the test drive (`link 5 . tag 4 . contact 7 .
  language 3 . media 1 . folder 4`) match the underlying
  payload.
* Self-dispatch worked again: @@Frontend and @@Backsystacean
  opened wave-1 task files before @@Architect finished
  orientation, and @@WebtestA had the live test service running
  before the journal was written. Reconciliation took one
  cycle; no rework.
* @@WebtestB's parallel rounds turned up five real defects
  (hardlink double-count, missing `frontmatter_kind`,
  asymmetric frontmatter shape, missing symlinks in
  `/api/files`, fs-graph special-file collapse) that
  @@Backsystacean folded into a single [backsystacean-8](./backsystacean-8.md)
  fix bundle inside one work session. Contract review PASS on
  every item.
* @@WebtestA caught **BLOCK-WT6-A** (the `unsafe` block /
  `#![forbid(unsafe_code)]` collision in
  [backsystacean-10](./backsystacean-10.md) WIP) before the
  build break propagated. @@Backsystacean pivoted to a
  subprocess-based `lsof` lookup on macOS within the same
  session, keeping the chan-server unsafe gate intact.
* The decision pattern from phase 5 carried forward: @@Architect
  recorded best-read decisions in [architect-2](./architect-2.md)
  so implementation tracks could move; Alex confirmed or
  corrected each in turn (royal-pink hex, "Graph from here"
  wording, option-a tab-rename contract, etc.). One correction
  (canonical frontmatter shape is the **nested** `chan:` map,
  not the flat shorthand the memo showed) caught early by
  @@WebtestA reading source.
* Rich-prompt overlay ([frontend-14](./frontend-14.md)) landed
  inside the wrap window: per-terminal markdown composer,
  Cmd+Enter to PTY verified byte-for-byte live, image paste
  end-to-end live (PNG -> `/api/attachments` 2xx -> drive
  attachment file -> source mode reads the markdown reference
  -> Cmd+Enter writes the markdown to PTY).
* `summary.md` mention of process: phase-7 stood up the new
  per-author directory tree + append-only journal rule in
  parallel with the phase-6 wrap, so the next phase starts on
  the new format.

## Lowlights

* The agent who picked up [backsystacean-10](./backsystacean-10.md)
  pushed a WIP state that introduced `unsafe` blocks before the
  build was checked. Build broke for a transient window. Caught
  fast by @@WebtestA but the gap is real: `cargo build` (or at
  least `cargo check`) should run before HEAD writes when a
  crate-wide lint forbids the path the agent is exploring.
* Two early frontmatter docs in [architect-2](./architect-2.md)
  showed the flat `chan.kind: contact` shorthand instead of the
  registry's nested `chan: { kind: ... }` shape. @@WebtestA's
  fixture had to be rewritten and the memo corrected. Should
  have been caught by reading the parser before drafting the
  memo, not after.
* Frontend chip counter mismatch (OBS-WT6-WTA-9) on the merged
  graph payload landed visible at REVIEW: underlying data
  correct but chip counts overcount on certain scopes. Parked
  to phase 6.1 ([frontend-11](./frontend-11.md)) because
  underlying data is right; cosmetic gap, not a regression.
* @@Architect orientation lag again at the start, same shape as
  phase 5. @@Frontend, @@Backsystacean, @@WebtestA were already
  in motion before the journal was written. Reconciliation
  ate one cycle and the dispatch table had to absorb in-flight
  state rather than dispatch from a clean baseline.
* The desktop bundle version in
  `desktop/src-tauri/tauri.conf.json` was not part of the
  workspace version-bump file list; first DMG shipped as
  `Chan_0.9.0_aarch64.dmg` despite the embedded chan being
  0.10.0. Caught after build; tauri.conf.json was bumped
  and the DMG rebuilt. Worth adding to the version-bump
  checklist for next phase.

## Bugs found and fixed

| Label             | Source                                            | Status                          |
|-------------------|---------------------------------------------------|---------------------------------|
| OBS-WT6-WTA-1     | `/api/files` filtered symlinks                    | RESOLVED via [backsystacean-8](./backsystacean-8.md) |
| OBS-WT6-WTA-4     | `/api/inspector` missing `frontmatter_kind`       | RESOLVED via [backsystacean-8](./backsystacean-8.md) |
| OBS-WT6-WTA-5     | fs-graph collapsed FIFO/socket to `kind: ghost`   | RESOLVED via [backsystacean-8](./backsystacean-8.md) |
| OBS-WT6-WTA-6     | Ctrl+D close-hint actionable                      | RESOLVED (chrome MCP keystroke quirk; real keyboard works) |
| OBS-WT6-WTA-8     | Chip counts language=0 on the live merged graph   | RESOLVED via [backsystacean-9](./backsystacean-9.md) |
| OBS-WT6-WTA-10    | Tab-rename stale-env prompt UI                    | RESOLVED (prompt lives inside the bubble menu)       |
| OBS-WT6-WTA-12    | Image-paste end-to-end                            | RESOLVED via [frontend-14](./frontend-14.md) live smoke |
| OBS-WT6-I (WTB)   | Inspector hardlink double-count                   | RESOLVED via [backsystacean-8](./backsystacean-8.md) |
| OBS-WT6-K (WTB)   | Asymmetric frontmatter shape doc                  | RESOLVED via [backsystacean-8](./backsystacean-8.md) (nested is canonical) |
| OBS-WT6-L         | `/api/health` indexer block exercisable           | RESOLVED via @@WebtestA backend-only restart         |
| BLOCK-WT6-A       | `unsafe` block vs `forbid(unsafe_code)`           | RESOLVED via subprocess `lsof` pivot                  |

Still-open observations (carried into phase 6.1 or recorded as
not a phase-6 regression):

* OBS-WT6-WTA-2 — `/api/files?path=<file>` returns the drive
  root listing. Pre-existing; new `/api/inspector?path=` is the
  intended per-path inspector route. No phase-6 regression.
* OBS-WT6-WTA-7 — Mention chips render under the "CONTACTS"
  inspector header alongside contact files. UX nit. Carries
  forward.
* OBS-WT6-WTA-9 — Graph filter chip counter overcounts on the
  merged graph payload (cosmetic; underlying data correct).
  Carries to 6.1 via [frontend-11](./frontend-11.md).
* OBS-WT6-WTA-11 — File editor opens new files in WYSIWYG by
  default; suggestion to default non-markdown to source. UX
  preference. Carries forward.

## Test and hardening coverage

Pre-push gate on the final HEAD (`scripts/pre-push`):

* `cargo fmt --check` ✓.
* `cargo clippy --all-targets -- -D warnings` ✓.
* `cargo build --no-default-features` ✓.
* `cargo test --all-targets` ✓ (includes
  chan-tunnel `listener_e2e`, `public_e2e`, per-crate suites).
* `npm --prefix web run check` ✓ (0 errors, 0 warnings,
  3929 files).
* `npm --prefix web test -- --run` ✓ (20 files, 192 tests).
* `npm --prefix web run build` ✓ (existing Vite chunk-size +
  INEFFECTIVE_DYNAMIC_IMPORT warnings only; no blockers).

Per-task focused verification trail captured in
[webtest-1](./webtest-1.md) and [webtest-2](./webtest-2.md).
Architect contract review PASS on every backend lane
(backsystacean-2/3/4/7/8/9/10) and the corresponding frontend
consumers (frontend-4/6/10/12 dir-half/13/15).

Live smoke against the seeded test drive
(`/private/tmp/chan-test-phase6`):

* Path classifier on the four edge cases
  (internal + off-drive symlink, hardlink pair, read-only
  directory, FIFO, Unix socket).
* `/api/inspector` payload for every declared kind
  (drive / directory / markdown contact + non-contact / text /
  media binary / non-media binary / special) plus path-traversal
  rejection and missing-path 404.
* `/api/graph?scope=drive` chip counts non-zero on the seeded
  drive; layered view renders (orange files + gray directories
  + pink language + green tags + yellow contacts).
* `/api/health` indexer block on a backend-only restart.
* PTY CWD lookup: `cd contacts && pwd` -> right-click "Copy
  path to CWD" -> clipboard reads `contacts`.
* Rich-prompt overlay: Alt+Space opens, Cmd+Enter ships raw
  markdown to the PTY, image paste uploads to
  `/api/attachments` with the file landing on disk, source
  mode reads the markdown reference, Cmd+Enter writes the
  reference to the PTY.

Latency probe on a 298-file / 67 MB throwaway fixture (Webtest
A round 4): `/api/inspector`, `/api/fs-graph`, `/api/graph` all
return in ~10 ms warm. No watcher-cached rollup needed for this
phase.

## Remaining follow-ups (phase 6.1 manifest)

Three deferred items, each with a parent task file in this
directory ready to be copied / re-filed under
`../chan-pre-release-phase-7/` (or a sibling 6.1 directory) when
Alex opens that phase.

| Item                                       | Source task                                         |
|--------------------------------------------|-----------------------------------------------------|
| Broad `folder` -> `directory` identifier codemod (wire-format compat pass: `kind: "folder"` in graph filters, persisted scope keys, canvas internal aliases) | [frontend-5](./frontend-5.md)  |
| Graph filter chip counter overcount on the merged payload (cosmetic)                | [frontend-11](./frontend-11.md) |
| Graph overlay scope breadcrumb (`drive / notes / sub`, each clickable to re-scope) | [frontend-12](./frontend-12.md) breadcrumb half |

Additional carry-forward minor items (recorded but not parked
as named tasks):

* OBS-WT6-WTA-2: `/api/files?path=<file>` returns drive root
  (pre-existing; recommend documenting `/api/inspector?path=`
  as the per-path route or relaxing the behavior).
* OBS-WT6-WTA-7: Mention chips under "CONTACTS" inspector
  header.
* OBS-WT6-WTA-11: File editor opens new files in WYSIWYG by
  default; default non-markdown to source mode.
* Frontmatter kinds beyond `contact` (`chan.note`,
  `chan.task`, etc.) — explicitly Alex's "next phase" decision.
* Live `claude` / `codex` CLI end-to-end verification on
  Cmd+Enter, Shift+Enter, and rich-prompt image attachments
  on a host where those CLIs are installed (carried from
  phase 5).
* PATCH `/api/config` semantics (carried from phase 5).
* Version-bump checklist update: include
  `desktop/src-tauri/tauri.conf.json` next to the workspace
  `Cargo.toml` so the DMG label tracks the workspace
  version automatically.

## Agent rankings and feedback

Phase-6 fit rankings, not absolute ability. Every agent
delivered.

* **@@WebtestA** — top of the phase again. Held the live test
  service across at least four rebuild cycles, browser-drove
  the live UI through chrome MCP, ran a 298-file latency probe
  on a throwaway snapshot, caught BLOCK-WT6-A early enough to
  prevent it propagating into a long broken-build window,
  re-verified OBS-WT6-WTA-10 ("the prompt is inside the
  bubble") to clear an earlier false-negative, and produced
  the pre-push gate + click-through closure sweep that gave
  @@Architect the verdict to start commits. Constructive
  feedback: the canonical frontmatter shape correction
  (nested vs flat) came from reading the parser source
  directly — would have been even higher impact if that read
  had happened during the design-memo round-trip instead of
  after the fixture was set up. The "read the source before
  drafting" pattern is reusable.

* **@@Backsystacean** — highest implementation throughput,
  ten task lanes (backsystacean-1..10) all REVIEW. The
  defining moment was the BLOCK-WT6-A pivot: a WIP that
  introduced `unsafe` blocks for `proc_pidinfo` got flagged
  by @@WebtestA, and within the same session the agent flipped
  to a safe subprocess `lsof` lookup that honored
  `#![forbid(unsafe_code)]` end to end. Constructive
  feedback: the WIP commit broke the build at HEAD, even
  briefly. `cargo build` (or `cargo check`) before HEAD writes
  is cheap insurance when the crate has a load-bearing
  forbid-lint. Add to the per-task self-review default.

* **@@Frontend** — fifteen task lanes (frontend-1..15) with a
  mix of single-session bug-fix bundles, a medium-size feature
  (the rich-prompt overlay), and the architectural close-out.
  Strong instinct for reuse: rich-prompt composer pulled in
  the existing Wysiwyg / Source / StyleToolbar surfaces
  instead of forking; `terminalFromHereTarget` shared between
  the file-tree row menu and the editor tab menu; the
  filesystem-graph "Graph from here" lifted into the shared
  `scopeFsGraphFromHere` helper. Constructive feedback: the
  broadcast bar peer-group rework needed two iterations because
  the original spec (asymmetric source -> targets) didn't match
  the symmetric model Alex was after. Reading the existing
  state model in the first design pass would have saved the
  re-architect; on the other hand, the "lift the model when
  the UX changes" call was the right architectural read.

* **@@WebtestB** — five clean smoke rounds in
  [webtest-2](./webtest-2.md) including the round-2 defect
  cluster that became [backsystacean-8](./backsystacean-8.md).
  Parallel scenarios let @@WebtestA focus on the live service
  and the deeper click-throughs while @@WebtestB chipped
  through API-level probes. Constructive feedback: the
  OBS-WT6 vs OBS-WT6-WTA / -WTB label collision in round 1
  cost one reconciliation cycle. The prefix split landed
  cleanly after the first round; carry the prefix forward
  as the default from phase 7 onward.

* **@@Architect** (me) — orientation lag again. Same shape as
  phase 5: by the time the journal was written, three lanes
  were already in flight. The reconciliation pattern is now
  rehearsed (one cycle), but the cost is repeatable.
  Constructive feedback for phase 7: with per-author
  directories + append-only journals, the first thing
  @@Architect does is post their own `architect/journal.md`
  plan entry **before** wave-1 dispatch, even if it's
  one paragraph. That single append starts the clock on
  the canonical plan and lets other agents pause their
  self-dispatch instinct until the dispatch table exists.
  The phase-6 wave-2 architectural shape calls
  (drive-rooted graph default, option-a tab-rename,
  layered `/api/graph` server merge) all landed cleanly;
  carrying the same "best read first, lock with Alex
  inside one round" pattern forward.

## Final delivery

Local `main` at phase close is **7 commits ahead of
`origin/main`** (6 committed at this point + the upcoming
release wrap commit this file lands in):

```
f1f7c8c chore: bump version to 0.10.0
56075ce release: close phase 6 tasks + open phase 7 scaffold
382d56d web: phase-6 frontend
b41df09 chan-server: inspector + merged graph + indexer state + PTY CWD
85ddf54 chan-report: byte counts + sorted language rollups
50b3159 chan-drive: file classifier + frontmatter kind registry
```

Plus this commit:

```
?           release: phase 6 wrap + 0.10.0 notes
```

Six-commit area-grouped shape (chan-drive / chan-report /
chan-server / web / release-phase6 / chore) per
[architect-3](./architect-3.md), with the wrap commit added on
top by convention.

Workspace + desktop bundle now at **0.10.0**:

* `target/release/chan` — chan 0.10.0 (88 MB release binary).
* `target/release/bundle/macos/Chan.app` — bundled with the
  0.10.0 chan sidecar (hard requirement: Chan.app uses the
  embedded chan, never `$PATH`, per
  `desktop/src-tauri/src/main.rs:557-582`).
* `target/release/bundle/dmg/Chan_0.10.0_aarch64.dmg` — signed
  with the Developer ID, ad-hoc signed bundle, ready for Alex
  to install.

Phase-7 process tree stood up in parallel under
`../chan-pre-release-phase-7/` with per-author directories
and stub journals using the new append-only format.

Push to `origin/main` is the last action; @@Architect holds
on Alex's explicit go.
