# Channel: @@LaneC -> @@Architect

Append-only. @@LaneC writes progress reports here; @@Architect reads. Never
edit prior entries. Curated highlights/lowlights/contention; link your journal.

## 2026-05-27 @@LaneC -> @@Architect: addendum-1 investigation, REVIEW REQUESTED

@@Alex handed me `lane-c-addendum-1.md/` (untracked dir: draft.md + screenshot):
4 bugs across Rich Prompt + Editor. He chose "investigate all 4, plan first" and
asked me to journal + request your green-light before executing. Full per-bug
detail w/ file:line in `docs/journals/phase-12/lane-c/journal.md`. Summary:

HIGHLIGHTS (solid static root cause, ready to execute on green-light):
- Bug 3 (false "external edits" while editing / after image paste): timing race
  in self-write suppression. `self_writes.note(&path)` is called AFTER the
  blocking write returns, but the fs watcher can deliver the event (checked in
  `bus.rs:80-92`) before the path is registered. Sites: `routes/files.rs:663,734,
  829`, `routes/attachments.rs:176` (image paste), verify `contacts.rs:1432`. Fix
  = register suppression before/inside the write, not after. Small surgical diff.
  NOTE: chan-server backend, broader than pure web/src cosmetics - flagging scope.
- Bug 2 (Drafts paths unreachable by spawned agents): prompts say "read
  Drafts/team-X/docs/bootstrap.md" but a generic agent resolves that against CWD
  (drive root) where it doesn't exist; Drafts lives at ~/.chan/.../drafts outside
  the root. The chan MCP read_file tool ALREADY resolves Drafts/ (drive.rs:508-548,
  prompts.rs:56-69) - the agent just isn't told to use it. Fix = teach the prompt
  builder (`teamOrchestrator.svelte.ts:236,359`) to direct Drafts/ reads through
  MCP. DECISION for @@Alex: ratify "teach prompt to use MCP" vs materialize docs
  filesystem-reachable (I recommend the former; matches documented design).

LOWLIGHTS (need in-browser repro before a fix - flagging, not guessing):
- Bug 1 (terminal glitch): @@Alex clarified the live symptom is a RENDER glitch
  on the focus->blur transition when clicking between panes (the OLD pane
  glitches) - NOT the draft's "agent runs after bootstrap" framing. Located in
  TerminalTab.svelte focus/blur effects + WebGL renderer (heavily-worked area);
  needs multi-pane repro to root-cause. Open Q: one bug or two.
- Bug 4 (`-` lists become `*`): could NOT find a static conversion site. Editor
  preserves markers (list.ts:62-70, tested), drive doesn't transform, no
  remark/prettier in web deps. Needs repro to catch the exact action; possibly
  already partly fixed (@@Alex may be on a stale build).

CONTENTION: my likely touches - TerminalTab.svelte, teamOrchestrator.svelte.ts,
chan-server routes/{files,attachments}.rs, editor commands/. None overlap
@@LaneA's graph/FB scope; all are in @@LaneB's codemod blast radius - will keep
diffs small + declare touches on cross-lane channels, rebase onto the codemod
window.

SUGGESTED EXECUTION ORDER once green-lit: (3) self-writes race first - highest
confidence, contained backend diff, biggest daily-annoyance win; (2) Drafts MCP
prompt fix - needs @@Alex's one-line ratify on approach; (1) + (4) - spin a
multi-pane test server, reproduce, then fix. Each slice gated + reported
ready-to-merge separately.

NOT blocked on anything but your review + green-light. @@Alex will poke you. Will
cut a task here if I need you to unblock something specific.

## 2026-05-27 @@LaneC -> @@Architect: SLICE 1 (Bug 3) READY TO MERGE

ready to merge: phase-12-lane-c@daed33a

Bug 3 (phantom "external edits" while editing a draft / after image paste):
self-write suppression race. `self_writes.note()` ran AFTER the spawn_blocking
await; the fs watcher (own thread) could reach should_suppress() before the path
was recorded -> the editor's own autosave + image-paste writes echoed as phantom
external edits. Fix = note BEFORE/INSIDE the blocking write (never after the
await):
- files.rs api_write_file + api_create_file: note before the spawn (path known).
- attachments.rs api_post_attachment: note inside closure before write_bytes.
- files.rs api_upload_file: note inside closure once the collision-free path is
  resolved, before the await.
- bus.rs: unit test locking the event_is_self_echo decision the handlers rely on.

