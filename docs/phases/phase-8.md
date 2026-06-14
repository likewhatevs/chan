# Phase 8 - bug sweep, signed-DMG pipeline, and public-flip prep

Status: closed
Span: 2026-05-19 to 2026-05-23 (estimate; basis: git commit dates on the tree, corroborated by dated section headers in the journals)
Versions: v0.11.1, v0.11.2, v0.12.0, v0.13.0
Tags: #bugfixes #signing #release #opensource #ci #desktop #docs

## Roadmap (the asks)

The north-star ask: ship a notarized macOS DMG (plus signed Windows and Linux equivalents) that users can install without Gatekeeper or SmartScreen friction, with tag-triggered CI producing signed installer artifacts hosted through the release pipeline.

The phase also carried a large pre-existing bug list (about 95 entries, roughly 186 KB; the durable audit trail is `raw/phase-8-bugs.md` in git history) plus a planned public-flip of the repository.

The work split into three rounds:

- Round 1: close every item in the bug list, plus a 2026-05-20 detour to stop embedding the BGE-small model in the binary (about 89 MB down to about 26 MB) and make semantic search opt-in.
- Round 2: seven backlog items from phase 7, plus exercise the full signed and notarized DMG pipeline with real Apple Developer ID secrets while the repo was still private.
- Round 3: open-source the repo (license and community files, history audit, then flip public), a multi-model search picker, and a whole-codebase cleanup, hardening, and docs-review pass.

The split from two rounds to three was deliberate: exercising the signing pipeline end-to-end before the repo went public de-risked an irreversible action.

## Rounds and waves

**Round 1** focused on the bug list and the model un-embed. No binary was cut at its close. The BGE-small detour removed the embedded model from the default binary; an `embed-model` feature flag preserved offline use.

**Round 2** cleared phase-7 backlog items 1-7 and exercised the signed DMG pipeline four times as dry-run tags before producing a notarized artifact. Releases v0.11.1 and v0.11.2 were cut here; v0.11.1 had been cancelled at the mid-phase restructure and reactivated later for a rich-prompt-fix mini-wave.

**Round 3** opened with public-flip prep (Apache-2.0 license, contributing, conduct, and security files, issue and PR templates, an outside-reader explainer of the multi-agent pattern, a clean history audit). It also added a multi-model search picker and ran a broad cleanup pass. Closed by cutting v0.12.0 and v0.13.0 (the public-flip version).

## Team and coordination

Ten agent handles were active across the three rounds. The full roster and role profiles live in ../agents/README.md; the phase-specific assignments were:

```
handle          role this phase
--------------  --------------------------------------------------
@@Architect     plan, dispatch, decisions, journal
@@FullStackA    backend + frontend; busiest lane (about 100 tasks)
@@FullStackB    same profile; chan-desktop + PTY work
@@Systacean     CLI, build, deps, indexer, release cuts
@@CI            new 6th slot: Actions, signing, release pipeline
@@WebtestA      Chrome-MCP walkthrough lane
@@WebtestB      Chrome-MCP walkthrough lane
@@Desktect      chan-desktop product architect (Round 3 only)
@@Desktacean    Tauri/Rust + macOS/Linux desktop (Round 3 only)
@@Desktest      desktop tester (Round 3 only)
```

The chan-desktop team (@@Desktect, @@Desktacean, @@Desktest) spun up mid-Round 3 and ran as a parallel team, with @@Alex bridging decisional traffic between leads.

Coordination scheme: per-author directories under the phase, each holding numbered append-only task files plus one canonical `journal.md`. The shared event hub was `alex/`. Corrections were new dated appends with back-links, never rewrites. Event channels were `alex/event-<from>-<to>.md` (one file per directed channel). The architect-orchestrated loop: cut a task, poke the lane, the lane implements and runs the pre-push gate and pokes back commit-ready, the architect clears, the lane self-commits with per-path staging and a pre- and post-commit audit.

This hand-run dispatch shape was the deliberate automation blueprint. The process spec pinned a watcher event-file naming convention enforced across three filter sites, and `raw/rich-prompt/events/` (in git history) holds real JSON event files from a live watcher smoke test where @@Alex pointed chan's own rich-prompt watcher at the journals directory. That smoke surfaced the watcher-versus-journal shape gap recorded in the retrospective.

## What shipped, tried, and undone

**Shipped:**
- Large slice of the bug list closed across all lanes.
- BGE-small model un-embedded; semantic search made opt-in; binary size reduced from about 89 MB to about 26 MB.
- Rich-prompt session-conductor evolution: clear-on-submit, on-disk history, shell/agent submit toggle, team-spawn band.
- Signed and notarized DMG pipeline exercised end-to-end with real Apple Developer ID keys while the repo was private.
- Hybrid back-side as a per-surface settings surface with an About section and a donation QR code.
- chan-report per-directory aggregation.
- Drafts metadata folder.
- Five-surface right-click menu revamp.
- Config-driven Team feature.
- Screensaver with PIN unlock.
- chan-server async-blocking cleanup across many handlers.
- Public-flip docs (license, contributing, conduct, security, issue and PR templates, multi-agent pattern explainer).
- Releases v0.11.1, v0.11.2, v0.12.0, and v0.13.0.

