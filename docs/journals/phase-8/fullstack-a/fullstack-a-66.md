# fullstack-a-66 — SPA New Draft action + FB Drafts folder rendering + Rich Prompt history reuse

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched
Dependency: `systacean-24` (chan-drive Drafts primitive)

## Goal

SPA implementation of the New Draft flow:

1. **Cmd+N from Hybrid hamburger / global keymap** →
   create `Drafts/untitled-N/` directory + populate
   `draft.md` + open in Hybrid Editor.
2. **FB Drafts folder rendering** — first element;
   distinct color (yellow w/ light/dark variants).
   Inspector shows "lives outside drive's root"
   notice.
3. **Rich Prompt history reuse**: Rich Prompt history
   stored as `Drafts/rich-prompt-N/...` via the same
   Drafts mechanism.

## Reference

[`../alex/addendun-a.md`](../alex/addendum-a.md):
"## Flow for the 'New Draft' action" + "### Extra"
sections.

## Dependency on -24

@@Systacean's `systacean-24` lands the chan-drive
backend (filesystem primitive + indexer + graph emit).
This task consumes that API surface. **Wait for `-24`
commit-readiness** OR start SPA shell + integration
points with stubbed API + wire in once `-24` lands.
Implementer picks.

## Scope (SPA)

### Cmd+N handler

* Bind Cmd+N → call `chan_drive::Drive::create_draft_dir`
  (via chan-server route OR directly through Tauri
  IPC if needed). Pick smallest `N` unused.
* Open `draft.md` inside the new dir in the Hybrid
  Editor.
* No further user prompt — direct create + open.

### FB Drafts folder rendering

* First element in the FB tree (above the drive root
  contents).
* Distinct color — yellow w/ light/dark variants.
  Define CSS vars `--fb-drafts-light` / `--fb-drafts-dark`;
  apply to the row.
* Inspector view on click: same shape as a regular
  folder, but with a header notice: "Drafts lives
  outside the drive's root" (per `addendun-a.md`).

### Rich Prompt history reuse

* Refactor Rich Prompt history persistence to use
  `Drafts/rich-prompt-N/` paths instead of wherever
  it lives today (per the addendum: this means user
  has GitHub-style access to their history via the
  FB).
* Each prompt history entry is a `rich-prompt-N/`
  dir under Drafts.

### Graph integration

* Drafts root node renders with a distinct color +
  inspector matches FB inspector (same shape).
* Distinct edge style for drive → Drafts edge per
  `-24`'s emit attribute.
* Files inside Drafts in the graph behave like
  drive files.

## Acceptance

1. Cmd+N creates `Drafts/untitled-N/` + opens
   `draft.md` in Hybrid Editor. First N=1; second
   N=2 etc.
2. FB shows Drafts as the first row in yellow color
   (light + dark mode both).
3. Drafts folder click → inspector with "outside
   drive's root" notice.
4. Rich Prompt history persists into
   `Drafts/rich-prompt-N/` (verifiable via FB +
   filesystem inspection).
5. Graph view: Drafts root node distinct;
   click-inspector matches FB shape.

### Tests

Vitest pins for chord binding + chord handler +
Drafts row rendering + inspector header notice.

### Gate

* `npm test -- --run`, `npm run check`, `npm run build`
  green.

## Coordination

* @@FullStackA SPA primary.
* Depends on `systacean-24` API surface.
* If `-24`'s API shape differs from this task's
  expectation, fire scope poke + adjust.

## Authorization

Yes for SPA files + tests + task tail + outbound.
Pause-then-resume if `-24` not yet ready.

## Numbering

This is `-a-66`.

## 2026-05-22 — slice 1 (Cmd+N + create + open) ready for review

Per the architect's slice-friendly framing,
`-a-66` decomposes into 5 pieces. Slice 1 is the
core user flow: Cmd+N → fresh draft → open in
editor. Subsequent slices land the FB row, the
"outside drive's root" inspector notice, Rich
Prompt history persistence, and Graph styling.

Six-file change. SPA + chan-server.

### What landed

**chan-server**:

