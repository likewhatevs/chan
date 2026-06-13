# task-Desktop-Conductor-36 — round-close instrumented WKWebView walk: report

From: @@Desktop. To: @@Conductor. Re: task-Conductor-Desktop-33
(joint with @@Editor — co-sign slot below). Date: 2026-06-13.

## Provenance

- Source: worktree detached at final HEAD `b82a0a27`, clean except
  declared instrumentation (debug IPCs + ACL entries, launcher/SPA
  drivers, conf: CSP connect-src + backgroundThrottling, workspace-
  window throttling line in serve.rs). Clean-base binary sha was
  `58b6d195`; walk binary `5d7d5b0f` = base + instrumentation
  (web/dist assets are runtime-read in debug, so SPA-driver edits
  never re-entered the binary). Isolated $HOME
  (/private/tmp/chan-rc-home) — real registry/config untouched.
- Environment caveat that shaped the split: the host display was
  ASLEEP+LOCKED throughout (overnight). WKWebView never composites
  in that state: rAF doesn't fire (driver-probed, recorded
  "degraded"), document.hasFocus()=false, CM6 deep-scroll clamps,
  and a hidden terminal's fit-loop emits continuous SIGWINCH redraws
  that STARVE the cs-write queue's output-idle gate. Every line
  below marked [blocked-env] is attributable to one of those four,
  with the driver-recorded evidence inline in the archived reports.

## Results table

LEGEND: PASS = instrumented green on real WKWebView+Rust.
[hand-smoke/<reason>] = honest split per the spec's gates.
[blocked-env] = automatable in principle, needs an awake display —
re-runnable in ~2 min via the retained harness (below).

**Bring-up**
| line | verdict |
|---|---|
| text-input probe (addendum contract) | PASS — execCommand branch live (insert+readback+Cmd+Z clean) |

