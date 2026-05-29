# @@Architect task 1: coordination and dispatch

Owner: @@Architect
Status: IN_PROGRESS

## Goal

Keep [journal.md](./journal.md) current across phase 6's parallel
tracks. Hand wave-1 to the named team, gather review residue into
wave-2 task files, coordinate commit groupings + final push, write
[summary.md](./summary.md) at close.

## Relevant links

* Request: [request.md](./request.md)
* Process: [process.md](./process.md)
* Design memo: [architect-2.md](./architect-2.md)

## Wave-1 dispatch summary

Spawned in parallel; no hard ordering except that
[architect-2](./architect-2.md) lands the contract for the layered
graph + color + permission semantics so the @@Backsystacean and
@@Frontend tracks share the same picture.

* [backsystacean-1](./backsystacean-1.md) file classifier
* [backsystacean-2](./backsystacean-2.md) chan-report aggregation +
  inspector payload
* [backsystacean-3](./backsystacean-3.md) frontmatter kinds + tag/
  mention scope doc
* [backsystacean-4](./backsystacean-4.md) terminology codemod (crates)
* [backsystacean-5](./backsystacean-5.md) terminal ^D + tab rename to
  env
* [frontend-1](./frontend-1.md) bug-fix bundle
* [frontend-2](./frontend-2.md) right-click menus
* [frontend-3](./frontend-3.md) same-name tab disambiguation
* [frontend-4](./frontend-4.md) graph default + inspector + color
* [frontend-5](./frontend-5.md) terminology codemod (web)
* [webtest-1](./webtest-1.md) @@WebtestA live service + smoke
* [webtest-2](./webtest-2.md) @@WebtestB parallel scenarios

## Wave-2 planning (populated as wave 1 closes)

* Aggregate review feedback into per-area tasks.
* Hardening pass: end-to-end click-through across every checklist
  item in [journal.md](./journal.md).
* Commit groupings by area: chan-drive, chan-server, chan, web, docs,
  release.
