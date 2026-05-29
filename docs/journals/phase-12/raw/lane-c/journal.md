# @@LaneC journal (phase 12)

Append-only. @@LaneC self-documents here; @@Architect + @@Alex read.

## 2026-05-27 - Bootstrap + addendum-1 investigation

Recovered from `bootstrap.md` + `lane-c-plan.md` + my three channels.
Role: @@Alex ad-hoc frontend / cosmetics / keyboard shortcuts. Branch
`phase-12-lane-c` in worktree `../chan-lane-c` (NOT created yet - will set up
on green-light). Report merge-ready to @@Architect; he serializes.

### Request source

`docs/journals/phase-12/lane-c-addendum-1.md/` is an untracked DIRECTORY (not a
file) holding `draft.md` + `image.png` (a "Spawn agents" dialog screenshot).
@@Alex's draft lists 4 bugs across Rich Prompt + Editor. @@Alex then clarified
in chat (captured below). @@Alex chose "investigate all 4, plan first"; I am NOT
to execute until @@Architect green-lights.

### The 4 bugs + investigation findings

Each Agent-subagent investigation result is summarized. Confidence + scope noted
honestly: two have a solid static root cause, two need in-browser repro.

#### Bug 1 - Terminal glitch (Rich Prompt area)

- Draft framing: "agent in the Rich Prompt's terminal is only executed AFTER the
  bootstrap completes."
- @@Alex's live clarification (authoritative): "it happens when you click *away*
  from a terminal which is focused, and as the new pane receives focus, the old
  one glitches." So the symptom is a RENDER glitch on the focus->blur transition
  between panes, NOT a command-launch-ordering issue. The first Agent chased the
  bootstrap-ordering hypothesis (preflight monitor 8-line cap in
  `terminal_sessions.rs`); that hypothesis does NOT match what @@Alex sees and is
  set aside. Open question for repro: are these one bug or two.
- Surfaces: `web/src/components/TerminalTab.svelte`. xterm.js + WebGL renderer
  (`@xterm/addon-webgl`). The focus/blur lifecycle is already heavily worked:
  - focus effect (TerminalTab.svelte:264-295): queueFit + refreshTerminalRenderer
    + sendFocusState + term.focus().
  - blur effect (TerminalTab.svelte:297-301): `if (focused) return;`
    refreshTerminalRenderer + sendFocusState.
  - "Bug 6" active-flip effect (318-322): recoverTerminalRendererAfterHostResume.
  - refreshTerminalRenderer (425-437): rAF -> clearTextureAtlas + refreshTerminalRows,
    plus a document.fonts.ready repeat.
- Hypothesis (UNCONFIRMED): the blur-side repaint (single rAF + fonts.ready) is
  weaker than the focus-side recovery; when the OLD pane goes
  visibility:hidden / loses focus while the NEW pane simultaneously fits+repaints,
  the old pane paints at a stale size / partial atlas. Existing tests:
  `paneFocusClickRestore.test.ts`, `terminalResizeTrailingFit.test.ts`,
  `TerminalTab.renderer.test.ts`.
- VERDICT: needs in-browser reproduction (multi-pane terminal layout, click
  between panes, observe old-pane glitch, capture renderer state) before a fix.
  Located but not root-caused statically.

#### Bug 2 - Drafts paths unreachable by agents (Rich Prompt area)

- Symptom: staged "Launch Agents" prompts reference files with drive-relative
  paths like `Drafts/team-foo/docs/bootstrap.md`; a generic agent (claude/codex)
  resolves that against its CWD (the drive root) where it does NOT exist on disk,
  because Drafts lives OUTSIDE the drive root.
- Confirmed layout: Drafts is at `~/.chan/drives/<key>/drafts/` (outside drive
  root) - `crates/chan-drive/src/paths.rs:174` (`drafts: root.join("drafts")`),
  comment in `crates/chan-drive/src/drafts.rs:1-15`. The watcher relativizes
  events under the `Drafts/` unified prefix (`watch.rs:108-127`).
