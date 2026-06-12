# task-Chan-Lead-1 — COMPLETE: tidy-up, crates/** + web/** (tasks 1+2+3-rider)

From: @@Chan. To: @@Lead. Covers task-Lead-Chan-1, the task-2
addendum, and task-3's chanwriter rider. Own-gate green AFTER the
last edit: fmt 0 / clippy --workspace --exclude chan-desktop 0
warnings / cargo test --workspace 34 suites all ok / make web-check
0 errors 0 warnings (vitest 173 files, 1706 tests).

## Commits (mine, pathspec-atomic, oldest first)

- d7d0a7e0 chan-shell fixture/help neutralization
- bb049d6c chan-server scrub (31 files)
- 53fe79d3 core crates scrub + reports help-text truth fix
- 01d0cba6 param refactors (ServeArgs, ControlSocketCtx)
- fbeb5c13 8 design.md rewrites + chan-llm README
- dc94b16e chanwriter purge rider
- 51664864 web scrub (60 files)
- a9daa17b shortcuts registry entries + SERVE_LONG_ABOUT resync
- c92e4d14 web warnings to zero
- e60ab688 final stragglers (Slice D/E/F, -a-NN)

## Highlights

- ~330 archaeology hits cleared across both lanes (incl. ~45 hidden
  in GraphPanel.svelte by the grep-shim large-file skip — the rg
  finding you ratified). Final sweep is clean; the ONLY survivor is
  routes/graph.rs:4-5 "Two-phase typeahead UX. Phase 1/2" which is
  product UX phasing, kept deliberately.
- Three comments were FALSE against current code and got corrected,
  not just de-historied (GraphPanel onSetAsScope "dropped" but
  passed; date.ts header denying the calendar popover it opens;
  GraphPanel "display-only" row that is clickable).
- Help-text truth fix (your task-2 item, verified in source before
  editing): `chan add` intro, --reports flag, and Reports subcommand
  no longer claim reports are off by default (IndexConfig::default()
  is true for new workspaces); --semantic-search keeps its correct
  off-by-default wording.
- Warnings: Rust was already 0 → kept 0. Web was 1 svelte-check
  warning + vite chunk-size + 4 ineffective-dynamic-import → all 0.
- 22 ?raw pin-test files audited; every anchor re-anchored at equal
  strength, none weakened. Ticket-coded identifiers
  (MARKDOWN/SOURCE/MEDIA_EXT_RE_FA57) renamed bare with pins synced.

## Shortcuts (the named @@Alex ask) — full enumeration

Store recon: web/src/state/shortcuts.ts was ALREADY the source of
truth for every chord except three. All three are now registry
entries (a9daa17b), and SERVE_LONG_ABOUT was resynced (+3 rows):

1. terminal.richPrompt Cmd+Shift+P (escapeTerminal) — was ad-hoc in
   App.svelte; the terminal menu label was a hardcoded
   formatChord("Mod+Shift+P") that LIED on Linux (claimed
   Ctrl+Shift+P; handler requires physical Cmd). Label now chordFor().
2. app.pane.closeEmpty Mod+W — conditional (empty pane only, else
   browser/native close); note documents the conditionality.
3. terminal.find Mod+F — terminal-local, registered like
   terminal.copy/paste.

Deliberately NOT moved (correct as-is): CodeMirror keymaps (editor-
scoped by design), modal/component-local keys (ConfirmModal,
PathPromptModal, FileTree nav, find bars, popovers), xterm protocol
handlers, Ctrl+D close-exited-tab (terminal-internal state).
Dispatch stays predicate-based in App.svelte — that is the existing
idiom for every registry chord; converting dispatch to a matcher
loop is a behavior-risk refactor I did not take this round.

## Param-struct refactors

Done (01d0cba6): cmd_serve 15-arg tail → ServeArgs (private,
main.rs); control_socket::start 9+tenant → ControlSocketCtx +
handle_request(req, &ctx); both allow(too_many_arguments) dropped.
Recon's "13 desktop/gateway call sites" for start was a generic-name
overcount — verified zero cross-workspace callers, no desktop/
gateway edits needed.

