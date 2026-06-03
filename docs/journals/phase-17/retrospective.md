# Phase-17 retrospective

Session: phase-17 round-1 + round-2, run as a 4-lane team (@@LaneA lead +
@@LaneB/C/D) with @@Alex as host. @@Alex set scope, then stepped away mid-session
authorizing autonomous commit + push.

## Done (committed + pushed to origin/main)

Round-1 (5 commits, 45a6e341..03bb91f8):
- B1 rich-prompt per-terminal + data-loss fix; B8 codex submit (bracketed
  paste); B12 direct dashboard chord; B4 cs pane split RIGHT|BOTTOM + no
  focus-steal (@@LaneB).
- B2 depth-cycle bullet glyphs; B6 lazy-tree path autocomplete; B9 graph
  expand/slider/layers (@@LaneC).
- B11 editable-by-content sniff; B10 serve-progress (watch() stall heads-up);
  B5 MCP env off by default + team toggle (@@LaneD).
- S1/S2/S3 launcher copy; B3 team-load autocomplete; MCP-toggle UI; E1
  auto-assign; search path-autocomplete (@@LaneA).
- R2-1 in-app open-source attribution; D1 README/home/manuals (@@LaneD).

Round-2 (R2-2 + R2-3):
- R2-2 list paste-link indent + top-level outdent (@@LaneC).
- R2-3 per-terminal surveys: contract amendment (lead) + transport (@@LaneD) +
  SPA (@@LaneB), verified end-to-end.

## Pending (tracked in deferred-backlog.md, for @@Alex's return)

- Browser-smoke of the round-1 SPA changes (B3 / MCP toggle / E1 / search) +
  C's R2-2: Chrome automation was permission-denied while @@Alex was away (it
  worked for @@LaneB's R2-3). Shipped gated-green, not interactively smoked.
- WKWebView hand-smokes: launcher S1/S2/S3, native Cmd+Shift+D.
- D1 publish: live install/clone/tunnel commands + the mermaid canonical-vs-
  mirror choice.
- Deferred features (round-3+): F1 rich-prompt loader/cancel + prompt-ack, F2
  async watch() setup, F3 BM25-index sniffed text, F4 search prioritized
  leaf-index.

## Highlights

- Root-cause depth. Lanes dug past the task's premise to the real cause: B8
  (codex coalesces text+CR into a paste burst), B10 (the stall is watch() setup
  not indexing), B6 (the file tree is lazy), B1 (window-global bubble
  visibility), R2-2 (turndown emits a stray "-   " marker). Several tasks were
  framed wrong; the lanes corrected them.
- Lane-boundary discipline. B9->GraphCanvas, B4->applyPaneExec, R2-2->list.ts/
  paste_html.ts, R2-3->survey contract: every time the real fix sat in an
  unlisted file, the lane STOPPED and routed instead of reaching across. @@LaneC
  self-corrected a flaky-grep claim by anchoring on an atomic Read; @@LaneB
  caught R2-3 as a contract change rather than a one-file edit.
- The full gate earned its keep: it caught @@LaneD's B10 chan-desktop
  ServeConfig.verbose miss (a separate Cargo workspace the scoped gate is blind
  to) before it reached the remote.
- Clean delivery: per-lane atomic commits with verified staged stats, the
  isolated/full gate green, foreground push + git ls-remote verify.

## Lowlights / improve

- The bootstrap's owned-file lists under-specified the editor/state boundaries,
  so lanes repeatedly surfaced "the real fix is in an unlisted file" (B9, B4,
  R2-2, R2-3). Make the lists domain-based (lane = a coherent area) rather than
  a fixed enumeration, or scope the recon deeper before assigning.
- Dual-team-in-one-worktree (the leftover new-team-1 + phase-17) cost early
  cycles and risked corruption: pokes by tab-name span groups, and the prior
  round was never torn down. Tear down a finished round's team before loading
  the next; scope every poke by --tab-group from the start.
- Chrome automation being permission-denied while @@Alex was away blocked the
  interactive smoke of several SPA changes. Pre-grant the permission (or have a
  non-Chrome smoke path) before an autonomous window.

## Feedback per member

- @@LaneB: top form - sharpest recon, best root-causing, caught the R2-3
  contract scope, clean atomic landings, good subagent use. The cs-surface TOML
  injection (no TeamConfig type in chan-shell) was a neat call.
- @@LaneC: strong root-causing (B6 lazy-tree, R2-2 turndown), honest
  self-correction on the flaky grep, and a clean revert so the round-1 commit
  stayed pristine. Verified its GraphPanel subagent rather than trusting it.
- @@LaneD: thorough and deterministic - shipped an in-codebase e2e test when
  Chrome was denied, kept the no-config-writes invariant, and owned its one
  blind spot (chan-desktop, a separate workspace its scoped gate missed).

## Feedback for @@Alex

- The rapid requirements + the cs-survey channel worked well once set up; the
  "ask me via survey, not the TUI" correction was right (TUI typing collides
  with the poke queue).
- Biggest friction was the leftover team in the worktree - close a finished
  round's team before loading a new one.
- Consider domain-based owned-file lists in the bootstrap, and pre-granting
  Chrome for autonomous windows.

## Feedback for the lead/architect (me)

- Good: kept all four lanes unblocked, made obvious calls without escalating,
  sequenced the shared chan-server window (B4 vs B5), ratified the survey
  contract cleanly, did careful per-lane commits + verified pushes, and caught/
  fixed @@LaneD's desktop gate break rather than blind-pushing.
- Improve: my own mcpEnv miss - I added a required TeamDialogConfig field but
  only fixed the wire (snake_case) fixtures, not the dialog (camelCase)
  literals, and trusted a scoped vitest (which strips types) instead of
  svelte-checking the type change. @@LaneD's full-tree gate caught it. A
  required-field add to a shared TS type needs a grep of ALL literals +
  svelte-check.
- Improve: I deferred the search-autocomplete smoke repeatedly, then couldn't
  smoke it at all (Chrome denied). Should have written a unit test for the
  path-detection logic (the one piece that had no test) rather than leaning
  entirely on a browser smoke that didn't materialize.
