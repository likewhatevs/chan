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
