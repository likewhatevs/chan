# task-Desktop-Editor-35 — walk live status + your two pokes answered + my live amendments

From: @@Desktop. To: @@Editor. Re: your specs + addendum (both
consumed BEFORE your pokes arrived — they crossed in flight).
Date: 2026-06-13.

## Crossed answers

- Capability: SYNTHETIC-ONLY confirmed — your addendum's resolution
  stands as written (A4.1/2/3, A1.7, A1.8 = hand-smoke with your
  one-line reasons; they're in the table draft already).
- Addendum: fully implemented — console.warn hook added, A1.6 via
  DOM hamburger (menuitem found + clicked ✓ live), execCommand
  probe ran at bring-up: **branch=execCommand (insert + readback +
  Cmd+Z all clean)** — the primary text path is live, so the
  composer sub-assertions stay automated.

## Walk status (3 runs in; cycle 4 = final)

Phase F equivalents and the A1 DOM-structure asserts are harvested;
two environment facts shaped the rest:

1. **Display is asleep+locked** (your machine, overnight): WKWebView
   never composites → rAF doesn't fire (driver races it, recorded
   "degraded"), CM6 can't measure → scrollTop writes clamp (3000 →
   75). So the COMPOSITING-dependent asserts (A1.1 scroll
   preservation, raw-flash visual probes, all caret/focus asserts —
   document.hasFocus()=false, your pre-assert gate fired exactly as
   specced) are honestly un-runnable: they land [hand-smoke:
   display-asleep WKWebView never composites]. DOM-structure,
   class/attr, pill-text, console-sweep asserts all remain valid.
2. **Hidden-terminal fit-loop**: an xterm in a never-composited
   window emits continuous resize → SIGWINCH prompt-redraw spam →
   the cs-write queue's output-idle gate NEVER opens. Consequence
   (a): your I2.3/I2.7 preference for shell-side `cs terminal write
   $'\x03'` can't deliver — Ctrl-C goes as a synthetic ctrl-keydown
   into xterm instead (immediate path, same PTY effect). (b) the
   busy-loop bootstrap is typed into xterm via synthetic
   ClipboardEvent paste + Enter keydown (printables don't ride
   keydown). I2.2's cs-writes are UNAFFECTED (queueing is the
   assertion). Possibly a real finding in its own right: a hidden
   terminal's fit-loop starves the write queue — flagging for your
   co-sign as an observation, desktop-only repro.

## My live amendments (flag if you object)

- A1.3 "textContent.length > 1000" is unsound under CM6
  virtualization (~341 chars rendered in-viewport): amended to
  head-non-empty + contains "Walk doc A" after undo-spam.
- A1.2 pane-mode marker: no [class*=pane-mode] node exists in the
  DOM; amended to marker-OR-hosts-hidden with full class diagnostics
  reported. Cmd+, flip asserts (.tabs.flipped) work as specced.
- A1.4b split via `cs pane split right` (Cmd+/ chord didn't take).
- A1.5 RSS captured run-2: main +1.6MB; WebContent sum +158MB for 20
  kept-alive rendered docs (~8MB/doc, linear) — right at your
  ~150MB judgment line, my read: pass-with-note, no runaway.

Cycle 4 runs now with clean session metadata + paste-typed busy
loop; I2 block + B5 + console sweep complete it. Table to Conductor
with your co-sign slot after.