GATE GREEN (backend-only, no web): fmt --check; clippy --all-targets -D warnings
(workspace); cargo test full workspace (chan-server 347 incl. new test); build
--no-default-features. 3 files: routes/{files,attachments}.rs + bus.rs (+65/-12).

DEFERRED (same race, outside the reported symptom; proposing a follow-up slice if
you want it closed everywhere): post-await note() also at files.rs rename/delete
(1431/1432/1434, 1570), drafts.rs (117/167/189/190/240), rich_prompts.rs
(197/198/232), contacts.rs:271, control_socket.rs:254.

Note: thread-timing race isn't deterministically unit-testable; static root cause
is airtight. Will also smoke-test in-browser during the Bugs 1+4 session.

NEXT: Bug 2 - opening event-lane-b-lane-c.md to declare the
teamOrchestrator.svelte.ts touch + sequence vs @@LaneB chunk 0, then the
prompt/guidance fix (@@Alex already ruled it PROMPT-only). Will report that slice
separately.

## 2026-05-27 @@LaneC -> @@Architect: SLICE 2 (Bug 2) READY TO MERGE

ready to merge: phase-12-lane-c@cb1f113

Bug 2 (Drafts bootstrap paths unreachable by spawned agents): PROMPT/GUIDANCE
fix, per @@Alex's ruling (no plumbing, no doc materialization). The
team-orchestrator `identityPrompt` told agents to "read Drafts/team-X/docs/
bootstrap.md"; a generic agent resolves that against cwd (drive root) where it
doesn't exist (Drafts lives in chan metadata outside the root). The chan MCP
read_file tool already resolves Drafts/ (chan-llm/prompts.rs:56-69). Fix:
identityPrompt now marks Drafts/-prefixed paths as "read with the chan MCP
read_file tool"; plain drive paths get no hint (conditional on the Drafts/
prefix).

Files: web/src/state/teamOrchestrator.svelte.ts (+14/-1) + its test (verbatim
assertion updated, 2 tests added).

CROSS-LANE: @@LaneB slimmed chunk 0 to Rust-only (b-c channel) and cleared me to
edit teamOrchestrator.svelte.ts without rebasing - no live overlap. Acked on
event-lane-c-lane-b.md; flagged my identityPrompt region for their chunk-2
frontend freeze.

GATE GREEN (frontend-only; Rust untouched, stays green from slice 1): web vitest
1598 passed/11 skipped; svelte-check 0 errors/0 warnings; vite build clean.

Branch state: phase-12-lane-c now has slice 1 (daed33a, Bug 3) + slice 2
(cb1f113, Bug 2). Both independently gated; merge in commit order.

NEXT: Bugs 1 + 4 need an in-browser repro session (scoped /tmp drive + port,
fresh binary, scoped pkill). Asking @@Alex about the test drive per the
test-server workflow before spinning up. Bug 1 will touch TerminalTab.svelte
(declared on b-c).

## 2026-05-27 @@LaneC -> @@Architect: SLICE 3 (Bug 1) READY TO MERGE + Bug 4 CLOSED

ready to merge: phase-12-lane-c@3b587a7

Ran a scoped in-browser repro (fresh binary built from the worktree -> renamed
/tmp/chan-lane-c-srv, seeded /tmp/chan-test-lane-c on port 8477, --standalone,
scoped pkill; torn down after - server killed, drive deregistered+rm'd, port
closed; left @@Alex's :7799 tab untouched).

BUG 4 (dash->asterisk): NOT REPRODUCED across 5 paths (Enter-continue,
type-from-scratch, nested-indent, HTML-paste/turndown, toolbar-toggle) - `-`
preserved on disk every time. Markdown serialization is renderer-independent, so
fixed/absent on HEAD regardless of client. @@Alex's call: CLOSE as already-fixed
(declined a regression test). NO commit for Bug 4.

BUG 1 (terminal focus-switch glitch): web Chrome (Blink) repaints the
focus-losing pane CLEAN both directions - no glitch. @@Alex confirmed he sees it
in the DESKTOP app (Tauri/WKWebView), NOT a browser. Fix (slice 3): the blur
effect (TerminalTab.svelte:297) ran only refreshTerminalRenderer(); the focus +
Bug-6 active-flip effects use the stronger recoverTerminalRendererAfterHostResume()
(fit + atlas clear + delayed re-fits). Blur now uses that same recovery so the
blurred WebGL canvas gets the deferred repaint WebKit needs (no-op on Blink).
Files: TerminalTab.svelte (+13/-1) + renderer source-pin test (+9/-1).

GATE GREEN (frontend-only): web vitest 1598 passed/11 skipped; svelte-check 0
errors; vite build clean.

CAVEAT: I CANNOT verify the WKWebView fix - no WebKit driver via Chrome
automation. Needs @@Alex to verify in chan-desktop built from phase-12-lane-c. If
not fully resolved, he'll capture a desktop screenshot for a targeted second pass.
Flagging in case you want to HOLD slice 3's merge until @@Alex confirms (slices 1
+ 2 are independently verifiable and need no hold).

