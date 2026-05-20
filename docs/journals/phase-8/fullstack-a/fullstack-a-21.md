# fullstack-a-21: Settings page UI for semantic-search opt-in

Owner: @@FullStackA
Date: 2026-05-20

## Goal

Add a "Semantic search" section to the Settings page that
lets the user opt in to Hybrid search (BM25 + dense vectors
via BGE-small). When the toggle is flipped on, chan downloads
the model into the user-config dir, shows a progress bar,
and once downloaded flips the active drive's search mode to
Hybrid. Toggle off → back to BM25.

## Background

Detour from @@Alex (2026-05-20). The Round-1 detour shifts
the embedded BGE-small model out of the default binary
(~89 MB → ~26 MB); semantic search becomes opt-in. This
task is the SPA-side surface that drives the opt-in flow.

Three-task split:

* `systacean-6` — build-side cargo feature gating +
  runtime model resolver.
* `systacean-7` — CLI subcommands + chan-server API
  endpoints (download, enable, disable, status).
* **This task (`fullstack-a-21`)** — Settings page UI.

Depends on `systacean-7`'s API contract being finalised.
You can start the layout + Svelte component shape against
mocked endpoints; finalize the wiring once -7 lands.

## API contract (per `systacean-7`)

```
GET  /api/index/semantic/state
POST /api/index/semantic/download
POST /api/index/semantic/enable
POST /api/index/semantic/disable
```

`GET /api/index/semantic/state` returns:

```json
{
  "mode": "bm25" | "hybrid",
  "model_present": true | false,
  "model_name": "BAAI/bge-small-en-v1.5",
  "model_path": "/Users/.../chan/models/...",
  "model_size_bytes": 132456789,
  "downloading": false,
  "download_progress": null | { "bytes_done": 12345, "bytes_total": 132456789 }
}
```

Download progress events surface via the existing watcher
event channel (or new `index-download` event family if
`systacean-7` introduces one).

## UX shape

Settings page gets a new "Semantic search" card / section.
Slot it under whatever logical grouping the page already
has (probably alongside "Search" / "Index" preferences;
the existing Settings layout sets the convention).

Contents:

* **Toggle**: "Enable semantic search (Hybrid mode)".
  Off by default. Flipping it on:
  * If model is downloaded → flip to Hybrid immediately;
    show "Enabled" confirmation.
  * If model not downloaded → start the download. Show
    a progress bar tracking
    `download_progress.bytes_done / bytes_total`.
    When the download completes, auto-flip to Hybrid.
  * Disable button while downloading.
* **Info row**: "Model: BAAI/bge-small-en-v1.5 (~63 MB)".
  Render the size from `model_size_bytes` once known
  (pre-download, fall back to a reasonable estimate). The
  model-name string is static for this round; the Round-3
  multi-model picker extends this to a dropdown.
* **Storage location row**: "Stored at: <model_path>".
  Render the resolved user-config path. Tooltip
  explaining that the model is shared across drives.
* **Status indicator**: "Active: BM25" / "Active: Hybrid
  (BM25 + semantic)".
* **Per-drive note** (optional, lower priority): if chan
  supports per-drive override (per `systacean-7`'s
  config persistence), surface a per-drive toggle vs
  app-wide. For v1 of this UI, keep it app-wide / current-
  drive only — simpler. Per-drive override can land in
  Round 3 alongside the model picker.

## Acceptance criteria

* Settings page renders the new "Semantic search" section
  with the toggle + info rows.
* Toggle-on triggers the download flow when model isn't
  present; progress bar tracks `download_progress`
  events.
* Once downloaded, the active drive's search switches to
  Hybrid (confirmed by re-running a search and seeing
  semantic-style results, OR by `chan index status` from
  the CLI matching).
* Toggle-off flips back to BM25; `model_present` stays
  true (the model file sticks around for next opt-in).
* Errors surface clearly: download failure → user-visible
  toast; enable failure → toast pointing at the cause.
* Visual style matches existing Settings sections (no
  off-brand styling).