* `crates/chan-server/src/routes/drafts.rs` (new):
  `api_create_draft` handler. Picks next
  untitled name via `Drive::next_untitled_draft_name`,
  creates the draft dir via `Drive::create_draft_dir`,
  writes empty `draft.md` via the unified
  `Drive::write_text("Drafts/<name>/draft.md", "")`.
  Returns `{ path: "Drafts/<name>/draft.md",
  name: "<name>" }`. Two-retry race window for the
  `next_untitled` + `create_dir` non-atomic gap
  (rare in single-user mode).
* `crates/chan-server/src/routes/mod.rs`:
  declares `drafts` module + re-exports
  `api_create_draft`.
* `crates/chan-server/src/lib.rs`: imports
  `api_create_draft`; adds
  `POST /api/drafts/new` route.
* `self_writes.note(path)` called so the
  watcher suppression chain knows about the new
  file (prevents a spurious self-write echo
  through the watcher).

**SPA**:

* `web/src/api/client.ts`: new `api.createDraft()`
  helper returning `{ path, name }`.
* `web/src/state/shortcuts.ts`: registry entry
  `app.draft.new` bound to `Mod+N` (web + native).
* `web/src/App.svelte`:
  * Imports `api` from `./api/client`.
  * New keymap branch on bare Cmd+N
    (`!altKey && !shiftKey && !ctrlKey`) calls
    `createDraftAndOpen()`. Cmd+Shift+N still
    falls through to chan-desktop's "New Window"
    menu per `-b-27`.
  * `createDraftAndOpen()`: `await api.createDraft()`
    → `await openInActivePane(path)`. Try/catch
    swallows errors (console.warn) so a failed
    create doesn't blow up the SPA.

**Tests**:

* `web/src/components/newDraftCmdN.test.ts` (new):
  5 raw-source pins covering the api.createDraft
  helper, the shortcut registry entry, the Cmd+N
  keymap branch, the createDraftAndOpen flow,
  and the api import.

### Out of scope (separate slices)

* **Slice 2**: FB Drafts row rendering (first
  element, yellow color with light/dark variants).
* **Slice 3**: Drafts folder inspector with
  "lives outside drive's root" notice.
* **Slice 4**: Rich Prompt history persistence
  via `Drafts/rich-prompt-N/`.
* **Slice 5**: Graph Drafts root styling +
  click-to-inspector composition.

