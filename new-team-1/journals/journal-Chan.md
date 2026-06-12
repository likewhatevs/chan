# journal — @@Chan (round 1, tidy-up)

## 2026-06-12 — start

Read task-Lead-Chan-1.md + round-1-plan.md. Baseline main @ 37a0f001, clean.

Recon (my lanes only):
- phase refs in crates/ + web/src code: 72 lines
- agent-handle refs (excl. design.md): ~140 lines, clustered in
  chan-server (fullstack-* ticket comments, survey/team_config/wire
  fixtures), chan-workspace, chan-shell, web/src tests
- main shortcut store: web/src/state/shortcuts.ts (+ shortcuts.test.ts)

Plan: (1) baseline clippy + web-check in background for before-counts;
(2) scrub crates/ comments myself (judgment calls: constraint vs
narration); (3) fixture handle neutralization; (4) web/src scrub;
(5) hygiene (warnings, param structs, shortcuts); (6) design docs;
(7) own-gate, completion task task-Chan-Lead-1.md, poke @@Lead.
Commits: pathspec-atomic per coherent chunk, no push.

## progress — scrub fan-out

- Baselines: clippy 0 warnings; web-check 1 svelte-check warning
  (RichPrompt a11y) + vite chunk-size / ineffective-dynamic-import
  warnings; vitest green.
- TOOLING FINDING: the sandbox grep shim (ugrep) silently skips large
  files — GraphPanel.svelte (130KB) returned phantom "no match",
  hiding ~45 archaeology hits. All sweeps redone with rg --text.
  Worth flagging to other lanes.
- Done myself: chan-shell wire.rs+cli.rs fixture/help neutralization
  (@@Alice/@@Bob, 46/46 tests green), chan-server static_assets.rs +
  embed_seed.rs + build.rs, ~30 web/src files scrubbed incl. the
  pin-test couplings (GraphPanel comment anchors + client.ts ticket
  codes re-anchored in graphParentEdgeInvariant / graphChipCount /
  mentionBubble / reportsToggleClient tests).
- Delegated (running): chan-server scrub, chan-workspace+chan+llm+
  report scrub, web/src finisher (GraphPanel + R2-3/wave residue +
  full vitest), design docs x3 agents (tunnel trio; workspace/report/
  llm + llm README; web/src + editor).
- Shortcut recon done: store already authoritative; 3 ad-hoc
  offenders (Cmd+Shift+P Rich Prompt, Mod+W empty-pane close,
  terminal Cmd+F find) — will normalize after web agents land.
- Param recon done: 46 fns >5 params, all Rust. Priorities:
  cmd_serve(16), control_socket start(10, x-workspace),
  graph.rs merge_* family, handle_team(11). Deferred until scrub
  agents release the files.

## task-Lead-Chan-2 addendum received

- Gate correction: desktop/src-tauri is a ROOT-workspace member;
  clippy gate is now `--workspace --exclude chan-desktop`; never fix
  desktop warnings (collision with @@ChanDesktop).
- Extended patterns (round-N, wave-N, slice x, -a-NN, @@Host/@@CI,
  desktacean, track a/b) → full rg post-pass queued for after my two
  crates scrub agents land (their sweeps used the narrower pattern).
- Confirmed stale help: `chan add --reports` "Off by default" is
  wrong (IndexConfig::default() reports_enabled=true for new
  workspaces; legacy files stay false). Fix queued with the main.rs
  pass; will verify --semantic-search/Reports help against source.
- FYI noted: ping @@Lead if hygiene changes any persisted field
  (param-struct refactors don't — they're fn signatures, not serde).

## mid-round progress — commits landing

Commits on main so far (pathspec-atomic, verified staged/shown):
- d7d0a7e0 chan-shell fixture/help neutralization (46 tests green)
- bb049d6c chan-server scrub (31 files; 418 tests; one justified
  non-comment change: stale allow(dead_code) drop on survey_bus)
- 53fe79d3 core crates scrub + the task-2 help-text truth fix
  (reports ON by default for new workspaces — verified in
  IndexConfig::default + cmd_add before editing)
- 01d0cba6 param refactors: cmd_serve 15-arg tail -> ServeArgs;
  control_socket start/handle_request -> ControlSocketCtx; both
  allow(too_many_arguments) dropped; no cross-workspace callers
  (recon's "13 desktop/gateway call sites" was a generic-name
  overcount — verified only chan-server/src/lib.rs calls start)
- fbeb5c13 8 design.md rewrites + chan-llm README (agents read
  source first; real corrections incl. tunnel HelloAck enum,
  graph schema v6, nine StandardTools)
- dc94b16e chanwriter purge rider (task-3 part 2)

