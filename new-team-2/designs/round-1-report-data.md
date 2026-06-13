# Round-1 report — mechanical data sections

Owner: @@TeamFlow (per task-Conductor-TeamFlow-25, assignment 2).
Data only; judgment/retro sections are @@Conductor's. Snapshot taken
2026-06-13 ~00:15 at HEAD 3c45f35a; the round funnel is still open
(4a/4b + wave-3 reviews, badge, narrow undo fix, B5/B6/B4 in flight),
so late rows will need appending — this file is append-only.

## 1. Commit table (landing order, e0ec0d3c..HEAD)

| sha | lane | item | files | description |
|---|---|---|---|---|
| ffbcc3ff | @@Editor | item 4 | Pane.svelte, tabSwitchFocusFollow.test.ts | tab-click focuses the terminal (mouseup re-pulse) |
| 86a0dce9 | @@TeamFlow | item 5B | routes/team_config.rs | survey-first "Reaching the host" + 1..N/F/X key docs in bootstrap template |
| 0f146fcf | @@TeamFlow | item 3 | teamOrchestrator.svelte.ts, teamBootstrapOrchestrator.test.ts | teams start with broadcast OFF (clear-all sweep kept) |
| c9fbb909 | @@TeamFlow | item 5A | BubbleOverlay.svelte, survey.svelte.test.ts | X key dismisses the survey overlay; [X] Dismiss label |
| 7c6a36af | @@CtxPass | B1 w1 | routes/graph.rs | TreeMergeCtx for graph tree-layer threading |
| 396ad164 | @@CtxPass | B1 w2 | indexer.rs | widen IndexerShared across the indexer spawn family |
| ca40ea6b | @@PromptQueue | item 2 (server) | routes/terminal.rs, terminal_sessions.rs | prompt-queue visibility: tagged messages, acks, depth events |
| c15f6b35 | @@CtxPass | B1 w3a | graph.rs, workspace.rs, design.md | FileRecord for GraphView::replace_file |
| 6e4253d4 | @@CtxPass | B1 w3b | drafts.rs | DraftScanAccum for the drafts tree walk |
| 3d4f564b | @@Desktop | item 6 | desktop/src/main.js | launcher Open always enabled + auto-turn-on + failure dialog |
| 54b65a60 | @@Desktop | B3 | desktop/src-tauri/src/serve.rs | launcher capability negative pins (drag-pasteboard grant) |
| 86d50a25 | @@PromptQueue | item 2 (web) | RichPrompt/TerminalTab/tabs + 3 tests | Rich Prompt queue visibility: pending message + depth state |
| f82aae50 | @@CtxPass | B1 w3c | contacts/slug.rs, import.rs, main.rs, design.md | SlugAllocator for contact filename allocation |
| 8f070e36 | @@CtxPass | B1 w3d | routes/fs_graph.rs | pass FsGraphParams through build_fs_graph_paged |
| e249de55 | @@CtxPass | B1 w3e | routes/survey.rs | FollowupSpec for survey follow-up creation |
| f198df7b | @@Desktop | B5 | desktop main.rs, serve.rs | exclude buried windows from the per-workspace window cap |
| dadd5e64 | @@Editor | item 1 | FileEditorTab/Pane/Wysiwyg/Source + 2 tests | keep file editor tabs mounted across switches (keep-alive) |
| 126d9285 | @@CtxPass | B1 w4b | control_socket.rs | handle_team takes (TeamRequest, &ControlSocketCtx) |
| 3c45f35a | @@CtxPass | B1 w4a | routes/terminal.rs, terminal_sessions.rs | RestartOverrides for terminal session restart |
| bb877a87 | @@Editor | undo narrow fix (task-24) | editor/base.ts, valueSyncUndoBoundary.test.ts | initial file-load apply made non-undoable (addToHistory(false), reload path unchanged) |
| 7c976a68 | @@PromptQueue | item 2 (badge) | Pane.svelte + wiring pin | terminal tab-strip queue-depth pill (held for dadd5e64, incl. flipped-strip mirror) |
| b82a0a27 | @@PromptQueue | item 2 (N1 follow-up) | terminal_sessions.rs | docs: registry-guard broadcast nuance comment on enqueue_write (closes CtxPass review note N1) |

Attribution sources: task-TeamFlow-Conductor-9, task-Editor-Conductor-23,
task-PromptQueue-Conductor-23, task-Desktop-Conductor-15,
task-CtxPass-Conductor-15; f198df7b via journal-Conductor + git log.

## 2. Review matrix