**Item 1 (keep-alive) — the round's headline**
| line | verdict |
|---|---|
| both editor hosts stay in DOM across switch | PASS (hosts=2) |
| inactive host visibility:hidden, active visible | PASS |
| raw-flash probe: no `**bold**`/`[link](` visible, decorations >50 | PASS ×4 readbacks (same-tick, +frame, settled, post-flip; 102 decorations) |
| undo-across-switch (A1.3): marker inserted, switch b→a, Cmd+Z removes | PASS |
| undo-spam never empties doc (bb877a87 boundary) | PASS (head intact; length-assert amended for CM6 virtualization, flagged to @@Editor) |
| flip cycle (Cmd+,): .tabs.flipped on/off, hosts hidden on back, post-flip raw-flash clean | PASS |
| scroll restore mechanism | PASS at clamp magnitude (same-tick 0 → restored next measure, exactly the CM6 async-restore shape) |
| deep-scroll (3000px) preservation | [blocked-env: CM6 cannot measure uncomposited; scrollTop clamps] |
| Cmd+. Hybrid-Nav cycle | [hand-smoke: chord never engaged headless — no DOM marker found either; diagnostic in report for @@Editor] |
| A1.4 session-restore caret-lands-once (+2-pane variant) | [blocked-env: document.hasFocus()=false → spec's own fallback gate fired] |
| A1.6 new-draft caret | menuitem found+clicked PASS; caret assert [blocked-env: no OS focus] |
| A1.5 ~20-tab memory | PASS-with-note: main +1.6MB; WebContent sum +158MB ≈ 8MB/doc linear, no runaway (right at the ~150MB judgment line) |
| A1.7 DnD reorder / A1.8 OS drop | [hand-smoke: no synthetic dragstart / OS drops] per addendum ledger |

**Item 4 (tab-click focus)**
| line | verdict |
|---|---|
| A4.1/A4.2/A4.3 | [hand-smoke: synthetic dispatchEvent skips the mousedown default action that IS the bug — vacuous pass risk; @@Editor's 30-second human script stands] |

**Item 2 (queue visibility)**
| line | verdict |
|---|---|
| runtime reactivity watch (state_unsafe_mutation) — § 5's [instrumentable] line | **PASS: 0 errors, 0 state_unsafe_mutation, 0 warns** across boot, 22 tabs, splits, reloads, paste storms (console.error+warn+onerror+unhandledrejection hooked from t0) |
| dynamic SPA block (busy submit chip/pill, ×3 counts, drain timing, reload mid-pending, idle fast path, cap reject, hide/reshow, second window, flipped pill) | [blocked-env ×2: (a) Rich-Prompt chord swallowed by focused xterm — fixable in harness, but (b) the idle-gate starvation makes delivery semantics untestable while the display sleeps]. Wire level already 18/18 via @@PromptQueue's walker + vitest pins; SPA visuals land on the awake smoke |
| I2.7 kill-serve variant (flag 6) | N/A-on-desktop: the embedded serve cannot die independently of its windows (standalone-only edge) — recorded, needs no smoke |

**B5 / Item 6 / cross-cutting**
| line | verdict |
|---|---|
| B5 30-second check | [hand-smoke per the accepted decision note] — macOS automation attempted, not cheap (launcher driver went quiet mid-composite); semantics already GTK-proven (B6: header text exact through 13 cycles) + unit-gated |
| Item 6 | DONE previously: 36/36 instrumented walk (task-15) + pixel pass stays on @@Alex's list |
| Item 5A X-dismiss / Item 3 broadcast-OFF on desktop | [hand-smoke: as drafted in § 5 — not in @@Editor's automation specs] |

## Findings (tasks via you, per protocol)

1. **Hidden-terminal fit-loop spam** (finding-candidate, severity
   TBD): an xterm in a never-composited window emits continuous
   resize→SIGWINCH→prompt-redraw (~1.7KB/s ring growth), which
   starves the cs-write queue's output-idle gate indefinitely.
   If this reproduces with a COMPOSITED but hidden/buried window, it
   is a real item-2 hazard (queued writes never deliver while a
   terminal tab is hidden) + battery cost; if it is asleep-display
   only, it is benign. 2-minute check on the awake smoke: hide a
   terminal tab, `cs terminal write` to it, watch delivery.
2. WKWebView-asleep automation lessons (rAF never fires; CM6
   unmeasurable; chords-on-xterm swallowed; xterm input needs
   paste-pipeline not keydown; WKWebView caches the SPA hard —
   purge ~/Library/Caches+WebKit between instrumented runs) —
   recorded for the next walk harness.

## Retained harness (awake re-run, ~2 min)

Worktree + walk binary kept as-is. Recipe: start
`python3 /tmp/chan-rc-report-server.py` (recreate from the report
archive note if /tmp rotated), launch
`HOME=/private/tmp/chan-rc-home <gate-target>/debug/chan-desktop`,
drive launcher seq turn-on/open-window via /tmp/chan-rc-state, ack
the driver's needs. The I2 dynamic block + deep-scroll + caret
asserts then run with compositing live. Evidence archives:
/tmp/chan-rc-report-{attempt1,attempt2,attempt3,final}.jsonl.

## Teardown

App + report server killed (PID-scoped); isolated HOME + fixture
retained for the awake re-run; b6gtk container stopped earlier; no
peer processes touched. Clean-binary rebuild for @@Alex's smoke on
request (strip instrumentation -> cargo build, ~30s; new sha will be
recorded then).

## Co-sign

@@Editor: amendments I made live are listed in task-Desktop-Editor-35
(A1.3 virtualization fix, A1.2 marker fallback, cs-split, typed
ctrl-c/paste path). Please co-sign or contest in your report.

## APPENDED 2026-06-13 — @@Editor co-sign received (task-Editor-Desktop-36): framing amendments applied

All four live amendments APPROVED by the spec owner. Two table lines
reframed per their flags — these supersede the wording above:

- **A1.2 Cmd+. Hybrid-Nav line** → [degraded: chord didn't engage in
  this environment — re-check awake], NOT app-FAIL. @@Editor
  re-checked source: `.pane-mode-preview` exists (Pane.svelte:1392),
  so my selector was right and the missing node means non-engagement
  (Cmd+, demonstrably engages, so it's chord-specific or
  environmental).
- **I2.3/I2.7 drain asserts** → [degraded: the write queue's idle
  gate (output-quiet >= 800ms, WRITE_QUEUE_QUIET_MS,
  terminal_sessions.rs:35/1348) is held closed by the fit-loop spam
  — environment]. I2.2's depth-climb asserts stay valid either way
  (queueing is the assertion, delivery isn't).
- Their caffeinate note corroborates my display-wake attempt: a
  LOCKED session keeps the app non-key even with the display forced
  awake — the compositing set stays hand-smoke until a human
  unlocks. No one should burn time on rescue attempts.

**Finding 1 upgraded to a JOINT OBSERVATION (@@Desktop + @@Editor),
production framing:** buried windows are exactly "never-composited
windows kept warm". IF the fit-loop also runs for a buried window on
an AWAKE display, then every buried agent terminal (a) spins CPU on
resize/SIGWINCH/redraw and (b) holds its own write queue's idle gate
closed — i.e. pokes to a session whose window was buried starve
until unbury ("bury the lead's window and the lead stops receiving
pokes"). That directly prices B5's "kept warm in memory" affordance.
Proposed follow-up (next round): awake-display repro — bury a window
with a live terminal, watch resize events + queue delivery; if it
reproduces, fix candidates are a visibility-gated fit observer
(cheap) or exempting fit-loop redraws from the idle signal.
@@Editor's co-sign attached in task-Editor-Desktop-36; their copy
routes to you separately per protocol.
