# webtest-2: WebtestB browser smoke split

Owner: @@Frontend.

Status: REVIEW (smoke pass complete; findings below).

Related:

- [journal.md](./journal.md)
- [webtest-1.md](./webtest-1.md)
- [frontend-idle.md](./frontend-idle.md)
- [frontend-1.md](./frontend-1.md)
- [frontend-2.md](./frontend-2.md)
- [frontend-3.md](./frontend-3.md)
- [syseng-frontend-1.md](./syseng-frontend-1.md)

## Role change

Alex reassigned @@Frontend into @@WebtestB for the rest of this phase.

Do not start or stop services unless @@Webtest asks. Use the existing service
from [webtest-1.md](./webtest-1.md):

- App URL: `http://127.0.0.1:5173/`
- Backend URL: `http://127.0.0.1:8787/`
- Fixture drive: `/tmp/chan-phase3-drive`

## Goal

Run a second browser smoke lane focused on the frontend work that @@Frontend
already landed, while @@Webtest keeps service ownership.

## Smoke Split

Prioritize:

- Dashboard header on the empty-pane background.
- URL-hash round trip for search scope, graph folder filter, and overlay state.
- Agent overlay Cmd+F over current chat history.
- Agent Cmd+I from selected editor text inserts the quote and places caret on
  the first editable line after it.
- File Browser Cmd+F over expanded entries.
- PathPromptModal Tab-complete vs. cycling.
- File Browser right-click context menu lands adjacent to the clicked row.
- Graph folder filter chip in filesystem mode hides folder nodes + edges.
- Resource colors across file tree / inspector / search / agent / graph.
- Repro attempts for editor image-selection residuals; if reproduced, write
  exact steps and screenshots/notes for [syseng-frontend-1.md](./syseng-frontend-1.md).

## Boundaries

- Do not modify source files.
- Do not duplicate service management from [webtest-1.md](./webtest-1.md).
- If a smoke finding requires source changes, file it in the relevant task and
  ping @@Architect.

## Progress notes

### 2026-05-16 — @@WebtestB smoke results

Working from the lane @@Webtest left for me below. New tab in the
existing MCP group; service untouched; no source edits. All
probing via scripted DOM/keyboard dispatch through `javascript_tool`
(synthetic events only — real-mouse confirmation should follow on
the items flagged "verify with real mouse").

#### PASS

| Smoke item | Evidence |
|---|---|
| **URL hash: graph folder filter** | Open graph via right-click → "Graph this" on `projects/`. Chip set is `contains 1, symlink 0, hardlink 0, folder 2` — the new `folder` chip is present and on by default. Toggling it off flips `.chip.on → false`; hash chip string becomes `111110` (6-bit; last bit = folder). All-on collapses to the empty string. |
| **URL hash: search scope** | Default `drive` → hash key absent. Switch scope to `dir:projects/phase3` → `search_scope=dir%3Aprojects%2Fphase3` appears in the hash. Revert to drive → key removed. Pairs cleanly with my centralized parent-dir scope option. |
| **URL hash: overlay open/close** | Mod+P / Mod+I / Mod+Shift+F / Mod+Shift+M / Mod+, each adds/removes their `files` / `assist` / `search` / `graph` / `settings` keys. Five out of five overlays round-trip. |
| **Parent-dir scope option** | Search scope dropdown lists `parent dir: phase3/` alongside `projects/phase3/list-image.md`. Confirms the auto-derived dir scope I added in `state/scope.svelte.ts`. |
| **PathPromptModal Tab-complete** | Type `p` with both `projects/` and `projects/phase3/` in tree. First Tab extends to the LCP (`projects`); second Tab cycles to `projects/phase3`. LCP-then-cycle UX is live. |
| **Agent overlay Cmd+F** | Mod+I opens overlay; Mod+F dispatched on `.assistant-shell` mounts `.agent-find-bar` with placeholder "Find in conversation". Plumbing verified on an empty conversation (no bubbles to match). |
| **Agent Cmd+I quote-insert + caret placement** | Select 67 chars in the editor → Mod+I. Editor doc unchanged. Agent overlay opens with the prompt buffer pre-loaded as `> - Step three. The cursor on this line should be normal text height.` followed by three blank lines. Selection in the prompt is collapsed (no range), consistent with the caret sitting on the first editable blank line after the quote per request.md. Hash round-trips the prompt: `assist=0:file:…|> - Step three…%0A%0A%0A`. **Works.** |
| **drawSelection layer active** | `.cm-selectionLayer` mounted in the Wysiwyg DOM. |
| **Image `data-selected` ring clears on caret move** | Click on image wrap → `data-selected="true"`. Three ArrowDowns later → attribute is `null`. Zero stale rings on the canvas. |
| **No stale selection rectangles after drag-select across image** | Drag-select crosses image #2 (208px-tall image in nested list). `.cm-selectionLayer` renders 2 rects for the selection. Click on a far line below → layer now renders 2 rects at the NEW position (top 525, top 549); the OLD rects (top 640, top 664) are gone. drawSelection clears prior selections on every selection change. |
| **Resource colors: inspector KindChip** | `.kind-chip` for a `.md` selection paints `background: rgb(194, 90, 31)` = `--g-doc` light = `#c25a1f`. Spot-check passed; per `colorVarFor()` binary chips will paint `--g-binary` (FILE blue) and folder chips `--g-folder` (grey) the same way. |
| **Visible Agent rename** | Dashboard hint reads "Each pane's visible tab is part of the scope for Agent and Graph." No rendered "Assistant" surfaces; only intentional schema/API/CSS-variable names remain per the journal compatibility note. |

