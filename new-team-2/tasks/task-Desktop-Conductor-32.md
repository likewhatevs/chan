# task-Desktop-Conductor-32 — B5/B6/B4 complete

From: @@Desktop. To: @@Conductor. Re: task-Conductor-Desktop-17 (+27).
Date: 2026-06-13.

## B5 — DONE (code + decision note)

- Commit `f198df7b` (desktop/src-tauri/serve.rs + main.rs only,
  pathspec-atomic, staged/show-stat verified): the per-workspace cap
  now counts VISIBLE windows only; Window-menu header carries the
  affordance "Hidden Windows (N, kept warm in memory)".
- Decision note per your constraint (old/new semantics, deliberate
  consequences incl. unbury-past-cap and unbounded-buried-accrual
  trade-off, one-commit revert path, suggested 30-second human
  check): new-team-2/designs/b5-buried-window-cap-decision.md —
  ready to feed the round-close survey.
- Gate: isolated worktree, fmt + clippy `-D warnings` + full desktop
  suite green after last edit. Empirical bonus: the header affordance
  was OBSERVED rendering correctly on GTK during the B6 walk
  (header-text asserts over 13 cycles), including the destroy path.

## B6 — DONE (finding: mutation SAFE, fallback not needed, no code)

new-team-2/designs/b6-gtk-menu-mutation-finding.md. Summary: 12+1
bury/unbury cycles + destroy storm + recovery on a real GTK build
(aarch64 Ubuntu 26.04 container, webkit2gtk 2.52.3) with menu-model
readback after every mutation: structure and texts correct
throughout, ZERO Gtk-CRITICAL/WARNING, visible menubar intact in the
end-of-walk screenshot. The documented `set_menu` fallback stays
unwired — closing the phase-22 unknown with a clean answer.

Incidentals recorded in the note (not menu issues):
1. muda `text()` reads empty for pre-existing static items after
   window destroys — debug-read artifact; visible menubar fine.
2. 2nd/3rd window for one workspace did not materialize on Linux in
   the container (Ok return, no error, 90s) + one label-collision
   WARN from a stale persisted window-config stack. Needs a real-
   Linux-desktop check someday; macOS multi-window unaffected.
   Follow-ups candidate.

## B4 — CLOSED per task-27 (option 1)

Corrected note stands as the deliverable
(designs/b4-linux-drop-path-print-note.md); capture-shim recorded for
follow-ups. Nothing further from me.

## Build note for docs routing (as promised)

aarch64-linux builds need `RUSTFLAGS="-C target-feature=+fp16"`
(gemm-common 0.19 fp16 inline asm vs the default target; fine on
Apple-Silicon-hosted VMs). Captured in the B6 finding note's "Build
note" section for lifting into docs/ at round close.

## Lane state / standing duty

- Worktree build base re-synced through f198df7b; B6 instrumentation
  exists ONLY in the worktree (debug IPCs, driver, conf patches) and
  I will strip it before the round-close build. Container `b6gtk`
  stopped, harness retained for cheap re-runs; I tear it down at
  round close.
- Ready for the round-close WKWebView walk: one build at final HEAD
  per task-27 (badge + narrow-undo + B5 in), instrumented per
  @@TeamFlow's checklist when it lands. Provenance check included.

## Journal

journals/journal-Desktop.md appended (B5/B6/B4 entry).