Shortcuts (task-1 named ask): store recon found the registry already
authoritative; 3 true offenders. All three now have registry entries
(terminal.richPrompt Cmd+Shift+P escapeTerminal, app.pane.closeEmpty
Mod+W conditional, terminal.find Mod+F terminal-local); Rich Prompt
menu label de-hardcoded to chordFor (fixes a wrong Ctrl+Shift+P label
claim on Linux); SERVE_LONG_ABOUT resynced via shortcuts-table.mjs;
129 tests across the 8 chord-pinned vitest files green. Dispatch
stays predicate-based in App.svelte/TerminalTab (matches the existing
idiom for every other registry chord; converting dispatch to a
matcher loop would be a behavior-risk refactor for another round).

task-Lead-Chan-2 addendum: absorbed (gate = clippy --workspace
--exclude chan-desktop; extended patterns swept across crates/, web
sweep finishing via the tail agent). task-Lead-Chan-3: rider done;
file-drop guard queued post-tidy (tracked).

In flight: web residue tail agent (R2-x/B9/DB2/Part C/@@Alex/@@Host
attributions + FA57 identifier rename with pin sync + date-pill stale
header). Then: web commits, full own-gate, completion file.

## round-1 tidy task CLOSED

Own-gate green after last edit (fmt 0 / clippy --exclude
chan-desktop 0 warnings / 34 test suites ok / web-check 0+0,
1706 vitest). Completion: task-Chan-Lead-1.md. Final commits:
51664864 (web scrub), a9daa17b (shortcuts registry), c92e4d14
(web warnings zero), e60ab688 (stragglers). Sole surviving
"phase" string is graph.rs:4-5 product-UX phasing (deliberate).
Next: task-Lead-Chan-3 part 1 (file-drop guard, contract frozen
with @@ChanDesktop per Lead amendments).

## file-drop guard LANDED + task-5 stragglers in flight

- a19d7d40: SPA-global guard (Files-type discriminator, capture
  dragover/drop + bubble net) + terminal path-print wired against
  @@ChanDesktop's 79de0e95 (contract verified name-for-name).
  13 new vitest; 174/1719 green; svelte-check 0/0; Chrome smoke on a
  throwaway standalone workspace (torn down): no-takeover confirmed
  outside zones, editor zone untouched (probed at document-capture
  between guard and CM6), in-page DnD unaffected. Synthetic-event
  caveat recorded (Chrome coerces dropEffect on constructed
  DataTransfers — vitest carries that assertion).
  Completion: task-Chan-Lead-2.md; @@Lead + @@ChanDesktop poked.
  Desktop hand-smoke (@@Alex) is the remaining end-to-end check.
- task-Lead-Chan-5 accepted rulings noted (all 8 flags endorsed,
  param-deferral to carryover). G1/B9 ruling item is already
  satisfied (zero hits in web/src tests today — the "two pinned
  regexes" note was stale by the time of the ruling). Stragglers
  (GI-N/F1/F4/A6 + Slice4b/SliceF filename renames) delegated with
  pin-sync rules + full vitest; review + commit on completion.
- @@Lead ACCEPTED the guard completion (bubble net + FB no-zone
  correction both endorsed; synthetic-DataTransfer caveat added to
  the smoke checklist). Alex-smoke signal pending @@Lead's
  integrated gate re-run. Stragglers still in flight.

## tasks 5+6 CLOSED — all routed work complete

- 03f1d2b2 (review findings F-W1/F-W2), 4c9addff (straggler
  de-coding + the two test-file renames). Combined gate green
  (svelte-check 0/0, vitest 1719, chan tests).
- De-coding agent hit the 600s stall watchdog but had finished all
  edits incl. 4 same-genre finds beyond the list; diff-reviewed,
  gated, committed by me. G1/B9 ruling item was already satisfied.
- Completion: task-Chan-Lead-3.md. Worktree clean on crates/ + web/.
  Awaiting round close + @@Alex desktop smoke signal.
- @@Lead ACCEPTED tasks 5+6 (recursive residue sweep clean).
  Stalled-subagent handling noted for the retro as the model case.
  ALL routed work closed; standing by for round close.

## task-7 SVG embed bug FIXED (b7d2b205)

- Pre-existing at v0.31.1 (image-path diffs since baseline are
  comment-only — settled without a tag rebuild).
- Root cause in one network observation: /api/files/x.svg served the
  editor JSON envelope (application/json) because read_file_sync let
  the editable-text CONTENT SNIFF decide raw-vs-text and SVG is XML
  text; <img> rejects JSON -> widget broken box. PNG is binary ->
  fails sniff -> raw. Fix: classify FIRST at the route; Image|Pdf ->
  raw bytes + content_type_for. Workspace gate + MCP untouched.
- +1 unit pin (svg-as-text -> Binary), Chrome re-verify green
  (fragment-bearing embed renders 100x100), -D warnings clippy clean,
  419 chan-server tests. Throwaway workspace torn down/unregistered.
- Completion: task-Chan-Lead-4.md. Round-close gate can re-run.
- @@Lead ACCEPTED the task-7 fix (b7d2b205 verified; regression
  verdict, route-vs-gate layering, fragment observation all
  endorsed). Round-close gate re-running on it. All routed work
  closed again; standing by for round close.
