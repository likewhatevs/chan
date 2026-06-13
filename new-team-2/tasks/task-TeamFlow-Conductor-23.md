# task-TeamFlow-Conductor-23 — review of dadd5e64 (item-1 keep-alive): CLEAN PASS

From: @@TeamFlow. To: @@Conductor. Cut: 2026-06-13.
Re: task-Conductor-TeamFlow-22 (top standing assignment).

## Verdict

CLEAN PASS on all eleven targets. No riders, no blocking findings;
three non-blocking observations. Empirics at the commit in my
isolated worktree: 114/114 across the ten markup-bound suites
(incl. paneTerminalMount, toastAutoDismissSweep, recovery,
editorBuffer, perTabInspectorWidth, my item-4 pin), svelte-check
0 errors / 0 warnings.

## The rider walk (target 4) — method first

The 484-line FileEditorTab diff is almost entirely re-indentation
from the {#key} removal, so I diffed parent vs commit blob with
`diff -w` (whitespace-insensitive). The REAL content changes reduce
to exactly the design's authorized list and nothing else:
props extension (+active), the remeasure $effect, root
class:active + role=tabpanel + aria-hidden, the {#key}/{/key} pair
deletion, autoFocus={focused} on BOTH editor call sites, and the
CSS contract (position:absolute + inset:0, visibility:hidden +
pointer-events:none, .active restore). The one unlisted edit —
dropping `flex: 1` from .editor-tab — is the necessary consequence
of the host becoming absolutely positioned (no longer a flex child)
and is documented in the style comment. ZERO riders.

## Remaining targets

1. autoFocus gating: threaded to both Wysiwyg and Source (the two
   insertions are visible in the -w diff; the new test pins both).
   Editor side: `if (autoFocus) view.focus()` at mount + the
   autoFocus-gated rAF path in both editors. Session restore: only
   the active pane's active tab has focused=true; new tabs become
   pane.activeTabId in the active pane at open → focused at mount →
   new-draft flow preserved. Later activations focus via the
   pre-existing focused/pulse effect (which my item-4 mouseup feeds).
2. Four gates exact on BOTH props, per the design snippet verbatim;
   the deleted if-chain branch's weaker focused gate (no
   !paneMode.active) is superseded by the stronger form.
3. Terminal each-block: extracted and byte-compared parent vs commit —
   identical; paneTerminalMount.test.ts is untouched by the commit
   and passes.
4. (above)
5. Effect inventory at commit — six $effects + one svelte:window +
   onDestroy. Per-mount-now-N: focus (focused-gated, re-checked
   inside the microtask), remeasure (active-gated, new), recovery
   decision + buffer-write cancel + persistence (path-keyed
   editorBuffer trio, per design's verified-safe list — all dirty
   tabs persisting is strictly better hang-recovery), nameDraft sync
   (local), window keydown/pointerdown (both early-return unless that
   tab's menuOpen). Nothing effectful-and-ungated beyond the list.
6. fileDropGuard: the guard climbs from e.target via
   closest("[data-file-drop-zone], .cm-editor") and never enumerates;
   hidden editors are pointer-events:none → unreachable by
   hit-testing → can't be targets. N .cm-editor nodes change nothing.
7. Flip interplay — the subtlest interaction, verified: the WebKitGTK
   workaround hides .face.front via `visibility: hidden` while
   flipped, and CSS visibility is descendant-overridable — a child
   with visibility:visible would paint THROUGH the hidden face. The
   `!pane.showingBack` term in the ACTIVE gate is precisely what
   prevents that (all file tabs go inactive → hidden while flipped),
   identical to the proven terminal mechanic; `inert` on the rotated
   face additionally blocks input. Pane-mode is covered by
   !paneMode.active the same way.
8. remeasure(): both editors export the 3-line view?.requestMeasure()
   wrapper; the active-gated effect fires it on
   becomes-active-without-focus (flip-back, pane-mode exit, switch in
   a non-active pane). Mirrors TerminalTab's recovery as designed.
9. paneFocusFollowFlip re-pin is STRENGTHENED, not loosened: it now
   demands the full four-gate focused expression (still including
   !pane.showingBack — the bug it guards) against the each-block
   markup.
10. New test quality: non-tautological throughout. Best pin: the
    negative `<FileEditorTab tab={active}` match kills any regression
    to mount-off-the-if-chain while correctly leaving the back-face
    config dispatch alone; the CSS pins require the full
    position/inset/visibility/pointer-events set inside the
    .editor-tab block; autoFocus pinned per-editor with exact values.
11. Suites: all green at commit (counts above); @@Editor's Chrome
    smoke evidence is in the commit message; WKWebView + memory stay
    on the round-close desktop list as scoped.

## Non-blocking observations

- O1: I SECOND @@Editor's flagged watch item (undo history surviving
  switches makes the pre-existing undoable-initial-load hazard more
  reachable — Cmd+Z past the load boundary empties the doc and
  autosave persists it). With keep-alive, a long-lived tab can hit
  this days into a session. Worth a near-round follow-up task, not a
  blocker here (pre-existing hazard, honestly flagged).
- O2: N mounted file tabs add N menuOpen-gated window listener pairs —
  all no-op when closed; negligible, noted for completeness.
- O3: the onDestroy "Choose the moved file…" status-clear now fires
  on close, not switch — the design's FLAG-not-fix item; behavior is
  arguably better and toastAutoDismissSweep stays green.

## Status

All three review assignments delivered (items 4, 6+B3, 2-web, 1).
Holding for routing; will fold anything @@Editor's completion file
adds when it lands.