* Pre-push gate on the final HEAD: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`,
  `cargo build --no-default-features`,
  `cargo test --workspace`,
  `npm --prefix web run check`,
  `npm --prefix web test -- --run`,
  `npm --prefix web run build`.
* Push to `origin/main` at phase close on Alex's go signal.
* [summary.md](./summary.md) with outcomes, highlights, lowlights,
  bugs, coverage, follow-ups, and agent rankings.

## Notes / decisions

* Team named directly in [request.md](./request.md); capacity
  validated without a Q&A round since Alex already specified the
  five-slot shape.
* Decisions in [journal.md](./journal.md) Decisions section are
  @@Architect's best reads. Alex may override at any point; tasks
  encode the current best read so implementation can move.
* @@Backsystacean carries Backend + Syseng + Rustacean review surface
  on its own; flag any task where the surfaces split into separate
  subtleties (route + filesystem + thread / async safety) before
  commit.

## Progress

* 2026-05-18 Wrote [journal.md](./journal.md) and dispatched wave-1
  task files.
* 2026-05-18 Reconciled in-flight self-dispatched work
  ([frontend-1](./frontend-1.md), [backsystacean-1](./backsystacean-1.md),
  [webtest-1](./webtest-1.md)).
* 2026-05-18 Contract review PASS on
  [backsystacean-2](./backsystacean-2.md) (PathClass shape matches
  [architect-2](./architect-2.md)).
* 2026-05-18 Contract review PASS on
  [backsystacean-3](./backsystacean-3.md) (InspectorKind covers
  drive / directory / markdown / text / media / binary / special;
  media + special are natural extensions on top of architect-2).
* 2026-05-18 Contract review PASS on
  [backsystacean-4](./backsystacean-4.md) (CHAN_KIND_REGISTRY with
  contact only + tag/mention markdown-only enforcement + design.md
  doc).
* 2026-05-18 Alex confirmed all open decisions: royal-pink LGTM,
  "Graph from here" across all surfaces, frontmatter kinds defer
  to next phase, ghost-node UX in this phase, lane sequencing on
  @@Architect.
* 2026-05-18 Added [backsystacean-7](./backsystacean-7.md) +
  [frontend-6](./frontend-6.md) for the ghost-node indexer-progress
  UX gap.
* 2026-05-18 Added broadcast-mode indicator + Select All / Deselect
  All picker (including source tab) to
  [frontend-2](./frontend-2.md) and journal checklist.
* 2026-05-18 Drafted commit groupings in
  [architect-3.md](./architect-3.md) (six-commit shape mirroring
  phase 5).
* 2026-05-18 All seven @@Backsystacean lanes (1-7) flipped to
  REVIEW. @@WebtestB ran two parallel rounds in
  [webtest-2](./webtest-2.md) and @@WebtestA ran browser smoke +
  backend probes + a 298-file latency probe in
  [webtest-1](./webtest-1.md); five real defects bundled into
  [backsystacean-8](./backsystacean-8.md): OBS-WT6-I hardlink
  double-count, OBS-WT6-J / WTA-4 missing `frontmatter_kind`,
  OBS-WT6-K canonical frontmatter shape is the **nested** `chan:`
  map (correction; arch-2 memo + design doc fixed),
  OBS-WT6-WTA-1 include symlinks in `/api/files`,
  OBS-WT6-WTA-5 surface `path_class.kind` in fs-graph for
  special files. Block / character device coverage gap accepted
  via chan-drive unit-test coverage.
* 2026-05-18 Frontend lanes 2, 4, 5 are PARTIAL after
  @@Frontend's first pass: tokens + "Graph from here" + PANE
  Inspector toggle + outside-overlay backdrop + visible
  "directory" copy all landed. Remaining: inspector payload
  consumption ([frontend-4](./frontend-4.md)), terminal right-
  click + broadcast picker + bubble reorder + tab-rename prompt
  + file-browser-collapsed ([frontend-2](./frontend-2.md)),
  broad identifier cleanup ([frontend-5](./frontend-5.md)).
* 2026-05-18 OBS-WT6-L still open: @@WebtestA owes a backend-
  only restart so [backsystacean-7](./backsystacean-7.md)'s
  `/api/health` `indexer` block becomes exercisable.
* 2026-05-18 [frontend-3](./frontend-3.md),
  [frontend-7](./frontend-7.md), and [frontend-8](./frontend-8.md)
  all REVIEW: same-name tab disambiguation, WYSIWYG defensive
  `{#key tab.id}` (WYSIWYG surface is CodeMirror 6 not
  Tiptap/ProseMirror, couldn't reproduce; live verification owed),
  and file-browser dismiss-then-load. Only [frontend-2](./frontend-2.md),
  [frontend-4](./frontend-4.md), [frontend-5](./frontend-5.md),
  and [frontend-6](./frontend-6.md) remain non-REVIEW on the
  frontend side. Backend [backsystacean-8](./backsystacean-8.md)
  fix bundle is unclaimed.
* 2026-05-18 New broadcast-bar spec from Alex:
  `[broadcast-icon] [member x] [member x] ... [off]` in the
  freed-up status bar slot; `[x]` per member is peer (any
  participant can remove any other); needs lifting the broadcast
  model from source/target asymmetric to a symmetric group.
  Folded into [frontend-2](./frontend-2.md).
* 2026-05-18 [backsystacean-8](./backsystacean-8.md) REVIEW with
  all five OBS items closed. Contract review PASS by @@Architect:
  `frontmatter_kind: Option<String>` populated via `chan_kind()`
  lookup; dedupe key `HashSet<(dev, ino)>` from
  `symlink_metadata` (presentation-layer only, watcher/indexer/
  search/graph stay path-based per
  [backsystacean-8](./backsystacean-8.md) self-review). All 8
  backsystacean lanes (1-8) are now REVIEW. Frontend pile: lanes
  2/4/5 PARTIAL + lane 6 TODO (unblocked).
* 2026-05-18 [frontend-4](./frontend-4.md) and
  [frontend-6](./frontend-6.md) REVIEW. frontend-4 closes the
  architectural close-out (royal-pink tokens, "Graph from here"
  rename across surfaces, `/api/inspector` consumption across
  file browser / graph / search, classifier badges for
  read-only / symlink / special / hardlink / outside-drive,
  directory subtree counts with tree-derived fallback, chan-
  report COCOMO roll-up preserved). frontend-6 closes the
  ghost-node UX (typed `/api/health` client, 1s poll while ghost
  inspector open, busy hint with `catching up (N pending)` /
  `rebuilding (full pass)`, idle/failure fallback). Only
  [frontend-2](./frontend-2.md) (PARTIAL: broadcast bar group
  model, right-click expansion, collapsed file browser, tab-
  rename prompt) and [frontend-5](./frontend-5.md) (PARTIAL:
  broad identifier cleanup) remain non-REVIEW on the frontend
  side. OBS-WT6-L (backend-only restart) still pending.
* 2026-05-18 Alex spotted the headline architectural gap from
  the live test service: graph overlay chip counts show
  `language 0`, `media 0`, `folder 0` because `/api/graph`
  emits only markdown-centric nodes. `/api/fs-graph` and
  `/api/language-graph` already exist but the overlay reads
  `/api/graph` only. Filed [backsystacean-9](./backsystacean-9.md)
  to fold the layers into `/api/graph` server-side (Alex picked
  option A over a client-side merge). Frontend already supports
  the chip kinds; the producer is what's missing. Architectural
  checklist items "Make the filesystem the primary graph layer"
  and "Language binds to directory in the graph" flipped from
  done back to PARTIAL until backsystacean-9 lands.
* 2026-05-18 Wrap mode locked by Alex.
  [architect-4](./architect-4.md) carries the plan: must-land
  is [frontend-2](./frontend-2.md), [frontend-10](./frontend-10.md),
  [frontend-12](./frontend-12.md) dir-half, and the OBS-WT6-L
  restart. Parked: [frontend-5](./frontend-5.md) broad
  compat-sensitive codemod, [frontend-11](./frontend-11.md) chip
  counter, [frontend-12](./frontend-12.md) breadcrumb. Refreshed
  [architect-3](./architect-3.md) commit groupings to absorb
  backsystacean-6/7/8/9 and frontend-3/4/6/7/8/10/12-dir-half
  into the existing six-commit shape.
* 2026-05-18 (later) Alex added [frontend-13](./frontend-13.md)
  (modifier-Enter chord gap: Cmd+Enter / Ctrl+Enter via CSI-u)
  and [frontend-14](./frontend-14.md) (rich-prompt overlay on
  top of a terminal: markdown composer triggered by Alt+Space
  / right-click, ships raw markdown via Cmd+Enter, image paste
  through `/api/attachments`, "New File from here" save). Both
  added to the must-land queue. Also stood up the phase-7
  process tree (`phase-7/`) with per-author
  directories + stub journals per Alex's new process design.
* 2026-05-18 [frontend-10](./frontend-10.md) REVIEW. Contract
  PASS by @@Architect: shared `terminalFromHereTarget` helper
  with unit tests, backend `cwd=` param on `/api/terminal/ws`
  routes through `resolve_safe_strict` (chan-drive sandbox)
  and is gated to fresh sessions only.
* 2026-05-18 [frontend-12](./frontend-12.md) dir half REVIEW.
  Contract PASS by @@Architect: directory nodes pivot to
  `dir:<path>`, depth resets to 1, selected node pending for
  refresh. Breadcrumb half stays parked to 6.1.
* 2026-05-18 [frontend-13](./frontend-13.md) REVIEW. Contract
  PASS by @@Architect: Ctrl+Enter `\x1b[13;5u`, Cmd/Meta+Enter
  `\x1b[13;9u`, Shift+Enter unchanged, Alt+Enter / Shift+Tab
  left to xterm defaults with regression coverage. Live CLI
  verification owed on the rebuilt test service.
* 2026-05-18 [frontend-2](./frontend-2.md) REVIEW with one
  known gap. Contract PASS by @@Architect for the in-scope
  surface; the CWD-dependent right-click rows (Copy path to
  CWD / Show Dir / Graph dir / CWD-seeded New File) render
  with the fallback `PTY did not report CWD` until
  [backsystacean-10](./backsystacean-10.md) ships. All other
  items closed: broadcast peer-group + mute icon + member
  `[x]` + Select All + bubble reorder + PANE Inspector +
  outside-overlay menu + collapsed FB + tab-rename prompt +
  Ctrl+D close-hint actionable.
* 2026-05-18 [frontend-14](./frontend-14.md) PARTIAL. The
  feature is functionally complete (per-terminal overlay,
  Alt+Space / right-click trigger, Wysiwyg/Source composer
  with StyleToolbar, height-only resize, Esc hide + buffer
  persist, Cmd+Enter raw PTY send, "New File from here", per-
  window session persistence). Test gap: component-level
  Vitests (open/close, submit ordering, resize bounds, per-
  terminal isolation) still owed; session-serialization
  coverage is the only regression test. Live image-paste +
  CLI receive verification owed @@WebtestA.
* 2026-05-18 [frontend-15](./frontend-15.md) REVIEW. Contract
  PASS by @@Architect: `broadcastTerminalInput` resolves
  targets only via `allTerminalTabs()` (current window's
  Svelte layout registry), doc-comment pins the per-window
  invariant warning against sink-id/server-bus fan-out, and
  cross-window sink-id regression test confirms no delivery
  outside the current layout.
* 2026-05-18 (close-out) @@WebtestA cleared bs-10 live
  (right-click "Copy path to CWD" returns the drive-relative
  CWD after `cd contacts && pwd`), fe-14 partial live
  (Alt+Space opens overlay; Cmd+Enter ships raw markdown to
  the PTY verified byte-for-byte), fe-15 audit + Vitest per
  contract, OBS-WT6-L resolved, BLOCK-WT6-A resolved.
  Deferred from the live smoke: fe-14 image-paste + "New
  File from here". Alex confirmed: do the deferred smoke
  before commit gate. Recipe filed at the bottom of
  [webtest-1](./webtest-1.md) as the outstanding ask;
  expecting OBS-WT6-WTA-12 onwards. Once that clears,
  sequence commits per [architect-3](./architect-3.md).
* 2026-05-18 (close-out, +30 min) **OBS-WT6-WTA-12 RESOLVED**
  by @@WebtestB live smoke: PNG paste → `/api/attachments`
  200 → drive `attachments/image.png` on disk → source mode
  shows `![](attachments/image.png#w=250)` → Cmd+Enter
  writes the markdown bytes to the PTY end-to-end. Bash
  history-expansion noise on `![...]` is bash being bash;
  claude/codex read raw stdin and see the literal markdown.
  "New File from here" covered by `TerminalRichPrompt.test.ts`
  test #7. **Commit gate is CLEAR.** Open OBS at close are
  WTA-2 (pre-existing), WTA-7 + WTA-11 (UX nits), and WTA-9
  (parked to 6.1 via [frontend-11](./frontend-11.md)). Ready
  to sequence commits per [architect-3](./architect-3.md)
  on Alex's go.
* 2026-05-18 BLOCK-WT6-A filed by @@WebtestA caught a
  transient WIP state in [backsystacean-10](./backsystacean-10.md)
  that introduced `unsafe` blocks for `proc_pidinfo` (chan-
  server is `#![forbid(unsafe_code)]`). Agent pivoted before
  the next poke: Linux uses `/proc/<pid>/cwd` (safe),
  macOS shells out to `/usr/sbin/lsof` per probe (safe via
  subprocess, no FFI), other platforms return `None`.
  Build now green at HEAD; block resolved.
* 2026-05-18 [backsystacean-10](./backsystacean-10.md) REVIEW.
  Contract PASS by @@Architect: PID on Session,
  `AttachHandle::cwd()`, WebSocket `cwd` request/response
  frames + initial `ready` frame, sandboxed to drive root,
  frontend menu rows wired. lsof-per-probe cost is acceptable
  given the frontend only queries on right-click menu open
  (not per keystroke); flagging a potential 6.1 follow-up to
  swap macOS path to `libproc` (safe wrapper crate) for
  performance if the lsof spawn cost ever shows up in a
  profile.

## Completion notes

* 2026-05-18 Phase 6 closed by Alex's "yes" to wrap. Commits
  1-6 landed (chan-drive, chan-report, chan-server, web,
  release: close phase 6 tasks + open phase 7 scaffold, chore:
  bump version to 0.10.0). Workspace + desktop bundle both at
  0.10.0; Chan_0.10.0_aarch64.dmg signed + ready for Alex to
  install. Pre-push gate green on the final HEAD before the
  wrap commit. [summary.md](./summary.md) carries the full
  outcome + highlights + lowlights + bug fix trail + coverage
  + 6.1 manifest + agent rankings. Final
  `release: phase 6 wrap + 0.10.0 notes` commit lands the
  summary + this closing note. Push on Alex's explicit go.
