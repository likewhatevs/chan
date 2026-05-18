# webtest-b-2: wave-1.5 walkthrough Lane B

Owner: @@WebtestB
Cut by: @@Architect
Date: 2026-05-18

## Goal

Once wave-1.5 lands (the new `fullstack-6` pane cluster,
`fullstack-7` light-mode contrast, plus the `fullstack-2`
external-link revision once it lands), walk through the Lane
B-relevant pieces on the running 8810 server.

Until those land you have one task you can run NOW: confirm
B14 (terminal sessions silent after reload) really is closed
on current main with the latest commits in (`systacean-1` /
`fullstack-1` / `fullstack-5` / `systacean-2` all in). Your
earlier pass said NOT REPRO — re-verify after a fresh
rebuild.

## Relevant links

* [./webtest-b-1.md](./webtest-b-1.md) — your prior Lane B
  baseline + adjacent passes.
* [../fullstack/fullstack-6.md](../fullstack/fullstack-6.md)
  — pane cluster (B15 click semantics, pane menu reorg,
  color, next/prev, doc tab menu).
* [../fullstack/fullstack-7.md](../fullstack/fullstack-7.md)
  — light-mode terminal contrast.
* [../fullstack/fullstack-2.md](../fullstack/fullstack-2.md)
  — external-link revision (tunnel-aware Tauri shell.open).

## Acceptance criteria

### Immediate (do now)

* B14 confirmation: rebuild `cargo build -p chan`, restart
  the 8810 chan serve, repro your prior B14 test (output in
  background terminal, reload the page). Confirm session
  re-attaches, input enabled, scrollback retained. If
  scrollback retention still missing: file as a clean
  follow-up.

### After wave-1.5 lands

When @@FullStack pings each of fullstack-6 / fullstack-7
ready (and after my architect-side clearance), run these:

* **fullstack-6 Lane B coverage**: B15 click handlers, pane
  right-click menu shape, doc tab right-click menu
  (terminal already had one — verify the new doc-tab menu
  works), focus border color toggle (try all three on the
  same pane), Next/Prev pane shortcut + menu entries.
* **fullstack-7**: spot-check legibility of a 16-color test
  in light-mode terminal. Quick visual.
* **systacean-3 follow-up** (if it lands): re-repro the
  drift bug; verify whatever fix @@Systacean ships actually
  prevents the hop.

For each, append a dated section with verdicts in this
task file.

## Out of scope

* `fullstack-2` external-link walkthrough — that's
  @@WebtestA's `webtest-a-3` lane.

## Hand-off

Fire `alex/event-webtest-b-architect.md` (type `poke`) on
completion of each batch.

## Permission scope

Your earlier permission grant covers cargo build + chan
serve + browser automation. Wave-1.5 testing reuses the
same shell scope; no fresh permission event needed unless
you're testing a tunnel-loop variant.
