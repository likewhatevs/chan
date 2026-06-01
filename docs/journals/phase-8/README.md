# Phase 8 - bug sweep, signed-DMG pipeline, and public-flip prep

Status: closed
Span: 2026-05-19 to 2026-05-23 (estimate; see Duration)

Tags: #bugfixes #signing #release #opensource #ci #desktop #docs

## Initial asks

North star, from `raw/request.md`: "Ship a notarized
macOS `.dmg` (plus signed Windows + Linux equivalents) that users can
download and install without Gatekeeper / SmartScreen friction.
Tag-triggered CI produces the signed installer artifact, hosted via the
release pipeline."

The phase ran in three rounds (split from two so the signed-release
pipeline could be exercised end-to-end with real keys before the repo
went public, since opening a repo is one-way):

- Round 1: close every item in `raw/phase-8-bugs.md` (no binary cut at
  its end),
  plus a 2026-05-20 detour to stop embedding the BGE-small model in the
  binary (about 89 MB down to about 26 MB) and make semantic search
  opt-in.
- Round 2: phase-7 backlog items 1-7, plus the full signed and notarized
  DMG pipeline exercised with real Apple Developer ID secrets while the
  repo stayed private.
- Round 3: open-source the repo (license and community files, history
  audit, then flip public), a multi-model search picker, and a
  whole-codebase cleanup, hardening, and docs-review pass.

The bug list itself (`raw/phase-8-bugs.md`, roughly 186 KB, 95-plus
entries) is the durable per-bug ask record.

## Team, profiles, and coordination

Two parallel teams. Cards under `../../agents/`, mapped via
[../../agents/README.md](../../agents/README.md).

```
handle         role this phase                          card
-------------  --------------------------------------   ----------------
@@Architect    plan, dispatch, decisions, journal       architect.md
@@FullStackA   backend + frontend; busiest lane         fullstack-a.md
               (about 100 tasks)
@@FullStackB   same profile; chan-desktop + PTY work    fullstack-b.md
@@Systacean    CLI, build, deps, indexer, release cuts  systacean.md
@@CI           new 6th slot: Actions, signing, release  ci.md
@@WebtestA     Chrome-MCP walkthrough lane              webtest-a.md
@@WebtestB     Chrome-MCP walkthrough lane              webtest-b.md
@@Desktect     chan-desktop product architect (R3)      desktect.md
@@Desktacean   Tauri/Rust + macOS/Linux desktop (R3)    desktacean.md
@@Desktest     desktop tester (R3)                      desktest.md
```

Coordination scheme: per-author directories under the phase, each holding
numbered append-only task files plus one canonical `journal.md`, with
`alex/` as the shared event hub. Corrections are new dated appends with
back-links, never rewrites. Event channels are `alex/event-<from>-<to>.md`
(one file per directed channel), including cross-team channels once the
chan-desktop team spun up mid-Round-3, with @@Alex bridging decisional
traffic between leads. The architect-orchestrated loop is: cut a task,
poke the lane, the lane implements and runs the pre-push gate and pokes
back commit-ready, the architect clears, the lane self-commits with
per-path staging and a pre/post audit.