| commit(s) | reviewer | verdict | report |
|---|---|---|---|
| ffbcc3ff | @@TeamFlow | CLEAN PASS (5/5 targets) | task-TeamFlow-Conductor-11.md |
| 86a0dce9 + 0f146fcf + c9fbb909 | @@Editor | CLEAN PASS (batch, no findings) | task-Editor-Conductor-23.md |
| 3d4f564b + 54b65a60 | @@TeamFlow (rerouted from @@Editor) | CLEAN PASS (6/6 targets, 3 obs) | task-TeamFlow-Conductor-17.md |
| ca40ea6b | @@CtxPass | CLEAN PASS (8/8 targets) | task-CtxPass-Conductor-14.md |
| 86d50a25 | @@TeamFlow | CLEAN PASS (9/9 targets, 3 obs + 6 smoke flags) | task-TeamFlow-Conductor-20.md |
| 7c6a36af + 396ad164 | @@PromptQueue | CLEAN PASS (both) | task-PromptQueue-Conductor-18.md |
| dadd5e64 | @@TeamFlow | CLEAN PASS (11/11 targets, 0 riders) | task-TeamFlow-Conductor-23.md |
| c15f6b35 + 6e4253d4 + f82aae50 + 8f070e36 + e249de55 (w3 batch) | @@PromptQueue | ALL CLEAN | task-PromptQueue-Conductor-28.md |
| 126d9285 + 3c45f35a (w4a+4b batch) | @@PromptQueue | BOTH CLEAN (closes the B1 review queue) | task-PromptQueue-Conductor-29.md |
| f198df7b | @@Editor | CLEAN PASS (5 targets; empirical bury-click stays on the hand-smoke list) | task-Editor-Conductor-29.md |
| bb877a87 | @@TeamFlow | CLEAN PASS (3/3 targets + 2 mutation bite-tests, 1 obs) | task-TeamFlow-Conductor-27.md |
| 7c976a68 + b82a0a27 (badge + N1 rider) | @@Editor | CLEAN PASS (badge 4 targets + rider) | task-Editor-Conductor-31.md |

## 3. Evidence index

Task files (33 at snapshot): tasks/task-{Conductor-*,*-Conductor-*}.md —
attribution + verdicts above; each completion file carries its own gate
evidence inline.

Journals (6): journal-Conductor.md (master coordination log),
journal-CtxPass.md, journal-Desktop.md, journal-Editor.md,
journal-PromptQueue.md, journal-TeamFlow.md — all append-only.

evidence/item-2/:
- ws-recipe-run-PASS.log — wire-level manual-recipe walker transcript, 18/18 PASS
- ws-recipe-run2-walker-bugs.log — earlier run, 6 FAILs from walker fd-capture bugs (kept deliberately for transparency)
- ws-recipe-walker.mjs — Node WS walker harness (drives the recipe against a throwaway standalone server)

Design docs (9 + this file): item-1-4-editor-keepalive-and-tab-focus.md,
item-2-prompt-queue-visibility.md, item-3-broadcast-default-off.md,
item-5-survey-first-x-dismiss.md, item-6-launcher-open-auto-on.md,
b1-ctx-pass-design.md, backlog-ctx-pass.md (superseded by b1 design),
b4-linux-drop-path-print-note.md (documented no-op + corrected finding),
b5-buried-window-cap-decision.md (cap semantics; feeds survey).

Ratified design amendments (2, both B1, recorded in journal-Conductor):
- w3a: replace_file's third allow(too_many_arguments) retired; decision-3 rationale extended.
- w3d: reuse the existing module-scope FsGraphParams query type instead of minting a new <'a> struct.

Plan docs: round-1-plan.md (scope + lanes + funnel), bootstrap.md, config.toml.

## 4. Follow-ups compilation (deduped, attributed)

Authorized / in flight this round:
- Undo narrow fix: initial-load apply becomes non-undoable — @@Editor, task-Conductor-Editor-24 (in flight). The wider reload-undo question is a survey item (below). Origin: dadd5e64 commit-message watch item, seconded in task-TeamFlow-Conductor-23 (O1).

Survey items for @@Alex (round close):
- Undo-after-file-watch-reload semantics (recover-from-overwrite vs current) — task-Conductor-Editor-24. Boundary context per Conductor ruling on TeamFlow O1 (task-27): an empty-at-open file whose first content arrives via reload already gets the non-undoable annotation (it IS a first-content fill); the survey decides only the content-over-content reload case.
- B5 cap semantics: should MAX_WINDOWS_PER_WORKSPACE exclude buried windows permanently? — b5-buried-window-cap-decision.md, task-Conductor-Desktop-17.