BRANCH: phase-12-lane-c = daed33a (Bug3) + cb1f113 (Bug2) + 3b587a7 (Bug1). All
3 slices independently gated. ALL 4 addendum-1 bugs now handled (Bug4 closed, no
commit). Journal: docs/journals/phase-12/lane-c/journal.md.

## 2026-05-27 (round-2) @@LaneC -> @@Architect: REBASE note + self-write follow-up READY

Picked up round-2 ("check your tasks" from @@Alex). Two things:

1. REBASE: rebased phase-12-lane-c onto main (f72b8a7) - my follow-up touches
   drafts.rs/rich_prompts.rs which @@LaneB chunk 0 renamed, so I needed its merge
   under me. daed33a+cb1f113 dropped (already in main via 34f0b3f). HELD Bug 1
   replayed 3b587a7 -> **e080829** (same change, new sha). Merge e080829 (not
   3b587a7) on @@Alex's desktop confirmation. It's preserved, not rebased away.

2. ready to merge: phase-12-lane-c@14fb535 (self-write follow-up, AUTHORIZED).
   Closed the SAME post-await note() race at every remaining write handler:
   files.rs (delete/move/fs_transfer), drafts.rs (create/discard/promote/
   rich-prompt-create), rich_prompts.rs (submit/close), contacts.rs import,
   control_socket.rs open-create. Invariant: note() runs before/inside the
   blocking write, never after the await. Backend-only, no behavior change beyond
   closing the window. 5 files +112/-73.
   GATE GREEN: fmt; clippy --all-targets -D warnings (workspace); cargo test full
   workspace (chan-server 347); build --no-default-features.

BRANCH: phase-12-lane-c = f72b8a7 (main) + e080829 (Bug1 HELD) + 14fb535
(follow-up). The follow-up is backend, independent of e080829 (frontend), so it
cherry-picks/merges onto main without dragging the held commit.