Remaining 6+-param inventory (private fns, flagged not rushed —
threaded-state clusters that deserve a designed ctx pass):
chan-server routes/graph.rs merge_* family (11/9/9/8),
handle_team (11; half its params are request payload, 8 test call
sites), indexer.rs spawn/spawn_coordinator/spawn_watcher_loop
(6/9/8), terminal_sessions::restart (8), fs_graph
build_fs_graph_paged (7), routes/survey create_followup_file (7);
chan-workspace graph::replace_file (10), drafts scan_entries (9) +
promote (6), contacts slug_for (6), contacts/import run (6).

## Design docs (8 + README)

All rewritten from a fresh source read (fbeb5c13). Substantive
corrections are in the commit message; notable: tunnel HelloAck
enum + /{workspace} prefix, tunnel-server registry/header policy,
chan-workspace graph schema v6 + SearchMode, chan-llm nine
StandardTools (README list + stale 0.11 version-pin example → 0.31).
Agents flagged two soft spots they could not fully trace: mobile
sandbox path table (simplified) and hybrid→BM25 silent degrade
(kept, supported by facade comments).

## Judgment flags (behavior-adjacent calls for your review)

1. state.rs: dropped a stale #[allow(dead_code)] on survey_bus — its
   own comment said to remove it once the reply route consumed the
   bus, which it does. Only non-comment scrub change in crates.
2. facade.rs runtime tracing string: dropped the "C-CAP: " prefix
   (ticket leak to users); message body unchanged, nothing asserts it.
3. Test-only renames: $SYSTACEAN_RESTART → $CHAN_TEST_RESTART env
   var, chan-test-b5-mcp.sock → chan-test-mcp.sock.
4. vite: chunkSizeWarningLimit 1600 (main chunk ~1.5 MB; embedded
   localhost bundle, splitting is a non-goal — documented inline,
   ceiling kept so regressions warn) + targeted onwarn drop of
   INEFFECTIVE_DYNAMIC_IMPORT (deliberate cycle-breaker + codemirror
   vendor overlap, both documented). If you'd rather carry the
   warnings than suppress them, both are one-line reverts.
5. RichPrompt a11y warning fixed via svelte-ignore extension (the
   container keydown is an Escape trap, not an interactive control).
6. mermaid-widget/date-widget/etc. comment fixes rode the scrub; the
   date.ts header rewrite corrects a false claim, worth a glance.
7. Left in place, enumerated for a future pass (no mandated pattern
   matches, some test-pinned): GI-1/2/5/6/8 + F1 codes in GraphPanel,
   F4 (FileEditorTab), A6 (EmptyPaneCarousel), G1/B9 inside two test
   regders pinned by tests, and SliceF/Slice4b test FILENAMES
   (renaming files felt like churn without a mandate — say the word).
8. chan-llm README "0.11" dependency example bumped to "0.31" —
   flagging since version-pin policy is yours/release's.

## Lowlights / friction

- The grep-shim large-file skip cost a real detour and nearly
  shipped a false "GraphPanel clean" — worth a permanent note in the
  round process docs; rg --text everywhere now.
- richPromptTerminalWiring pin: my shortcuts edit staled it and my
  scoped 8-file test run missed it; the tail agent's full vitest
  caught it. Lesson: full vitest after any pinned-source edit, not a
  curated file list.
- Recon overcounts (param call sites) — verify before refactoring.

## Cross-lane

- gateway/** and desktop/** untouched by me (one desktop README line
  was in my chan-llm README, not desktop/). No persisted-field
  changes anywhere, so docs/config-reference.md is unaffected.
- task-3 file-drop: contract acked + frozen with @@ChanDesktop per
  your task-4 amendments (Files discriminator vitest-pinned, ACL
  degrade); I start the guard half now.
