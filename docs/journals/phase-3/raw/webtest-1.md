# webtest-1: Phase 3 web service and browser smoke

Owner: @@Webtest.

Status: REVIEW.

Related:

- [request.md](./request.md)
- [journal.md](./journal.md)
- [frontend-1.md](./frontend-1.md)
- [frontend-2.md](./frontend-2.md)
- [frontend-3.md](./frontend-3.md)

## Goal

Own the phase 3 live web test service and browser smoke coverage.

As of 2026-05-16, @@Architect carved a second smoke lane to @@WebtestB
in [webtest-2.md](./webtest-2.md). @@Webtest retains service ownership,
baselines on rebuild, backend smoke, fixture-drive maintenance, the
narrow-viewport pass, and follow-through on smokes already in flight.
@@WebtestB takes frontend-1 surfaces, graph filters, PathPromptModal
Tab-complete, full multi-surface resource-color sweep, and continued
image-cluster repro attempts. Split details and "already smoked, no
need to redo" list are recorded in [webtest-2.md](./webtest-2.md).

## Responsibilities

- Start and maintain a test service URL for Alex/manual inspection.
- Consolidate duplicate reload/restart requests from other agents.
- Record URL, command, PID/session ownership, fixture path, and restart notes.
- Run desktop and narrow browser smoke for phase 3 UI behavior.
- File blockers back to @@Architect via task files.

## Smoke coverage targets

- Agent terminology and agent-specific banner.
- Agent overlay Cmd+F searches the current chat session and navigates matches.
- Agent Cmd+I from selected editor text inserts the quote and places the caret
  after the quote.
- Status bar event click opens the matching overlay.
- Reloading URL restores active screen/overlay/resource.
- Layout setting standard/compact behavior.
- Dashboard appears behind tabs.
- Document Cmd+F Enter cursor placement.
- File Browser Cmd+F over expanded visible entries.
- File Browser right-click context menu placement near clicked row/label.
- New-file tab completion.
- New file/folder and rename/move path prompt Tab completion, including
  directory trailing `/` and suggested `.md` filename in new-file flow.
- Graph filters/scope options and resource colors.
- Image/list guide and cursor-height regressions.

## Test expectations

- `cd web && npm run check`.
- `cd web && npm test -- --run` if available/relevant.
- Browser smoke at desktop and narrow viewport.
- Backend health/API smoke for any changed endpoints.

## Progress notes

### 2026-05-16 — service up, awaiting implementation slices

Live test service for phase 3 is up. Backend (chan debug binary) +
Vite dev frontend with HMR; Vite proxies `/api` and `/ws` to the
chan-server at 8787 (see [vite.config.ts](../web/vite.config.ts)).

| Service     | URL                          | PID    | Log                              |
|-------------|------------------------------|--------|----------------------------------|
| chan-server | http://127.0.0.1:8787/       | 81899  | /tmp/chan-phase3-logs/server.log |
| Vite dev    | http://127.0.0.1:5173/       | 40674  | /tmp/chan-phase3-logs/vite.log   |

(chan-server PID 40529 was rebuilt + relaunched as 81899 at 2026-05-16
to pick up [backend-3](./backend-3.md). Same command, same fixture,
same port; auth still `--no-token`.)

Alex should open **http://127.0.0.1:5173/** — Vite serves the SPA
and HMR picks up frontend edits live; backend changes need a
rebuild + relaunch by @@Webtest. The 8787 URL is the same app
but with the bundle frozen at build time, useful for confirming
behavior matches what gets shipped.

Commands:

- chan-server: `target/debug/chan serve /tmp/chan-phase3-drive --host 127.0.0.1 --port 8787 --no-token --no-browser`
- Vite: `cd web && npm run dev`

Auth: `--no-token` is set, so no bearer needed. The fixture drive
is loopback-only.

Fixture drive: `/tmp/chan-phase3-drive`. Layout covers the phase-3
smoke surface:

- Markdown notes incl. Cmd+F / indent / list-image / hashtag cases
  (`notes/`, `projects/phase3/`).
- Two contact flavors: `chan.kind: contact` and `@@mentions`
  (`contacts/`, `inbox/mentions.md`).
- Media: image (png/jpg), audio (mp3), video (mp4) under `media/`.
- Binary: zip, tar.gz, .bin, executable .sh under `binary/`.
- Nested folders deep enough to exercise GitHub-style folder icons
  (`archive/2025/q4/`, `projects/phase3/research/`).
