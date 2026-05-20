# Commit-grouping plan for v0.11.1

Author: @@Architect
Date: 2026-05-20

Round-1 close artifact. All Round-1 commits land in v0.11.1
as-is — they're already at the right thematic granularity,
no re-grouping needed. This file lists the canonical commit
set, the push order, the tag-message draft, and the gating
verifications before @@Systacean cuts `systacean-3`.

**2026-05-20 STATUS — v0.11.1 cut CANCELLED; file repurposed**:

@@Alex restructured the rounds. Round 1 now closes
WITHOUT a binary cut. No v0.11.1 tag. First proper binary
release ships at end of Round 2 (likely v0.12.0 or v1.0 —
@@Alex's call at the time) once the signed+notarized DMG
pipeline has been exercised with real Apple Developer ID
keys.

This file is repurposed as the **Round-1 close plan**:

* The commit list below stays valid — it's the set of
  Round-1 commits that need to land in tree before the
  recycle.
* The "Gating verifications" section stays valid — those
  are the Round-1 close prerequisites (webtest verdicts,
  bug 14 re-attempt, `-b-7` runtime click).
* The "Push order" + "Tag message draft" + "After v0.11.1
  lands" sections are HISTORICAL. **DO NOT execute the
  push / tag commands in those sections.** The
  Round-2-close cut will get its own plan when the time
  comes.
* `systacean-3` (version bump + tag + push) is **cancelled
  for Round 1**. A new task replaces it at Round-2 close.

**Detour added before recycle**: stop embedding the
BGE-small semantic-search model (~89 MB → ~26 MB binary).
Settings toggle + CLI command for opt-in semantic search.
Plus a small UI add: pane-flip animation. These land in
Round 1 (the detour brought them forward so they're in
the first proper release at Round-2 close).

**SHA volatility note**: hashes below are point-in-time
snapshots taken on 2026-05-20. Concurrent rebases / hook-
driven re-commits in a multi-agent working tree mean the
SHA you see at push time may differ even if the content +
parent + metadata are identical (@@Systacean caught one
such drift on `systacean-4`: `d35bbd7` → `07561b2`, same
content, no journal rewrite per append-only). **Subject
lines are the durable identifier — when @@Systacean cuts
`systacean-3`, spot-check by subject + `git show --stat`,
not by trusting the SHAs in this file.**

## Commits to ship (chronological, ~30 commits)

All sit on `main` locally, unpushed. Hashes pinned at the
2026-05-20 snapshot; final values are what @@Systacean
sees when they cut the release.

### Pre-existing on `main` since v0.11.0

```
6d5d8ac ci: gate web/ on every PR (svelte-check + vitest + vite build)
```

### Wave-1 — bug sweep + scaffold (already in HEAD)

```
ebbd4c5 FB tab title (fullstack-a-1)
ec983d3 Status bar (fullstack-a-2)
ccd2f09 Hybrid cluster: status-bar copy + drop flash + immediate-commit (fullstack-a-3)
59fc2ec Rich prompt cluster (fullstack-a-4)
05e00fa Editor cluster (fullstack-a-5)
d98ebc9 Cmd+K F focus (fullstack-a-6)
808c0a4 Hybrid NAV: Cmd+K → Cmd+. (fullstack-a-7)
424dd98 Restore CSS wobble (fullstack-a-8)
203c6e8 chan-desktop window-config LRU stack (fullstack-b-1)
315fcc1 Terminal cluster (fullstack-b-2)
a9579f0 Watcher dialog v1 (fullstack-b-3)
ca8a441 Indexing chart pan/zoom (fullstack-b-4)
28b168a Per-Hybrid theme propagation (fullstack-b-5)
f3ec455 FB watcher scope (fullstack-b-6)
51984c8 CLI scriptability: chan list --json + chan remove --name (systacean-1)
97b82df ci: tag-triggered chan-desktop release scaffold (ci-2)
97ca38a ci: drop stranded desktop/.github/workflows/ci.yml (ci-2 cleanup)
041de34 fullstack-b: poke note for -2/-3/-4/-5/-6 commits landed
```

### Wave-2 — verification + cleanups (already in HEAD)

```
4a04917 Graph: link resolver universe (systacean-2)
d753775 Hybrid NAV [ / ] / - / = fixed direction (fullstack-a-9)
a28f9b2 Tab strip + FB tree fade-out + full-path hover (fullstack-a-10)
a230262 Pin: last back-side tab keeps showingBack (fullstack-a-11)
141aa4d docs: macOS signing brief (ci-3)
887d19c Editor: re-anchor scroll on image decode reflow (fullstack-a-13)
9971bd3 Graph inspector: drop lazy-tree second-ghost (fullstack-a-12)
7513ea2 Editor: autoFocus prop (fullstack-a-14)
6b10272 desktop/Makefile workspace target (systacean fill-in)
a6c02e4 Grant opener IPC to drive/tunnel/main-N windows (fullstack-b-7)
8f339cf Blur xterm-helper-textarea on rich prompt open (fullstack-b-8)
8962893 Hybrid NAV: t alias for terminal spawn (fullstack-b-9)
07561b2 Graph: drop directory link targets from ghost emission (systacean-4) (was d35bbd7 pre-rebase)
641830a Watcher dialog: flip to mode "attach" (fullstack-b-10)
385da20 ci: swap cargo install for taiki-e/install-action (ci-4)
80a34ee event_watcher: skip directory paths silently (systacean-5)
```

### Wave-3 — small follow-ups (pending commit, currently in working tree)

These will land as @@FullStackA processes the 2026-05-20
clearance batch. Each is a single-file commit per the
suggested subjects in each task tail.

```
fullstack-a-15 — New file dialog: select stem + .md (no double-append)
fullstack-a-16 — PaneModeHelp: "Stage:" → "Spawn"
fullstack-a-17 — TerminalTab focus effect: gate on richPrompt.open
fullstack-a-18 — TerminalRichPrompt: thread submit to Wysiwyg
fullstack-a-20 — TerminalRichPrompt onKeydown: respect defaultPrevented (HARD GATE — -a-18 regression)
```

### Wave-3 — last side-observation (just dispatched, in flight)

```
fullstack-a-19 — Hybrid NAV chord-table doc drift cleanup
ci-5           — Cache BGE-small model dir (F3 from CI's audit, pulled into Round 1)
```

**Hard gate added 2026-05-20**: `fullstack-a-20` MUST land
before v0.11.1 tag fires. The `-a-18` thread + missing
`defaultPrevented` check in the wrapper produces a
double-dispatch on wysiwyg-mode Cmd+Enter (typing `pwd`
arrives as `pwdpwd` in the terminal). @@Alex flagged this
on return. -a-20 is a one-line fix; expected to land in
the same session as -a-15/-16/-17/-18/-19 commit pickup.

### Working-tree housekeeping (to commit before tag)

```
docs/agents/ci.md + docs/agents/ci/   — @@CI agent contact card + skill copies (untracked since CI lane stood up)
docs/journals/phase-8/architect/*     — round-2-plan.md + commit-plan-v0.11.1.md (this file) + journal updates
docs/journals/phase-8/alex/event-*.md — all 2026-05-20 event appends
docs/journals/phase-8/*/journal.md    — all 2026-05-20 journal updates
docs/journals/phase-8/*/*-*.md        — task-file clearance + landed appends
docs/journals/phase-8/phase-8-bugs.md — wave-2 + wave-3 entries
docs/journals/phase-7/*               — any phase-7 doc updates that snuck in
docs/agents/bootstrap.md              — sitting from prior session; @@Alex's call whether to roll into this release
notes.md                              — untracked scratch file; @@Alex's call
```

## Gating verifications before tag

@@Systacean does NOT cut `systacean-3` until ALL of these
land:

1. **Wave-3 commits in tree** (fullstack-a-15/-16/-17/-18/-19
   committed by @@FullStackA per the clearance batch).
2. **Webtest verdicts on wave-2**:
   * @@WebtestA: re-verify bug 8 (systacean-2 + -a-12 +
     systacean-4 stack) + verify bug 11 (-a-13) + verify
     bug 20 (-a-14) + verify side observations
     (-a-15/-16/-17).
   * @@WebtestB: verify -b-7 source-side (runtime click
     parked for @@Alex) + verify -b-8/-b-9/-b-10 + verify
     systacean-5.
3. **Webtest CNR re-attempt on bug 14** (watcher first-try
   hang): either reproduces and gets dispatched, or stays
   CNR and gets struck from the Round-1 list.
4. **@@Alex's runtime click on `fullstack-b-7`** (Chan.app
   external link test). Either @@Alex runs it themselves
   per option A of the alex-bound permission ask, OR
   @@Alex approves @@FullStackB to run `make run` per
   option B. Either way, the empirical click-check must
   pass before tag.
