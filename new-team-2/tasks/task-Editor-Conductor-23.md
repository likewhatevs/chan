# task-Editor-Conductor-23 — items 4+1 complete + TeamFlow x3 review (clean)

From: @@Editor. To: @@Conductor. Cut: 2026-06-13 (round 1).
Closes: task-Conductor-Editor-1.md, task-Conductor-Editor-11.md.

## Commits (both pathspec-atomic, verified pre/post)

- **item 4** — `ffbcc3ff` fix(web): clicking a terminal tab focuses
  the terminal (mouseup re-pulse). Pane.svelte +
  tabSwitchFocusFollow.test.ts (new mouseup pin documenting the
  default-action steal).
- **item 1** — `dadd5e64` fix(web): keep file editor tabs mounted
  across switches (terminal-style keep-alive). Pane.svelte,
  FileEditorTab.svelte, Wysiwyg/Source (remeasure() export), new
  paneFileTabKeepAlive.test.ts (9 pins), paneFocusFollowFlip
  re-pinned (props + markup; its focus-follow coverage intact).
  Implemented exactly per the design (each-block + active/focused
  gates, {#key} removed, visibility CSS contract, autoFocus={focused}
  threading, active-flip remeasure). The design's optional jsdom
  mount test was skipped per its own fallback clause (source pins;
  browser is the source of truth).

## Gates

- Own-gate `make web-check` (svelte-check + vitest + build) green
  AFTER the final edit of each commit: item 4 at 174 files/1721
  tests; item 1 at 176 files/1743 tests, exit 0.
- Sweeps with `rg --text --no-ignore` (mount sites, debug residue).
- Note: gate ran in the shared tree (own-gate model); @@PromptQueue
  had RichPrompt.svelte WIP in flight — green anyway.

## Verification evidence

**Chrome-verified** (vite dev :5173 → standalone /tmp/editorsrv-chan
on :8787, throwaway /tmp/editor-lane-ws; all torn down):
- Phase-0 probe reproduced the design's item-4 root cause exactly:
  after tab click, activeElement was the `.tab` div (default action
  beats the pulse microtask). Post-fix: xterm-helper-textarea;
  file-tab click lands in cm-content; typing flows immediately.
- Keep-alive: 2 docs + terminal in one pane; both editor hosts stay
  mounted (one .active/visible, one hidden); scrollTop preserved
  exactly (3112 → 3112) across switch-away/back; unsaved edit
  preserved; **undo history survives the switch** (Cmd+Z after
  switch-back removed the pre-switch edit); decorations live
  instantly (218 decoration nodes, no raw markdown in viewport).
- Hybrid Nav enter/exit and flip/flip-back: hosts hidden while
  covered, scroll still 3112 after return.
- New-draft flow: tab created, "# Draft" template with title
  selected, editor interactive; close-all honors the dirty-draft
  guard; empty-pane welcome renders (0 hosts).
- Console sweep (vite client log): no runtime errors, no
  state_unsafe_mutation. ownership_invalid_mutation dev advisories
  at FileEditorTab:1292/1311 + Pane:1108 are PRE-EXISTING prop-
  mutation style, present before this change.

**Not Chrome-verifiable, WKWebView-pending** (route to @@Desktop
build; happy to drive the checklist):
1. THE item-1 repro: long doc, scroll mid, switch away/back →
   decorated instantly, no raw flash, scroll preserved (also after
   Hybrid-Nav exit + flip-back).
2. Item-4 repro: terminal tab click → activeElement is xterm's
   textarea; rich-prompt-open terminal click → caret stays in bubble.
3. Session restore ~5 tabs/2 panes → caret lands once, in the active
   pane's active tab. In Chrome automation the window has no OS
   focus, and Chrome ignores page-initiated focus() outside a
   user-activation window — restore/new-draft caret landing is
   UNTESTABLE there (verified end-state-identical pre/post change by
   stashing my diff and re-probing, so no regression signal; but the
   positive check needs a real focused window).
4. Tab DnD reorder + cross-pane drag: CDP synthetic mouse events
   never fire HTML5 dragstart, so DnD is statically-verified only
   (handlers untouched; no preventDefault added on mousedown — the
   design's stated DnD risk).
5. OS-file drop allowlist (hidden editors are pointer-events:none →
   can never become drop targets).
6. ~20 file tabs memory sanity (Activity Monitor).

## Follow-ups for round close

- **Undo-past-load wipe (flag, pre-existing, now more reachable):**
  external content applies (createValueSync.applyExternal in
  web/src/editor/base.ts — including the INITIAL disk load) carry no
  addToHistory(false) annotation, so Cmd+Z can walk back past the
  load boundary to the empty pre-load doc, and autosave then writes
  the EMPTY file to disk. Pre-existing (reachable today by undoing
  right after open), but keep-alive preserves undo history across
  switches, widening the window. Hit it live during the smoke
  (long-doc-b.md briefly 0 bytes; restored via redo). Candidate fix:
  annotate at least the initial empty→content apply non-undoable;
  needs a decision on whether undo-after-file-watch-reload is a
  wanted recovery path. base.ts is shared editor infra → not fixed
  unilaterally in this lane.
- Design's noted scrollTop-across-RELOAD session field +  LRU
  eviction: still open, unchanged scope.
- Stretch B2: not started (per task gating).

## Review: @@TeamFlow x3 (task-Conductor-Editor-11) — CLEAN PASS

1. `0f146fcf` (item 3, broadcast OFF): conforms. Clear-all sweep
   retained; only the lead-enable + worker-target loop deleted;
   workerTabs/setTerminalBroadcastTarget removal verified genuinely
   dead in teamOrchestrator (rg: no remaining readers; the primitive
   itself still serves TerminalTab UI + tabs.svelte.ts); membership-
   EMPTY re-pin asserts both [] membership and broadcastEnabled=false
   for all tabs; pre-existing-group clear test retained (193-226) and
   still meaningful. NIT (non-blocking): that test's title still says
   "...before the team's set is applied" — no set is applied any
   more; suggest retitle at next touch.
2. `c9fbb909` (item 5A, X dismiss): conforms. x/X added inside the
   same guard as Escape (preventDefault+stopPropagation, focused-card
   scope); 1..9 and F/f untouched; "[X] Dismiss" matches the [F]
   convention; comments updated; ?raw pin asserts binding + label.
   Adversarial check: no input/textarea exists inside the card
   (follow-up POSTs immediately), so typed "x" can't be swallowed
   from a text field. Observation only: the handler ignores
   modifiers (Cmd+X would dismiss) — same property as the
   pre-existing F/1..9 bindings, no new risk class.
3. `86a0dce9` (item 5B, template): conforms. All three design
   requirements present (survey-first "whenever possible" incl.
   status checks/smoke requests; 1..N / F-with-paper-trail / X
   dismiss key docs; command example + --tab-name fallback to the
   lead's tab / tab group). ASCII assertion retained and still
   meaningful (template uses plain hyphens); interpolation style
   consistent; new test pins assert OUTPUT strings (not tautologies)
   incl. the exact "option\nwith 1..N" wrap; diff confined to
   generate_bootstrap_md + its tests. Their isolated-worktree gate
   claim is consistent with what the diff needs.

No behavior-preservation findings. Nothing routed back to @@TeamFlow
beyond the one cosmetic test-title nit.

## Process notes

- Smoke infra torn down: vite dev server, /tmp/editorsrv-chan (pkill
  scoped to its path), Chrome MCP tabs. /tmp/editor-lane-ws left on
  disk (throwaway, harmless) — say the word and I delete it.
- @@PromptQueue's RichPrompt.svelte WIP was HMR-ing into my vite
  session mid-smoke (shared working tree). No interference with
  editor-focus assertions, but worth knowing the pattern exists:
  lane smokes on the shared tree see peers' uncommitted frontend WIP.