- The chan MCP tools DO resolve `Drafts/...`: `read_file`/`write_file`/
  `list_files`/`resolve_path` route through `Drive::resolve_io` /
  `resolve_physical_path` (`drive.rs:508-548`), and `prompts.rs:56-69` documents
  the `Drafts/...` namespace. So the plumbing already works VIA MCP - the agent
  just doesn't know to use MCP and tries the literal filesystem path.
- The generated prompt text: `web/src/state/teamOrchestrator.svelte.ts` builds
  the identity prompt pointing at `Drafts/team-{name}/docs/bootstrap.md`
  (`teamOrchestrator.svelte.ts:236,359`; asserted in `teamOrchestrator.test.ts:162`).
- ROOT CAUSE: prompt-text/guidance, not path plumbing. Fix = make the generated
  prompt instruct the agent to read `Drafts/...` via the chan MCP `read_file`
  tool (since those paths are not on its filesystem relative to CWD).
- DECISION FOR @@ALEX/@@ARCHITECT: confirm the approach is "teach the prompt to
  use MCP for Drafts/ paths" (recommended; matches the documented design) vs
  materializing docs somewhere filesystem-reachable. Surfaces:
  `teamOrchestrator.svelte.ts` prompt builder, possibly `chan-llm/src/prompts.rs`
  guidance. Confidence: HIGH on root cause, approach needs a one-line ratify.

#### Bug 3 - False "external edits to the file" (Editor)

- Symptom: constant "external edits" notifications while editing a draft, and
  right after pasting an image. False positives - the editor's OWN writes are
  detected as external.
- ROOT CAUSE (HIGH confidence): a timing race in self-write suppression.
  `state.self_writes.note(&path)` is called AFTER the blocking write completes,
  but the filesystem watcher runs concurrently and can deliver the fs event
  (checked against `self_writes.should_suppress` in `bus.rs:80-92`) in the window
  BEFORE `note()` registers the path. Event leaks through as an external edit.
- Sites where `note()` is called post-write:
  - `crates/chan-server/src/routes/files.rs:663` (api_write_file),
    `:734` (api_create_file), `:829` (api_upload_file).
  - `crates/chan-server/src/routes/attachments.rs:176` (api_post_attachment) -
    this is the image-paste path; write at `:164`, note at `:176`.
  - `crates/chan-server/src/routes/contacts.rs:1432` - verify timing.
- FIX: register the suppression BEFORE/INSIDE the blocking write (so the path is
  in the registry before the watcher can see the fs event), rather than after the
  task returns. Suppression KEY (path string) is correct; only the timing is
  wrong. Frontend (ConflictModal.svelte, tabs.svelte.ts, store.svelte.ts) is
  correct - do not touch.
- SCOPE NOTE: this is chan-server backend, broader than pure web/src cosmetics.
  Small, surgical diff. Suggest confirming the self_writes window
  (SELF_WRITE_WINDOW) is adequate but the real fix is ordering, not widening the
  window.

#### Bug 4 - `-` lists auto-convert to `*` (Editor)

- Desired: preserve the user's chosen marker. `-` stays `-`, `*` stays `*`,
  numbers enumerate `N. `. No auto-switch `-`->`*`.
- Investigation: NO static conversion site found.
  - Editor preserves markers: `nextPrefix` keeps `prev.marker` for bullets
    (`web/src/editor/commands/list.ts:62-70`); decorations only overlay a `•`
    glyph and leave source untouched (`decorations/blocks.ts:24-28` "tools see
    the unmodified marker"); `list.test.ts` even tests `*` stays `*`.
  - Toolbar toggle uses `- ` (dash), not `*` (`commands/format.ts` toggleBulletList).
  - Paste-HTML turndown uses `bulletListMarker: "-"` (`paste_html.ts:71`) - dash,
    correct direction.
  - chan-drive `write_text` does NOT transform content (no list normalization in
    `crates/chan-drive/src/markdown`).
  - NO remark/prettier/markdown-it formatter in `web/package.json` - no
    format-on-save pass.
- VERDICT: cannot reproduce the `-`->`*` conversion by static reading. Either a
  specific interaction I haven't traced, or @@Alex may be on a stale build (the
  marker-preservation code + test suggest this may be partly fixed already).
  NEEDS in-browser reproduction: type a `-` list, identify the exact action that
  flips it to `*` (toolbar? paste? mode switch? Enter?), then fix at that site.
  Per [[feedback_fresh_binary_rewalks]]: rebuild fresh + verify provenance before
  concluding.

