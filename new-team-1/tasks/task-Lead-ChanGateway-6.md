# task-Lead-ChanGateway-6 — second-pass review, part 2: web commits + the drop IPC

From: @@Lead. To: @@ChanGateway. Queue behind task-5 (same method,
same review-only rule). @@Chan's completion landed; this covers what
task-5 excluded, plus the security-sensitive desktop IPC.

## @@Chan's web commits

- 51664864 — web scrub, 60 files. Spot-check: the 22 ?raw pin-test
  files were "re-anchored at equal strength, none weakened" — sample
  5 pins and judge the anchor strength claim yourself; check the 3
  corrected-FALSE comments (GraphPanel onSetAsScope, date.ts header,
  GraphPanel display-only row) against component behavior.
- a9daa17b — the 3 shortcut registry entries. Check: chord dispatch
  predicates match the registry entries (esp. app.pane.closeEmpty
  Mod+W conditionality — wrong predicate = browser tab close eaten);
  the Linux label fix (chordFor vs hardcoded) renders the right
  chord on both platforms; SERVE_LONG_ABOUT resync rows match
  shortcuts.ts.
- c92e4d14 — warnings to zero. Verify the two suppressions are as
  narrow as claimed (chunk ceiling KEPT at 1600 not removed; onwarn
  drops ONLY INEFFECTIVE_DYNAMIC_IMPORT and only for the documented
  modules if scoped).
- e60ab688 — final stragglers. Sweep-verify with your rg method.

## @@ChanDesktop's drop IPC (79de0e95)

The security piece — review against the frozen contract
(task-ChanDesktop-Chan-1.md + my amendments in
task-Lead-ChanDesktop-6.md):

- ACL: allow-read-dropped-paths must be reachable ONLY from
  workspace-* + terminal-* windows (capabilities/local-drop.json);
  confirm tunnel-*/outbound-* CANNOT invoke (and that the serve.rs
  contract tests pin exactly that, not just the file's existence).
- Pasteboard read on the main thread; command returns [] off-macOS
  and for non-file pasteboard content; no path normalization that
  could mangle spaces/unicode before the web-side escaping.
- dropped_paths.rs is new + 79 lines: read it whole.

Findings to me, no edits. Note your task-5 (core commits) report can
land first or both can come together — your call on batching.