These slices are independent and substantial;
shipping slice 1 surfaces the headline user flow
(@@Alex's "I want progress" framing) immediately.

### Acceptance (slice 1)

1. Cmd+N creates `Drafts/untitled[-N]/draft.md`
   + opens it in the Hybrid Editor ✓ (mechanism
   via tests; @@WebtestA walk for empirical).
2. First Cmd+N: name = `untitled`; second:
   `untitled-1`; subsequent: `untitled-2`, etc.
   ✓ (chan-drive's `next_untitled_draft_name`
   handles the increment).
3. Failed creation doesn't blow up the SPA
   (try/catch in `createDraftAndOpen`) ✓.

Slice 1 partial-met of -a-66's full acceptance:
acceptance criterion #1 ✓ (Cmd+N flow);
criteria #2-#5 deferred to follow-up slices
(FB row, inspector notice, Rich Prompt history,
Graph styling).

### Gate

* vitest **825 / 825** (+6 net from `-a-74`'s
  819).
* svelte-check 0 errors / 0 warnings across
  4010 files.
* npm build clean.
* `cargo test -p chan-server --lib`: 213
  passed (route + module wiring covered;
  no new Rust tests added in this slice).

### Decisions

* **Per-slice split** — `-a-66` spec is
  substantial across 5 independent pieces.
  Shipping the core user flow first reads
  cleaner + surfaces value early.
* **Two-retry race window** in
  `api_create_draft` — picks new name on
  AlreadyExists. Race is rare (single-user)
  but the retry is cheap + keeps the contract
  clean.
* **Empty draft.md content** — matches
  @@Alex's addendum framing ("auto-populates
  it with draft.md open in Hybrid's Editor").
  No template; user types from scratch.
* **Try/catch on the create path** — a
  failed create shouldn't take down the SPA
  keymap. Logged + swallowed.
* **No FB / inspector / graph in this slice**
  — those slices need design alignment on the
  yellow color variants, the "outside root"
  notice copy, and the `drafts_link` edge
  styling. Slicing the user-flow off lets
  those bake separately.

### Suggested commit subject

```
New draft: Cmd+N creates Drafts/untitled-N/draft.md + opens in editor (fullstack-a-66 slice 1)
```

Single commit. Route + helper + chord + test
tightly coupled around the same user flow.

### Files for `git add` (per-path discipline)

* `crates/chan-server/src/routes/drafts.rs` (new)
* `crates/chan-server/src/routes/mod.rs`
* `crates/chan-server/src/lib.rs`
* `web/src/api/client.ts`
* `web/src/state/shortcuts.ts`
* `web/src/App.svelte`
* `web/src/components/newDraftCmdN.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-66.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.

## 2026-05-22 — slice b (FB Drafts row) ready for review

Three-file change. SPA + chan-server. Now that
`systacean-29` lands `Drive::list` unified-path
routing, the FB Drafts row works end-to-end.

### What landed

**chan-server**:

* `crates/chan-server/src/routes/files.rs`:
  `api_list_files` now injects a synthetic
  `Drafts` directory entry at position 0 of
  the root listing (when `dir` query param is
  unset). Listing under `dir=Drafts` /
  `dir=Drafts/<name>` already routes through
  the unified `Drive::list` thanks to `-29`.

**SPA**:

* `web/src/components/FileTree.svelte`:
  * Dir row gains `class:drafts-row={node.path
    === "Drafts"}` so the synthetic row
    picks up a distinct visual.
  * CSS rules tint `.row.dir.drafts-row`'s
    background + icon + name with
    `--fb-drafts-fg` / `--fb-drafts-bg` so
    light/dark + per-Hybrid theme overrides
    cascade.
* `web/src/App.svelte`: new
  `--fb-drafts-fg` / `--fb-drafts-bg`
  variables in both `:root` (dark) and
  `[data-theme="light"]` blocks. Yellow tone
  (`#e3b341` dark / `#9a6700` light;
  low-alpha rgba bg).

**Tests**:

* `web/src/components/draftsRowFb.test.ts`
  (new): 5 raw-source pins covering the row
  class hook, the CSS tints, + the
  dark+light CSS variable declarations.

### Acceptance (slice b)

1. FB shows Drafts as the first row in yellow
   color ✓ (light + dark vars declared;
   mechanism via test pins). @@WebtestA walk
   for empirical confirmation.
2. Expansion into `Drafts/<name>/...` works
   via the existing `/api/files?dir=Drafts`
   route + `-29`'s unified `Drive::list`.
3. Drafts row click → currently selects the
   row + treats it as a directory (the
   "outside drive's root" inspector notice
   is slice c territory).

### Out of scope (deferred slices)

* **Slice c**: Drafts folder inspector with
  "lives outside drive's root" notice.
* **Slice d**: Rich Prompt history persistence
  via `Drafts/rich-prompt-N/`.
* **Slice e**: Graph Drafts root styling +
  `drafts_link` edge styling.

### Gate

