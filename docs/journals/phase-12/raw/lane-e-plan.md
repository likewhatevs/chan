# Lane E plan: cross-platform keyboard shortcuts (phase 12)

Added 2026-05-27 by @@Alex (the addendum-2 shortcuts policy). @@LaneE = the
keyboard-shortcuts lane. @@Lead (the @@Architect orchestrator seat) serializes
merges + re-gates; @@Alex launches this session.

SOURCES (read all):
- This plan + `bootstrap.md` (shared protocol) + `coordination/README.md`.
- THE SPEC: `docs/journals/phase-12/lane-c/addendum-2/request.md` (Shortcuts
  section) - the policy.
- RATIFIED ANSWERS: `docs/journals/phase-12/lane-c/addendum-2/round-n-review.md`
  (@@Alex's Q5-Q9 - they refine request.md and win where they differ).
Channels: `event-architect-lane-e.md` (in), `event-lane-e-architect.md`
(reports), `event-lane-e-alex.md` (escalation). main baseline: `f72b8a7`.

## Mission

Implement the cross-platform keyboard-shortcut policy across 3 targets: web,
desktop-native macOS, desktop-native Linux. Platform key: cmd on macOS, ctrl on
Linux. MUCH of this is VERIFY/WIRE existing behavior consistently, not greenfield
(@@Alex flagged several "we already have this") - so AUDIT FIRST.

## Platform model (ratified, Q5)

- WEB uses alt for nav (cmd+1..9 / cmd+[ are browser-reserved): alt+shift+[/] =
  prev/next TAB in the pane; alt+[/] = prev/next PANE. preventDefault the web
  chords the browser would otherwise eat (cmd+s, etc.).
- DESKTOP-NATIVE (Tauri, no browser chrome) uses cmd/ctrl: cmd+1..9 = tabs of the
  current pane; cmd+shift+[/] = prev/next tab; cmd+[/] = prev/next pane.

## Behavior rulings

- CLOSE CASCADE (Q6): cmd+w (Linux: ctrl+w or ctrl+d) closes the current TAB; if
  no tabs -> closes the PANE (already so today); if no panes -> closes the WINDOW
  and returns focus to the native-desktop workspace list. CONTEXT-AWARE: a focused
  terminal KEEPS its readline ctrl+w/ctrl+d; the close binding applies outside the
  terminal / on empty panes (same shape as ctrl+a below).
- ctrl+a (Q7): Linux -> select-all in the EDITOR, readline "beginning of line" in
  the TERMINAL (context-dependent). macOS -> ctrl+a stays beginning-of-line
  everywhere; cmd+a = select-all.
- FIND TRIAD (Q9): cmd+f (find in document), cmd+g (next), cmd+shift+g (prev).
  Already in Tauri (disabled on web). VALIDATE it matches familiar browser
  behavior; ESC closes the find bar; cmd+g / cmd+shift+g keep working and must NOT
  auto-scroll except on the explicit keypress. Distinct from cmd+s = drive-wide
  search.
- INFOGRAPHICS (Q8): the infographics tab already EXISTS; only the shortcut is in
  question. Verify whether cmd+. i (Hybrid Nav chord) already opens it; add
  cmd+i / cmd+. i if missing (may live in the Hybrid Hamburger only).
- Other desktop chords (request.md): cmd+, settings; cmd+s search; cmd+. Hybrid
  Nav; cmd+/ split right; cmd+\ split bottom; cmd +/-/0 window zoom (whole window,
  Chrome-style). Hybrid "start from here" chords to VERIFY exist + keep: cmd+t
  terminal, cmd+o file browser, cmd+n new draft, cmd+shift+m graph, cmd+p rich
  prompt.
- Baseline editor shortcuts (cmd+c/v/x/a, cmd+f/g): disambiguate which the browser
  already provides vs which native-desktop must implement.

## Surfaces

- The web/desktop chord registry + `web/src/terminal/keymap.ts`.
- The desktop key-bridge `desktop/src-tauri/src/serve.rs` + native menu
  accelerators.
- Tab/pane nav + close/split/zoom in the app shell (App.svelte + pane
  components); editor find (CodeMirror); terminal (xterm readline collisions).

## First step: AUDIT, then gated slices

Identify as @@LaneE, create worktree `../chan-lane-e` on `phase-12-lane-e` from
`f72b8a7`, and FIRST produce an AUDIT: per binding in the policy, what exists
today (keymap.ts / serve.rs / native menu / CodeMirror) vs the target, per
platform - a gap table. Post it on event-lane-e-architect.md for my review before
large changes (every other lane posted a plan before executing). Then implement
in gated slices - suggested grouping: (i) tab/pane nav + close cascade + split +
zoom; (ii) find triad polish; (iii) context-aware terminal collisions; (iv)
Hybrid "start from here" chord verification.

## Boundaries + cross-lane

- Full gate before ready-to-merge: cargo fmt --check; clippy --all-targets -D
  warnings; cargo test; build --no-default-features; web npm run check + build +
  npm test (vitest). Report `phase-12-lane-e@<sha>`; @@Lead serializes + re-gates.
- web/src overlap: @@LaneB chunk 2 (drive->workspace frontend codemod) is HELD
  until the web/src lanes (@@LaneA, @@LaneC, @@LaneE) go quiescent. DECLARE your
  web/src + serve.rs touches; the codemod rebases onto your settled tree.
- serve.rs also gets renamed by @@LaneB chunk 1 (drive->workspace) - coordinate on
  event-lane-b-lane-e.md / event-lane-e-lane-b.md (created on first use).
- Terminal readline collisions (ctrl+a/w/d) touch the SAME terminal area as
  @@LaneC's terminal-recovery + Bug-1 work (TerminalTab.svelte) - coordinate on
  event-lane-c-lane-e.md / event-lane-e-lane-c.md so you don't both edit it blind.
- Need an unblock? Cut a TASK to @@Lead. Routine I auto-resolve; contention /
  high-stakes goes to @@Alex.