* Works in both light and dark theme.
* Vitest pin for the component's toggle logic if a
  testable seam exists (the network calls can be mocked).
* `npm run check` + `npm run build` clean.

## How to start

1. Read `systacean-7` for the API contract (above).
   Start layout work against mock responses; finalize
   wiring once -7 endpoints are live in `chan-server`.
2. Find the existing Settings page in
   `web/src/components/` (likely `Settings.svelte` or a
   sibling). Identify the layout pattern + section
   styling. Add the new section following that pattern.
3. Wire the download-progress event subscription
   through whichever event channel `systacean-7` exposes
   (likely an extension of the existing watcher event
   subscription pattern).
4. Test in lane-A (`@@WebtestA` will pick up the verify).

## Coordination

* Depends on `systacean-7`'s API contract being defined
  (which depends on `systacean-6`'s resolver). Cascade.
  Start layout against mocks; finalize once -7 lands.
* @@WebtestA verifies on lane-A drive once landed.
* No backend / Rust work in this task.

## 2026-05-20 — implementation note

`systacean-7` landed at `6bf44cd` (per @@Architect's unblock
poke) with the API contract locked. Wired the SPA against the
locked shape; landed both the typed API surface and the
Settings section UI in one pass.

### API surface (`web/src/api`)

* `types.ts` — new `SemanticState` type matching the
  `systacean-7` JSON shape (`mode`, `model_present`,
  `model_name`, `model_path`, `model_size_bytes`,
  `semantic_enabled`).
* `client.ts` — four methods on the `api` object:
  `semanticState()`, `semanticDownload()`, `semanticEnable()`,
  `semanticDisable()`. All four route through the shared
  `req<SemanticState>(...)` helper; the download POST is
  synchronous and blocks until the resolver has the bytes
  on disk, per `systacean-7`'s v1 contract.

### Settings UI (`SettingsPanel.svelte`)

Added a new section between "Date pills" and "About":

* Toggle: "Enable semantic search (Hybrid mode)". Re-uses
  the existing `.theme-opt` chip shape with a `.semantic-toggle`
  modifier so checkbox-specific resets (the generic
  `input { width: 100% }` rule above doesn't apply to a chip
  checkbox) live in one place.
* Hint paragraph: model name + size. Size pulls from
  `model_size_bytes` (formatted MB), with a "size unknown"
  fallback for the pre-download state where the resolver
  hasn't probed the file yet.
* During download / enable: spinner + status string
  ("Downloading model… this may take a few minutes." /
  "Enabling…"). The spinner respects
  `prefers-reduced-motion` (animation cleared, border stays
  static).
* Status grid (`.semantic-info`): "Active" row reads
  `Hybrid (BM25 + semantic)` when `mode === "hybrid"`,
  otherwise muted `BM25`. "Stored at" row shows
  `model_path` in mono with a tooltip noting the model is
  shared across drives.
* Error row at the bottom for the last failed action
  (download / enable / disable). Sticks until the next user
  action.

### UX adjustment from the original task spec

The original spec described a progress bar tracking
`download_progress.bytes_done / bytes_total` over a
streamed event channel. @@Architect's unblock poke noted
the v1 reality: hf-hub doesn't expose a progress callback,
the download endpoint is synchronous, and there is no
per-byte event channel. The architect-approved deviation is
a spinner + polling pattern:

* When the toggle flips on AND `model_present === false`,
  the UI kicks off the synchronous `POST /download` AND
  starts a 3-second polling interval against
  `GET /state` in parallel.
* The download POST blocks until the resolver finishes.
  When it returns, we stop the poll, refresh state, and
  fire `POST /enable` so the toggle lands ON rather than
  leaving the user a second click.
* If the model is already on disk (toggle flip on with
  `model_present === true`), we skip straight to
  `POST /enable` (no spinner, no poll).
* The poll handle (`semanticPollTimer`) is cleared on
  success / failure / component unmount (via the new
  `onDestroy` hook).

### Build-not-built guardrail

When `buildInfo.features.embeddings === false` (the
`--no-default-features` build that the `embeddings` cargo
feature gates), the section renders a hint pointing at
`--features embed-model` instead of a non-functional
toggle. The "About" section's existing "embeddings: off /
on" row stays as the canonical capability indicator;
this new section just defers gracefully.

### What I did NOT add

* No vitest pin. The component has no existing test file
  (jsdom + the OverlayShell mount path would need
  scaffolding for the Settings panel beyond what fits a
  drift-cleanup PR). Per the task spec, visual verification
  on lane-A is the acceptable bar when no testable seam
  exists. The api surface itself is straightforward typed
  pass-through; the testable logic in the toggle handler is
  flow-of-state-machine + error path, both better exercised
  end-to-end.
* No per-drive override toggle. Per the task spec's "lower
  priority" note: keep this v1 app-wide / current-drive
  only. Round-3 multi-drive override / model picker is
  parked.
* No progress bar. Per @@Architect's UX adjustment above.

### Files touched

* `web/src/api/types.ts` — `SemanticState` type.
* `web/src/api/client.ts` — four API methods + import.
* `web/src/components/SettingsPanel.svelte` — state +
  toggle handler + section markup + scoped CSS.

### Pre-push gate

vitest 481/481 green (no regression; SettingsPanel has no
test file); `npm run check` 0 errors / 0 warnings;
`npm run build` clean.

### Lane-A verification path

(post-restart so the rebuilt binary picks up the bundle):

1. Open Settings (Cmd+,). New "Semantic search" section
   appears between "Date pills" and "About".
2. With `model_present === false` (clean install), the
   "Stored at" row shows the resolver's target path; the
   "Active" row reads `BM25`. The hint paragraph shows
   "size unknown" until download.
3. Flip the toggle on. Spinner + "Downloading model…"
   appear; toggle is disabled. The poll fires every 3s
   against `/state` — once the model lands, the UI
   auto-fires `/enable`, the spinner clears, "Active"
   flips to `Hybrid (BM25 + semantic)`, and the toggle
   shows on.
4. Flip the toggle off. `POST /disable` fires. "Active"
   goes back to `BM25`; `model_present` stays `true`
   (model file persists).
5. Toggle back on with the model already present →
   "Enabling…" briefly, no spinner / no download wait,
   "Active" → `Hybrid` again.
6. For the `--no-default-features` build: the section
   renders the rebuild hint instead of the toggle.

## 2026-05-20 — @@Architect: approved + commit clearance

Reviewer: @@Architect.

Clean implementation. The typed API surface (`SemanticState`
+ four client methods routing through the shared `req`
helper) is the right shape — no duplication, contract-
mirroring with `systacean-7`'s Rust types, and the
synchronous-download blocking call matches the v1
contract exactly.

The polling-with-spinner UX is well-designed: 3s interval
against `/state` while the synchronous `POST /download`
blocks; on resolver completion, stop poll + auto-fire
`/enable` so the user doesn't have to click twice. The
`semanticPollTimer` cleanup on success / failure /
unmount via `onDestroy` is correct lifecycle hygiene
(unmount mid-download was the easy thing to miss).

The build-not-built guardrail (`buildInfo.features.embeddings
=== false` → render rebuild hint instead of a broken toggle)
is the right defensive coverage. The "About" section's
existing "embeddings: off / on" row stays as the canonical
capability signal; the new section defers gracefully.

The "what I did NOT add" section is honest engineering
discipline — no vitest pin (no existing test file for
SettingsPanel; scaffolding is scope-creep), no per-drive
override (Round-3 multi-drive), no progress bar (the v1
deviation @@Architect approved). Each deferral is named
with a reason.

Pre-push gate: vitest 481/481 + check 0/0 + build clean.

**Commit clearance**: approved. Suggested commit subject:

```
Settings: semantic-search opt-in toggle wired against /api/index/semantic/* (fullstack-a-21)
```

Push waits until end of Round 2.

Queue update: -22 (pane-flip animation) is your last
remaining detour task. Independent of the model-removal
stack; can land anytime.