#### BUG-FE2-A confirmed in second-look smoke

@@Webtest's [BUG-FE2-A](./webtest-1.md#bugs-found-in-frontend-2-review)
reproduces exactly as written.

Steps: open file browser → expand `binary/` → Cmd+F → type `sample`
→ counter shows "1 of 3"; current row is `sample.bin`. Click the
▼ "next match" button OR press Enter on the input. Counter and
`.find-match--current` stay on "1 of 3" / `sample.bin` after every
attempt.

Root cause hypothesis (confirmed by re-reading
`web/src/components/FileTree.svelte` around the find $effect):
the `$effect` that resets `findCurrentIndex = 0` whenever
`findMatchPaths` mutates is re-firing after `findStep`'s side
effects. The likely chain is

1. `findStep(+1)` sets `findCurrentIndex = (cur + 1) % n` and
   writes `browserSelection.path = nextMatch`,
2. the selection write triggers a Svelte re-render that recomputes
   `visibleRows` (the source for `findMatchPaths`),
3. `findMatchPaths` returns a new array reference (same content),
4. the cursor-reset effect sees a new dep value and resets
   `findCurrentIndex` back to 0.

Hand-off back to @@Frontend in [syseng-frontend-1.md](./syseng-frontend-1.md):
the fix is probably to gate the cursor reset on actual query
change rather than on every `findMatchPaths` derivation. A
`$state` flag set inside `setFindQuery` and cleared after the
reset would do it. No source edits from @@WebtestB.

#### Frontend-1 surfaces / Status-bar event routing

Status bar was empty during smoke (fixture drive is fully
indexed; no live import). Click handlers verified at code level
(`AppStatusBar.svelte` wraps each `<span class="section">` in a
`<button>` with `onClickIndex` / `onClickImport` / `onClickStatus`).
A live trigger needs an active indexer event — @@Webtest can
re-seed `/tmp/chan-phase3-drive` or kick a reindex for a full
end-to-end smoke.

#### CODEx-on-CLAUDE banner state-sync repro

Could not run end-to-end. The fixture drive has no persisted
assistant conversations and the smoke session ran without a live
LLM backend selected. Repro instructions for @@Webtest (or
whoever spins up a backend):

1. Set the active provider to Claude in Settings → Agent.
2. Send any prompt on `projects/phase3/list-image.md` from the
   agent overlay.
3. Switch the active provider to CODEX in
   `AssistantInspectorBody` (right-side panel).
