# task-Chan-Lead-3 — COMPLETE: task-5 stragglers + task-6 review findings

From: @@Chan. To: @@Lead.

## Commits

- 03f1d2b2 task-6 findings: terminal.richPrompt registry entry now
  carries note "physical Cmd on every platform" (SERVE_LONG_ABOUT
  regenerated — the Linux mislabel's help-text sibling is gone);
  "can workspace status" codemod scar renamed to "can feed the
  status bar".
- 4c9addff task-5 stragglers: every remaining work-item code
  de-coded to its behavior/constraint (GI-1/2/5/6/7/8, F1, F4 across
  the whole right-click-revamp family — Wysiwyg/Source/clipboard/
  external_links/link_preview/TerminalTab/FileEditorTab/tabMenu —
  plus F2, A3, A6, "(Graph slice)", "slice-3" found by the sweep);
  test files renamed fileInfoBodyKindWiringSlice4b ->
  fileInfoBodyKindWiring, graphEdgePaletteSliceF ->
  graphEdgePalette (nothing referenced the old names; in-file
  describe strings de-coded).

## Notes

- G1/B9 from your ruling were ALREADY clean — earlier passes removed
  them; the "two test-pinned regexes" note had gone stale between
  enumeration and ruling. Verified zero hits before starting.
- The de-coding agent stalled at the 600s watchdog mid-pass; on
  inspection it had finished all edits (including 4 same-genre sites
  beyond the enumerated list — DashboardTab A3, FileInfoBody F2,
  graphScopeHeaderRow "(Graph slice)", screensaverSettings
  "slice-3"). I reviewed the full diff (judgment quality held:
  constraints kept, history dropped, pins stable), ran the gates
  myself, and committed.

## Gate (after the last edit, combined state)

svelte-check 0 errors / 0 warnings; full vitest 174 files / 1719
tests green; cargo check + cargo test -p chan green (help-table
edit); final rg sweep over web/src returns only genuine identifiers
("f1" fixture tab ids). Worktree clean on my lanes.

That closes every task routed to me this round: tidy (1+2+3-rider),
file-drop guard + path-print (3/4), stragglers + rulings (5), review
findings (6). Standing by for round close / the @@Alex smoke signal.
