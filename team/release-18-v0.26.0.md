# Phase 18 - hybrid-surface bug sweep, inspector pills, repo/docs fold (v0.26.0)

Status: closed
Span: 2026-06-04 (one day; opened, run as a single round, and cut v0.26.0 the same day, continuing phase-17's v0.25.0). Based on git author dates and dated journal headers.
Versions: v0.26.0 (cut 2026-06-04)
Tags: #bugfixes #editor #graph #file-browser #inspector #terminal #desktop #docs #release

## Roadmap (the asks)

Phase 18 ran on one @@Alex report against the live IDE (the v0.26.0 TODO), a domain-by-domain bug-and-enhancement sweep plus a repo/docs consolidation and the inspector redesign. The release wave (a new desktop bug, the version bump, the gate, and a stream of hand-smoke fixes) expanded live at round close after the team was wound down to a single release lane. Summarized by area:

**Repo cleanup.** Consolidate `docs/journals` into per-phase `docs/phases/ phase-N.md` docs (each phase's roadmap, rounds, waves, retrospective) and distill `docs/agents` into a minimal referenced set plus a lessons-learned playbook, capturing the ESSENCE so new agents learn from prior execution, successes, and mistakes. Then delete `.claude`, `.codex`, `docs/archive`, the trimmed `docs/agents` leftovers, and `docs/journals`.

**Editor.** Make bullet and hyphen lists behave like ENUMERATED lists for cursor, indent, and clicks (today arrow-down between unordered items lands the caret BEFORE the glyph). Restore distinct HYPHEN lists (phase-17's Google-Docs glyph change was meant only for bullet lists, but hyphen lists regressed into bullets too). Fix trackpad free-scroll: the scroll hangs, jumps opposite, then settles when the caret is far from the scroll target. Make `[[` complete LOCAL WORKSPACE PATHS, not only filename/heading targets.

**File Browser.** The tab right-click menu got merged with the less-complete docked file-browser menu (a phase-17 regression). Remove "Reload"; below "Expand all directories" add "New file or Directory", "New Terminal", and "New Graph" (all from the workspace root). Show keyboard-shortcut hints in the selection context menu (New Terminal, New Graph, Delete, Settings) and record any missing chord in the central shortcut store so it ports to linux/ macos/web. Fix the "Loading" hang on directory expand that only a window reload clears (a `history.replaceState` SecurityError in the console).

**Graph.** "Graph from here" must SELECT the originating node on the redrawn graph. Plot no directory node without a visible edge to the workspace root (a Drafts folder outside the workspace, or `src/`, was floating). Stop a binary file / symlink from rendering as a contact node, and STOP the graph reloading every few seconds on ANY workspace file edit (even out-of-scope files). Add a "Copy link to graph" right-click action (replacing "Reload") that reproduces the tab, openable from a markdown file.

**Terminal.** Hiding the rich prompt (menu or Cmd+Shift+P) must return focus to the terminal. Show context-menu chords and wire Cmd+C / Cmd+V copy/paste. Fix UTF-8 garble in both `less` and `vim` (multibyte renders as raw bytes).

**Inspector.** Replace the flat action-button stack with a single PILL (main action) plus a dropdown (secondary actions) per item category, across all hybrid surfaces: File Browser Directory / File / Media / Binary, and the editor "Show Details". "New terminal here" seeds the terminal with `{cursor}{space}{relative-path}`.

**chan-desktop.** The local-disk New-workspace flow still shows the OLD pre-flight dialog, which conflicts with the SPA boot menu. Pre-flight moved to the SPA in phase-17; remove the desktop-side dialog entirely.

## Rounds and waves

The phase ran as ONE round of six worker lanes plus a coordinating lead, dispatched in three waves, then wound down into a single release lane.

### Round 1: domain lanes, three waves

The lead split the v0.26.0 TODO across six coherent domains (not fixed file enumerations, the phase-17 lesson) and dispatched in waves: Wave 1 all six lanes start in parallel on their isolated items; Wave 2 the lead sequences the shared-file convergence and runs the consolidated smoke; Wave 3 the repo/docs fold and the deletions as the final close-out step. Concrete outcomes, by lane:

- Editor (@@LaneA): list cursor/click/indent parity, distinct hyphen lists, free-scroll (removed `scroll-behavior: smooth` from the CM scroller, which was animating CM6's own scrollTop height-estimation corrections and fighting the trackpad pan), and `[[` path autocomplete. The `[[` work was recon'd into a CLIENT-SIDE solution off the existing `/api/files` tree, so it needed NO chan-server route change and NO chan-workspace `graph.rs` change (the recon dissolved a pre-set cross-lane sequence). Three @@Alex live-test bugs followed, all bullet-specific (nested EOL-click and mid-text click landing the caret at line start), which led to the round's highest-value steer: replace the bullet caret-snap scaffolding with REAL glyph-character replace-widgets so CM handles cursor/click/arrow natively, net minus 81 lines.
- Graph (@@LaneB): select-on-from-here (resolve the pending id against node id OR path, since a directory id is `directory:<path>` while a file id is the bare path); dir-edges (gate the synthesized Drafts layer to Workspace scope so the floating `drafts_link` does not appear at dir/file scope); binary-node (a FRONTEND fix, mapping the `symlink` filesystem kind to a file-shaped node, NOT a Rust indexer stamp as first hypothesized); stop the spurious reload (carry changed paths in the reload signal and gate each panel's reload on whether a changed path is in ITS scope); and copy-link (the `chan://graph?...` serializer half). A follow-up @@Alex bug fixed persisting the selected node across a window reload.
- File Browser (@@LaneC): the tab-menu regression (drop "Reload" and its dead handlers, add the three workspace-root actions), shortcut hints read from the central store via `chordFor`, and the loading hang (dedup `replaceState` when the URL is unchanged plus a 150ms debounced `schedulePersistStateToHash`, keeping the synchronous path for the pagehide flush).
- Inspector (@@LaneD): the whole pill-plus-dropdown redesign in `FileInfoBody.svelte` across all five categories. `fromHere.ts` was left unchanged (the existing seed already produced `{cursor}{space}{path}`), which closed the D/C coupling cleanly.
- Terminal + chan-desktop (@@LaneE): rich-prompt hide returns focus to the xterm (one reactive watcher covers all three hide paths); copy/paste chords (Cmd+C/V on macOS, Ctrl+Shift+C/V elsewhere so bare Ctrl stays SIGINT); UTF-8 locale on PTY spawn (`LANG=C.UTF-8` when the inherited env selects no UTF-8 codeset); and removal of the desktop pre-flight (the JS dialog AND the now-dead Rust backend, including a gate-blind stale Tauri permission the cargo gate could not see).
- Repo / docs (@@LaneF): consolidated phases 1-17 into `docs/phases/` (a six-section template, fanned out one subagent per phase), wrote the agent playbook, and scrubbed stale `docs/journals` references from shipping code comments, CHANGELOG, the public `coordination.md`, and the kept agent cards BEFORE any deletion (so URLs/links did not become dead). Nothing was deleted during the round; the deletions were the held final step.

### Release wave: single lane, version cut, hand-smoke fixes

At round close @@Alex wound the team down and recycled @@LaneE as the single RELEASE lane (the others and the lead cleared out). The release lane then:

- Fixed a NEW desktop OFF/ON toggle race (fix(desktop) 20526d0c): turning a workspace OFF flipped the toggle in the UI before the chan-server actually shut down, so a quick OFF->ON hit a still-held workspace flock and stranded the row "ON but no Open". The fix disables the toggle for the whole start/stop transition, force-reconciles the DOM to the true serve state (bypassing the list-JSON dedupe), and retries `open_workspace` on a still-releasing flock (8x150ms, mirroring the close-side `unregister_with_retry`).
- Bumped all version pins 0.25.0 -> 0.26.0 in lockstep (ca9cb40b): the workspace `[workspace.package]` plus the internal dependency pins, `tauri.conf.json`, `web/package.json`, the root `Cargo.lock`, AND `gateway/Cargo.toml` plus `gateway/Cargo.lock`.
- Landed five more fixes during @@Alex's hand-smoke: list alignment guides pinned to a fixed x (7a114943; the per-depth margin had been dragging the guide `::before`); the Drafts NODE inspector showing a single Terminal-from-here button (9fb3ec4c); the draft FILE inspector populating (625debf5; draft files live outside the workspace tree and `/api/inspector`'s classify errored, so the server now resolves drafts via `resolve_physical_path` plus `classify_abs` plus a new optional `InspectorPayload.abs_path`, and FileInfoBody synthesizes the draft entry with a single Terminal-from-here seeded with the absolute path); and the dashboard search-index root anchored near the bottom above the carousel scroller (ca8d1ea1).

## Team and coordination

The phase ran as a seven-agent cs-terminal team (`phase-18-team`) under the Team Work bus, with @@Alex as host, @@Lead as the lead and architect, and @@LaneA through @@LaneF as the workers (one coherent domain each). @@Alex set the scope, then tested by hand throughout and authorized autonomous commit (but not push).

The coordination scheme was the per-author-journal-plus-task-file bus carried over from prior phases: the lead cut lean domain-scoped task files (`tasks/task-<from>-<to>-N.md`, context living in the round plan and the draft rather than the poke), pinged a one-line poke pointing at each, and workers wrote completion notes back into the task files plus their own append-only journal. Each lane gated its own slice green (cargo fmt / clippy -D warnings / test for Rust; `make web-check` for the SPA; desktop dev test/clippy for the Tauri crate) and reported a pathspec fingerprint; the lead owned the full-tree gate from an isolated `gate.sh` worktree (which gates the COMMITTED state, immune to peers' WIP), the per-lane atomic commits with verified staged stats, and the shared-file merges. Surveys over the `cs terminal survey` channel were the agreed way to reach the host. One early gotcha: this team's config has no @@Alex member tab, so a survey must target a tab the host's window owns (the lead used `--tab-name=@@Lead`, the tab @@Alex had poked).

At round close the lead handed the RELEASE end-to-end to @@LaneE via `RELEASE-HANDOFF.md` (the committed state, @@LaneF's documented-and-staged pending Wave-3, the release mechanics and caveats, and a smoke-checklist of CHECKED vs YET-TO-CHECK items), then cleared out once @@LaneE confirmed "release handoff accepted". The lead left an explicit reminder to fold `phase-18.md` BEFORE the `git rm docs/journals` (since that deletion removes the journals, the handoff, and the bus the fold draws from).

## What shipped, tried, and undone

**Shipped (v0.26.0).** Editor list cursor/glyph/click parity, distinct hyphen lists, free-scroll, `[[` workspace-path autocomplete, and the bullet-marker cleanup (real glyph-widget markers, snap scaffolding deleted, outline indent); inspector pill-plus-dropdown per category; terminal UTF-8 locale on PTY spawn, copy/paste chords, rich-prompt hide-to-focus, and the desktop pre-flight removal; the five graph items (select-on-from-here, dir-root edges, binary node, stop spurious reload, copy-link-to-graph click-to-open) plus the persist-selection-across-reload fix; the File Browser tab-menu root actions with shortcut hints and the debounced hash write fixing the Loading hang; the repo/docs consolidation (phases 1-17 into `docs/phases`, the agent playbook, the scrubs, the `coordination.md` rewrite); the desktop OFF/ON toggle-race fix; the unified 0.26.0 version bump; and the five hand-smoke fixes (list alignment guides, Drafts-node and draft-file inspector, dashboard search-index anchor).

**Tried then corrected.** Several tasks were framed wrong by the report or by the lead's recon and the lanes corrected to the real cause: the binary-as-contact bug was a frontend symlink-mapping miss, not a Rust indexer stamp; the `[[` path completion needed no backend at all (a pre-set graph.rs cross-lane sequence became moot); the persist-selection gap was the persist TRIGGER not a missing write (the lead's grep missed the `tab.` form and the NUL-binary file). On the editor, @@LaneA first patched the nested-click bugs by adding more caret-snap guard branches; @@Alex's "cleanup not scaffolding" steer redirected that into deleting the scaffolding entirely (real positioned glyph characters give both the Google-Docs look AND native CM positioning, so the band-aids were unnecessary). A fmt nit slipped @@LaneB's scoped gate (run before its last edit) and the integrated gate caught it. Two hand-smoke "bugs" (the dashboard inspector still flat; graph files missing parent-dir edges) turned out to be ALREADY-FIXED in the current code; @@Alex was viewing his older long-running team environment, not the freshly-built binary.

**Deliberately not done / deferred.** The `[[` directory drill-down rows (files-only meets the "complete paths" spec; directory drill-down is an optional bounded follow-up). The pre-existing NUL-byte edge-key separators in `GraphPanel.svelte` (out of scope; semantically risky for no payoff). The WKWebView and real-trackpad hand-smokes (rich-prompt hide-to-focus, terminal clipboard, the desktop double-dialog removal, the desktop toggle-race fix, and the editor free-scroll) stayed @@Alex's to run, since agents can drive Chrome (Blink) but not WKWebView or a real trackpad. The known external gap is NOT a code bug: `chan.app/dl/*/latest.json` 404s because the chan.app -> Pages routing for `/dl` is unfixed external infra, so desktop self-upgrade 404s; that was not a release blocker.

## Retrospective

This is the learning payload, distilled from the round retrospective.

**Highlights.**

- Recon dissolved contention. Two cross-crate worries the lead pre-set (the `[[` chan-server/graph.rs route, the contact-stamp Rust lockstep) both evaporated once the lanes recon'd the real code (@@LaneA's client-side `[[` off `/api/files`; @@LaneB's frontend symlink fix). Let the lane recon BEFORE locking a cross-lane sequence.
- @@Alex's "cleanup not scaffolding" steer was the single highest-value signal. It turned a growing pile of bullet caret-snap band-aids into a minus-81-line simplification (real glyph-widget markers giving native CM cursor/click). This validates the "ground in source / simplify" instinct.
- Agents verified rather than followed. @@LaneB corrected the lead's recon twice; @@LaneE's grep caught a gate-blind dead Tauri permission; @@LaneF's hold-time recon caught five scrub misses (including the public `coordination.md`) before a deletion would have made dead links; @@LaneA empirically root-caused (large negative text-indent breaks CM6 `posAtCoords`) instead of trusting the hypothesis.
- Clean delivery: per-lane atomic commits with verified staged stats, the isolated full gate green (core fmt/clippy/test plus `--no-default-features` plus gateway build plus the codesigned desktop DMG), and the held-then-ordered deletions never removed the live bus mid-fix.

**Lowlights / contention.**

- The lead's recon was repeatedly off (the graph.rs `[[` contention was moot; the contact-stamp Rust hypothesis was wrong; a `selectedNodeId` grep missed the `tab.` form plus the NUL-binary; the bullet bug was mis-attributed to item 1). It cost a couple of redirect cycles. Ground recon harder, use `grep -a` on the NUL-bearing `GraphPanel.svelte`, and offer hypotheses as hypotheses so the lane corrects them.
- @@Alex found three click-mapping bugs by hand that both the agent self-smokes AND the lead's consolidated Chrome smoke MISSED. They were runtime pointer-geometry issues (mid-text and EOL clicks on nested rows). Smoke real POINTER interactions at depth, not just element presence.
- Late-session hand-smoke false positives. Two reported "bugs" were already fixed in the current code; @@Alex was smoking a stale long-running team environment, not the freshly-built binary. When hand-smoking late in a long session, smoke the freshly-built artifact, not the stale running one.
- A fmt nit slipped a lane's scoped gate (gate ran before the last edit); the integrated gate caught it. Reinforces gate-after-last-edit.
- The survey-target gotcha (the host has no member tab) cost an early retry.

**Lessons worth carrying forward.**

- Let lanes recon before locking cross-lane sequences or lockstep. Several pre-emptive couplings were unnecessary and a couple of hypotheses were simply wrong.
- CSS `::before` plus zero-width-source marker glyphs decouple the rendered glyph from the source position and break CM6 click/cursor mapping, which then needs a snap band-aid per case. Use real positioned glyph characters (a replace-widget) so the default editor coordinate mapping just works.
- Smoke pointer interactions at depth (mid-text, EOL, nested) on a fresh build, since static gates and element-presence smokes miss runtime pointer geometry, and a stale running server hides the fix.
- A required field added to a cross-workspace struct, and Tauri permission/ manifest entries, are gate-blind: the cargo gate stays green while a stale permission or a missing construction site rides through. Grep the whole repo (including the separate gateway and desktop workspaces) and build them.
- The full gate from an isolated worktree earns its keep: it gates the committed state immune to peers' WIP, and it must build EVERY workspace CI ships (core plus the separate gateway Cargo workspace) plus `--no-default-features` plus the desktop DMG before any tag.
- Hold irreversible deletions until the round is settled. Deleting `docs/journals` would have killed the live team bus mid-fix; the fold of `phase-18.md` has to land BEFORE the journals are removed.

**Feedback recorded for the agents.** Excellent work: empirical root-causing, recon corrections, commit discipline, and clean pathspec hygiene in a shared tree. The willingness to push back on the architect's wrong recon is exactly what kept quality up.

**Feedback recorded for @@Alex.** The hands-on testing plus the architectural "cleanup" steer were the round's quality backbone; the automated gates would have shipped the over-scaffolded bullets green. Direct-to-lane asks worked because the lane routed the OUTCOME back to @@Lead. One process note: smoke the freshly-built binary, not the long-running team server, when hand-checking late finds, to avoid chasing already-fixed "bugs".

**Feedback recorded for the architect (@@Lead).** Held the irreversible deletions correctly (never deleted the live bus mid-fix), kept the round open rather than declaring done with known bugs, ran the clean handoff to the release lane, and the lean task-file plus one-line-poke bus held. But tighten recon before cutting cross-lane sequencing, and offer hypotheses as hypotheses; several pre-emptive couplings were unnecessary and a couple of recon claims were wrong.

## Notes

Terminology drift, for mapping old names to current ones:

- "Inspector" is the per-item detail panel (`FileInfoBody.svelte`) rendered on five surfaces: File Browser, the editor "Show Details", the dashboard index graph, and Search. This phase replaced its flat button stack with the pill-plus-dropdown shape per item category.
- "Rich Prompt" is the floating Cmd+Shift+P compose bubble over a terminal; this phase returned focus to the xterm on hide.
- "Drafts" is the uncommitted-workspace area whose paths can resolve to chan metadata OUTSIDE the workspace root; both the graph (a floating `drafts_link` node) and the inspector (a draft file outside the workspace tree) needed draft-aware handling this phase.
- "graph from here" produces a graph scoped to the chosen node; the node id is `directory:<path>` for directories and the bare path for files, which is why selection resolution must match id OR path.
- The `chan://graph?s=&d=&m=&f=&n=` link scheme serializes a graph tab (scope, depth, mode, filters, selected node) so it can be copied into a markdown file and clicked to reopen the tab.
- "C.UTF-8" is the locale set on PTY spawn when the inherited env selects no UTF-8 codeset (present on macOS, every musl Linux build, and glibc >= 2.35).
- The known `/dl` Pages routing gap is external chan.app infra, distinct from any CI or release-build failure.

The raw working material (the per-lane and lead journals, the task and followup files, the round-1 draft and plan, the release handoff and smoke checklist, the desktop-toggle bug note, and the docs-consolidation specs) is preserved in git history under docs/journals/phase-18/; that tree was removed from the working tree in the docs cleanup.
