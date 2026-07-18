# Phase 9 - desktop-native vision, drive isolation, Rich Prompt revamp

Status: closed (v0.14.0 shipped; the deeper desktop-native charter is only partially landed and carried forward)
Span: 2026-05-23 to 2026-05-24 (estimate; basis: git author dates plus in-file dated headers)
Versions: v0.14.0
Tags: #features #bugfixes #reliability #release #desktop #mcp

## Roadmap (the asks)

Three source documents from Alex plus the architect's phase request drove this phase. All were marked read-only to agents.

**Carry-overs from phase 8 (request.md):** the open-source flip, a multi-model search picker, chan metadata import/export, a three-mode drive connection model (local fork / attached outbound / attached inbound), and a default "Chan" drive lifecycle.

**Bug and backend work (roadmap-round1.md):** terminal fonts after tab switch, the Codex MCP break in v0.13.0, unexpected edits to the open file, `[[` search mismatch, "too many open files", `---` rendering as a horizontal rule, and em-dash rendering. Backend asks: isolate each drive's data under `~/.chan/drives/{name}/`, make `~/.chan` the metadata root on macOS and Linux, and embed chan-server in chan-desktop instead of forking the binary.

**Rich Prompt revamp (rich-prompt-revamp.md):** Cmd+P / Cmd+.P always opens a new Terminal with the Rich Prompt wired in, a Codex-like composer, a `spool/` directory beside `draft.md` (with `process.md`, `events/`, `journals/`, `tasks/`), an fsnotify watcher on `events/`, a teardown sequence on terminal close, and "Spawn Agents" (rebrand of New Team, min 1 max 9 agents).

## Rounds and waves

Phase 9 ran as a single round with two waves, using a two-Architect Core/Web split as an experiment before scaling to three leads.

**Wave 1** addressed bugs and backend work in parallel: @@architect owned the Rust side (MCP transport fix, FD pressure, drive metadata path migration, Drafts lifecycle, lock-poisoning hardening, metadata archive export/import) while @@architect owned the Svelte side (Rich Prompt four-route contract, Spawn Agents UI, page breaks, toolbar changes). Test lanes (WebtestA, WebtestB) filed wave reports in a fixed shape: scope, repro-status matrix, evidence, suspected owner, recommended commit boundary, and known gaps.

**Wave 2** was the desktop runtime walk, handled by the desktop-specialist handles. It did not fully land; most desktop-native charter items were deferred to phase 10 and later.

v0.14.0 was cut at the end of the round with green CI, a recorded desktop run, and a recorded CLI release run.

## Team and coordination

Agent roster is in ../agents/README.md.

```
handle           role this phase
---------------  ---------------------------------------------------
@@architect      request, design notes; split into two for the round
@@architect  chan-drive/server/MCP/terminal, event watcher,
                 Drafts safety, FD work (no separate card; role split)
@@architect   Svelte UI, Rich Prompt UX, Spawn Agents, page breaks
                 (no separate card; role split)
@@syseng      MCP transport fix, FD control, CAS, path-key metadata
WebtestA       terminal/editor/search/list triage
WebtestB       hamburger/focus/FB/Graph/Draft smoke
WebtestLive    live iab walks, found two live bugs (no card)
@@architect       cross-platform desktop-native lead
Desktest       fresh-Mac Gatekeeper DMG walk (deferred)
@@rustacean     desktop systems lane
```

Coordination scheme: per-author subdirectories under the phase directory, with wave-based reporting from each test lane to the architect. Dispatch was architect-orchestrated through copy-paste handover prompts naming the first dispatch per lane.

The distinctive feature this phase was the Core/Web Architect split: @@architect ran as two concurrent roles (@@architect and @@architect) with an explicit boundary rule (Core owns Rust routes; Web owns Svelte and the contract surface) and a closing survey before merging conclusions. This was a controlled trial ahead of scaling to three leads. The two lanes converged on the same four-route contract without rework, validating the boundary rule.

The desktop handles (@@architect, Desktest, @@rustacean) have cards but did not land a phase-9 journal because the desktop runtime walk was deferred to the end of the round and then carried forward.

The Rich Prompt revamp itself proposed turning this coordination scheme into a product feature: a per-prompt `spool/` with `events/`, `journals/`, and `tasks/` mirroring the manual multi-agent process.

## What shipped, tried, and undone