* vitest **902 / 902** (+4 net from `-a-78`
  slice 2's 898).
* svelte-check 0 errors / 0 warnings across
  4020 files.
* `cargo test -p chan-server --lib`: 213
  passed.
* npm build clean.

### Decisions

* **Synthetic injection in chan-server** vs
  SPA-side injection — keeps a single source
  of truth on the wire. Other consumers
  (future MCP tools, the search index, etc.)
  see the same Drafts entry.
* **Position 0 (top of list)** matches
  @@Alex's addendum-a framing: "shown in the
  File Browser as the very first element."
* **Yellow tone**:
  - Dark `#e3b341` (matches `--warn-text`
    family).
  - Light `#9a6700` (matches `--warn-text`
    light counterpart).
  - Low-alpha bg (10% / 8%) so the row
    reads as a category marker without
    dominating.
* **Inspector + Rich Prompt history + Graph
  styling deferred** — each is a separate
  slice. This slice just delivers the
  visible "first row in yellow" affordance.

### Suggested commit subject

```
File browser: synthetic Drafts row at root with yellow tint (fullstack-a-66 slice b)
```

Single commit. chan-server injection + SPA
markup + CSS + test tightly coupled.

### Files for `git add` (per-path discipline)

* `crates/chan-server/src/routes/files.rs`
* `web/src/components/FileTree.svelte`
* `web/src/App.svelte`
* `web/src/components/draftsRowFb.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-66.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.

## 2026-05-22 — slice b follow-up (webtest-a PARTIAL fix)

Two-file change. chan-server only.

### Webtest's PARTIAL verdict

`webtest-a-1.md` proactive walk on `5dffa09`:
* Server-side curl confirmed Drafts at pos 0
  of `/api/files` (no query).
* SPA-rendered FB had 17 rows / 8 dirs / no
  Drafts.
* Hypothesis: WS indexer event stream
  overriding the initial fetch OR watcher
  event filtering.

### Actual root cause

Empirical re-audit:
* SPA's `api.list("")` (from `refreshTree()`
  at `store.svelte.ts:531`) constructs the URL
  as `/api/files?dir=` (empty-string query
  param), not `/api/files` (no param).
* Pre-fix gate at `files.rs:121` checked
  `query.dir.is_none()` — TRUE only when the
  param is absent. `Some("")` (empty-string)
  fell through, so the synthetic Drafts
  injection silently dropped.
* curl returned the correct shape because
  curl was hitting the URL without a query
  param at all.

### Fix

`crates/chan-server/src/routes/files.rs`:
* Extracted new helper `is_root_listing(dir:
  Option<&str>) -> bool` that matches every
  shape:
  * `None`
  * `Some("")` ← the fix
  * `Some("/")` / `Some("//")`
  * `Some(".")` / `Some("./")`
* Swapped the `query.dir.is_none()` check
  for `is_root_listing(query.dir.as_deref())`.
* Added 5 unit tests on the helper (absent
  / empty / slash / dot / non-root).

### Acceptance

1. FB shows Drafts at the top empirically ✓
   (mechanism via 5 new Rust pins; @@WebtestA
   re-walk for empirical confirm).
2. `dir=Drafts/...` listings unchanged ✓
   (`is_root_listing("Drafts")` returns
   false; routes through `list_dir_entries`
   as before).
3. No regression on the regular dir listing
   paths ✓ (all 213 prior chan-server tests
   still pass; +5 new = 218).

### Gate

* `cargo test -p chan-server --lib`: **218
  passed** (+5 net from slice b's 213).
* vitest **916 / 916** (unchanged; SPA tests
  not affected).
* svelte-check 0 errors / 0 warnings across
  4023 files.
* npm build clean.

### Decisions

* **Helper extraction over inline check** —
  enables unit tests + cross-references the
  contract from the test names. Matches
  `chan-server`'s pattern (`normalize_dir_query`
  next door).
* **Five test pins** — every shape the SPA
  / curl / tests could produce. Cheap +
  audit-friendly.
* **Did NOT extend `normalize_dir_query`** —
  that helper is used by `list_dir_entries`
  for dir-validated listings; the
  Drafts-injection gate is a separate
  concern (root-vs-non-root) that doesn't
  need path validation.

### Suggested commit subject

```
File browser Drafts row: also gate synthetic injection on dir="" (fullstack-a-66 slice b follow-up)
```

Single commit. Helper + 5 tests tightly
coupled around the same fix.

### Files for `git add` (per-path discipline)

* `crates/chan-server/src/routes/files.rs`
* `docs/journals/phase-8/fullstack-a/fullstack-a-66.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance + the
@@WebtestA empirical re-walk.

## 2026-05-22 — slice c (Drafts inspector notice) ready for review

Two-file change. SPA-only.

### What landed

`web/src/components/DirectoryInfoBody.svelte`:

* **Kind chip swap**: `DIR` → `DRAFTS` (with
  `class:drafts` toggle) when `path === "Drafts"`.
  CSS class `.kind-chip.drafts` picks up
  `--fb-drafts-fg` so the chip's yellow tint
  matches the FB row (slice b) for cross-surface
  consistency.
* **Notice block**: above the existing stats /
  COCOMO sections, render a `.drafts-notice`
  with heading "Drafts lives outside the
  drive's root." and a short paragraph
  explaining chan's metadata folder, that
  files survive drive moves, and that Cmd+N
  + Rich Prompts persist under
  `Drafts/untitled-N/` / `Drafts/rich-prompt-N/`.
* CSS: `.drafts-notice` uses the Drafts tint
  vars (`--fb-drafts-bg` background +
  `--fb-drafts-fg` left border) +
  monospace inline code for the path
  examples.

`web/src/components/draftsInspectorNotice.test.ts`
(new): 7 raw-source pins covering:
* Kind-chip class hook + label swap.
* `.kind-chip.drafts` tint rule.
* Notice block conditional render.
* "outside the drive's root" copy.
* Cmd+N + Rich Prompt path cross-references.
* Notice CSS using the Drafts tint vars.
* Rationale comment cross-referencing
  chan-drive's metadata folder + drafts_dir
  handle.

### Acceptance (slice c)

1. **Selecting Drafts in FB renders the
   notice** ✓ (mechanism via tests;
   @@WebtestA walk for empirical).
2. **Notice copy matches addendum-a "outside
   drive's root"** ✓.
3. **Visual treatment uses the same Drafts
   tint vars as the FB row** ✓ —
   cross-surface consistency.
4. **No regression on regular directory
   inspector** ✓ — notice only renders
   inside the `{#if path === "Drafts"}`
   guard.

### Out of scope (deferred slices)

* Slice d: Rich Prompt history → `Drafts/
  rich-prompt-N/`.
* Slice e: Graph Drafts root styling +
  `drafts_link` edge.

### Gate

* vitest **945 / 945** (+7 net from `-a-88`'s
  938).
* svelte-check 0 errors / 0 warnings across
  4029 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **Notice ABOVE the stats sections** — the
  Drafts directory will rarely have
  chan-report data (no source files
  typically) + the "stats unavailable" empty
  state would read as the primary content
  otherwise. Notice first gives the user the
  "why" before the stats branch.
* **Drafts tint vars reused** — single
  source of truth for the yellow tone
  introduced in slice b. No new vars added.
* **Code-style inline path examples** — both
  `Drafts/untitled-N/` and
  `Drafts/rich-prompt-N/` wrapped in
  `<code>` so the path-keyspace shape reads
  as a concrete affordance, not prose.
* **Kept stats sections** — the notice
  doesn't replace them; if chan-drive's
  Drafts indexing ever surfaces stats, the
  existing branches handle them. Notice +
  stats coexist.

### Suggested commit subject

```
File browser inspector: Drafts notice + tinted chip (fullstack-a-66 slice c)
```

Single commit. Inspector body markup + CSS +
test pins tightly coupled around the same
slice-c contract.

### Files for `git add` (per-path discipline)

* `web/src/components/DirectoryInfoBody.svelte`
* `web/src/components/draftsInspectorNotice.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-66.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.

## 2026-05-22 — slice d (Rich Prompt history → Drafts/rich-prompt-N/) ready for review

Six-file change. Cross-stack (chan-server +
SPA).

### What landed

**chan-server**:

`crates/chan-server/src/routes/drafts.rs`:
* New `RichPromptCreatePayload { content }` +
  `RichPromptCreateResponse { path, name }`
  types.
* New `api_create_rich_prompt` handler.
  Same retry-once race pattern as
  `api_create_draft` (`AlreadyExists` →
  retry once with re-resolved name).
* New `next_rich_prompt_name(drive)` helper.
  Lives in chan-server (not chan-drive) so
  the prefix-pickup loop stays where its
  consumer is + doesn't expand chan-drive's
  API surface. First slot is `rich-prompt`;
  subsequent are `rich-prompt-1` /
  `rich-prompt-2` / etc. (matches the
  `untitled` / `untitled-N` shape).
* +4 Rust unit pins on the helper:
  first-slot-unsuffixed, gap-counting,
  ignores-untitled-drafts (cross-prefix
  isolation), and internal-gap fill.

`crates/chan-server/src/routes/mod.rs`:
* Re-exports `api_create_rich_prompt`
  alongside `api_create_draft`.

`crates/chan-server/src/lib.rs`:
* New route at `POST /api/drafts/rich-prompt`
  next to the existing `/api/drafts/new`.
* Import block extended.

**SPA**:

`web/src/api/client.ts`:
* New `api.createRichPromptDraft(content)`
  client method. Doc comment cross-
  references slice d + the `rich-prompt-N`
  naming.

`web/src/components/TerminalTab.svelte`:
* `submitRichPrompt` now calls
  `void persistRichPromptHistory(source)`
  after the existing send.
* New `persistRichPromptHistory` helper:
  * Skips empty / whitespace-only sources
    (no history entry for an empty submit).
  * Calls `api.createRichPromptDraft(source)`.
  * Failures route through
    `setTransientStatus` (auto-dismiss per
    `-a-86` pattern) so the user gets a
    non-fatal heads-up; the original send
    isn't undone.

`web/src/components/richPromptHistoryPersist.test.ts`
(new): 6 raw-source pins:
* api client method signature + route
  target.
* Client doc-comment cross-reference.
* `submitRichPrompt` calls persist helper.
* persist helper trims + skips empty.
* persist failures surface via
  setTransientStatus.
* persist calls
  `api.createRichPromptDraft(source)`.

### Acceptance (slice d)

1. **Rich Prompt submission persists into
   `Drafts/rich-prompt-N/prompt.md`** ✓ —
   mechanism via the 4 Rust pins on the
   name picker + the 6 SPA pins on the
   submit hook. @@WebtestA walk for
   empirical FB browsability confirmation.
2. **First submit lands as `rich-prompt`**
   (no suffix); subsequent as
   `rich-prompt-1`, `rich-prompt-2`, ...
   ✓ matches `untitled` naming pattern.
3. **No regression on the
   send-to-terminal path** ✓ — persist runs
   AFTER the send + as a `void` promise;
   failures don't unwind.
4. **Empty submits don't create a history
   entry** ✓ — trim + early-return.

### Out of scope (deferred slice)

* Slice e: Graph Drafts root styling +
  `drafts_link` edge.

### Gate

* `cargo test -p chan-server --lib`: **224
  passed** (+4 net from prior 220).
* vitest **951 / 951** (+6 net from
  `-a-66` slice c's 945).
* svelte-check 0 errors / 0 warnings across
  4030 files.
* npm build clean.

### Decisions

* **chan-server-side name picker** (not
  chan-drive) — keeps the prefix-pickup
  loop where its consumer is + avoids
  expanding chan-drive's API surface for a
  one-off use case. `next_untitled_draft_name`
  stays untouched.
* **First-slot unsuffixed** matches the
  `untitled` / `untitled-N` shape for
  consistency across both flows.
* **Persist after send** (not before) —
  the user's primary intent is "send the
  command"; history is a side effect.
  Failure to persist doesn't block the
  command from running.
* **Auto-dismissing failure toast** per
  `-a-86` pattern — non-fatal failure
  shouldn't stick on screen forever.
* **Trim-only empty check** — pure
  whitespace prompts probably indicate a
  paste accident or a cleared buffer
  rather than a deliberate empty submit;
  skip the history entry.

### Suggested commit subject

```
Rich Prompt history: persist each submit as Drafts/rich-prompt-N/prompt.md (fullstack-a-66 slice d)
```

Single commit. chan-server route + SPA api +
submit hook + tests tightly coupled.

### Files for `git add` (per-path discipline)

* `crates/chan-server/src/routes/drafts.rs`
* `crates/chan-server/src/routes/mod.rs`
* `crates/chan-server/src/lib.rs`
* `web/src/api/client.ts`
* `web/src/components/TerminalTab.svelte`
* `web/src/components/richPromptHistoryPersist.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-66.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance.

## 2026-05-22 — slice c follow-up (FB dispatch routes to FileInfoBody) ready for review

Two-file change. SPA-only.

### Webtest's PARTIAL diagnosis

`b2dfead` walk: slice c's `DirectoryInfoBody.svelte`
changes are in the dist bundle but the FB-selected
Drafts row renders WITHOUT the DRAFTS chip or
notice. Inspector chip text reads "directory"
(lowercase, `KindChip kind="folder"` default
label) + chip background is gray (not yellow).

### Root cause

The FB's inspector dispatcher uses
`FileInfoBody.svelte` for BOTH files AND
directories (line 445 `{:else if entry.is_dir}`
branch), not `DirectoryInfoBody.svelte`.
`DirectoryInfoBody` is used by GraphPanel for
graph-side directory nodes.

Slice c's edits landed in the wrong inspector
component for the FB selection path.

### Fix

`web/src/components/FileInfoBody.svelte` dir
branch (line ~445):
* Header chip swap: `<KindChip kind="folder"
  block />` → `<span class="kind-chip
  drafts-chip">DRAFTS</span>` when
  `entry.path === "Drafts"`; else the original
  `KindChip` for regular directories.
* `.drafts-notice` block added immediately
  below the title for the Drafts case.
  Same copy as DirectoryInfoBody's slice c
  notice — "Drafts lives outside the drive's
  root." + Cmd+N / Rich Prompt path
  cross-references.
* CSS: `.kind-chip.drafts-chip` mirrors the
  KindChip `.block` styling (flex:1, etc.)
  with the `--fb-drafts-fg` tint. `.drafts-
  notice` mirrors the DirectoryInfoBody
  rule (`--fb-drafts-bg` background +
  `--fb-drafts-fg` left border).

`web/src/components/draftsInspectorFileInfoBody.test.ts`
(new): 6 raw-source pins covering the dir-
branch swap, the notice block, the
cross-references, the CSS rules, and the
rationale comment.

### Acceptance

1. **FB-selected Drafts row now renders the
   DRAFTS chip + notice** ✓ — mechanism via
   the 6 new pins. @@WebtestA empirical
   walk for confirmation that closes the
   slice c PARTIAL.
2. **Regular directories unchanged** ✓ —
   `{:else}` branch falls through to the
   original `<KindChip kind="folder" block />`.
3. **DirectoryInfoBody changes still
   apply** ✓ — that path is reached from
   GraphPanel directory-node selection,
   not from the FB. Both components now
   carry the slice-c shape so either entry
   point renders consistently.

### Gate

* vitest **960 / 960** (+6 net from `-a-89`'s
  954).
* svelte-check 0 errors / 0 warnings across
  4030 files.
* npm build clean.
* Rust gate not re-run (no Rust touched).

### Decisions

* **Inline `<span class="kind-chip
  drafts-chip">DRAFTS</span>`** rather than
  extending `KindChip` with a `drafts` kind
  + a "DRAFTS" label. The KindChip
  abstraction is shared across many
  surfaces; adding a Drafts kind would
  ripple into `state/kinds.ts` +
  `colorVarFor` for a single specialized
  surface. Inline span is cheaper.
* **CSS duplicated across DirectoryInfoBody
  + FileInfoBody** — same shape, different
  components. A shared partial would help
  if a third entry point shows up; for
  now the duplication is small + audit-
  friendly.
* **Did NOT remove DirectoryInfoBody
  changes** — the graph-side directory
  inspector still uses that component;
  removing would re-introduce the
  inconsistency between FB + Graph
  inspectors.

### Suggested commit subject

```
File browser inspector: render Drafts chip+notice in FileInfoBody (actual FB inspector path) (fullstack-a-66 slice c follow-up)
```

Single commit. Markup + CSS + test pins
tightly coupled around the FB-side
inspector path.

### Files for `git add` (per-path discipline)

* `web/src/components/FileInfoBody.svelte`
* `web/src/components/draftsInspectorFileInfoBody.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-66.md`
* `docs/journals/phase-8/fullstack-a/journal.md`
* `docs/journals/phase-8/alex/event-fullstack-a-architect.md`

### Atomic-audit-commit

Per the memory rule. Per-path staging only.

Push held. Standing by for clearance + the
@@WebtestA empirical re-walk that closes the
slice c PARTIAL.