This hand-run dispatch shape is the deliberate automation blueprint. The
process spec pins a watcher event-file naming convention enforced across
three filter sites, and `raw/rich-prompt/events/` holds real JSON event
files from a live watcher smoke test (where @@Alex
pointed chan's own rich-prompt watcher at the journals directory). That
smoke surfaced the watcher-versus-journal shape gap recorded below.

## Duration

Estimate: 2026-05-19 to 2026-05-23, five days. Basis: git commit dates on
the tree (earliest 2026-05-19, latest 2026-05-23) corroborated by the
dated section headers, which is the more reliable record because the
journals are committed in bulk at round close.

## Highlights and lowlights

Highlights:
- The signed and notarized macOS DMG pipeline was exercised end-to-end
  with real Apple Developer ID keys behind a private repo (four dry-run
  tags, then a notarized DMG on the release); the de-risking goal of the
  round split was achieved.
- Three real releases cut during the phase (v0.11.1, v0.11.2 signed DMG,
  v0.12.0), opening on v0.11.0 and closing by cutting v0.13.0, the
  public-flip version.
- Public-flip pre-flight landed clean: Apache-2.0 license, contributing,
  conduct, and security files, issue and PR templates, and an
  outside-reader explainer of the multi-agent pattern; the history audit
  came back clean.
- Empirical-audit-at-pickup worked in both directions; lanes caught
  architect-side scope errors before editing code.

Lowlights:
- The per-PR CI gate was silently broken for about 15 commits (missing
  Linux GTK/glib dev headers on the clippy job) and took several
  gate-unblocker tasks to fully green.
- Several cross-agent commit-hygiene incidents in the shared worktree (a
  broad `git add` swept another lane's files; a commit absorbed a
  stowaway hunk). All recoverable; they reinforced per-path staging.
- The live watcher smoke could not surface journal pokes: the runtime
  watcher handles create and rename but not data-append, and parses every
  fired file as event JSON, so markdown journal appends never dispatch.
  The audit-trail-versus-wire-shape split the blueprint assumed resolved
  was, in fact, not.
- The architect twice invented descriptions from a name plus intuition
  rather than reading source; both fed the ground-descriptions-in-source
  rule.
- A webtest Gatekeeper verification overstepped scope (overwrote the
  installed app, killed a live PID, quarantined a system path), leading
  to a tightened standing-permission subset for DMG-install walks.

## Constructive feedback

For the team:
- The shared-worktree commit discipline is right but slipped; collapse
  add, audit, and commit into one chained invocation to close the
  inter-command race window.
- Webtest lanes must capture the launched PID at spawn and only signal
  that PID; never infer ownership by elapsed time, never touch the
  installed app or quarantine system paths.

For the architect:
- Ground every capability description and scope claim in the source
  before writing it; do not paraphrase a peer's functional framing as
  location information.
- Write recycle handover entries closer to the actual tear-down beat; two
  lanes self-committed past the committable markers because the handover
  was written before tear-down.

For @@Alex:
- The mid-phase restructures (two rounds to three; the cut cadence
  drifting through v0.11.1/.2, v0.12.0, v0.13.0) were well-reasoned but
  churned the plan repeatedly; the journals absorbed it cleanly.
- The secrets-boundary pattern (architect directs CI on names, @@Alex
  populates values in Actions secrets) worked; keep it.

## What shipped, tried, and undone

Shipped: a large slice of the bug list across all lanes; the BGE-small
model un-embedded with semantic search opt-in (an `embed-model` feature
flag kept for offline use); the rich-prompt session-conductor evolution
(clear-on-submit, on-disk history, a shell/agent submit toggle, a
team-spawn band); the signed/notarized DMG pipeline; the Hybrid back-side
as a per-surface settings surface plus an About section with a donation
QR; chan-report per-directory aggregation; a Drafts metadata folder; a
five-surface right-click menu revamp; the config-driven Team feature; a
screensaver with PIN unlock; a chan-server async-blocking cleanup across
many handlers; the public-flip docs; and releases v0.11.1, v0.11.2,
v0.12.0, and v0.13.0.

Tried, undone, or deferred:
- The v0.11.1 tag was cancelled at the 2026-05-20 restructure, then
  reactivated days later for a rich-prompt-fix mini-wave; the cut
  happened but the plan flip-flopped.
- The terminal-glyph one-liner hypothesis was a no-op (already default in
  xterm.js 6.x); the real fix loaded the WebGL addon, because the DOM
  renderer ignored custom glyphs entirely.
- The watcher-versus-journal convergence chose to capture the gap as
  design work and leave the smoke test as-is; no code change.
- A history rewrite of a cross-agent commit incident was declined
  (later commits already referenced the SHAs); resolved via an
  audit-trail note instead.

Deferred to phase 9: the operational repo-flip steps (docs ready), the
multi-model search picker, metadata import/export, the desktop-native
single-binary vision, and the chan-desktop runtime walk on the DMG.

## Raw material

Raw working material (per-author journals, task/request/roadmap files,
coordination logs) is preserved in git history under this phase's `raw/`
tree; it was removed from the working tree in the phase-15 docs cleanup.
The load-bearing files for anyone reading that history:

- Source ask and round structure: `raw/request.md`
- Process spec with the watcher naming convention: `raw/process.md`
- The durable bug audit trail: `raw/phase-8-bugs.md`
- The load-bearing architect journal: `raw/architect/journal.md`
- The live watcher-smoke event files: `raw/rich-prompt/events/`

The bug list originally embedded four screenshots (a graph false-missing
node, the spawn-agent overlay, and the iTerm-versus-chan terminal glyph
comparison); per the journals-wide image removal each is now a short text
note in `raw/phase-8-bugs.md`.