**Tried then corrected:**
- v0.11.1 was cancelled at the 2026-05-20 restructure, then reactivated days later for a rich-prompt-fix mini-wave; the cut happened but the plan flip-flopped.
- The terminal-glyph one-liner hypothesis turned out to be a no-op (xterm.js 6.x already sets that default); the real fix was loading the WebGL addon, because the DOM renderer ignores custom glyphs entirely.
- The watcher-versus-journal convergence gap was captured as design work rather than fixed; the smoke test was left as-is.
- A history rewrite of a cross-agent commit incident was declined because later commits already referenced the affected SHAs; resolved via an audit-trail note instead.

**Deferred to phase 9:**
- Operational repo-flip steps (docs ready, flip itself deferred).
- Multi-model search picker.
- Metadata import/export.
- Desktop-native single-binary vision.
- chan-desktop runtime walk on the DMG.

## Retrospective

**Highlights:**
- The signed and notarized macOS DMG pipeline was exercised four times as dry-run tags before producing a real notarized artifact. The decision to split into three rounds specifically to hold that exercise behind a private repo was correct; it de-risked an irreversible action.
- Three releases cut during the phase, opening on v0.11.0 and closing on v0.13.0 (the public-flip version). The release cadence was denser than any prior phase.
- Public-flip pre-flight landed clean: the history audit came back with nothing to rewrite, and all community files were in place before the flip.
- Empirical audit at pickup worked in both directions; lanes caught architect-side scope errors before touching code.
- The secrets-boundary pattern (architect directs CI on secret NAMES in workflow YAML; @@Alex populates VALUES in Actions Secrets) held cleanly across the signing work.

**Lowlights and contention:**
- The per-PR CI gate was silently broken for about 15 commits because the clippy job was missing Linux GTK and glib dev headers. It took several gate-unblocker tasks to fully green the Actions matrix. A broken CI gate is not discovered by the lanes that were running; it surfaces only when someone checks the Actions tab.
- Several cross-agent commit-hygiene incidents in the shared worktree: a broad `git add` swept another lane's staged files into a commit; a commit absorbed a stowaway hunk. All recoverable, but each cost a round-trip correction and reinforced the per-path staging discipline.
- The live watcher smoke revealed that the runtime watcher handles create and rename events but NOT data-append. It also parses every fired file as event JSON, so markdown journal appends never dispatch. The audit-trail-versus-wire-shape convergence the blueprint assumed was resolved was, in fact, not resolved. The gap was captured as design work and left open.
- The architect twice invented module capability descriptions from a name and intuition rather than reading the source. Both misfires reached the journals before correction and fed the ground-descriptions-in-source rule (now in MEMORY.md).
- A webtest Gatekeeper verification task overstepped scope: it overwrote the installed application bundle, sent a signal to a running PID inferred by elapsed time rather than captured at spawn, and quarantined a system path. The incident tightened the standing-permission subset for DMG install walks.

**Constructive feedback / lessons:**

For the team:
- Shared-worktree commit discipline is correct but slipped under load. The fix is to collapse `git add <paths>`, the diff audit, and `git commit -- <paths>` into one chained invocation so no inter-command window exists for a peer's concurrent staging to contaminate the set. Plain `git add` + `git commit`, even chained with `&&`, does not close that window.
- Webtest lanes must capture the launched process PID at spawn and signal only that PID. Never infer ownership by elapsed time. Never touch the installed application bundle or quarantine system paths during a verification walk.

For @@Architect:
- Ground every capability description and scope claim in the actual source before writing it. Do not paraphrase a peer's functional framing as a location or structural claim; they are different kinds of assertions.
- Write recycle handover entries closer to the actual tear-down beat. Two lanes self-committed past committable markers in this phase because the handover was written before tear-down; the mismatch cost correction commits.

For @@Alex:
- The mid-phase restructures (two rounds to three, and a cut cadence that drifted through four version tags) were each well-reasoned, and the journals absorbed them cleanly. The cost was repeated plan-churn that required all active lanes to re-read their queues. Locking the round structure earlier would reduce this overhead, though the de-risking rationale for the split was valid.

## Notes

Terminology in use during this phase: "rich prompt" refers to what later versions call "Team Work". "chan-drive" or "drive" in earlier journal entries refers to the workspace directory managed by chan-workspace (not a cloud storage product or the tunnel domain). "folder" in older entries means directory.

The raw working material (per-author journals, task files, request and roadmap files, process spec, coordination logs, and live watcher smoke event files) is preserved in git history under `docs/journals/phase-8/`; that tree was removed from the working tree during the phase-15 docs cleanup.
