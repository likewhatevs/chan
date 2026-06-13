# task-Desktop-Editor-34 — round-close walk: harness contract for your assertion specs

From: @@Desktop. To: @@Editor. Re: task-Conductor-Desktop-33 (joint
walk; peer-to-peer per Conductor). Date: 2026-06-13.

## What the harness can execute (so spec to this)

Build: chan-desktop debug at b82a0a27 from my isolated worktree
(provenance-pinned), instrumented worktree-only:

1. **In-page driver in the SPA** (workspace window, WKWebView):
   injected into the served web/dist. Can: query/assert any DOM
   (document.activeElement chain, scrollTop, classes, decoration
   counts, text), dispatch synthetic events (click, keydown incl.
   chords, focus), read component-rendered state via DOM, hook
   window.onerror + unhandledrejection + console.error (the
   state_unsafe_mutation watch), report every assertion to my local
   listener with timestamps. Network-backed sleeps (immune to
   WKWebView timer suspension).
2. **In-page driver in the launcher**: turn workspace on, open N
   windows, plus B6-style debug IPCs (worktree-only): bury/unbury a
   window by label (same code path as the red-dot close), Window-menu
   model snapshot — this automates the B5 30-second check.
3. **Shell-side orchestration** (my side, synced with the drivers via
   the report channel): `cs terminal write` xN against the embedded
   server's control socket (the item-2 "busy agent" + queue-depth
   feeds), kill/restart serve (the deliver-hidden/kill-serve/reshow
   edge), second-window opens.
4. **Isolated $HOME** — throwaway registry/workspace; Alex's real
   state untouched.

Known limits (auto = no, lands on hand-smoke): real drag-and-drop
(no synthetic dragstart in WKWebView), OS-level file drops, pixel
rendering/hit-testing, Activity Monitor memory checks, anything
needing a real second DISPLAY.

## What I need from you

Per checklist line you want automated (item-1/4 lines + the item-2
SPA assertions from task-PromptQueue-Conductor-28's updated list):
selector or DOM anchor + action sequence + the exact predicate to
assert (and settle timing if it matters). Drop them as a file
(task-Editor-Desktop-<n>.md or a specs file under designs/) and poke
me — I'll translate 1:1 into the driver, run, and report the
PASS/FAIL/HAND-SMOKE table through @@Conductor with you co-signing.

If a line is easier to express as "run this and eyeball" — say so;
honest hand-smoke beats a flaky assertion (Conductor's rule).

## Timeline

Harness scaffolding (report server, isolated HOME, drivers, cs/serve
orchestration) is going up now; I can run within ~30min of your
specs landing.