**Shipped in v0.14.0:**
- MCP transport compatibility: fixed the Codex break by accepting both Content-Length framed JSON-RPC and newline-delimited JSON, with a stale-socket proxy fallback.
- Pane-mode terminal title/PTY collision fix: three staged panes all titled the same name were reattaching to one PTY; names are now allocated from the draft layout.
- Rich Prompt workspaces: four Core routes, `draft.md` plus `spool/` tree, session-aware watcher status, exact-buffer submit archival, and terminal-close teardown.
- Rich Prompt web UI: Cmd+P always spawns a fresh prompt terminal, new header and plus-menu, and an agent picker.
- Spawn Agents: min 1, max 9, JSON config.
- Drafts lifecycle: hidden from the File Browser while editor/graph/ terminal/MCP retain access; no-clobber promote; discard to trash; boot scan with broken-draft warnings.
- Editor keeps bare `---` as source text; page breaks and PDF export.
- Metadata archive export/import: CLI plus UI, manifest-first archive with an SCM guard.
- Path-keyed drive metadata under `~/.chan/drives/` with `~/.chan` as the canonical root on macOS and Linux.
- FD admission and index-layer budgets: an isolated repo rebuild indexed 714 files with zero errors under a 256-fd soft limit.
- Lock-poisoning hardened across many routes to map to errors instead of panics.

**Tried but partial or deferred (not undone):**
- Path-keyed metadata batches 3 and 4 (multi-drive server routing and UI labels) carried forward.
- `[[` search semantics: root-caused but awaiting Alex's product call (open into phase 10).
- Deterministic "too many open files" repro: addressed at the admission layer but a clean isolated repro was not achieved.
- Desktop-native charter items: multi-drive server routing, three-mode connection, default "Chan" drive lifecycle, the DMG Gatekeeper walk.

**Removed by design (no hard reverts recorded):**
- Auto-hide style toolbar replaced by explicit show/hide.
- Prompt-local Close, New File buttons, and manual watch/stop actions folded into terminal-close teardown and the internal watcher.
- History-only rich-prompt model superseded by the active workspace model.

## Retrospective

**Highlights:**
- The Codex MCP root-cause was precise: the transport framing mismatch (newline-delimited vs. Content-Length) was identified and fixed with a fallback path, unblocking external agent integration.
- The PTY identity collision was caught live by WebtestLive and fixed in the same round, preventing a silent regression from shipping.
- The Core/Web Architect split worked. The explicit boundary rule and the closing survey let two concurrent lanes converge on the same four-route contract without a rework cycle. This is the coordination pattern to carry forward when splitting front/back work.
- FD pressure was addressed structurally at two layers (admission and index), not patched at the symptom site.

**Lowlights / contention:**
- Two of three test lanes had no iab browser backend and could only produce static, unit-test, and curl evidence. Visual claims were honestly marked unverified, but the duplicated static effort across two blocked lanes was wasteful. A blocked lane should escalate the tooling gap immediately rather than producing a full report that cannot close visual checks.
- Even the one live iab lane could not type non-empty content into CodeMirror, so non-empty Rich Prompt submit and the full Spawn-agents preflight were only partially validated.
- A File Browser smoke failed live (tab activated but the body stayed on the welcome placeholder with a duplicate-key console error); treated as a follow-up rather than a blocker.
- The desktop-native charter was the headline ask and the item that did not land. The three-mode connection model and the default "Chan" drive lifecycle carried forward to phases 10 onward.

**Constructive feedback:**

For agents: a test lane without a functioning browser should escalate the tooling block at the start of wave 1, not at the end after producing a full static report. Static evidence can confirm Rust correctness but cannot close UI claims; filing a partial report honestly labeled as such is more useful than a complete report that silently omits live coverage. @@syseng's wave report shape (repro-status matrix, suspected owner, recommended commit boundary, known gaps) is the model to use.

For the architect: leaving the `[[` search semantics as an explicit product question for Alex rather than guessing was the right call. The Core/Web split with a closing survey is the right instrument before scaling the split further.

For Alex: several roadmap bugs were filed against the installed v0.13.0 binary but were already fixed at HEAD. Re-running repros against a fresh build before filing would cut false-positive triage loops. Literal Cmd+P validation on a native run is the one item that cannot be closed by CI or agents; Alex accepted owning that check post-release.

## Notes

**Terminology drift:** "Rich Prompt" later rebranded to "Team Work". "chan-drive" renamed to "chan-workspace" (the crate the workspace boundary logic lives in). "Drive" as a user-facing concept for a workspace folder persists in some phase-9 docs but "workspace" is the current term.

The raw working material (per-author journals, task/request/roadmap files, coordination logs, and the twelve original bug/UI screenshots which were replaced with text notes before the cleanup) is preserved in git history under docs/journals/phase-9/; that tree was removed from the working tree in the phase-15 docs cleanup.