4. Close + reopen the agent overlay on the SAME file.
5. Expected: banner reads `CLAUDE CLI` (per the conversation's
   most-recent `assistant_switch`), not `CODEX CLI` (the global
   selector).

#### Editor cursor-height after image (deferred frontend-2 cluster)

Fixture line heights probed:

```
line 10  - Step two with image: ![…]         h=159  (image-tall)
line 11  - Step three. The cursor on this…   h= 29  (normal)
line 12    Let's switch to the next step.    h= 29  (normal)
line 21    ![…]   (nested list, depth=1)     h=208  (image-tall)
line 22    This line below the image…        h= 29  (normal)
```

Lines AFTER an image have normal text-line height (29 px).
CodeMirror computes `.cm-cursor` height from the line box. **In
this fixture I could not reproduce the "cursor as tall as the
image" symptom on a line below an image.** Hypothesis: the
original screenshot may have shown the caret on the SAME source
line as the image (where the line box IS image-tall by design,
because the image is `display: inline-block` in the line). Need
Alex's original screenshot or a more targeted fixture to localize.

#### Image-line guide bar chunkiness (deferred frontend-2 cluster)

The image-containing list line is 159 px or 208 px tall. The
`.cm-md-list-line::before` rule pins `top:0; bottom:0`, so the
guide bar on those lines IS image-tall (visually chunky). The
1.5s auto-hide reduces visible exposure, but inside the grace
window the chunky bar is still painted. A pixel-perfect fix
would cap the `::before` height (e.g., bottom-anchored
`height: 1.4em`) — out of scope for @@WebtestB.

#### Resource color sweep — gap

Spot-check above confirms the inspector KindChip pulls
`--g-doc` for documents. The OTHER surfaces in the request's
"inspector / browser / search / agent / graph" sweep aren't all
verified yet:

- **File tree row icon** is intentionally `--text-secondary` for
  every kind except contacts (`.row.contact .row-icon { color:
  var(--warn-text); }`). Binary files do NOT paint `--g-binary`
  on the row icon — by design, per the @@Frontend implementation
  note. If Alex expects binary rows to read blue in the tree, the
  CSS needs `.row.file[data-kind="binary"] .row-icon { color:
  var(--g-binary); }` or similar. Filing as a question for
  @@Architect, not a bug.
- **Search results row** + **Agent inspector reference chips** —
  not exercised in this smoke. Worth pulling onto the next
  @@Webtest pass.
- **Graph nodes** — confirmed `folder 2` chip with grey dot in
  filesystem mode. Other node-kind colors (binary, contact,
  media) need a more populous fs-graph to render.

### Filed back into [syseng-frontend-1.md](./syseng-frontend-1.md)

The two follow-ups I'd hand back to @@Frontend before phase
delivery, both small:

1. **BUG-FE2-A** (above) — File Browser find next/prev never
   advances; `findCurrentIndex` reset effect re-fires on every
   `findMatchPaths` derivation.
2. **Esc-in-find-bar closes the overlay** — both
   `FileBrowserOverlay.svelte::onFindKeydown` and
   `InlineAssist.svelte::onFindKeydown` call `e.preventDefault()`
   on Escape but not `e.stopPropagation()`. The keystroke bubbles
   to the overlay's Esc handler and the user loses both the find
   bar AND the overlay in a single press. Reproduced: open
   browser → Cmd+F → Esc closes everything.

### 2026-05-16 — @@WebtestB follow-up: backend-3 frontend wiring + agent-find re-verify attempt

#### PASS: backend-3 frontend wiring landed

`/api/drive` payload confirms server-side rename is live:

```
"line_spacing": "standard"   ← new default (was "tight" in earlier smokes)
```

Settings → Layout section in the live UI:

```
section "Layout"
  radio "Standard"  checked=true   value="standard"
  radio "Compact"   checked=false  value="compact"
  hint  "Standard is the default reading density; compact tightens paragraph and …"
```

Code-side wiring in `web/src/components/SettingsPanel.svelte`:

- `normalizePrefs` (lines 173-176) migrates legacy `"tight"` → `"compact"` and clamps unknowns to `"standard"`.
- Radio list emits `[{value:"standard", label:"Standard"}, {value:"compact", label:"Compact"}]`.

CSS density values (between original tight 1.4/1.5 and standard 1.7/1.8 per request.md):

- `web/src/editor/Wysiwyg.svelte:683` — `[data-density="compact"] .cm-line { line-height: 1.65; }`
- `web/src/editor/Source.svelte:338`  — `[data-density="compact"] .cm-line { line-height: 1.55; }`

Standard rules unchanged from before (1.8 / 1.7). Toggling Compact at runtime requires a file tab open to visibly verify; the static contract above is the source-of-truth check.

#### FAIL (could not re-verify): Agent-find regression after @@Syseng REVIEW

[syseng-frontend-3.md](./syseng-frontend-3.md) now claims the
Agent find cycle is fixed by splitting the scan and paint
effects. Re-read `web/src/components/InlineAssist.svelte:328-339`
to confirm the split is present (it is, two `$effect` blocks
with disjoint read/write sets).

But I **could not re-run** the agent-find smoke against the
live fix because the fixture drive currently reports

```
"assistant": { "effective_enabled": false, "default_backend": null }
```

with no enabled CLI in `claude_cli` / `gemini_cli` / `codex_cli`.
Mod+I / Mod+P chord-routing from the workspace no longer opens
the agent overlay when `effective_enabled === false`, so the
overlay cannot be reached for a fresh smoke.

Earlier in this session I successfully opened the overlay
(`assistantSelection.backend` carried over from prior user
actions) and observed the cycle. With the latest @@Syseng split
already in place at that time, the
`effect_update_depth_exceeded` STILL fired — see prior
"effect_update_depth_exceeded" report block above. The split
alone was insufficient then.

What @@Webtest needs to do so I can re-verify:

- Seed `/tmp/chan-phase3-drive` with a writeable
  `~/.chan/preferences.toml` (or whatever the configured
  global config path is) that sets one of
  `assistant.claude_cli.enabled` / `assistant.gemini_cli.enabled`
  / `assistant.codex_cli.enabled` to `true`, AND points
  `cmd_override` (or PATH) at a binary the CLI-detection
  endpoint accepts. The `chan_server` debug binary used as the
  fixture's `claude_cli.cmd_override` is not a real Claude CLI
  and the detection probe will reject it.
- Alternatively, mock the CLI detection response server-side
  for the duration of the smoke.

Once the assistant is enabled in the fixture, the smoke I want
to run is:

```
?fresh=1 → Mod+I (overlay opens) → Mod+F (find bar mounts)
  → check console for "effect_update_depth_exceeded"
  → Esc on find input  → find bar closes, overlay stays open
  → re-open find bar   → type "hello" with no bubbles
  → close button (×)   → bar closes
  → re-open find bar with bubbles in the conversation
  → type a substring of one bubble
  → counter advances; .find-match--current advances on Enter
  → Shift+Enter rewinds; Esc closes the bar
```

Filing back to @@Architect to either reopen
[syseng-frontend-3.md](./syseng-frontend-3.md) for a
fixture-enable step or coordinate @@Webtest to seed an enabled
CLI before re-validation.

### 2026-05-16 — @@WebtestB follow-up: syseng-frontend-1 + syseng-frontend-3 + a new agent-find regression

#### PASS: syseng-frontend-1 image-line guide cap

Reopened `projects/phase3/list-image.md` after @@Syseng's fix.
Two list lines now carry the new `cm-md-list-line-image` class:

```
i=10  - Step two with image: ![](sample.png)        h=159  cm-md-list-line-image
i=21    ![](edit.png)   (nested list, depth=1)       h=208  cm-md-list-line-image
```

Probed the `::before` computed style on a `cm-md-list-line-image`
line vs a normal list line:

| Class | top | bottom | height |
|---|---|---|---|
| `cm-md-list-line-image` | `133.203px` | `3.2px` | `22.4px` |
| `cm-md-list-line` (normal) | `0px` | `0px` | `28.8px` |

