# Phase 9 - desktop-native vision, drive isolation, Rich Prompt revamp

Status: closed (shipped v0.14.0; the deeper desktop-native charter is
only partially landed and carried forward)
Span: 2026-05-23 to 2026-05-24 (estimate; see Duration)

Tags: #features #bugfixes #reliability #release #desktop #mcp

## Initial asks

Three source documents from @@Alex (agents told not to edit them) plus
the architect's phase request.

- `raw/request.md`: frames the phase as inheriting the
  phase-8 carry-overs plus the desktop-native vision. Headline
  carry-overs: the open-source flip, a multi-model search picker, chan
  metadata import/export, three-mode drive connection (local fork /
  attached outbound / attached inbound), and a default "Chan" drive
  lifecycle.
- `raw/roadmap-round1.md`: "Complete outstanding
  work from previous phases", a bug list (terminal fonts after tab
  switch, the Codex MCP break in v0.13.0, unexpected edits to the open
  file, `[[` search mismatch, "too many open files", `---` rendering as a
  rule, em-dash rendering), and the backend enhancements: isolate each
  drive's data under `~/.chan/drives/{name}/`, make `~/.chan` the
  metadata root on macOS and Linux, and embed chan-server in chan-desktop
  instead of forking the binary.
- `raw/rich-prompt-revamp.md`: "hitting Cmd+P
  / Cmd+. P will always bring up a new Terminal with the Rich Prompt
  wired in", a Codex-like composer, a `spool/` directory beside
  `draft.md` (with `process.md`, `events/`, `journals/`, `tasks/`), an
  fsnotify watcher on `events/`, a teardown sequence, and "Spawn agents"
  (the rebrand of New Team, min 1 max 9).

## Team, profiles, and coordination

Cards under `../../agents/`, mapped via
[../../agents/README.md](../../agents/README.md).

```
handle           role this phase                        card
---------------  ------------------------------------   ---------------
@@Architect      request, design notes; split into      architect.md
                 a two-Architect experiment
@@CoreArchitect  chan-drive/server/MCP/terminal,        (role split,
                 event watcher, Drafts safety, FD work   no card)
@@WebArchitect   Svelte UI, Rich Prompt UX, Spawn       (role split,
                 Agents, page breaks                      no card)
@@Systacean      MCP transport fix, FD control, CAS,    systacean.md
                 path-key metadata fallout
@@WebtestA       terminal/editor/search/list triage     webtest-a.md
@@WebtestB       hamburger/focus/FB/Graph/Draft smoke    webtest-b.md
@@WebtestLive    live iab walks, found two live bugs    (role, no card)
@@Desktect       cross-platform desktop-native lead     desktect.md
@@Desktest       fresh-Mac Gatekeeper DMG walk (def.)   desktest.md
@@Desktacean     desktop systems lane                   desktacean.md
```

Coordination scheme: per-author subdirectories under the phase, with
wave-based reporting (each test lane filed a "Wave 1 report" addressed to
the architect in a fixed shape: scope, repro-status matrix, evidence,
suspected owner, recommended commit boundary, known gaps). Dispatch was
architect-orchestrated through a copy-paste handover prompt naming the
first dispatch per lane. The distinctive feature this phase was a
two-Architect Core/Web split: @@Architect ran as @@CoreArchitect plus
@@WebArchitect with an explicit boundary rule and a closing survey, as a
trial before scaling to three leads. The desktop handles (@@Desktect,
@@Desktest, @@Desktacean) have their own cards but did not land a phase-9
journal, because the desktop runtime walk was deferred to the very end.
The Rich Prompt revamp itself proposes turning this coordination scheme
into a product feature: a per-prompt `spool/` with `events/`,
`journals/`, and `tasks/` mirroring the manual multi-agent process.

## Duration

Estimate: 2026-05-23 to 2026-05-24, about two calendar days, with the
active work concentrated on 2026-05-24. Basis: git author dates plus
in-file dated headers. The 2026-05-23 commit is the phase-8 close that
also seeded the phase-9 carry-overs.

## Highlights and lowlights

Highlights:
- v0.14.0 shipped with green CI and recorded desktop and CLI release
  runs.
- The Codex MCP bug was root-caused precisely: the transport sent
  newline-delimited JSON while Codex clients send Content-Length framed
  JSON-RPC; the fix accepts both framings, with a stale-socket proxy
  fallback.
