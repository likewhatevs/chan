# Phase 10 - desktop embedded-server merge and public site

Status: closed with named carryovers (open items migrated to phase 11)
Span: 2026-05-24 to 2026-05-26 (estimate; see Duration)

## Initial asks

There is no single request file; three roadmap files are the source asks.

- Track A ([raw/roadmap-track-a.md](raw/roadmap-track-a.md), "Desktop
  Merge and Carryover Closure"): "embed `chan-server` in `chan-desktop`
  for normal local drives", "preserve `chan serve` as a standalone
  CLI/server path", close the desktop-native gaps that did not
  materialize in phase 9, and sweep the highest-risk phase 8/9 validation,
  docs, config, and release-hygiene gaps into named tasks.
- Track B ([raw/roadmap-track-b.md](raw/roadmap-track-b.md), "Public
  Site, Manual Pages, Install Split"): make `web-marketing` the public
  site source of truth, publish `docs/manual/` as public documentation,
  and clean up the install surface now that desktop and CLI have different
  release shapes.
- Track C ([raw/roadmap-track-c.md](raw/roadmap-track-c.md), "Hybrid Pane
  and Editor Polish"): the Hybrid pane, terminal rendering, and
  editor-close polish to land after the phase 9 validation waves.

A mid-phase single-deliverable ask is recorded in
[raw/terminal-webgl-atlas-smoothness.md](raw/terminal-webgl-atlas-smoothness.md):
render rich/animated TUI output smoothly by removing the per-frame WebGL
texture-atlas clear that force-repainted every pane about 60 times a
second.

## Team, profiles, and coordination

Cards under `../../agents/`, mapped via
[../../agents/README.md](../../agents/README.md).

```
handle       role this phase                           card
-----------  ---------------------------------------   ------------------
@@Architect  plan, dispatch, review loop, decisions    architect.md
@@Alex       owner; manual macOS/desktop verification, (human owner)
             scope calls, the DNS-cutover runbook
@@Desktect   chan-desktop in-process-registry merge    desktect.md
@@IconDocs   one-off Track A handle: Tauri icon        (ad-hoc, no card)
             regeneration + desktop docs/config audit
@@Frontend   terminal WebGL atlas-smoothness change    frontend.md
                                                        (-> FullStack A/B)
```

Track B and Track C authors are referred to by track rather than by a
named handle in most files.

Coordination scheme: flat per-topic files at the phase root (three
roadmaps plus focused notes and handoff files), not one directory per
author. Dispatch was architect-orchestrated with an explicit review loop:
the architect pastes a bootstrap prompt into a new agent, the agent
implements and self-verifies, sends a REVIEW task back, the architect
reviews (one item went CHANGES REQUESTED, then re-review, then APPROVED),
and the agent commits atomically and waits for an ack. Cross-track
handoffs were written as separate notes so one track never edited
another's live roadmap. The phase ran in a shared single worktree with
strict commit hygiene (path-scoped staging, a chained add plus staged-diff
audit plus commit, a post-commit check) because multiple agents shared one
tree. The phase-10 to phase-11 boundary used an explicit migration pointer
rather than carrying items forward implicitly.

## Duration

Estimate: 2026-05-24 to 2026-05-26, roughly three days. Basis: git commit
dates and the dated headers inside the journals agree.

## Highlights and lowlights

Highlights:
- The desktop app became fully self-contained: all registry mutations and
  feature toggles run in-process through one embedded library, the `chan`
  binary is no longer probed or shipped in the bundle, and the
  "drive not registered" bug on opening a new folder is fixed and pinned
  by a test.
- The public site and manual shipped: a `web-marketing` static generator
  and Pages workflow, `docs/manual/` published, the Windows installer
  surface removed, and upgrade/install rewired to GitHub Releases.
- Track C closed with broad live-browser regression coverage (streaming
  relationship UI, shared inspector upload/download, explicit
  Draft-save-to-drive, drag-and-drop upload plus native macOS Finder
  drag-out, the graph filesystem spine, a screen-lock port, menu
  placement, Hybrid surface themes).
- Async hardening moved several blocking operations off the runtime;
  a low-FD stress smoke held.
- The terminal WebGL atlas-clear workaround was removed, fixing the
  per-frame force-repaint.

Lowlights:
- macOS native drag-out needed two launch-blocker fixes before it could
  even be tested, then a full native AppKit bridge plus an ACL fix because
  the webview would not produce a Finder file from the browser payload.
- Linux desktop remained a launch blocker the whole phase (white window,
  no File menu, blank duplicate window); Linux native drag-out was never
  smoked. Both carried to phase 11.
- A transfer route returned HTTP 500 instead of an actionable 4xx for
  non-UTF-8 bytes into editable text; fixed to 415 later.
- The item-4 review found a named audit target (`desktop/design.md`) left
  untouched and contradicting a sibling doc.
- Cross-pane terminal smoothness after the WebGL removal was left to
  Alex's manual verification, not verified at gate time.

## Constructive feedback

- Address all named audit targets in a pass, not just the easy ones, and
  reconcile sibling docs together.
- A standing note from the item-4 review, addressed to all agents: write
  review-loop information into the task and the journal summary directly,
  not as a chat message for Alex to relay. Alex is not a courier.
- Once an agent starts a task, a new ask becomes a new task, not an
  amendment.
- Shared-worktree hygiene was load-bearing: collapse staging, audit, and
  commit into one chained invocation, and never stage another agent's
  dirty files.
- The desktop and terminal changes leaned on the automated gate as
  verification of record and deferred GUI/OS smoke to Alex; recorded as
  acceptable pre-release, but it left desktop and Linux behaviors
  empirically unverified.

## What shipped, tried, and undone

Shipped:
- The chan-desktop in-process registry: all registry and feature
  operations in-process, friendly lock/already-open mapping, a bounded
  unregister retry, and removal of the `chan` binary from the bundle,
  Makefile, config, and docs.
- The public site and manual: static generator, Pages workflow,
  `docs/manual/`, Windows removal, GitHub-Releases upgrade/install, and a
  stale-copy guard.
- Website fixups: manual cross-links rewritten to drive-relative siblings
  for the seeded desktop drive, then rewritten back to clean URLs at
  build time so published HTML is byte-identical; a DNS-cutover runbook.
- Removal of MCP global config registration (the `CHAN_MCP_*` terminal env
  is now the only discovery contract).
- The terminal WebGL atlas-clear removal with a negative pin so it cannot
  silently return.
- The Track C UI bundle and a Tauri app-icon regeneration.
- An async sync-I/O audit and several smaller cleanups.

Tried then abandoned or undone:
- A ghostty-web terminal renderer experiment was reverted back to
  xterm.js.
- The local desktop child-process serving (a per-drive `chan serve`
  sidecar) was fully removed; there is no sidecar fallback.
- An MCP media tool and a CLI cap were renamed with no compatibility path
  (intentional, pre-release).
- Theme-change atlas-clear insurance was deliberately not applied
  (deferred to manual verification).

Decisions deferred to phase 11: Linux desktop launch and the
CLI-to-desktop handoff, release verification once the repo went public,
the manual/site streaming-copy update, and three Rich Prompt
watcher-audit follow-ups.

## Raw material

- Roadmaps: [raw/roadmap-track-a.md](raw/roadmap-track-a.md),
  [raw/roadmap-track-b.md](raw/roadmap-track-b.md),
  [raw/roadmap-track-c.md](raw/roadmap-track-c.md)
- Summary and the round-3 planning index with the phase-11 migration note:
  [raw/summary.md](raw/summary.md),
  [raw/round-3-pending.md](raw/round-3-pending.md)
- Per-topic implementation notes and the dispatch/review-loop handoffs
  live alongside them in [raw/](raw/).