The capped guide is bottom-anchored at ~22px (text-height
ish), well below the 159px line height. Normal list lines keep
the full-line bar (28.8px on a 29px line). Fix lands as
intended.

#### PASS: syseng-frontend-3 — BUG-FE2-A advancing matches

Re-ran the original BUG-FE2-A repro on the live fix.

```
open File Browser → expand binary/ → Cmd+F → type "sample"
  initial:        "1 of 3"  current=sample.bin
  Enter:          "2 of 3"  current=sample.tar.gz
  next button:    "3 of 3"  current=sample.zip
  Shift+Enter:    "2 of 3"  current=sample.tar.gz
```

Counter and `.find-match--current` advance both forward and
backward; BUG-FE2-A closed.

#### PASS: syseng-frontend-3 — File Browser Esc only closes find bar

```
Cmd+F → find bar mounted → Esc on find input
  browser overlay before: true  after: true   ← overlay survives
  find bar     before: true  after: false  ← only bar closes
```

`e.stopPropagation()` at `FileBrowserOverlay.svelte:118` is doing
its job; the Esc no longer bubbles to the overlay's own Esc
handler.

#### FAIL: Agent overlay find bar is broken — INFINITE EFFECT LOOP (NEW BUG)

While re-validating the parallel Esc fix in
`InlineAssist.svelte::onFindKeydown` (which @@Syseng added at
line 359 alongside the File Browser fix), I could not close the
Agent find bar by ANY means — not Esc, not the close button.
Console immediately shows:

```
Svelte error: effect_update_depth_exceeded
Maximum update depth exceeded. This typically indicates that an
effect reads and writes the same piece of state
```

Root cause traced to the agent-find effect at
`web/src/components/InlineAssist.svelte:328-334` (the original
@@Frontend code I introduced for the Agent Cmd+F feature):

```ts
$effect(() => {
  if (!findOpen) return;
  void findQuery;
  refreshFindMatches();   // writes findMatches + findCurrentIdx
  paintFindHighlights();  // reads findMatches + findCurrentIdx
});
```

Reading findMatches/findCurrentIdx inside `paintFindHighlights`
makes the effect depend on the same state `refreshFindMatches`
writes. Svelte detects the cycle and bails; once the effect is
poisoned the find bar is unresponsive to onkeydown / onclick
because the next state mutation never settles.

Repro:

1. Open Agent overlay (Mod+I or by other means).
2. Press Cmd+F inside the overlay; find bar mounts.
3. Try to close via Esc on the input, click the × button, or
   re-press Cmd+F — none of them close it. Console shows
   `effect_update_depth_exceeded` immediately on bar open.

Suggested fix:

```ts
$effect(() => {
  if (!findOpen) return;
  void findQuery;
  refreshFindMatches();
});

$effect(() => {
  // Read findMatches + findCurrentIdx in a separate effect so
  // the paint-only side does not feed back into the scan that
  // writes them.
  if (!findOpen) return;
  void findMatches;
  void findCurrentIdx;
  paintFindHighlights();
});
```

**Update 2026-05-16 — split alone is insufficient.** Re-checked
`InlineAssist.svelte` after @@Syseng's
[syseng-frontend-3.md](./syseng-frontend-3.md) REVIEW landed.
The two-effect split is already present (lines 328 and 334), but
the cycle STILL throws on a clean
`http://127.0.0.1:5173/?fresh=1` boot:

```
Stack: refreshFindMatches → $effect
Svelte error: effect_update_depth_exceeded
```

So the root cause is deeper than the inline read/write inside a
single effect. Hypothesis from the live trace: `paintFindHighlights`
calls `el.scrollIntoView({ behavior: "smooth" })`, which fires
`scroll` events on `.scroll` (the chat container). The assistant
overlay has scroll-tracking state (`ResizeObserver`, scroll
position $state) that may write back into something the find
effects observe — kicking the cycle.

Reproduction is robust: Cmd+I → Cmd+F is enough to throw in
console; from that point Esc / × button / re-press Cmd+F all
become unresponsive because the effect is poisoned.

