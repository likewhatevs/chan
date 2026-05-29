# WebtestB Wave 1 Report

To: @@Architect
From: @@WebtestB
Date: 2026-05-23

## Scope

Assigned smoke lane:

- Hybrid hamburger
- Focus
- File Browser
- Graph
- New Draft

Browser status: blocked. The Browser plugin setup succeeded, but the
`iab` browser backend was unavailable in this thread:

```text
Browser is not available: iab
agent.browsers.list() -> []
```

I used current-main build, API probes, and existing focused Vitest coverage
as the repeatable baseline. Live visual/browser confirmation still needs a
rerun by a thread with Browser access.

## Files Changed

- `docs/journals/phase-9/webtestb/wave-1-report.md`

No product code changed.

## Repro Status

| Area | Status | Evidence |
| --- | --- | --- |
| Hybrid hamburger | Partial pass | Unit coverage passes for spawn ordering, focus colour, flipped CSS, and left-trigger clamp. Live viewport/menu placement not verified because Browser `iab` is unavailable. |
| Focus | Partial pass | `Pane.test.ts` focus-related assertions passed. No live tab-focus/browser interaction verified. |
| File Browser | Pass for API/unit baseline | Root listing returned synthetic `Drafts`, seeded dirs, and `index.md`. Drafts listing after create returned `Drafts/untitled`. Existing Drafts FB tests passed. |
| Graph | Pass for API/unit baseline | `/api/graph` returned file, tag, mention, directory, language nodes and expected link/contains edges. `/api/fs-graph` returned directory tree. Graph Drafts styling tests passed. No canvas render verified. |
| New Draft | Pass for API/unit baseline | `POST /api/drafts/new` returned `{"path":"Drafts/untitled/draft.md","name":"untitled"}`. Reading that path via `/api/files/...` returned writable empty draft. Existing Cmd+N/New Draft tests passed. |

## Suspected Owner / Module

- Hybrid hamburger viewport bug, if reproduced visually:
  `web/src/components/HamburgerMenu.svelte`,
  `web/src/components/menuClamp.ts`,
  `web/src/components/Pane.svelte`.
- Focus regressions:
  `web/src/components/Pane.svelte`,
  tab-specific focus exports in editor/terminal components.
- File Browser / Drafts:
  `crates/chan-server/src/routes/files.rs`,
  `crates/chan-server/src/routes/drafts.rs`,
  `web/src/components/FileTree.svelte`.
- Graph:
  `crates/chan-server/src/routes/graph.rs`,
  `crates/chan-server/src/routes/fs_graph.rs`,
  `web/src/components/GraphPanel.svelte`,
  `web/src/components/GraphCanvas.svelte`.
- New Draft:
  `crates/chan-server/src/routes/drafts.rs`,
  `web/src/state/tabs.svelte.ts`,
  `web/src/components/Pane.svelte`.

## Root Cause

No product root cause established in this pass.

The main test blocker root cause is environment/tooling: the in-app Browser
backend is not registered for this thread, so viewport-sensitive and canvas
render checks cannot be honestly marked pass/fail.

Static/code evidence suggests the known left-edge hamburger issue may already
have a mitigation in `triggerMenuX`: left-edge triggers open to the right when
right-alignment would place the menu off-screen, then `HamburgerMenu` refines
with actual rendered size through `clampToViewport`.

## Behavior Changed

None. No product code changed.

## Tests Run

```bash
cargo build -p chan
```

Result: pass.

```bash
HOME=/tmp/chan-webtestb-wave1.ino342/home \
  target/debug/chan serve --no-token --no-browser --port 8892 \
  /tmp/chan-webtestb-wave1.ino342/drive
```

Result: server started. Stderr included the expected semantic-search fallback:

```text
Embedding model not downloaded; falling back to BM25-only keyword search.
chan is ready:
http://127.0.0.1:8892/
```

```bash
curl -sS http://127.0.0.1:8892/api/files?dir=
```

Result: pass. Response included `Drafts`, `images`, `notes`, and `index.md`.

```bash
curl -sS -X POST http://127.0.0.1:8892/api/drafts/new
```

Result: pass.

```json
{"path":"Drafts/untitled/draft.md","name":"untitled"}
```

```bash
curl -sS 'http://127.0.0.1:8892/api/files?dir=Drafts'
```

Result: pass.

```json
[{"path":"Drafts/untitled","is_dir":true,"mtime":1779566307,"size":0}]
```

```bash
curl -sS 'http://127.0.0.1:8892/api/files/Drafts/untitled/draft.md'
```

Result: pass.

```json
{"path":"Drafts/untitled/draft.md","content":"","mtime":1779566307,"writable":true}
```

```bash
curl -sS http://127.0.0.1:8892/api/graph
```

Result: pass. Response contained expected file nodes for `index.md`,
`notes/alpha.md`, `notes/beta.md`, plus `#tag`, `@@alex`, directory nodes,
language node, and link/tag/mention/contains edges.

```bash
curl -sS 'http://127.0.0.1:8892/api/fs-graph?scope=directory&path=&depth=2'
```

Result: pass. Response contained root, `images`, `notes`, `index.md`,
`notes/alpha.md`, `notes/beta.md`, and contains edges.

```bash
curl -sS http://127.0.0.1:8892/api/index/status
```

Result: pass.

```json
{"state":"idle","indexed_docs":3,"indexed_vectors":0,"model":"BAAI/bge-small-en-v1.5"}
```

```bash
curl -sS http://127.0.0.1:8892/api/health
```

Result: pass.

```json
{"status":"ok","indexer":{"status":"idle","queue_depth":0,"last_event_at":1779566307,"last_settled_at":1779566307,"coalesced_rebuild":false},"terminal_event_watcher":{"dropped_events":0}}
```

```bash
npm --prefix web test -- \
  src/components/Pane.test.ts \
  src/components/hybridHamburgerNewDraft.test.ts \
  src/components/draftsRowFb.test.ts \
  src/components/graphDraftsStyling.test.ts \
  src/components/newDraftCmdN.test.ts \
  src/components/paneModeKeymap.test.ts
```

Result: pass.

```text
Test Files  6 passed (6)
Tests       73 passed (73)
```

```bash
npm --prefix web test -- src/components/menuClamp.test.ts
```

Result: pass.

```text
Test Files  1 passed (1)
Tests       2 passed (2)
```

## Known Gaps

- No live browser screenshots.
- No live Hybrid flipped-left hamburger viewport measurement.
- No live focus traversal/click routing verification.
- No Graph canvas nonblank/rendered-node verification.
- No visual File Browser tree expansion verification.
- No post-fix rerun yet.

## Proposed Fix Order

1. Restore Browser access for a WebtestB rerun, or hand this matrix to an
   agent with a registered `iab` backend.
2. Verify flipped Hybrid hamburger at narrow and desktop viewports. If it
   fails, fix `HamburgerMenu`/`menuClamp` first because it is the known
   viewport-sensitive bug.
3. Verify New Draft via UI path: pane hamburger, empty pane carousel, and
   Hybrid Nav `n`.
4. Verify File Browser Drafts expansion and Graph tab render/canvas after UI
   path opens them.
5. Re-run API probes and focused Vitest after any frontend fix.

## Blockers

- Browser plugin cannot provide an `iab` backend in this thread:

```text
Browser is not available: iab
agent.browsers.list() -> []
```

## Recommended Commit Boundary

No commit for this pass unless @@Architect wants test reports committed as
coordination artifacts. Product fixes should be separate from this report.
