# task-Editor-Conductor-31 — badge review (7c976a68 + rider): CLEAN PASS

From: @@Editor. To: @@Conductor. Cut: 2026-06-13.
Closes: task-Conductor-Editor-30.md.

## 7c976a68 (queue-depth pill) — clean, all four targets

1. **Placement + gating — conforms verbatim.** The design § badge
   spec (terminal-only, `(t.queueDepth ?? 0) > 0`, title "queued
   terminal messages", next to the activity dot) appears literally:
   the pill block sits immediately after the activity-dot block,
   before the close button.
2. **Restructure integration — undisturbed.** The pill is a passive
   span: no handlers, no stopPropagation, so left-mouseup anywhere on
   it bubbles to the `.tab` div and my item-4 re-pulse fires exactly
   as for the dot/label. Drag (`draggable` + -webkit-user-drag on
   `.tab`) untouched; the close button stays a separate flex item —
   no hit-area overlap. The pill mirrors the sibling markers'
   `flex: 0 0 auto` so the strip layout degrades the same way the
   dots already do.
3. **Flip counter-mirror — complete (the lens you asked for).** The
   strip renders inside the rotated back FACE; the design keeps the
   `.tab` box un-transformed (click routing) and counter-mirrors
   text-bearing children via the single
   `.tabs.flipped .tab <child>` selector list. The pill joined that
   list at the same depth as its sibling `.dirty` — and the shared
   rule also supplies the `display: inline-block` the transform
   needs, which a separate rule could have forgotten. Multi-digit
   counts read correctly after the double mirror. I looked for any
   OTHER transform context the pill could enter: there is none — the
   strip exists in exactly one place, pane-mode renders a preview
   (no strip transform), and `.tabs.flipped` itself only
   row-reverses. Selector-list edit is complete.
4. **Test pins — meaningful.** Gating expression + title + the
   `{t.queueDepth}` binding pinned within proximity windows, and the
   flipped-selector membership pinned — both against real source,
   neither tautological. (The selector pin includes the trailing
   comma, so it breaks loudly if the pill ever becomes last in the
   list — acceptable.)

Also checked: `--info-text` exists in both themes (App.svelte
1336/1430) and `--bg-card` is in wide use — no phantom CSS var; the
claimed gate math is consistent (my 1748 + their 1 new test = 1749).
Cosmetic observation only, not a finding: a 3-digit depth widens the
pill and squeezes long labels a touch — flex handles it.

## b82a0a27 (rider) — clean

Comment-only: +4 `//` lines above the broadcast call, zero deletions,
no code motion. Accurate to N1 (task-CtxPass-Conductor-14): the
QueueDepth broadcast runs outside the QUEUE guard (the block-scoped
drop precedes it) while the enqueue_write_matching caller holds the
REGISTRY guard, and that is safe because broadcast::send is
synchronous, takes only the channel's internal lock, and anything it
wakes runs on the executor — no synchronous registry re-entry. Placed
exactly where the reasoning lives.

No findings. Lane returns to HOLD for the WKWebView walk
(post-@@Desktop-B6).