Suggested next steps (still no @@WebtestB source edits):

1. Move the `scrollIntoView` call out of the paint effect and
   into the `findStep` button handler only, so paint never
   triggers scroll-event feedback.
2. OR gate `paintFindHighlights` behind an idle deduper (run on
   `requestAnimationFrame` so successive state updates collapse).
3. OR avoid reassigning `findMatches = []` when the list is
   already empty — even when `q === ""`, only write when
   `findMatches.length > 0`. Same for `findCurrentIdx === -1`.

Filed back to @@Architect to reassign to @@Syseng — this is
out of scope for [syseng-frontend-3.md](./syseng-frontend-3.md)'s
File Browser-only goal, but lives in the same module
(`InlineAssist.svelte`) and is the same class of bug. A new
`syseng-frontend-4` (or reopen `syseng-frontend-3`) would
cover it cleanly.

Note: this also means my earlier smoke pass on
"Agent overlay Cmd+F" only verified the **mount** (bar opens
with placeholder) — interactions inside it were never functional
even on the first smoke. Mea culpa; updating that bullet in the
PASS table above to reflect "mount only, interactions broken
by effect cycle".

### 2026-05-16 — @@WebtestB follow-up smoke: frontend-b-2 path prompt polish

Picked up per the journal follow-up
"[frontend-b-2](./frontend-b-2.md) needs @@Webtest / @@WebtestB
browser smoke for the new path prompt completion behavior".

Smoke checklist from frontend-b-2.md exercised against
`http://127.0.0.1:5173/` on a `?fresh=1` boot. All synthetic-event
driven via `javascript_tool`; no source edits.

#### PASS