- Multi-md SCOPE case: `projects/phase3/research/notes-a.md` +
  `notes-b.md` share `projects/phase3/research/` as first common
  ancestor; together with `overview.md` the common ancestor is
  `projects/phase3/`.

### Baselines (pre-implementation)

Captured at /tmp/chan-phase3-logs/ for regression comparison once
phase-3 work lands.

- `cd web && npm run check` -> 3911 files, 0 errors, 0 warnings
  ([baseline-check.log](file:///tmp/chan-phase3-logs/baseline-check.log)).
- `cd web && npm test -- --run` -> 9 files, 111 tests, all green
  ([baseline-test.log](file:///tmp/chan-phase3-logs/baseline-test.log)).
- chan-server `/api/health` -> `{"status":"ok"}` via both 8787
  direct and the 5173 Vite proxy.

### Notes for other agents

- @@Frontend: edits in `web/src/` HMR-reload automatically. If a
  reload looks stale, hard refresh (Cmd+Shift+R). For backend
  changes ping this task file or open a follow-up; I'll consolidate
  rebuild/restart requests rather than racing on `cargo build`.
- @@Backend / @@Rustacean: when API behavior changes, drop a note
  here (or a `webtest-N.md`) listing the changed endpoints and any
  fixture content that needs to land before I re-smoke. I'll rebuild
  the chan binary, relaunch, and post the new PID.
- @@Architect: status-bar event tests need a stream of real events.
  Once indexing kicks in, the embedded BGE model fetches from
  HuggingFace on first launch (see server.log "seed-models" line).
  Subsequent restarts reuse the cache, so first-launch latency is
  one-off. Flag this only if it bites first-time Alex.

### Known background warnings (benign)

- tokei "Unknown extension" warnings on zip/gz/bin/png/jpg/mp3/mp4
  in server.log are the line-counter declining non-source files,
  not errors.

### Awaiting

- Smoke coverage against the targets in this file is blocked on
  the first frontend/backend slice that touches each area. I am
  idle and watching for slice-ready signals in
  [journal.md](./journal.md) or task files from
  @@Frontend / @@Backend.

### 2026-05-16 — frontend-2 REVIEW browser smoke

Re-baseline after the frontend-2 slice landed (per
[frontend-2.md](./frontend-2.md)):

- `npm run check` -> 3917 files, 0/0 (was 3911 before, also clean).
- `npm test -- --run` -> 13 files / 145 tests pass (frontend-2 note
  said 130; assumes more landed after that note). Logs at
  /tmp/chan-phase3-logs/check-after-frontend-2.log and
  test-after-frontend-2.log.

Browser smoke summary at 1440x900 desktop viewport on Chrome (via
claude-in-chrome over the Vite dev server). Narrow-viewport pass
still owed.

| Landed slice | Smoke result |
|--------------|--------------|
| Cmd+F Enter cursor placement (editor) | PASS — fixture: indent-bug.md. Triggered FindBar via `chan:command` `app.find.open` (browser owns native Cmd+F per [shortcuts.ts](../web/src/state/shortcuts.ts) `web` build note). Counter advanced 1→2 of 11 on Enter; URL `c:[N,N]` confirms the editor caret moved to the start of each match (e.g. `c:[148,148]` = start of "Level" in "Level 1 item A"). Esc closed the bar; cursor stayed at the navigated offset. |
| File Browser Cmd+F over expanded/visible entries | PARTIAL — bar opens with placeholder "Find in visible entries". Query scoping is correct: `notes-` returns "0 of 0" because `notes-a.md` / `notes-b.md` are under collapsed `research/`; `sample` returns "3 of 3" across binary/. **BUG-FE2-A** below blocks the rest. |
| Multi-level indent hang on long-sentence wrap | PASS — fixture: indent-bug.md, Level 3 item A.1.1 with a 3-row wrap. Continuation rows align under the bullet content, NOT the gutter; Level 3 item A.1.2 stays at depth 3. |
| List-guide auto-hide after 1.5s | PASS — fixture: indent-bug.md. Guides visible while caret in a list line; after clicking into the paragraph below the list and waiting ~2s, the vertical guide bars completely fade. Minor wrinkle: after FindBar-driven cursor navigation back into a list line, the guides did not visibly re-appear; visibility may reset only on user editor input, not on programmatic selection. Flagging for @@Frontend, not blocking. |
| GitHub-style chevrons + folder icons | PASS — file tree shows lucide ChevronRight/ChevronDown next to folders and Folder/FolderOpen glyphs. Matches the GitHub-style reference. |
| File Browser right-click context menu placement | PASS — dispatched a contextmenu on a `list-image.md` row at clientX/Y center (page (934, 586)); the `.ctx` menu portaled to `<body>` rendered at exactly (934, 586), w=182, h=117. The `portal` action in [FileTree.svelte:620](../web/src/components/FileTree.svelte) correctly anchors the menu out of `OverlayShell`'s transformed `.panel`. Tested with the inspector pane visible. |
| Tabless background dashboard | PASS — Esc-to-dashboard on the test drive shows the new layout: enso logo, drive name "chan-phase3-drive", "13 files · 13 folders" stats, the "Each pane's visible tab is part of the scope for Agent and Graph." tagline, and three keyboard-shortcut sections (App, File, Tabs). Replaces the old shortcuts-only background without becoming marketing. |
| Resource colors (samples) | PASS for what I touched in DETAILS pane: BINARY pill blue (sample.bin), DOCUMENT pill orange (indent-bug.md), FOLDER pill grey (notes/). Backlinks chip for `projects/phase3/overview.md` rendered orange (markdown). Did not yet sweep contacts/media/tag colors across inspector + browser + search + agent + graph. |
| URL state reloadable | PASS for the surfaces exercised: file-browser overlay encodes `#files=1%3A<selected-path>`; editor encodes `#s={k:l,t:[{p,m,a,c}],f:1}` including tab path, mode, active flag, and caret range. Reloading restores the active tab and selection. |
| New-file / path-prompt tab completion | NOT YET — picking up next (new line 39-40 target). Will exercise Ctrl+Alt+N, rename, move with LCP completion, directory trailing `/`, and the suggested `.md` filename. |

### BUGS found in frontend-2 REVIEW

**BUG-FE2-A — File Browser find next/prev never advances.** With query
"sample" returning 3 matches, repeated Enter / Shift+Enter / ▼ / ▲
all leave the current at "1 of 3" / `sample.bin`. Confirmed via DOM:
the contextmenu defaultPrevented=true (handler fires) and
`document.querySelector('[aria-label="next match"]').click()` ×2 leaves
`.find-match--current` on `sample.bin`. Repro is stable across
browser_batch keypress and direct JS dispatch. Suspected cause: the
`$effect` in [FileTree.svelte:556](../web/src/components/FileTree.svelte)
that resets `findCurrentIndex = 0` after `findMatchPaths` recomputes is
firing again after `findStep`'s side effects, instead of only on a real
query change. Worth checking whether the
`findOnCount` callback in [FileBrowserOverlay.svelte:111](../web/src/components/FileBrowserOverlay.svelte)
or the `browserSelection.path = …` write in `findStep` triggers a
re-evaluation chain that re-enters the reset effect. Hand-off to
@@Frontend; not a blocker for the rest of the file-browser smoke.

**BUG-FE2-B (minor / web-only) — Cmd+F leaks keystrokes into doc.**
Per [shortcuts.ts](../web/src/state/shortcuts.ts), browser builds
intentionally let the browser own Cmd+F so chan's FindBar isn't bound
there. In automated browser drivers (and likely some embedded/popup
focus contexts), the native Cmd+F doesn't open the browser find UI,
and the modifier is silently ignored — the trailing characters get
typed into the editor instead. Reproduced accidentally: after pressing
Cmd+F and typing "Level" while focused in `indent-bug.md`, "Level"
was inserted between "whi" and "ch", producing "whiLevelch" (now
persisted to disk). Not a frontend-2 regression — it's the documented
web-build behavior. Filing as a UX risk for non-desktop hosts where the
native find UI may not appear. The `app.find.open` `chan:command`
bridge is the correct way to drive the editor FindBar in tests.

### Deferred cluster (image/cursor/selection) — re-smoked after drawSelection

@@Frontend pulled the drawSelection() fix forward in
[frontend-2.md](./frontend-2.md) per the 2026-05-16 journal entry. Re-ran
the cluster against
[projects/phase3/list-image.md](file:///tmp/chan-phase3-drive/projects/phase3/list-image.md)
(200x150 PNG fixture + "Repro variant" section).

- **Stale blue selection rectangles around image/list — RESOLVED.**
  Drag-selected from above the image down through "Step three…"; the
  selection rendered as one clean band, image included. Clicked into
  "Plain paragraph" below to move caret: selection cleared completely,
  image went back to its normal color, no residual rectangle around the
  image. Repeated with a click+drag starting INSIDE the image (which
  selects the image widget for a moment, dark blue outline); clicking
  away cleared the outline cleanly.
- **Cursor height inherited from image on previous line — RESOLVED.**
  Caret on "Let's switch to the next step." (offset 457, the line right
  after the image-bearing bullet) measured **height=19px** via
  `.cm-cursor` getBoundingClientRect — same as every other text line.
  Image line itself measures ~159px tall, but does not propagate height
  to the next line.
- **Image-line guide bars breaking around images — INCONCLUSIVE.**
  list-image.md is a single-level bulleted list; no `.cm-md-list-line`
  vertical guides render at this depth, so there's nothing to "break".
  Would need a deeper nested list with an embedded image to confirm.
  Flagging for @@WebtestB / @@Syseng to extend the fixture if they want
  to settle this; happy to seed a `nested-image-list.md` on request.

Baselines after this round (frontend-1 REVIEW + syseng-frontend-2 REVIEW
+ frontend-b-2 REVIEW landed since the last entry):

- `cd web && npm run check` -> 3918 files, 0/0
  ([check-after-frontend-1.log](file:///tmp/chan-phase3-logs/check-after-frontend-1.log)).
- `cd web && npm test -- --run` -> 14 files / 166 tests pass (was 13/145)
  ([test-after-frontend-1.log](file:///tmp/chan-phase3-logs/test-after-frontend-1.log)).
- chan-server `/api/health` still ok; no rebuild needed (no backend
  changes landed, backend-3 still pending).

### 2026-05-16 — backend-3 + syseng-frontend-1 smoke

Rebuilt chan and relaunched on :8787 to pick up
[backend-3](./backend-3.md). New PID 81899.

**backend-3 (`editor.line_spacing` config) — PASS.** Via CLI:

```
chan config get editor.line_spacing       -> standard       (default)
chan config set editor.line_spacing tight -> echoes "tight", get returns "compact"   # serde alias works
chan config set editor.line_spacing compact -> compact
chan config set editor.line_spacing standard -> standard
```

Legacy `tight` alias correctly normalizes to `compact` on read.
Default is `standard`. Frontend wiring (Settings overlay
standard/compact radios per [frontend-1.md](./frontend-1.md)) still
owed for end-to-end smoke; CLI/config layer is good.

**syseng-frontend-1 image-line guide cap — PASS.** Re-smoked
list-image.md after the fix. Vertical list guide bars now render as
short text-height segments anchored at each list line's bottom; the
image-bearing line ("- Step two with image:") no longer renders a
"chunky" ~160px guide bar through the image's region. Screenshot in
session; happy to attach if useful.

### Cross-references to other agents' findings

- @@WebtestB reported in [webtest-2.md](./webtest-2.md) that Esc in
  overlay find bars bubbles up and closes the whole overlay (not just
  the find bar). @@Webtest observed the same earlier (screenshot
  ss_0004j8ht5); the active owner is @@Syseng on
  [syseng-frontend-3.md](./syseng-frontend-3.md), bundled with
  BUG-FE2-A. No duplicate report from this side.

### 2026-05-16 — syseng-frontend-3 browser validation

Re-baseline after the fix:

- `cd web && npm run check` -> 3918 files, 0/0
  ([check-after-sf3.log](file:///tmp/chan-phase3-logs/check-after-sf3.log)).
- `cd web && npm test -- --run` -> 14 files / 168 tests pass (was 166)
  ([test-after-sf3.log](file:///tmp/chan-phase3-logs/test-after-sf3.log)).

| Acceptance | Result |
|------------|--------|
| File Browser next/previous advances through all visible matches | **PASS**. Fixture: `binary/sample*`. Enter cycles `sample.bin → sample.tar.gz → sample.zip → wrap → sample.bin`; Shift+Enter wraps `sample.bin → sample.zip`. Counter (`1 of 3 → 2 of 3 → 3 of 3 → 1 of 3`) and `.find-match--current` DOM class both advance correctly. **BUG-FE2-A resolved.** |
| Enter / Shift+Enter in File Browser find input step forward/back | **PASS** (covered above). |
| Esc in File Browser find input closes only the find bar | **PASS**. With find bar open + sample.zip selected, Esc clears the find bar but the File Browser overlay stays open with sample.zip still highlighted. Previously Esc bubbled up and closed the whole overlay (see ss_0004j8ht5). |
| Esc in Agent find input closes only the Agent find bar | **NOT VALIDATED — needs configured LLM backend.** `app.assistant.toggle` is gated on `drive.info.preferences.assistant.effective_enabled`, which requires an active LLM backend on this dev drive. No backend configured for the phase-3 fixture (no API keys), so the `.assistant-shell` never mounts. Code path now mirrors `FileBrowserOverlay.svelte::onFindKeydown` per @@Syseng's notes; route Agent-overlay browser validation to @@WebtestB if they have a configured backend, or to me if Alex wants me to seed one. |
| Agent overlay Cmd+F has no `effect_update_depth_exceeded` loop | **NOT VALIDATED in browser — see above.** The 168-test vitest run is clean (was 166 before; +2 new) and `npm run check` is clean, which covers the split-effect refactor at the type/test layer. Same routing note. |

### 2026-05-17 — syseng-frontend-4 Settings layout + narrow viewport

Re-baseline after syseng-frontend-4:

- `cd web && npm run check` -> 3918 files, 0/0
  ([check-after-sf4.log](file:///tmp/chan-phase3-logs/check-after-sf4.log)).
- `cd web && npm test -- --run` -> 14 files / 168 tests pass
  ([test-after-sf4.log](file:///tmp/chan-phase3-logs/test-after-sf4.log)).

**syseng-frontend-4 — full PASS.** Cmd+, opens the Settings overlay;
Layout section shows two radios labelled **Standard** and **Compact**
(no "Tight" anywhere). Default is Standard. Switching to Compact
visibly tightens the editor: `.cm-line` height goes from 28.797px
(standard) to 26.398px (compact), heading line 68.188 → 63.391, while
the image-bearing list line stays image-driven (~158/157px). Reload
preserves Compact (radio still checked, line heights still 26.4).
Legacy `tight` config via CLI (`chan config set editor.line_spacing
tight` → `get` returns `compact`) reloads as Compact in the UI with the
26.4px compact density. Restored to Standard after.

**Narrow viewport pass at 414x800 (iPhone-class portrait).**

- Editor / list-image.md renders with the same hang-indent on numbered
  list wraps ("Cursor height on the line after an embedded / image
  should match" wraps cleanly under the bullet content); image fits
  inside the column; status bar visible at bottom.
- File Browser overlay (Cmd+P) shows the side-by-side tree + DETAILS
  panes scaled into the narrow viewport.
- File Browser Cmd+F + "sample" works at narrow: find bar fits the
  tree column, counter shows "1 of 3", sample.bin highlighted, ▲ / ▼
  controls fit, DETAILS shows BINARY pill (blue) + sample.bin
  metadata.
- Esc once closes the find bar (file browser stays open); Esc again
  closes the overlay (back to editor) — same staged behavior as
  desktop.

### Agent-overlay validation — still owed (handoff)

For [webtest-3.md](./webtest-3.md) the Agent-overlay items still need
a configured backend. Setup attempt summary for whoever picks it up
(@@WebtestB, or me on request):

- `claude` is on PATH at `/Users/fiorix/.local/bin/claude`;
  `gemini` at `/opt/homebrew/bin/gemini`; `codex` at `/opt/homebrew/bin/codex`.
- Drive prefs read via `GET /api/drive` show `assistant.effective_enabled:
  false`, `assistant.claude_cli.enabled: false`, and a stale
  `cmd_override` pointing at a dev-test artifact
  (`target/debug/deps/chan_server-fb6ebd0fbca510cd`) — not a real CLI.
- The Settings panel's AGENT section only exposes Agent CLI selection
  and a `cmd_override` text field; there is no visible "Enable"
  toggle, so dropdown selection alone does not flip
  `claude_cli.enabled` to `true`.
- A bare `PATCH /api/drive` with
  `{"preferences":{"assistant":{"default_backend":"claude_cli",
  "claude_cli":{"enabled":true,"cmd_override":null}}}}` returns 200 but
  the server response body shows the preferences unchanged — the
  PATCH shape the backend expects is probably different (top-level
  config vs. drive prefs split). Worth a 5-min look from someone who
  knows the preferences API contract.
- Once enabled, `app.assistant.toggle` should mount `.assistant-shell`
  and the syseng-frontend-3 Agent-overlay items become testable.

## Commit readiness notes

- N/A for @@Webtest setup; no source-tree changes yet. If smoke
  uncovers a regression I introduce a fixture/test for, I will
  open a separate task for commit coordination.
- Final service teardown is complete in [webtest-5.md](./webtest-5.md).
  Ports 5173 and 8787 are unbound; no webtest-owned services remain.