NEXT: starting addendum-2 item 1 (EDITOR no-reload-while-typing + cmd+r caret/
focus restore - @@Alex's #1). Will coordinate terminal-area touches (items 2/3)
with @@LaneE on event-lane-c-lane-e.md. Reporting each item as its own slice.

## 2026-05-27 (round-2) @@LaneC -> @@Architect: REBASED onto chunk-1 rename; CORRECTED shas

Done per your heads-up: rebased phase-12-lane-c onto main (2140925, @@LaneB chunk
1 chan-drive->chan-workspace rename). Conflicts in drafts.rs + files.rs (my
note-ordering edits vs the rename in the same hunks) RESOLVED = my logic + the
chan_workspace:: names; also fixed 3 auto-merged `Ok::<_, chan_drive::ChanError>`
lines -> chan_workspace::. Re-gated the resolved tree:
fmt; clippy --all-targets -D warnings (workspace); cargo test full workspace (no
failures, chan-server 347); build --no-default-features. All green.

CORRECTED shas (rebase rewrote them):
- self-write follow-up: ready to merge: phase-12-lane-c@327960e  (was 14fb535)
- HELD Bug 1 (WKWebView blur repaint): cddc578  (was e080829/3b587a7) - still
  HELD for @@Alex desktop verify; merge cddc578 on his confirmation.

BRANCH: phase-12-lane-c = 2140925 (main) + cddc578 (Bug1 HELD) + 327960e
(follow-up, sits on chan_workspace:: code, conflict-free onto current main).

Cross-lane: replied to @@LaneE on event-lane-c-lane-e.md - I am NOT touching
TerminalTab's keydown/focus-tracking path (only the render-recovery effects), so
@@LaneE owns the terminal-focus signal + key-bridge change; no shared-handler edit.

NEXT: addendum-2 item 1 (editor no-reload-while-typing + cmd+r caret/focus
restore - @@Alex #1). web/src only, untouched by the rename.

## 2026-05-27 (round-2) @@LaneC -> @@Architect: rebased onto e927e90; ITEM 1 facet A READY

Rebased phase-12-lane-c onto e927e90 (your follow-up cherry-pick) - git dropped
the merged follow-up by patch-id; branch = e927e90 + 0a9fb27 (Bug 1 HELD, rebased
again) + the new facet-A commit.

ready to merge: phase-12-lane-c@1222a5f  (addendum-2 item 1, facet A)
Editor no-reload-while-typing: the watcher silently reloaded a clean buffer on a
watch event, replacing the doc + snapping the caret to 1:1 mid-edit. Watch path
now flags a per-tab `externalChange` (new flagExternalChange) instead of
refreshTabFromDisk; the editor shows a dismissable "changed on disk" banner
(Reload opt-in / dismiss), caret never moves. User-initiated file replace still
reloads (refreshTabFromDisk, unchanged). Regression test added.
Files: tabs.svelte.ts, store.svelte.ts, FileEditorTab.svelte, store.test.ts
(+137/-7). web/src only - DISJOINT from Bug 1's TerminalTab.svelte, so it
cherry-picks onto main cleanly like the follow-up did.
GATE GREEN (frontend): vitest 1594 passed; svelte-check 0/0; vite build clean.

REMAINING item-1 facets as separate slices: B = chmod -w -> LOCKED tab; C = cmd+r
exact caret + focus restore. Then item 2 (terminal recovery, folds in held Bug 1)
+ item 3 (drag-drop). @@Alex confirmed the banner shape (Reload + Dismiss).

## 2026-05-27 (round-2) @@LaneC -> @@Architect: item-1 facet C READY; facet B deferred (questions)

Rebased onto abac76c (facet A merged). Branch = abac76c + 3fc3a65 (Bug 1 HELD) +
the facet-C commit.

ready to merge: phase-12-lane-c@56b9dfc  (addendum-2 item 1, facet C)
cmd+r exact caret + focus restore. setTabCaret only set tab.caret in memory; the
URL hash's caret only updated on layout changes, and window.location.reload()
skips component cleanup - so cmd+r restored a stale caret AND (no caret ->
maybeRestoreCaret bailed) lost focus. Fix: persistLayoutToHash() in the
beforeunload/pagehide flush (App.svelte). 1 line + comment, web/src only,
disjoint from Bug 1 -> cherry-picks clean.
GATE GREEN: svelte-check 0/0; build clean.

FACET B (chmod -w -> LOCKED) DEFERRED - two open questions before I build:
1. Does chan-drive's watcher FORWARD a permission-only change (chmod with no
   content write)? If not, facet B can't be watch-triggered (needs polling /
   on-focus re-stat). Wants an empirical check.
2. No stat/HEAD endpoint exists; writable only rides api.readStream meta. Fix is
   either a frontend api.statWritable (readStream + abort after meta) or a server
   `writable` field on the watch frame (backend, touches renamed files, Rust
   re-gate). Your call on which - or punt facet B as lowest-value of item 1.

ITEM 1 STATUS: facet A (merged) + facet C (56b9dfc, ready) cover @@Alex's primary
pains (no caret-jump while typing; cmd+r restores caret+focus). Facet B is the
remaining, lowest-value piece.

NEXT (your steer): item 3 (drag-drop image easy case) is the cleanest fully-
frontend+testable remaining work; item 2 (terminal recovery) folds in the held
Bug 1 and needs @@Alex desktop verify like Bug 1. Leaning item 3 next unless you
redirect.

## 2026-05-27 (round-2) @@LaneC -> @@Architect: ITEM 3 READY (drag embedded image -> move whole row)

Rebased onto a477e62 (facet C merged). Branch = a477e62 + 107276f (Bug 1 HELD) +
the item-3 commit.

ready to merge: phase-12-lane-c@4e19fc3  (addendum-2 item 3, easy case)
Image drag-drop already moved images alone on a line. Now an image embedded in a
row of text (`text ![](..) text`) or in a bullet item moves the ENTIRE row to the
drop target - text + image + list marker travel together instead of stranding the
text. moveImageSource branches: standalone keeps existing behavior (incl. the
bullet-target inline merge); mixed line moves the whole source line as its own
row; drop on the source line itself is a no-op. Multi-line prose paragraphs OUT
of scope per @@Alex. Files: editor/bubbles/image_drop.ts + its test (3 cases
added). web/src only, disjoint from Bug 1 -> cherry-picks clean.
GATE GREEN: svelte-check 0/0; vitest 1599 passed (image_drop 11/11); build clean.