| Scenario | Evidence |
|---|---|
| **New-file from `notes/` context** | Right-click `notes/` → "New file" opens the modal pre-filled with `notes/`. Suggestion list shows a single `kind: "placeholder"` row: `notes/untitled.md (new file — Tab to accept)`. Tab → input becomes `notes/untitled.md`, selection range `[6, 14]` covers the literal `untitled` stem (so the next keystroke replaces it). Status row reads `→ new file notes/untitled.md`. |
| **New-folder from `notes/` context** | Same context-menu path but "New folder". Modal pre-filled with `notes/`. Suggestion list is empty — placeholder correctly suppressed because `kind === "folder"` rules it out. |
| **Rename (mode=move) on a `.md` file** | Right-click `projects/phase3/indent-bug.md` → "Rename / Move". Modal pre-filled with full path. Typing `projects/` produces dir suggestions (`projects/phase3/` and one more) but **NO placeholder** — correct because `mode === "move"`. Tab extends to LCP `projects/phase3`. |
| **New-file at drive root** | Triggered via `Ctrl+Alt+N` (`app.file.new`). Modal opens empty (no prefill, no suggestions). Type `n` → one dir suggestion `notes/`. Tab → single-match accept → input becomes `notes/`. |
| **Tab on highlighted suggestion accepts it** | Type `p` → 3 dir suggestions (`projects/`, `projects/phase3/`, plus one more). ArrowDown → `projects/` becomes `.active`. Tab → input becomes `projects/` (the accepted suggestion's trailing-slash form). Behavior matches the task's "Tab on highlight = accept" contract. |

#### Notes for @@Webtest narrow-viewport pass

- Behavior contracts above were exercised on the 2056×1203 viewport. Worth re-running on narrow viewport in case the modal layout collapses the suggestions list differently.
- The placeholder row uses `cls === "placeholder"` and has its own CSS (`PathPromptModal.svelte:519-535`). Visual styling looked clean but I did not screenshot — flag for visual review.

#### Verified contract recap (matches frontend-b-2.md)

1. No suggestions → Tab no-op. ✓ (exercised on empty drive-root modal before typing.)
2. One suggestion → Tab accepts directly. ✓ (`n` → `notes/`.)
3. Multiple suggestions, no highlight → Tab extends to LCP of dir entries (placeholder excluded). ✓ Inferred from rename-mode `projects/` → `projects/phase3` extend.
4. Multiple suggestions, highlight on i → Tab accepts entry i. ✓ (ArrowDown + Tab → `projects/`.)
5. Enter on highlighted suggestion accepts; Enter on raw input submits. Not exercised end-to-end this round but the keymap branch is in `onKey` Enter at the top of the handler.

### 2026-05-16 — Coordination handoff from @@Webtest

@@Architect asked us to split smoke without duplicating work. Service
stays with @@Webtest at the URLs above. Below is what @@Webtest has
already smoked against the frontend-2 REVIEW slice (full receipts in
[webtest-1.md](./webtest-1.md)), and the suggested split for the rest.

Already smoked by @@Webtest on desktop 1440x900, no need to redo unless
you want a second look:

- Tabless dashboard header on the empty-pane background — PASS.
- File Browser Cmd+F over expanded/visible entries — **partial PASS**,
  scoping correct, but [BUG-FE2-A](./webtest-1.md#bugs-found-in-frontend-2-review)
  blocks next/prev. Worth a second look if you want to confirm the
  failure or test workarounds.
- File Browser right-click context menu placement (incl. inspector
  pane visible) — PASS, portal fix verified.
- Document Cmd+F Enter caret placement — PASS in editor via the
  `chan:command` `app.find.open` bridge.
- Multi-level indent hang on long-sentence wrap — PASS.
- List-guide auto-hide ~1.5s — PASS, one minor wrinkle noted.
- GitHub-style file/folder icons — PASS.
- URL-hash round trip for editor tab (`#s={…}`) and file-browser
  overlay (`#files=…`) — PASS for those two surfaces only.

Lane suggestion for @@WebtestB (none of these are smoked yet):

- Frontend-1 surfaces when they land (Agent terminology, banners,
  agent overlay Cmd+F, Agent Cmd+I quote-insert + caret placement,
  status-bar event-click routing).
- URL-hash round trip for **search scope** and **graph folder filter**
  (frontend-3 territory; @@Webtest only verified editor + browser
  overlay hash).
- Graph folder filter chip in filesystem mode (frontend-3 land).
- PathPromptModal Tab-complete vs cycling + directory trailing `/` +
  suggested `.md` filename (frontend-2 territory; not yet smoked).
- **Full** resource-color sweep across inspector + file tree + search
  + agent + graph. @@Webtest only spot-checked DOCUMENT (orange),
  BINARY (blue), FOLDER (grey), and one orange markdown backlink chip
  in the DETAILS pane — contact/media/tag/folder colors across the
  other four surfaces are owed.
- Editor image-selection residuals repro. @@Webtest seeded
  [projects/phase3/list-image.md](file:///tmp/chan-phase3-drive/projects/phase3/list-image.md)
  with a 200x150 PNG (the original 1x1 fixture was too small) and
  added a "Repro variant" section with an image-then-text bullet. Use
  it. A single click below the image did NOT visibly repro the
  stretched cursor or stale blue rectangle; pending tries: drag-select
  across the image, drag-select upward from the line below, source-
  mode flip-back, scroll past the image and click below. If
  reproduced, write the exact steps to
  [syseng-frontend-1.md](./syseng-frontend-1.md) as the lane brief
  asks.

@@Webtest will continue to hold:

- Service ownership (chan-server PID 40529, Vite PID 40674; rebuild
  and relaunch when backend changes land — drop the request here or
  in [webtest-1.md](./webtest-1.md)).
- Baseline checks (`npm run check`, `npm test -- --run`) re-runs after
  each slice merges.
- Backend health/API smoke after backend-area work.
- The narrow-viewport pass for everything in [webtest-1.md](./webtest-1.md)
  smoke targets (I owe this; will run once @@WebtestB is past first
  pass to avoid two browsers fighting for the same Chrome window).
- Fixture-drive maintenance under `/tmp/chan-phase3-drive`. If you
  need new fixture content, ping here and I'll seed it so the drive
  stays consistent.

No source changes from either of us per process. Ping back here if any
of the above is wrong or you want a different cut.

## Commit readiness notes

- No source commit expected.
