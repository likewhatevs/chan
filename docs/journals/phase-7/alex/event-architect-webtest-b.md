# event-architect-webtest-b.md

From: @@Architect
To: @@WebtestB
Date: 2026-05-18

## 2026-05-18 12:05 — poke

Task in place:
[../webtest-b/webtest-b-1.md](../webtest-b/webtest-b-1.md).

Baseline walkthrough for Lane B (terminal sessions +
broadcast/mute + pane click semantics + keyboard shortcuts +
doc/terminal tab switching). Test drive is pre-decided
(`/tmp/chan-webtest-b-1/`); seed contents listed in the task
file. Keep it distinct from Lane A so the two of you don't
trip over each other.

Terminal-exec and browser-launch permissions go direct to
@@Alex via `alex/event-webtest-b-alex.md` (type
`permission`).

Note: phase dirs were renamed. Your working dir is now
`docs/journals/phase-7/webtest-b/`.

## 2026-05-18 12:15 BST — poke

Permission you requested in
[event-webtest-b-alex.md](event-webtest-b-alex.md) is now
approved in writing. Proceed with the full setup + baseline
walkthrough as scripted.

## 2026-05-18 13:00 BST — poke (fresh-agent handoff)

If you're a fresh @@WebtestB session: your predecessor died.
Resume where they left off:

* Your assigned task is
  [../webtest-b/webtest-b-1.md](../webtest-b/webtest-b-1.md)
  (baseline Lane B: terminal + broadcast + panes +
  shortcuts).
* Permission for drive create + seed + `cargo build` +
  `chan serve` + browser automation is **already approved
  in writing** (12:15 BST in
  [event-webtest-b-alex.md](event-webtest-b-alex.md)). Do
  not re-request unless scope expands.
* The previous you bootstrapped and filed the permission
  request but never started the build/server. Treat the
  walkthrough as not-yet-started.
* Verify `/tmp/chan-webtest-b-1/` state with `ls` before
  reseeding.
* New scope context: @@FullStack landed `fullstack-1`
  (docked side panes; affects layout but not your terminal
  lane directly) and @@Systacean landed `systacean-1`
  (`chan open` CLI). Both uncommitted in the tree. Your
  `cargo build` picks them up. The `chan open` CLI is
  worth a side-poke once your baseline is done — try
  `chan open ./notes.md` from inside the embedded terminal
  and verify it opens the file in the editor (this would
  fold into a future small webtest-b-N task; not now).
* Hand-off via
  [event-webtest-b-architect.md](event-webtest-b-architect.md)
  per the original task spec.

## 2026-05-18 14:50 BST — poke

Strong work across baseline + gap-fill + the adjacent-Lane-B
bonus pass. Concrete findings landed in
[../webtest-b/webtest-b-1.md](../webtest-b/webtest-b-1.md);
I'm folding the headlines into the architect journal status.

Highlights I want to flag back:

* **B14 NOT REPRODUCED** on current main — that's a freebie;
  may already be fixed by recent terminal-side work
  (`systacean-1` / `systacean-2`).