v2 / future-work items:
- Item 2 v2: cancel/dequeue by id (prompt-cancel frame + retain-filter), durable pending ids on the session frame, skip-fail-when-terminal hardening (TeamFlow O1, task-TeamFlow-Conductor-20). Owner @@PromptQueue.
- Item 1 follow-ups from design: FileTab.scrollTop session field (scroll across RELOAD), optional LRU eviction for tab counts — design §Regression risks, task-Editor-Conductor-23.
- Item 6: optional dedupe-by-path guard for the launch in-flight window (external registry-changed re-render re-arms the button; pill-parity hazard class) — TeamFlow O1, task-TeamFlow-Conductor-17.
- backgroundThrottling dev-flag (WKWebView suspends a displaced launcher ~10s after launch; bit @@Desktop's instrumented walk) — task-Desktop-Conductor-15.
- N1 (item 2 server): enqueue_write_matching broadcasts QueueDepth under the registry mutex — safe (sync send) but deserves a comment — @@CtxPass review, task-CtxPass-Conductor-14. DONE: landed as b82a0a27, reviewed clean in task-Editor-Conductor-31.

Nits / docs-only:
- Item-3 test title still says "before the team's set is applied"; no set is applied anymore — retitle at next touch (@@Editor review, task-Editor-Conductor-23).
- N2 (B1): design said "update the 4 existing queue tests"; reality was 4 new tests, 0 edits — design-doc drift note, task-CtxPass-Conductor-14.
- Round-1 param-count inventory in the lead task does not reproduce at HEAD; fix at source at round close — task-CtxPass-Conductor-6, journal-CtxPass.
- cs CLI prints control responses on stderr (pre-existing, surprised the walker; not a regression) — task-PromptQueue-Conductor-23.

Unstarted (carryover candidates):
- B2 dispatch-to-matcher-loop shortcut — @@Editor stretch, gated on its own design note + sign-off; never started (task-Conductor-Editor-1).
- B7 Xcode CI selection — watch item only; trigger = next release run (round-1-plan).

Cosmetic / recorded-only observations (no action):
- Stacked launcher failure dialogs share one Escape (TeamFlow O2, task 17).
- Item-2 chip shows immediately on reshow — deliberate fast-path deviation (TeamFlow O2, task 20).
- Item-2 "queued" phase has no delivery timeout — by design (TeamFlow O3, task 20).
- Item-1 onDestroy status-clear now fires on close, not switch — design's FLAG-not-fix (TeamFlow O3, task 23).
- B3 reviewed by parse-helper read + file grep, not a desktop-workspace build (TeamFlow O3, task 17).
- B1 w4b registry-resolve observable-order change — flagged for the in-flight wave-4 review's attention (task-Conductor-CtxPass-7).

## 5. WKWebView consolidated checklist (draft, grouped by item)

Item 1 + 4 (keep-alive + tab focus) — all pending, @@Desktop build:
- [hand-smoke] Long doc, scroll mid, switch away/back: instant decorations, no raw flash, scroll preserved (the item-1 repro).
- [hand-smoke] Repeat after Hybrid-Nav exit and flip-back.
- [hand-smoke] Terminal tab click -> document.activeElement is the xterm textarea (item-4 repro); file tab click -> cm-content.
- [hand-smoke] Session restore ~5 tabs / 2 panes -> caret lands once, in the active pane's active tab.
- [hand-smoke] Tab DnD reorder + cross-pane drag (mouseup-fix risk surface; CDP can't synthesize dragstart).
- [hand-smoke] OS-file drop: active editor accepts, non-zones blocked, hidden editors never targets.
- [hand-smoke] ~20 file tabs memory sanity (Activity Monitor).
- [hand-smoke] Flip on desktop (WebKitGTK face-visibility interplay; Linux build if available).

Item 2 (queue visibility) — all pending, @@Desktop build (wire level already 18/18 via walker):
- [hand-smoke] Busy-agent submit: text stays, dims read-only, chip after ~300ms.
- [hand-smoke] cs terminal write x3: idle label counts 2/3/4; drain counts down; prompt clears exactly when its message prints.
- [hand-smoke] Idle fast path: no chip flash, clears within ~1s.
- [hand-smoke] Reload mid-pending: draft restored, badge re-syncs from session frame, queued copy still delivers.
- [hand-smoke] Hide mid-pending, resolve while hidden, reshow (mount catch-up: delivered clears, failed shows note).
- [hand-smoke] Rejected path at cap (queue full ack): keep text + transient note.
- [hand-smoke] Second window: depth updates without owning the pending; composer never locks there.
- [instrumentable] Runtime reactivity watch (state_unsafe_mutation class): console clean through submit->queued->delivered (folded into the WKWebView gate per task-PromptQueue-Conductor-23).
- [hand-smoke] O1 edge if cheap: deliver while hidden, kill serve, reshow.

Item 6 (launcher) — complete except final pixel pass:
- [instrumentable] 36/36 instrumented WKWebView walk DONE (task-Desktop-Conductor-15: happy path, failure dialog x3 dismissals, pill consistency, remote rows).
- [hand-smoke] Pixel/hit-testing pass on @@Alex's final smoke checklist (design §Verification 4).

B5 (buried-window cap) — 30-second human check (source:
b5-buried-window-cap-decision.md §Verification status; gated-green,
empirically unverified — interactive bury can't be automated):
- [hand-smoke] Close a workspace window (red dot), open the Window menu: header reads "Hidden Windows (1, kept warm in memory)".
- [hand-smoke] Open windows past the cap with one buried: the 11th opens (buried excluded from the count).

Cross-cutting:
- [hand-smoke] Item 5A X-dismiss + 1..N/F on a real desktop survey (Chrome-verified by @@TeamFlow on standalone; WKWebView never exercised).
- [hand-smoke] Item 3 broadcast-OFF bootstrap on desktop (Chrome-verified; WKWebView never exercised).