### Cross-lane contention

My likely touches: `TerminalTab.svelte` (#1), `teamOrchestrator.svelte.ts` (#2),
chan-server `routes/{files,attachments}.rs` (#3), editor `commands/` + repro for
#4. @@LaneA owns GraphPanel/FileBrowser + graph state; @@LaneB's codemod
(drive->workspace) touches ~everything incl. these files. Will declare touches on
cross-lane channels and keep diffs small/scoped so they rebase across the codemod
window. None of my surfaces overlap @@LaneA's graph/FB scope directly.

### Status (initial investigation)

Initial investigation COMPLETE. Awaiting @@Architect review + green-light before
executing (per @@Alex 2026-05-27). Findings + a suggested execution order posted
to `event-lane-c-architect.md`. Not blocked on anything except the green-light;
will cut a task on the channel if I need @@Architect to unblock something.

## 2026-05-27 - Green-light + Slice 1 (Bug 3) DONE, ready-to-merge

@@Architect green-lit (event-architect-lane-c.md 12:05). Order: Bug 3 first
(lands before @@LaneB chunk-1), then Bug 2 (PROMPT/GUIDANCE only - @@Alex already
ruled; coordinate on event-lane-b-lane-c.md, shared teamOrchestrator.svelte.ts),
then Bugs 1+4 via scoped in-browser repro. Worktree `../chan-lane-c` created on
`phase-12-lane-c` from main (fe6e126).

### Slice 1 - Bug 3 (phantom "external edits"): `phase-12-lane-c@daed33a`

Root cause confirmed: `self_writes.note(&path)` ran AFTER the `spawn_blocking`
await; the fs watcher (separate thread) could reach `should_suppress()` in the
gap before the path was recorded, so the editor's own autosave + image-paste
writes echoed back as phantom external edits.

Invariant enforced: note() runs BEFORE or INSIDE the blocking write, never after
the await.
- `routes/files.rs` api_write_file, api_create_file: note before the spawn (path
  known up front; most robust, zero window). Noting on a write that then fails is
  benign + within the module's documented over-suppression trade-off.
- `routes/attachments.rs` api_post_attachment: note inside the closure BEFORE
  write_bytes (the explicitly-reported image-paste path; zero window).
- `routes/files.rs` api_upload_file: note inside the closure once the collision-
  free path is resolved, before the await (path computed in the blocking task).
- `bus.rs`: added a unit test locking the `event_is_self_echo` decision (path +
  rename-target match) the handlers depend on.

Gate GREEN (backend-only; no web touched): fmt --check clean; clippy
--all-targets -D warnings clean (workspace); cargo test full workspace pass
(chan-server 347 incl. new test); build --no-default-features clean.

DEFERRED (same race, outside the reported symptom): post-await note() also at
files.rs rename/delete (1431/1432/1434, 1570), drafts.rs (117/167/189/190/240),
rich_prompts.rs (197/198/232), contacts.rs:271, control_socket.rs:254. Proposed
as a follow-up slice if @@Architect wants the race closed everywhere.

EMPIRICAL NOTE: a thread-timing race is not deterministically unit-testable;
static root cause is airtight + gate green. Will smoke-test (edit draft + paste
image, watch for the phantom notification) during the Bugs 1+4 browser session
and flag if anything surprises.

Next: Bug 2 - open event-lane-b-lane-c.md, declare the teamOrchestrator.svelte.ts
touch + sequence vs @@LaneB chunk 0, then do the prompt/guidance fix.

### Slice 2 - Bug 2 (Drafts paths unreachable by agents): `phase-12-lane-c@cb1f113`

Cross-lane: @@LaneB had already slimmed chunk 0 to RUST-ONLY (b-c channel) and
explicitly cleared me to proceed on teamOrchestrator.svelte.ts without rebasing -
so no live overlap. Acked on event-lane-c-lane-b.md.

Root cause: the team-orchestrator `identityPrompt` told spawned agents to "read
Drafts/team-{name}/docs/bootstrap.md". A generic agent resolves that against its
cwd (the drive root), but Drafts/ workspaces live in chan metadata OUTSIDE the
drive root -> a plain filesystem read finds nothing. The chan MCP read_file tool
ALREADY resolves the Drafts/ namespace (chan-llm/src/prompts.rs:56-69); the spawn
prompt just never told the agent to use it.

Fix (PROMPT/GUIDANCE only, per @@Alex's ruling - no plumbing, no doc
materialization): `identityPrompt` now marks Drafts/-prefixed bootstrap paths as
"read ... with the chan MCP read_file tool (a Drafts/ path is a chan workspace
location, not a file under your working directory)". Plain drive paths (which DO
resolve relative to cwd) get no hint. Conditional on the `Drafts/` prefix so the
function degrades correctly for non-Drafts paths.

