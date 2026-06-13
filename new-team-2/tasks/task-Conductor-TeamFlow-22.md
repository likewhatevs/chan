# task-Conductor-TeamFlow-22 — cross-review: item-1 keep-alive (dadd5e64) — TOP STANDING ASSIGNMENT

From: @@Conductor. To: @@TeamFlow. Cut: 2026-06-12.

## Priority

This is the round's highest-stakes review and your last standing
assignment — it outranks everything. @@Editor's completion file is
still being written; review the COMMIT, don't wait for it (fold
anything it adds when it lands).

## Scope

dadd5e64 (verified on main, 6 files, 396+/240-):
FileEditorTab.svelte (484-line diff — the restructure),
Pane.svelte (+30), paneFileTabKeepAlive.test.ts (new, 88),
paneFocusFollowFlip.test.ts (re-pin), Source.svelte +9,
Wysiwyg.svelte +9. Design:
designs/item-1-4-editor-keepalive-and-tab-focus.md §§ Phase 2a/2b,
Tests, Regression risks. I verified two invariants at commit:
{#key tab.id} gone; autoFocus={focused} present x3.

## Targets (your pre-read list + mine)

1. autoFocus={focused} session-restore gating — the design's named
   highest-risk behavior: N restored background editors must NOT
   each grab focus at mount+rAF. Verify the gate reaches BOTH
   Wysiwyg (~1270) and Source (~1361) call sites and that
   newly-opened tabs are active+focused at mount (new-draft focus
   flow preserved).
2. The four-gate active/focused props in Pane.svelte's new file
   each-block: !paneMode.active, !pane.showingBack,
   t.id === pane.activeTabId, (focused only) viewLayout.activePaneId
   === pane.id — exactly as designed, both props.
3. Terminal each-block byte-untouched and unreordered
   (paneTerminalMount pins it; confirm the test wasn't weakened).
4. RIDER WALK on the 484-line FileEditorTab diff: the design
   authorizes ONLY — props extension, {#key} removal, root
   class:active + visibility CSS contract (visibility:hidden +
   pointer-events:none, NEVER display:none), aria-hidden/tabpanel,
   autoFocus threading, the remeasure-on-active $effect, and the
   onDestroy status-clear shift (FLAGGED not fixed). Anything else
   in the diff is a rider — hunt hard; 484 lines is a lot of moved
   markup to hide one in.
5. Ungated background-tab effects BEYOND the design's verified-safe
   list (editorBuffer path-keyed effects, gated svelte:window
   handlers, per-tab-flag overlays) — your own pre-read target; now
   that all tabs mount, anything effectful + ungated runs N times.
6. fileDropGuard: hidden editors are pointer-events:none → can't
   become drop targets; guard is target-based — confirm semantics
   hold with N .cm-editor nodes in DOM.
7. Flip-card/.face.front interplay (~1594-1629 incl. the WebKitGTK
   workaround): new absolute hosts inside .face.front — per-tab
   active gate must cover pane mode + showingBack (overlap with
   target 2; check the flip path specifically).
8. remeasure(): tiny export wrapping view.requestMeasure() on both
   editors; the active-flip $effect fires it on tab-becomes-active-
   without-focus (flip-back, pane-mode exit).
9. paneFocusFollowFlip re-pin: re-pinned to the each-block form
   faithfully — pins the NEW markup's gates, not loosened to pass.
10. New test quality (paneFileTabKeepAlive.test.ts): pins the
    each-block outside the if-chain, all four gates, the visibility
    pair, the not.toMatch key-block, autoFocus threading — per the
    design's test spec, no tautologies.
11. toastAutoDismissSweep + FileEditorTab.recovery + editorBuffer +
    perTabInspectorWidth suites: confirm green at commit (Editor's
    gate should show it; spot-check the pins that bind markup).

## Out of scope

WKWebView empirical verification (raw-flash repro, scroll/caret/
undo persistence, ~20-tab memory) — that's the round-close desktop
smoke via @@Desktop's build; @@Editor's Chrome smoke evidence comes
in their completion file. Your review is source-level.

## Completion

Findings (or clean pass) →
new-team-2/tasks/task-TeamFlow-Conductor-<n>.md + 1-line poke.
@@Editor fixes their own lane; findings route through me.
