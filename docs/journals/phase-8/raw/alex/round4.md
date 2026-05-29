# Round 4
Author: @@Alex
## Rich Prompt
TODO
## 2026-05-23 — @@Architect triage notes (preliminary; no reproduction yet)

### Bug 1 (Drafts FB not browseable)

Code-read pathway:

* SPA: clicking expand twirl calls `setExpanded(path, true)` (`FileTree.svelte:354`) which fires `loadTreeDir("Drafts")` (`store.svelte.ts:545`).
* `loadTreeDir` calls `api.list("Drafts")` → `GET /api/files?dir=Drafts`.
* Server: `list_dir_entries("Drafts")` → `Drive::list("Drafts")` → `drafts_dir_handle.read_dir(".")` (`chan-drive/src/drive.rs:779`).

The pathway LOOKS correct in code. Probable suspects (need reproduction to confirm):
* The drafts cap-std handle isn't initialized for @@Alex's existing drive (pre-v0.12.0 registered drive without the drafts metadata dir bootstrapped). `read_dir` may error → SPA stores error in `tree.dirErrors["Drafts"]` but the row UI may not surface it (silent failure → "can't expand").
* OR: error is surfaced but only on the row, easy to miss visually.
* OR: a different regression in v0.12.0 ship; would need bisect.

### Bug 3 (Graph tabs not loading)

@@Alex's framing suggests this may chain off bug 1. Graph tabs probably load drive-rooted graph data which intersects with the Drafts folder if it's part of the graph scope. If chan-drive errors on Drafts subtree during graph build, the graph load could fail.

Alternative independent root cause: the chan-server graph endpoint has its own load path. Could be a separate regression.

### Bug 4 (Cmd+N not working)

Chord handler is at `App.svelte:748` — `app.draft.new`. Routes to `createDraftAndOpen()` (`App.svelte:895`). Calls `POST /api/drafts/new`.

The POST endpoint is in `routes/drafts.rs`. If `chan-drive` errors during draft-dir creation (same cause as bug 1 — drafts handle broken), the POST fails, no draft created, no file opens. Silent failure on the UI side.

So bugs 1, 3, 4 likely share a root cause: **chan-drive's drafts metadata handle is broken / not initialized on @@Alex's existing drive**.

### Bug 2 (Tab-click focus)

Independent of the Drafts chain. Standard UX: clicking a terminal or editor tab header should focus the content area, ready to type. Probably a missing `focus()` call in the tab-click handler. Should be a 5-LOC fix once located.

### Suggested next steps

Two options:

1. **Fast-path investigation now** (assuming @@FullStackA is awake): I cut `fullstack-a-100` (Drafts chain triage) + `-101` (tab focus). They reproduce against a fresh drive + against @@Alex's actual drive shape to confirm/falsify the hypothesis.
2. **Defer to Round 4** (cleaner phase boundary): commit `round4.md` as the start of Round-4 backlog; cut tasks at the Round-4 fan-out beat post-v0.13.0.

Architect lean: option 2 if these aren't blocking @@Alex's daily use (they are visible but the v0.12.0 binary they're on has them either way; fix lands in v0.13.0 or v0.13.1). Option 1 if @@Alex wants the v0.13.0 cut to also close these.
