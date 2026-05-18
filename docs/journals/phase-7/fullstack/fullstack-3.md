# fullstack-3: find UX upgrade

Owner: @@FullStack
Cut by: @@Architect
Date: 2026-05-18

## Goal

Make every search / find / link-completion surface clearly
communicate state: empty, indexing, in-progress, and
no-matches. Stop leaving the user with blank result lists and
mysterious separators. Re-focus the Find buffer when it's
already open.

## Relevant links

* [../request.md](../request.md) Bugfixes (the Find / indexing
  / empty-state cluster; sub-bullets a/b/c under the image-
  paste bug).
* Repro images: `../image-2.png` (empty no-matches view with
  separator), `../image-3.png` (image search same issue),
  `../image-4.png` (Cmd+F when buffer already open: no-ops).

## Acceptance criteria

Covers six request items: B3 (find menu additions), B4 (`[[`
indexing state), B8 (no-matches text), B9 (`![`-search empty
state), B10 ("empty search, type something"), B21 (Cmd+F
re-focus).

### Cmd+F re-focus

* Pressing Cmd+F when the Find buffer is already open
  re-focuses the input (selects existing query if any). It
  must not no-op.

### `[[`-link auto-completion popup

* While the indexer is still running for the current drive,
  the popup shows "Indexing..." with a small spinner instead
  of "No matches" or silence.
* After indexing completes, "No matches" is shown with the
  documents-searched count below (see no-matches text below).
* When the user has typed nothing yet, show
  "Empty search, type something".

### `![`-image search popup

* Same state ladder as `[[` (empty / indexing / no-matches),
  same affordances.

### No-matches view (Find + popups)

* Replace the current blank-list-with-stray-separator with
  one of the explicit states:
  * "Empty search, type something" when the query is empty.
  * "Indexing... (searched N documents so far)" with spinner
    while the indexer is still scanning.
  * "No matches in N documents." after indexing is complete.
* The state line sits *above* the separator; no orphan
  separator when the list is empty.

### Find menu additions (Cmd+F dropdown menu)

* Add three actions:
  * **Highlight trailing whitespace** — toggles visualization
    inside the editor; does not move the cursor.
  * **Toggle code blocks** — markdown files only;
    show/collapse fenced code blocks; does not move the
    cursor.
  * **Remove trailing whitespace** — strips trailing
    whitespace from the document; does not move the cursor.
    Includes a persistent checkbox "Run automatically on
    save / auto-save" stored in preferences.

## Out of scope

* Underlying indexer behavior changes (we're surfacing state,
  not changing index semantics). If the indexer doesn't
  expose an "in-progress" signal yet, coordinate with
  @@Systacean (systacean-2 is poking at indexer-side
  questions concurrently).
* Right-click menu on the rich prompt — separate wave-2 task.

## How to start

1. Find the Find / link-completion components on the frontend
   side; identify where they currently render the no-result
   state.
2. Plumb the indexer state signal end-to-end (server should
   already publish indexing status; check `crates/chan-server/src/indexer.rs`
   for the existing channel).
3. Sequence the changes: re-focus first (cheap), state
   strings next, menu additions last.

## Hand-off

Same shape as fullstack-1.

## 2026-05-18 14:10 BST — @@FullStack partial implementation

Implemented the first `fullstack-3` slice:

- `app.find.open` now bumps a per-tab find focus nonce so Cmd+F / native find-open re-focuses an already-open Find input and selects the existing query.
- FindBar now renders explicit empty and no-match states instead of only a blank counter.
- `[[` and `![` bubbles now share explicit empty / indexing / no-match state rendering, including spinner text for active indexing and indexed-document counts from `indexStatus`.
- Empty bubble states suppress the orphan footer separator.
- Added focused Vitest coverage for bubble state wording and find re-open focus nonce.

Verification:

- `npm run test -- empty_state tabs`
- `npm run check`

Remaining for full acceptance: Find menu actions for trailing-whitespace highlighting, fenced-code-block toggling, whitespace removal, and persisted auto-remove-on-save preference.

## 2026-05-18 14:21 BST — @@FullStack implementation ready

Completed the remaining `fullstack-3` acceptance on top of the earlier state-surface slice:

- Added Find menu actions for trailing-whitespace highlighting, fenced-code-block collapse/expand, and trailing-whitespace removal.
- Added a persistent `strip_trailing_whitespace_on_save` preference through `/api/config`, server editor preferences, frontend API types, and local preference state.
- Save and auto-save strip trailing spaces/tabs when the preference is enabled without overwriting newer in-flight edits.
- Added shared CodeMirror editor tooling for trailing-whitespace decorations, whitespace removal, and code-block fold toggling.
- WYSIWYG and Source editors both expose the tool commands and preserve cursor mapping through CodeMirror transactions.

Verification:

- `npm run test -- tabs empty_state`
- `npm run check`
- `npm run build`
- `cargo fmt`
- `cargo check -p chan-server`
- `cargo test -p chan-server preferences`

Notes:

- `fullstack-2` files remain uncommitted and pending the existing @@WebtestA external-link walkthrough gate.
- This `fullstack-3` patch also touches `web/src/App.svelte` for global bubble empty-state CSS; that file already has committed `fullstack-1` history, so review should focus on the new CSS block.

## 2026-05-18 15:25 BST — @@Architect review: APPROVED for commit (gated on @@Alex)

Substantial scope for one task — and well-sequenced (slice-1 surface
states, slice-2 the menu + auto-strip preference). Acceptance coverage:

* B21 Cmd+F re-focus: per-tab find focus nonce — correct shape (no
  global state that'd race across tabs).
* B3 Find menu additions: highlight trailing ws, toggle code blocks,
  remove trailing ws, persistent auto-strip-on-save preference. All in.
* B4 `[[` indexing state, B9 `![` same, B8 no-matches view, B10 empty
  search: explicit ladder of states with indexed-doc counts. Orphan
  footer separator suppressed on empty.
* Vitest coverage on bubble wording + focus nonce.
* Save / auto-save strip-on-save doesn't clobber in-flight edits —
  important; that's the trap I was worried about.

Two micro-observations (non-blocking):

* The shared CodeMirror tooling (trailing-ws decorations, whitespace
  removal, fold toggling) is the kind of thing that wants a brief
  module-level comment naming the WHY (so future readers don't
  re-implement). If you can land that in the same commit, good;
  otherwise non-blocking.
* `web/src/App.svelte` CSS block coexists with the committed
  fullstack-1 history — confirmed the new addition doesn't overlap
  layout selectors. Clean.

### Commit clearance

**APPROVED architect-side.** Gated on @@Alex authorization.

### Proposed commit message

```text
Tighten Find / link-bubble UX with explicit state ladders

Cmd+F now re-focuses an already-open Find input and selects the
existing query. Find / [[ / ![ bubbles render explicit empty /
indexing / no-match states with indexing-spinner text and
indexed-document counts; the orphan footer separator no longer
appears on empty results.

Add Find-menu tools for trailing-whitespace highlighting, fenced-
code-block collapse/expand, and trailing-whitespace removal.
Add a persisted strip_trailing_whitespace_on_save preference
that strips trailing spaces/tabs on save and auto-save without
clobbering newer in-flight edits.
```

### Sequencing

Commit `fullstack-3` after I sign off here. Then we're still waiting
on `fullstack-2` revision + `webtest-a-3` walkthrough before the
patch bump.

Move to `fullstack-4` (list + image bugs) next — that's the last
queued wave-1 task on your side that isn't blocked on someone else.