* **B19** ("terminal sessions go silent after browser
  reload"): the *reattach* part works; input enabled, same
  shell survives. **Scrollback retention is the remaining
  gap** — narrower than the original bug. I'll re-scope the
  bug entry to that and credit your repro.
* **B20** light-mode contrast: concrete repro (`\e[37m`
  white-on-white invisible; green/yellow/cyan too pale). Gives
  whoever fixes this a clear target.
* **E3 doc-tab menu** asymmetry: doc tab has NO right-click
  menu at all vs the terminal's 22 items. That's bigger than
  the request implied ("re-order the terminal menu") —
  actually need to "build a doc tab menu" from scratch.
  Folding into the pane menu reorg task spec.
* **chan open** dir variant: opens parent + highlights, not
  into the dir. Small nit to file as systacean follow-up.
* **Cross-drive nav drift** (8810 → 8801): potential
  bug — needs the targeted repro you flagged. Logging as
  its own item; you can revisit when convenient.

Round-1 hand-off URL noted (8810 on
`/private/tmp/chan-webtest-b-1`). I'll relay to @@Alex.

Next for you (no rush): when you have cycles, the cross-drive
nav drift repro is the most useful next pass; we don't want
to ship the release with a known way to teleport between
drives without realising. Otherwise stand by; Round 2 will
fold in webtest-side work for the bubble overlay + agent
spawn flows.

## 2026-05-18 15:25 BST — poke

Excellent investigative arc on the drift bug:

* Initial hypothesis (welcome-state pane menu's `Files`
  entry) — **disproved** with the interceptor. Honest
  retract noted.
* Drift **RE-FIRES** when Lane A still running on a
  different port; multi-tab Lane B nav hops to Lane A's
  port during page load, before any JS interceptor can
  install. Server returns 200 with no `Location` header.
  No `location.assign/replace/href=` in `web/src`. Same
  hashed bundle on both ports (rust-embed).

That's a sharp diagnosis. The fact that the hop happens
**before page JS** runs is the load-bearing observation —
it rules out anything in the SPA code and points at
browser-level behavior: shared cache, same-host-different-
port prediction, or a ServiceWorker registration. Routed
this to @@Systacean (their domain — cache headers, SW
registration, rust-embed identical-bundle implications). No
action from you on this until they come back with
findings or want a re-repro.

### E4 finding is great news

You found that the rename indicator is **already
implemented** — "stale env" chip + inline "Restart now /
Later" banner. That's a free pass on half of E4 (the
remaining half is the standalone Restart menu item
bypassing confirmation, which @@FullStack can fold in
opportunistically).

### Round 1 Lane B sweep — wrapped from your side

Confirmed. Test server at 8810 stays up. You're idle now
until Round 2 work fans out. If you have cycles, the
drift bug repro recipe in your task file is gold for
@@Systacean's investigation — pls keep that note current
if anything new surfaces.

No new task assignment from me right now.

## 2026-05-18 15:35 BST — poke: round-1 closeout context

@@Alex is closing round 1: commit + patch bump (0.10.1) +
Chan.app build + push. Then all agent sessions recycle.

### Now (small)

Confirm **B14 stays NOT REPRO** on current main after the
recent commits (`systacean-1`, `fullstack-1`, `fullstack-5`,
`systacean-2`). Rebuild `cargo build -p chan` + restart 8810
chan serve + retry your prior B14 test (background terminal
output, reload, verify reattach + input + scrollback).
Append the verdict to
[../webtest-b/webtest-b-1.md](../webtest-b/webtest-b-1.md).

### Queued for post-recycle

[../webtest-b/webtest-b-2.md](../webtest-b/webtest-b-2.md)
is cut for the fresh @@WebtestB to pick up after the
recycle. Covers wave-1.5 walkthroughs (`fullstack-6` pane
cluster, `fullstack-7` light-mode contrast,
`systacean-3` drift fix if it lands). Don't start it in
this session.

## 2026-05-18 16:00 BST — poke

Three more good catches in your final sweep:

* **fs-move on open file**: reproduced verbatim from
  request.md. The wave-1.5 fix (fs-move UX wedge) already
  has this in scope. Your repro is gold.
* **Rich prompt right-click missing**: same shape as the
  doc-tab right-click missing. Folding into `fullstack-6`'s
  scope (the pane menu reorg cluster). The fresh
  @@FullStack will see both in the same task.
* **E1 docked file browser**: working perfectly. Confirms
  `fullstack-1` is solid.

You're parked. Round 1 closeout is fullstack-2 commit +
patch bump + Chan.app build + push, then recycle. The 8810
server stays up; the cross-drive drift bug repro recipe in
your task file is the most valuable artifact you've left
for @@Systacean's `systacean-3` post-recycle pickup.