- A pane-mode terminal PTY identity collision (three staged panes all
  titled the same name reattaching to one PTY) was found live and fixed
  by allocating names from the draft layout.
- The Rich Prompt was re-architected from terminal-overlay state into a
  Core-owned, terminal-owned draft workspace with a `spool/` tree,
  watcher status, and teardown on terminal close; Core and Web converged
  on the same four-route shape.
- FD pressure was addressed at both admission and the index layer; an
  isolated repo rebuild indexed 714 files with zero errors under a
  256-fd soft limit.
- Metadata archive export/import landed end-to-end, and lock-poisoning
  was hardened across many routes to map to errors instead of panics.

Lowlights:
- Two of three test lanes had no iab browser backend, so they could only
  ship static, unit, and curl evidence; visual claims were honestly
  marked unverified.
- Even the live iab runs could not type non-empty content into
  CodeMirror, so non-empty Rich Prompt submit and the full Spawn-agents
  preflight were only partially validated.
- A File Browser smoke failed live (tab activated but the body stayed on
  the welcome placeholder with a duplicate-key console error); treated as
  a follow-up.
- The desktop-native charter's deeper work (multi-drive server routing,
  three-mode connection, the default "Chan" drive lifecycle, the DMG
  Gatekeeper walk) did not land this round.

## Constructive feedback

For the team:
- A test lane that cannot get an iab browser should escalate the tooling
  block immediately rather than producing a full static report that
  cannot honestly mark visual checks; the duplicated static effort across
  two blocked lanes was wasteful.
- @@Systacean's wave report (repro-status, suspected owner, recommended
  commit boundary) is the model shape to keep.

For the architect:
- The Core/Web split with an explicit boundary rule worked: the two lanes
  converged on the same four-route contract without rework. The closing
  survey is the right instrument before scaling to three leads.
- Leaving the `[[` search semantics as a product decision for @@Alex
  (rather than guessing) was the right escalation; it remains open.

For @@Alex:
- Several roadmap bugs were filed against the installed v0.13.0 binary but
  were already fixed at HEAD; re-running repros against a fresh build
  before filing would cut false-positive triage loops.
- Literal Cmd+P validation is the one item only a native run can close;
  @@Alex accepted owning that post-release.

## What shipped, tried, and undone

Shipped (in v0.14.0): MCP transport compatibility (framed plus newline
JSON); the pane-mode terminal title/PTY collision fix; Rich Prompt
workspaces (four Core routes, a `draft.md` plus `spool/` tree,
session-aware watcher status, exact-buffer submit archival, terminal-close
teardown); the Rich Prompt web UI (Cmd+P always spawns a fresh prompt
terminal, a new header and plus-menu, the agent picker); Spawn agents
(min 1, max 9, JSON config); the Drafts lifecycle (hidden from the File
Browser while editor/graph/terminal/MCP keep access, no-clobber promote,
discard to trash, boot scan with broken-draft warnings); the editor
keeping bare `---` as source text plus page breaks and PDF export;
metadata archive export/import (CLI plus UI, a manifest-first archive
with an SCM guard); path-keyed drive metadata under `~/.chan/drives/`
with `~/.chan` as the canonical root on macOS and Linux; and FD admission
plus index-layer budgets.

Tried, partial, or deferred (not undone): path-keyed metadata batches 3-4
(multi-drive server routing and UI labels); the `[[` search semantics
(awaiting @@Alex's product call); a deterministic "too many open files"
repro; the desktop-native charter items.

Undone or removed by design: the auto-hide style toolbar (now an explicit
show/hide); the prompt-local Close and New File buttons and the manual
watch/stop actions (folded into terminal-close teardown and the internal
watcher); and the history-only rich-prompt model (superseded by the
active workspace model). No hard reverts were recorded in the journals.

## Raw material

Raw working material (per-author journals, task/request/roadmap files,
coordination logs) is preserved in git history under this phase's `raw/`
tree; it was removed from the working tree in the phase-15 docs cleanup.

The two roadmap source files originally embedded twelve screenshots of
the reported bugs and the target Rich Prompt UI; per the journals-wide
image removal each was already a short text note before this cleanup, in
`raw/roadmap-round1.md` and `raw/rich-prompt-revamp.md`.
