# task-Editor-Conductor-37 — walk co-sign (items 1/4 + item-2 SPA)

From: @@Editor. To: @@Conductor. Co-signs:
task-Desktop-Conductor-36 (joint walk, task-32/33). Date: 2026-06-13.

## Co-sign: YES, table verified line-by-line against my spec gates

Every verdict matches the spec + addendum semantics, including the
honest splits. The live amendments @@Desktop lists were already
approved in task-Editor-Desktop-36 (our pokes crossed; verdicts
unchanged): A1.3 virtualization fix (a genuine bug in MY spec),
A1.2 marker fallback, cs-split, typed-ctrl-C/paste path.

## What the walk PROVED for my lane (real WKWebView + Rust)

- **Keep-alive mechanism green where it matters most.** Both hosts
  persist in DOM, visibility contract holds, and the raw-flash probe
  passed ×4 readbacks (same-tick, +frame, settled, post-flip; 102
  decorations). Validity note for the table's reader: my probe is
  DOM-text based by design, NOT pixels — a pre-fix remount leaves
  literal `**bold**` in the DOM whether or not the display
  composites, so this PASS is meaningful on the sleeping display at
  clamp-magnitude viewport. The deep-scroll variant is correctly
  [blocked-env].
- **Undo across switches + the bb877a87 boundary held on the real
  engine** (marker undone after a switch; spam never empties the
  doc).
- **Zero runtime reactivity errors** — 0 errors / 0
  state_unsafe_mutation / 0 warns across boot, 22 tabs, splits,
  reloads, paste storms. That empirically closes the
  static-gates-miss-Svelte-5-runtime risk for dadd5e64 on WKWebView,
  which was the thing Chrome could not certify.
- **A1.5 memory PASS-with-note co-signed**: ~8MB/doc linear ×20, no
  runaway — the accepted keep-alive trade; LRU follow-up already on
  the round list.

## Framing notes for the record (no table changes needed)

1. A1.2 Cmd+. line: the anchor was right (.pane-mode-preview exists,
   Pane.svelte:1392) — the chord didn't ENGAGE in the headless env.
   Hand-smoke label fine; the awake list should treat it as
   re-check, not suspected regression (Cmd+, engaging proves the
   chord layer itself works).
2. Provenance: the walk binary (5d7d5b0f) = clean base (58b6d195) +
   DECLARED instrumentation (drivers, debug IPCs, CSP/throttling
   conf). Deviation is transparent and matches the accepted item-6
   precedent; the throttling line could only affect timing-sensitive
   product behavior, and every timing-sensitive line is blocked-env
   anyway. Recommend @@Alex's hand-smoke run on the offered CLEAN
   rebuild (new sha recorded then).
3. I2.7 kill-serve N/A-on-desktop reasoning is correct (embedded
   serve shares fate with its windows; the edge is standalone-only).

## Finding 1 (fit-loop) — co-signed with one sharpening

The awake 2-minute check should cover BOTH surfaces, which are
different code paths with the same symptom:
- hidden terminal TAB (in-pane keep-alive, visibility:hidden —
  pre-existing terminal behavior, my item-1 extended the same
  contract to editors), and
- buried WINDOW (B5's "kept warm" — never-composited window).
If either reproduces awake: queued `cs terminal write` pokes to that
session starve until reveal (team flow: bury/hide the lead →
lead stops receiving pokes) plus CPU/battery. My prior: the hidden
TAB case likely does NOT reproduce awake (visibility:hidden keeps
real geometry — that's why the contract uses it), the buried-window
case is the open one. Either way the check is cheap and the
candidate fixes are visibility-gating the fit observer or exempting
fit redraws from the idle signal. Proposed as a next-round task.

## Hand-smoke list for @@Alex (consolidated, all pre-scripted in
designs/round-1-walk-editor-assertion-specs.md)

Deep-scroll preservation; A1.4 restore caret + A1.6 draft caret
(positives); item-4 30-second script (A4.1-3); Cmd+. cycle re-check;
item-2 dynamic visuals (chip/pill/drain/reload/idle/cap/hide-reshow/
second-window/flipped-pill); DnD reorder; OS-drop allowlist; B5
30-second check. The retained harness re-runs the [blocked-env] set
in ~2 min awake if preferred over eyeballs.

Lane state: round work complete pending the awake smoke;
B2 unstarted (confirmed).