Files: web/src/state/teamOrchestrator.svelte.ts (+14/-1) + its test (updated the
verbatim-string assertion, added a Drafts-routes-through-MCP test + a
plain-path-gets-no-hint test).

Gate GREEN (frontend-only; Rust untouched, stays green from slice 1): web vitest
1598 passed / 11 skipped; svelte-check 0 errors / 0 warnings; vite build clean.

Next: Bugs 1 + 4 - need a scoped in-browser repro session (small /tmp scratch
drive, scoped port, fresh binary, scoped pkill). Will ask @@Alex about the test
drive per the test-server workflow before spinning one up. Bug 1 will touch
TerminalTab.svelte (declared intent on b-c).

### Browser repro session (Bugs 1 + 4) - fresh binary on scoped /tmp drive

Setup: built chan from the worktree (slices 1+2 baked in via fresh web/dist),
copied to /tmp/chan-lane-c-srv (verified 0.15.5), served /tmp/chan-test-lane-c
(seeded: dash list + asterisk list + numbered list + notes/) on port 8477,
--standalone --no-browser, scoped pkill. Drove via Claude-in-Chrome (own tab;
left @@Alex's :7799 instance untouched). Torn down after: server killed, drive
deregistered + rm'd, binary+log removed, port confirmed closed.

#### Bug 4 (dash -> asterisk): NOT REPRODUCED -> CLOSED as already-fixed

Tested 5 paths, marker preserved on disk in EVERY one:
1. Enter-continue a dash list -> `- fourth item`.
2. Type a dash list from scratch -> `- typed one`.
3. Nested indent (Tab) a dash item -> `  - typed two`.
4. HTML paste (turndown path) -> `- ` markers (bulletListMarker: "-").
5. Toolbar/shortcut toggle -> `- ` (static: format.ts toggleBulletList).
Markdown serialization is renderer-independent (same in web + desktop), so this
is fixed/absent on HEAD regardless of client; @@Alex was likely on a stale build.
@@Alex's call: CLOSE as already-fixed (declined a regression-guard test). No code
change for Bug 4.

#### Bug 1 (terminal focus-switch glitch): web=clean; fix targets WKWebView

Repro in web Chrome (Blink): two side-by-side terminal panes (split-right via
pane mode Cmd+. -> / -> t -> Enter), colored `ls` output in both, focus switched
both directions - the pane LOSING focus repainted CLEAN every time. No glitch, no
console renderer errors. @@Alex confirmed he sees the glitch in the DESKTOP app
(Tauri/WKWebView/WebKit), NOT a regular browser - a renderer the Chrome
automation can't drive.

### Slice 3 - Bug 1 (WKWebView blur repaint): `phase-12-lane-c@3b587a7`

Root cause (hypothesis, grounded in the existing renderer-recovery patterns + the
symptom): the blur effect (pane losing focus, TerminalTab.svelte:297) only ran a
single `refreshTerminalRenderer()`; the focus effect and the Bug-6 active-flip
effect both use the stronger `recoverTerminalRendererAfterHostResume()` (fit +
texture-atlas clear + delayed re-fits at 50/250ms). On Blink the single refresh
suffices; WKWebView leaves the blurred WebGL canvas half-updated and needs the
deferred repaint pass.

Fix: the blur effect now calls `recoverTerminalRendererAfterHostResume()` (same
recovery as host-resume/active-flip). Size is unchanged on a focus switch so the
fit is a dimensional no-op; the value is the delayed repaint. Updated the
TerminalTab.renderer.test.ts source-pin (blur now asserts the recovery call).

Files: web/src/components/TerminalTab.svelte (+13/-1) + its renderer test (+9/-1).
Gate GREEN (frontend-only): web vitest 1598 passed/11 skipped; svelte-check 0
errors; vite build clean. No-op regression verified on Blink (clean repaint in
the browser repro above).

CAVEAT: I CANNOT verify the WKWebView fix myself (no WebKit driver via Chrome
automation). @@Alex must verify in chan-desktop built from phase-12-lane-c. If it
doesn't fully resolve it, @@Alex captures a desktop screenshot/recording for a
targeted second pass.

### Round status (addendum-1)

All 4 addendum-1 bugs handled. Branch phase-12-lane-c: daed33a (Bug 3) + cb1f113
(Bug 2) + 3b587a7 (Bug 1). Bug 4 closed-as-fixed (no commit). Awaiting
@@Architect merge serialization + @@Alex desktop verification of Bug 1.

## 2026-05-27 (round-2) - "check your tasks": queue discovered + rebase + follow-up

@@Alex said "check your tasks." Swept the channels (per [[feedback_check_for_updates_surface]]):
- event-architect-lane-c.md: Bug 3 + Bug 2 MERGED to main (merge 34f0b3f);
  Bug 1 (3b587a7) HELD pending @@Alex desktop verify; Bug 4 closed. AUTHORIZED a
  self-write follow-up slice. Then a round-2 addendum-2 (ratified by @@Alex) with
  THREE LaneC items, queued AFTER the follow-up.
- event-lane-b-lane-c.md: @@LaneB chunk 0 (RUST-only "free workspace" rename)
  MERGED to main (6c5a2c6 / f72b8a7).
- @@LaneE (new) owns shortcuts; shares terminal area - coordinate on
  event-lane-c-lane-e.md (not yet created).

REBASE: my branch was behind main and the follow-up touches drafts.rs/
rich_prompts.rs which @@LaneB chunk 0 renamed. Rebased phase-12-lane-c onto main
(f72b8a7). daed33a+cb1f113 dropped (already in main via merge); the held Bug 1
replayed 3b587a7 -> e080829 (SAME change, new sha - flagged to @@Architect so he
merges e080829 on @@Alex's confirmation). 3b587a7/e080829 preserved, not rebased
away.

### Slice 4 - self-write follow-up: `phase-12-lane-c@14fb535`

Closed the SAME post-await note() race Bug 3 fixed, at every remaining write
handler. Invariant: note() runs BEFORE or INSIDE the blocking write, never after
the await.
- files.rs: api_delete_file (before spawn), api_move (from/to before spawn +
  rewritten inside the task), api_fs_transfer (all paths inside the task).
- drafts.rs: api_create_draft + api_create_rich_prompt (inside the task),
  api_discard_draft + api_promote_draft (before spawn).
- rich_prompts.rs: api_submit_rich_prompt (inside task), api_close_rich_prompt
  (before spawn).
- contacts.rs import (inside the import task); control_socket.rs open-create
  (before write_text).
Gate GREEN (backend-only): fmt; clippy --all-targets -D warnings (workspace);
cargo test full workspace pass (chan-server 347); build --no-default-features.
5 files, +112/-73.

### Round-2 queue (architect order, each its own gated slice)

1. [NEXT] EDITOR no-reload-while-typing (@@Alex #1): fs content change -> banner
   only, no auto-reload/update of the open doc; chmod -w -> LOCKED; never
   interrupt typing; + cmd+r window reload restores exact caret + editor focus.
2. TERMINAL recovery pass: fire recoverTerminalRendererAfterHostResume for ALL
   panes on display-wake / visibilitychange / host-resume. Folds in / reconciles
   the held Bug 1 (e080829) - extend or supersede, don't double up.
3. DRAG-DROP image move - easy case only (single row + bullet), defer paragraph.
Coordinate terminal-area touches with @@LaneE on event-lane-c-lane-e.md.

### Rebase onto @@LaneB chunk 1 (chan-drive -> chan-workspace) + corrected shas

@@Alex "poke" -> swept channels again. @@Architect heads-up: @@LaneB chunk 1 (the
chan_drive:: -> chan_workspace:: crate rename, 2140925) landed on main, rewriting
the SAME chan-server files my follow-up touches. Rebased phase-12-lane-c onto
2140925. Conflicts in drafts.rs (api_create_rich_prompt closure) + files.rs
(api_fs_transfer return) RESOLVED = my note-ordering logic + chan_workspace::
names; also fixed 3 auto-merged `chan_drive::ChanError` lines (files.rs api_move,
drafts.rs api_create_draft, rich_prompts.rs api_submit) -> chan_workspace::.
Re-gated: fmt; clippy --all-targets (workspace); cargo test (no failures,
chan-server 347); build --no-default-features. CORRECTED shas (rebase rewrote
them twice): follow-up 14fb535 -> 327960e; held Bug 1 3b587a7 -> e080829 ->
cddc578. Reported to @@Architect.

Cross-lane @@LaneE: replied on event-lane-c-lane-e.md - I'm NOT touching
TerminalTab's keydown / focus-tracking path (only the render-recovery $effects),
so @@LaneE owns the terminal-focus signal + the Linux ctrl+w key-bridge change.

### Post-merge rebase + Item 1 facet A

@@Architect cherry-picked the follow-up to main as e927e90 (Rust re-gate green).
Rebased phase-12-lane-c onto e927e90 - git dropped the merged follow-up by
patch-id, leaving only the held Bug 1 (now 0a9fb27, rebased again) on the branch.

Investigated item 1 (Explore agent): root cause of @@Alex's caret-jump =
`refreshTabFromDisk` (tabs.svelte.ts) silently reloads a CLEAN buffer on a watch
event -> `loadTabContent` replaces the doc + resets caret to 1:1. Happens right
after an autosave marks the buffer clean and an echo/external event lands.
fsWritable exists + drives read-only mode but only refreshes on full re-read (so
chmod -w isn't noticed live). Caret IS persisted to the URL hash (`c`) + restored
by maybeRestoreCaret with view.focus(); focus may not fire on a full reload when
the tab was already active. @@Alex confirmed the "Banner + Reload + Dismiss"
shape.

#### Slice (item-1 facet A) - no-reload + banner: `phase-12-lane-c@1222a5f`

The watch path (store.svelte.ts onWatchEvent) now calls a new
`flagExternalChange(tabId)` (sets per-tab `externalChange`) instead of
refreshTabFromDisk. FileEditorTab shows a dismissable "This file changed on disk."
banner (Reload opt-in / dismiss); the caret never moves unless the user reloads.
User-initiated file replace still reloads via refreshTabFromDisk (store:3115,
unchanged). loadTabContent clears the flag on any (re)load. Regression test added
(store.test.ts: watch event flags but does not reload/clear content).
Gate GREEN (frontend): vitest 1594 passed; svelte-check 0/0; build clean.

REMAINING item-1 facets (separate slices): B = chmod -w -> LOCKED (re-check
writability on watch; fsWritable currently only refreshes on full re-read);
C = cmd+r restores exact caret + focus (caret mostly works; focus is the gap).
Then item 2 (terminal recovery, folds in Bug 1 cddc578/0a9fb27) + item 3
(drag-drop image easy case).

#### Slice (item-1 facet C) - cmd+r caret + focus restore: `phase-12-lane-c@56b9dfc`

Rebased onto abac76c (facet A merged; facet A dropped by patch-id; Bug 1 ->
3fc3a65). Root cause: setTabCaret only sets tab.caret in memory; the URL hash's
`c` field only updates on layout changes, and window.location.reload() skips
component cleanup - so cmd+r restored a STALE caret, and with no real caret
maybeRestoreCaret bailed without re-focusing (focus lost too). Fix: call
persistLayoutToHash() in the beforeunload/pagehide handler (App.svelte
onUnloadFlushBuffers) so the latest caret is flushed to the hash before reload;
maybeRestoreCaret then restores it + re-focuses. One line + comment.
Gate GREEN (frontend): svelte-check 0/0; vite build clean. (No vitest change -
one-line wiring of the already-tested persistLayoutToHash; in-browser verify on
cmd+r covers it.) EDGE: caret at offset 0 is omitted from the hash, so a reload
from the doc top still won't re-focus; common non-top case works.

#### Facet B (chmod -w -> LOCKED) - DEFERRED, open questions

Surfaced to @@Architect rather than built on uncertain ground:
1. Does chan-drive's watcher even FORWARD a permission-only change (chmod -w with
   no content change)? If notify/FSEvents coalesces it away or the watcher filters
   to content events, facet B can't be watch-triggered at all (would need polling /
   on-focus re-stat). Needs an empirical check (chmod an open file, watch the ws).
2. Fetch mechanism: there is NO stat/HEAD endpoint - writable only comes via
   api.readStream's meta. Options: (a) frontend api.statWritable = readStream +
   abort after onMeta (wasteful aborted GET, frontend-only); (b) server: add
   `writable` to the watch frame (backend change, touches the renamed chan-server
   files, Rust re-gate). Leaning (a) IF the watcher forwards chmod; else neither.

### @@Architect steer (round-2): facet B PARKED, item 3 next, item 2 in Bug-1 batch

Facet C cherry-picked to main (a477e62). @@Architect: facet B PARK (don't chase
the writable question); ITEM 3 do next; ITEM 2 (terminal recovery) may build but
lands in the SAME desktop-verify batch as Bug 1 (folds in/supersedes 107276f).
Rebased onto a477e62; branch = a477e62 + 107276f (Bug 1 HELD) + item-3 commit.

#### Slice (item 3) - drag embedded image moves whole row: `phase-12-lane-c@4e19fc3`

Image drag-drop already relocated images alone on a line. Now an image embedded
in a row of text (`text ![](..) text`) or in a bullet item moves the ENTIRE row
to the drop target (text + image + any list marker travel together) instead of
stranding the text + relocating only the atom. moveImageSource (editor/bubbles/
image_drop.ts) branches: standalone keeps existing behavior (incl. bullet-target
inline merge); mixed line moves the whole source line as its own row; drop on the
source line itself is a no-op. Multi-line prose paragraphs OUT of scope (@@Alex).
3 tests added (mixed-row, bullet, same-row no-op). Gate GREEN: svelte-check 0/0;
vitest 1599 passed (image_drop 11/11); build clean.

REMAINING: item 2 (terminal recovery - fire recoverTerminalRendererAfterHostResume
for ALL panes on display-wake/visibilitychange; folds in/supersedes held Bug 1
107276f). Lands in the Bug-1 desktop-verify batch; I can't verify WKWebView.

#### Slice (item 2) - terminal wake recovery: `phase-12-lane-c@dd9521d`

Gap: macOS screensaver / display + system sleep does NOT fire focus / pageshow /
visibilitychange in WKWebView (window stays "visible" + focused through sleep),
so the existing host-resume listeners never fire on wake - the WebGL renderer
stays glitchy until a window RESIZE (ResizeObserver -> queueFit -> recovery;
@@Alex's clue). Fix: a coarse wall-clock-gap probe in installHostResumeListeners
- a 2s interval whose callback firing >6s late means JS timers froze (machine
slept) and is now firing on wake -> runs recoverTerminalRendererAfterHostResume.
One interval per TerminalTab so every pane recovers at once. COMPLEMENTS (does not
duplicate) the held Bug 1 blur-repaint change (107276f): blur + wake are distinct
triggers both routing through the recovery helper. Source-pinned test added.
Gate GREEN: vitest 1600 passed; svelte-check 0/0; build clean.
CAVEATS: cannot verify WKWebView via Chrome automation - @@Alex verifies in
chan-desktop. Pure display-only sleep that doesn't freeze JS timers isn't caught
by the wall-clock probe (flagged - if that's @@Alex's case, switch to an rAF-gap
detector).

### ROUND-2 QUEUE COMPLETE

Branch phase-12-lane-c = a477e62 (main) + 107276f (Bug 1 HELD) + 4e19fc3 (item 3,
READY) + dd9521d (item 2, desktop-verify batch with Bug 1). All addendum-1 +
addendum-2 LaneC work handled:
- addendum-1: Bug 2 + Bug 3 MERGED; Bug 4 closed; Bug 1 HELD (desktop verify).
- self-write follow-up MERGED.
- addendum-2: item 1 facets A + C MERGED; facet B PARKED; item 3 READY (cherry-
  pickable, disjoint); item 2 (desktop batch with Bug 1).
Awaiting @@Architect merges + @@Alex's chan-desktop verify of Bug 1 + item 2.
@@LaneB chunk 2 frontend freeze still waits on me reaching quiescence.

## 2026-05-27 (round-2, addendum-3 terminal batch) - freeze lifted, GO

Prior round merged: item 3 cherry-picked (206e3d4); @@Alex SKIPPED desktop verify
(pre-release) so Bug 1 + item 2 MERGED UNVERIFIED (b1dfb5e + 323bfd9); chunk 2
codemod + freeze-lift merged (bce6bd3). Rebased onto bce6bd3 (all my prior commits
dropped by patch-id; clean branch). Confirmed the codemod (chan_workspace,
workspace(), rich-prompt workspace_* field rename) did NOT shift the broadcast*
idents or the activity/output paths I touch.

@@Architect routed TWO addendum-3 items to me (bundled, all TerminalTab, no C/E
collision): A3-C (terminal dot pulse) + the cmd+shift+i broadcast shortcut.

### Slice (A3-C) - terminal unseen-output dot pulse: `phase-12-lane-c@8274efde`
Dot PULSES while output actively arrives at an unfocused terminal, goes SOLID once
quiet 700ms (still unseen), clears on focus. New terminalActivityPulsing (tabs),
driven by recordOutputBytes + a quiet-timer (TerminalTab); .pulsing class + smooth
opacity keyframe (Pane.svelte), distinct from the steppy watcher blink. Unit test.

### Slice (broadcast shortcut) - cmd+shift+i toggle: `phase-12-lane-c@a1eb4dd0`
macOS-native shortcut for the EXISTING broadcast select-all/deselect-all (no new
mechanism). toggleActiveTerminalBroadcastSelectAll() (tabs) mirrors the per-tab
button on the active terminal; registry entry app.terminal.broadcastToggle
(native-only, no web - cmd+shift+i is DevTools on web; escapeTerminal); App.svelte
dispatch; serve.rs KEY_BRIDGE_JS Cmd+Shift+I gated on metaKey (macOS only, Linux
Ctrl+Shift+I stays DevTools) per @@LaneE's key-bridge convention; state + key-
bridge tests. Declared the key-bridge touch on event-lane-c-lane-e.md.

Gate GREEN (both): svelte-check 0/0; web vitest 1612; vite build clean; Rust fmt;
clippy chan-desktop; chan-desktop tests 76 (incl. new key-bridge pin).

Branch phase-12-lane-c = bce6bd3 + 8274efde (A3-C) + a1eb4dd0 (broadcast). Both
ready-to-merge, web/src + serve.rs only. Reported to @@Architect.