QUEUE STATUS: addendum-2 items 1 (facets A+C merged; B parked) + 3 (4e19fc3,
ready) DONE. Only ITEM 2 (terminal recovery, host-wake -> recover ALL panes;
folds in/supersedes held Bug 1 107276f) remains - per your steer it lands in the
Bug-1 desktop-verify batch + I can't verify WKWebView. Starting to build it next
(investigate why the existing focus/pageshow/visibilitychange host-resume
recovery doesn't cover all panes on WKWebView wake), and will reconcile with
107276f so the blur-repaint change isn't doubled.

## 2026-05-27 (round-2) @@LaneC -> @@Architect: ITEM 2 READY (desktop batch); QUEUE COMPLETE

ready to merge (DESKTOP-VERIFY BATCH with Bug 1): phase-12-lane-c@dd9521d (item 2)
Terminal wake recovery. macOS sleep doesn't fire focus/pageshow/visibilitychange
in WKWebView, so the host-resume listeners never fire on wake (terminal glitchy
until a window resize - @@Alex's clue). Added a wall-clock-gap probe in
installHostResumeListeners: a 2s interval firing >6s late => timers froze (slept)
=> runs recoverTerminalRendererAfterHostResume. One per TerminalTab so all panes
recover at once. COMPLEMENTS the held Bug 1 blur change (107276f) - blur + wake
are distinct triggers, both route through the recovery helper, NO double-up.
GATE GREEN: vitest 1600; svelte-check 0/0; build clean.
CANNOT verify WKWebView here - @@Alex verifies in chan-desktop. Caveat: pure
display-only sleep that doesn't freeze timers isn't caught (would need an rAF-gap
detector) - flagged for @@Alex's verify.

BRANCH: phase-12-lane-c = a477e62 + 107276f (Bug1 HELD) + 4e19fc3 (item 3 READY,
disjoint web/src editor - cherry-pick anytime) + dd9521d (item 2, batch w/ Bug1).
Merge guidance: item 3 cherry-picks clean now; Bug 1 (107276f) + item 2 (dd9521d)
are both TerminalTab.svelte - merge them together AFTER @@Alex's desktop verify.

QUEUE COMPLETE. addendum-1 (Bug2/3 merged, Bug4 closed, Bug1 held) + self-write
follow-up (merged) + addendum-2 (item1 A+C merged, B parked, item3 ready, item2
desktop-batch) all handled. Standing by for merges + @@Alex desktop verify + any
new ad-hoc requests. I'm at quiescence on shippable work - @@LaneB chunk 2
frontend freeze can proceed once Bug1+item2 clear the desktop batch (they touch
TerminalTab.svelte, which chunk 2's rich-prompt field rename also touches).

## 2026-05-27 (round-2, addendum-3) @@LaneC -> @@Architect: terminal batch READY (2 slices)

Rebased onto bce6bd3 (chunk-2 codemod + freeze-lift); branch was clean (all prior
commits merged by patch-id). Confirmed the codemod didn't shift broadcast*/
activity idents I touch. Built both bundled addendum-3 terminal items:

ready to merge: phase-12-lane-c@8274efde  (A3-C: terminal dot pulse)
Unseen-output dot PULSES while output actively arrives, SOLID once quiet 700ms
(still unseen), clears on focus. New terminalActivityPulsing driven by
recordOutputBytes + quiet-timer; .pulsing keyframe in Pane.svelte. Unit test.

ready to merge: phase-12-lane-c@a1eb4dd0  (cmd+shift+i broadcast toggle)
macOS-native shortcut for the EXISTING broadcast select-all/deselect-all (no new
mechanism). State fn toggleActiveTerminalBroadcastSelectAll (mirrors the per-tab
button on the active terminal) + registry entry (native-only, escapeTerminal) +
App dispatch + serve.rs KEY_BRIDGE_JS Cmd+Shift+I gated on metaKey (macOS only;
Linux Ctrl+Shift+I stays DevTools) per @@LaneE's convention + state & key-bridge
tests. Declared the key-bridge touch on event-lane-c-lane-e.md (no overlap with
@@LaneE's merged slices). Documented via the shortcuts.ts registry note (single
source of truth; native-only chord doesn't reach the web-fallback serve --help).

GATE GREEN (both): svelte-check 0/0; web vitest 1612; build clean; Rust fmt;
clippy chan-desktop; chan-desktop tests 76 (incl. new key-bridge source-pin).

Branch = bce6bd3 + 8274efde (A3-C) + a1eb4dd0 (broadcast). web/src + serve.rs
only. Addendum-3 LaneC queue COMPLETE. Standing by for merges + new asks.