5. **@@Alex's return + explicit "cut v0.11.1"** signal.
   This file unblocks `systacean-3` only after @@Alex
   confirms.
6. **Final composite pre-push gate**: `scripts/pre-push`
   from a clean working tree (no uncommitted files
   beyond the housekeeping doc bundle).

## Push order

Standard chan release shape, lifted from prior releases
(see v0.11.0's pattern):

1. `cd <repo-root>` and confirm `git status --short` shows
   only the intended housekeeping doc bundle uncommitted.
2. Final commit pass on the housekeeping bundle:
   `git add docs/agents/ci.md docs/agents/ci/ docs/journals/`
   plus any other files explicitly listed above. **Do not
   `git add -A` — see @@Systacean's `systacean-4`
   commit-redo incident in the audit trail; cross-agent
   files can otherwise ride along into unintended
   commits.**
3. Commit message:
   ```
   docs: phase-8 journals + commit plan + round-2 plan + ci agent card
   ```
4. Run `scripts/pre-push` from the repo root. Must be
   green.
5. Bump version in `Cargo.toml` (workspace root and any
   crate-level pins) from 0.11.0 → 0.11.1.
6. `git commit -am "Release v0.11.1"`.
7. `git tag -a chan-v0.11.1 -m "<tag message below>"`.
8. `git push origin main --follow-tags` (single command
   pushes branch + tag together; tag-triggered
   `release-desktop.yml` fires automatically).
9. Wait for CI to upload artifacts to the GitHub Release
   page. Until @@Alex completes the cert provisioning per
   the ci-3 brief, the macOS DMG ships unsigned (Tauri
   plug-in updater key from `desktop/CLAUDE.md` covers
   tauri-updater signing; Apple Developer ID notarization
   awaits Round 2).
10. Smoke-test the published DMG / AppImage / MSI on
    @@Alex's workstation.

## Tag message draft

```
Release chan v0.11.1

Round-1 bug sweep + CI infrastructure scaffold. ~30 commits
spanning the phase-8 bug list + the CI lane's GitHub Actions
foundation for the phase-8 north star (notarized DMG via
tag-triggered release; signing pipeline lands in Round 2).

Highlights:

* Editor: image-insert no longer pushes the cursor off-
  screen; autoFocus prop keeps the rich prompt out of
  bubble overlays' way; lazy-tree second-ghost dropped
  from the graph inspector; new file dialog no longer
  double-appends .md.
* Hybrid NAV: rebinding from Cmd+K to Cmd+.; [ / ] / -
  / = move the divider in fixed directions; new t alias
  for terminal spawn covers Win/Linux web where Cmd+T is
  reserved; CSS wobble restored.
* Watcher: scope FB watcher to selection so cross-path
  drive activity stops flickering; dialog accepts
  outside-drive paths + silently creates missing dirs +
  attach-mode instead of move-mode + EISDIR no longer
  toasts on fresh dir attach.
* chan-desktop: external http/https links now open in
  the OS default browser; Cmd+N launcher windows inherit
  capabilities cleanly.
* Graph: resolver universe expands to all on-disk files
  (markdown + non-markdown); directory link targets no
  longer fabricate ghost file nodes.
* CLI: chan list --json + chan remove --name for scripting.
* CI: GitHub Actions scaffold for fmt + clippy + test +
  web/ check + vite build on every PR; tag-triggered
  chan-desktop release scaffold (unsigned for v0.11.1;
  signed in Round 2).

Full bug list at docs/journals/phase-8/phase-8-bugs.md.
Per-fix audit trail at docs/journals/phase-8/<agent>/.
```

## After v0.11.1 lands

* Round-1 sessions for all six working agents are
  recycle-eligible per `agent-recycle` events.
* Round-2 fan-out follows the staged plan in
  [`./round-2-plan.md`](./round-2-plan.md), pending
  @@Alex's confirmation on the six open decisions listed
  in the plan's header.
* `desktop/CLAUDE.md`'s tauri-updater bridge release
  (DEV key → release key rotation) lands in
  `systacean-6` per the round-2-plan numbering.

## What this plan is NOT

* A re-grouping of existing commits. They're at the right
  granularity — one thematic change per commit. No
  squashing.
* A push trigger. Push happens only when @@Alex says "cut
  it". This file just removes the ambiguity about WHAT
  ships when they do.
* A scope-creep gate. New bugs surfaced post-publish slip
  to v0.11.2 (or roll into Round 2 if substantive).

## RE-ACTIVATED 2026-05-20 — patch release with rich-prompt mini-wave

@@Alex re-activated the patch release with a tighter
scope: Round-1 commits already in HEAD + the rich-prompt
mini-wave (`-a-28..-35`, `-b-13` server + SPA, `-b-14`,
`-s-10` + dead_code follow-up). The signed-DMG north
star with real keys stays parked behind it (Round-2
work). Tag fires as **v0.11.1** per the original Round-1
versioning intent.

### Commit set (canonical, in landing order)

The Round-1 closeout commits already in HEAD (per the
original list above, frozen at the 2026-05-20 Round-1
close) are the baseline. The mini-wave layers on top:

#### Mini-wave additions (in landing order, all unpushed)

| Subject                                                                                                     | Commit          | Owner          |
|-------------------------------------------------------------------------------------------------------------|-----------------|----------------|
| Rich prompt: ResizeObserver-driven margin reactor for collapse + drag-resize parity (fullstack-a-29)        | `3d708a2`       | @@FullStackA   |
| Rich prompt: per-prompt page-width slider + cross-tile decoupling (fullstack-a-30)                          | `20ece30`       | @@FullStackA   |
| BubbleOverlay: explicit dismiss + dismissedIds persistence + Loading flicker fix (fullstack-a-28)           | `1a83050`       | @@FullStackA   |
| Terminal broadcast selector: drop umbrella toggle + include self + label (fullstack-a-31)                   | `18811e0`       | @@FullStackA   |
| event_watcher: silently skip non-matching filenames; document naming convention (systacean-10)              | `6bae20b`       | @@Systacean    |
| chan/src/main.rs: gate not_a_chan_drive_hint on embeddings feature (systacean-8 follow-up)                  | `c1e9c41`       | @@Systacean    |
| chan-server: per-session shell/agent submit-mode toggle + dispatch_agent_event chord branch (fullstack-b-13 server-side) | `e24b931` | @@FullStackB |
| chan-desktop: window title = drive path verbatim (fullstack-b-14)                                           | `8dbaaed`       | @@FullStackB   |
| Rich prompt: shell/agent submit-mode toolbar toggle + SerTab roundtrip + agent-chord submit path (fullstack-b-13 SPA-side) | `dce2373` | @@FullStackB |
| Graph from here default + ancestor breadcrumb navigation (fullstack-a-33)                                   | TBD             | @@FullStackA   |
| Chord migration + context-aware spawn + surface unification (fullstack-a-32)                                | TBD             | @@FullStackA   |
| Wysiwyg: paste markdown unescaped via turndown identity escape (fullstack-a-34)                             | TBD             | @@FullStackA   |
| File editor: inline rename band above page-width cap (fullstack-a-35)                                       | TBD             | @@FullStackA   |

The TBD rows are sitting in @@FullStackA's working tree
ready to commit per the batch clearance at the tail of
[`../alex/event-architect-fullstack-a.md`](../alex/event-architect-fullstack-a.md).
Recommended order in their commit pass: `-33` → `-32` →
`-34` → `-35` (hard-pair sequencing for the chord
handler's dependence on the default-mode graph render).

### Gating verifications before tag

* @@FullStackA commits `-32 / -33 / -34 / -35` (the
  four TBD rows). Pre-commit `git diff --staged --stat`
  + post-commit `git show --stat HEAD` per
  multi-agent-tree discipline.
* @@WebtestA + @@WebtestB walkthroughs on the rebuilt
  binary:
  * @@WebtestA — bubble overlay regressions (-28),
    collapse dead space (-29), page-width tile
    decoupling (-30), broadcast selector self+checkbox
    shape (-31), chord migration + context-aware spawn
    (-32), graph from-here default + breadcrumb (-33),
    Wysiwyg paste-unescaped (-34), file rename band
    (-35).
  * @@WebtestB — submit-mode toggle against a live
    Claude Code session (-13 end-to-end), chan-desktop
    title (-14), event_watcher silent-skip on a
    stray non-event file (-10).
* @@CI standby (no signing work this wave; tag is
  unsigned).
* Permission events that stayed parked from prior
  recycle (`-b-7` runtime click, `-b-1` empirical LRU
  walk) — @@Alex's call whether to clear before tag
  or roll to v0.11.2.

### Push order

1. @@FullStackA commits the four TBD rows (per the
   recommended order above) in their session — each as
   single-purpose, push held.
2. @@Architect publishes the final commit list in this
   plan once all 13 mini-wave commits are in HEAD.
3. @@Systacean re-activates `systacean-3` with the
   version-bump + tag draft below + executes the push.

### Tag draft (v0.11.1)

Subject line (under 50 chars):

```
chan v0.11.1 — rich-prompt mini-wave + bug sweep
```

Body (under 72 chars/line):

```
First Round-1 patch release after the bug-sweep + detour.

Rich-prompt mini-wave:
* BubbleOverlay regression cluster (filter + dismiss + flicker).
* Collapse/expand dead-space recompute.
* Per-prompt page-width slider + cross-tile decoupling.
* Terminal broadcast selector polish (self in list +
  checkboxes + label).
* Shell/agent submit-mode toolbar toggle + chord encoding
  for Claude Code (`\x1b[27;9;13~`).
* Chord migration: Cmd+O / Cmd+P / Cmd+Shift+M with
  context-aware spawn semantics + surface unification.
* Graph "from here" as default + ancestor breadcrumb
  navigation back to drive root.
* Wysiwyg paste-as-markdown (no escape).
* Inline file rename band above the page-width cap.

Plus:
* chan-desktop window title = drive path.
* event_watcher silent-skip on non-matching filenames.
* CLI dead_code gate (no-default-features build clean).
* Round-1 closeout (27 bug fixes + detour tasks).

Known known: codex submit-chord diverges (`\r`); single-
chord ship with Claude Code's encoding per the
acceptance directive. Per-agent encoding map locked as
Round-3 Track 5.

Push held until @@Alex says "cut it".
```

### After v0.11.1 lands

* @@Systacean's `systacean-3` task closes with the push
  confirmation; commit-readiness append records the
  tag SHA.
* @@WebtestA / @@WebtestB run post-release smoke tests
  against the cut binary (full chrome walk on the
  rebuilt SPA + new chord set + agent submit-mode).
* Round-2 broader fan-out resumes per
  [`./round-2-plan.md`](./round-2-plan.md) — sequencing
  decisions still open at the head of that plan.
* Round-3 Track 5 (per-agent submit-chord encoding map)
  locked + listed for the eventual Round-3 fan-out.
* chan-desktop drive-onboarding redesign sits as
  separate-team work in
  [`./chan-desktop-onboarding-redesign.md`](./chan-desktop-onboarding-redesign.md).

### What this re-activated plan is NOT

* A re-grouping of the existing 13 mini-wave commits.
  They're at the right granularity — single-purpose
  per commit. No squashing.
* A signed-DMG release. v0.11.1 ships unsigned; the
  signed-DMG north star with real keys is Round 2.
* A push trigger. Push happens only when @@Alex says
  "cut it" after the gating verifications land.