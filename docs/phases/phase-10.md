# Phase 10 - desktop embedded-server merge and public site

Status: closed with named carryovers (open items migrated to phase 11)
Span: 2026-05-24 to 2026-05-26 (estimate; git commit dates and dated
      journal headers agree)
Versions: none cut this phase (release prep deferred to phase 11)
Tags: #desktop #release #docs #performance #bugfixes #terminal

## Roadmap (the asks)

Three parallel tracks, each with its own roadmap file, plus one
mid-phase focused ask.

Track A ("Desktop Merge and Carryover Closure"): embed `chan-server`
in `chan-desktop` for normal local drives, preserve `chan serve` as a
standalone CLI/server path, close the desktop-native gaps that did not
materialize in phase 9, and sweep the highest-risk phase 8/9 validation,
docs, config, and release-hygiene gaps into named tasks.

Track B ("Public Site, Manual Pages, Install Split"): make
`web-marketing` the public site source of truth, publish `docs/manual/`
as public documentation, and clean up the install surface now that
desktop and CLI have different release shapes.

Track C ("Hybrid Pane and Editor Polish"): the Hybrid pane, terminal
rendering, and editor-close polish to land after the phase 9 validation
waves. Included streaming relationship UI, shared inspector
upload/download, Draft-save-to-drive, drag-and-drop upload plus native
macOS Finder drag-out, the graph filesystem spine, a screen-lock port,
menu placement, and Hybrid surface themes.

Mid-phase focused ask: remove the per-frame WebGL texture-atlas clear
in the terminal that force-repainted every pane roughly 60 times a
second, causing visible jank in rich/animated TUI output.

## Rounds and waves

Single round. The phase ran three parallel tracks dispatched
concurrently. Track A (desktop merge) was the highest-risk and had an
explicit review loop; one review item went CHANGES REQUESTED before
re-review and APPROVED. Track B (public site) ran mostly independently.
Track C (UI polish) was the largest surface and relied on live-browser
regression coverage. The mid-phase terminal WebGL ask was a focused
one-agent task slotted alongside Track C.

The phase-10 to phase-11 boundary used an explicit migration pointer
rather than carrying items forward implicitly. No version was tagged;
release verification was deferred to phase 11 once the repo went public.

## Team and coordination

Agent roster is at ../agents/README.md. Handles active this phase:

```
handle       role
-----------  -------------------------------------------------------
@@Architect  plan, dispatch, review loop, decisions
@@Alex       owner; manual macOS/desktop verification, scope calls,
             DNS-cutover runbook
@@Desktect   chan-desktop in-process-registry merge (Track A)
@@IconDocs   ad-hoc handle: Tauri icon regeneration + desktop
             docs/config audit (no permanent agent card)
@@Frontend   terminal WebGL atlas-smoothness change (became
             FullStackA/B in later phases)
```

Track B and Track C work is attributed to track rather than a named
handle in the source files.

Coordination scheme: flat per-topic files at the phase root (three
roadmap files plus focused notes and handoff files), not one directory
per author. The architect dispatched each track by pasting a bootstrap
prompt into a new agent session. Each agent implemented, self-verified,
and sent a REVIEW task back to the architect. Cross-track handoffs were
written as separate notes so one track never edited another's live
roadmap.

The phase ran in a shared single worktree. Commit hygiene was
load-bearing: path-scoped staging, a chained add + staged-diff audit +
commit, and a post-commit show check, because multiple agents shared one
tree. Failing to scope staging would have let one agent's staged files
contaminate another's commit.

## What shipped, tried, and undone

Shipped:

- chan-desktop in-process registry: all registry and feature-toggle
  operations moved in-process through one embedded library call.
  Friendly lock-conflict and already-open-window error mapping added.
  A bounded unregister retry replaced the open-ended polling. The
  `chan` binary was removed from the Tauri bundle, Makefile, config,
  and docs. The "drive not registered" bug on opening a new folder is
  fixed and pinned by a test.
- Public site and manual: static generator for `web-marketing`, a
  Pages workflow, `docs/manual/` published. The Windows installer
  surface was removed; upgrade and install paths were rewired to GitHub
  Releases. A stale-copy guard prevents the seeded desktop drive from
  shipping outdated pages.
- Website cross-links: manual links rewritten to drive-relative
  siblings for the seeded desktop drive, then rewritten back to clean
  URLs at build time so the published HTML is byte-identical.
- DNS-cutover runbook authored and committed.
- MCP global config registration removed; `CHAN_MCP_*` terminal env
  variables became the only discovery contract.
- Terminal WebGL atlas-clear removal: the per-frame atlas clear was
  removed with a negative test pin so it cannot silently return.
- Track C UI bundle: Hybrid pane, streaming relationship UI, shared
  inspector upload/download, explicit Draft-save-to-drive,
  drag-and-drop upload, native macOS Finder drag-out (required two
  launch-blocker fixes plus a full native AppKit bridge and an ACL
  fix), graph filesystem spine, screen-lock port, menu placement,
  Hybrid surface themes.
- Tauri app-icon regeneration with squircle and correct centering.
- Async sync-I/O audit with several blocking operations moved off the
  runtime; a low-FD stress smoke held.

Tried then corrected or abandoned:

- A ghostty-web terminal renderer experiment was reverted back to
  xterm.js.
- The local desktop child-process serving model (a per-drive `chan
  serve` sidecar) was fully removed; there is no sidecar fallback.
- An MCP media tool and a CLI subcommand were renamed with no
  compatibility path (intentional, pre-release).
- Theme-change atlas-clear insurance was deliberately not applied and
  left to manual verification.
- A transfer route that returned HTTP 500 for non-UTF-8 bytes into
  editable text was corrected to 415.

Deferred to phase 11: Linux desktop launch and the CLI-to-desktop
handoff, release verification once the repo went public, the
manual/site streaming-copy update, and three Rich Prompt watcher-audit
follow-ups.

## Retrospective

Highlights:

- The desktop app became fully self-contained in one phase. Removing
  the `chan` binary from the bundle closed a whole class of version-
  skew bugs and simplified distribution.
- The public site and manual shipping together with the install-surface
  cleanup made the release posture coherent for the first time.
- Track C achieved broad live-browser regression coverage across a
  large surface area, including the AppKit drag-out bridge which
  required diagnosing three separate blocking layers.
- The review loop caught a real deficiency (a named audit target left
  untouched) before merge.

Lowlights / contention:

- macOS native drag-out needed two launch-blocker fixes before it
  could even be tested, then a full native AppKit bridge plus an ACL
  fix because the webview would not produce a Finder file from the
  browser payload alone. The scope was discovered incrementally rather
  than up front.
- Linux desktop remained a launch blocker the whole phase (white
  window, no File menu, blank duplicate window). Linux native drag-out
  was never smoked. Both items carried to phase 11, which means they
  were known-broken across a release boundary.
- The item-4 review found `desktop/design.md` left untouched and
  contradicting a sibling doc. Named audit targets must all be
  addressed in the same pass; reconcile sibling docs together.
- Cross-pane terminal smoothness after the WebGL atlas-clear removal
  was deferred to Alex's manual verification and was not verified at
  gate time. The automated gate does not substitute for OS/GPU smoke
  on desktop changes.

Constructive feedback / lessons for future agents:

- Write review-loop findings into the task file and journal summary
  directly. Do not send findings as a chat message for Alex to relay;
  Alex is not a courier.
- Once an agent starts a task, a new ask is a new task, not an
  amendment to the in-flight one. Append-only beats rewriting under
  someone.
- Shared-worktree commit hygiene is a hard requirement when multiple
  agents share one tree. Collapse staging, audit, and commit into one
  chained invocation. Never stage another agent's dirty files.
- The automated gate (cargo + clippy + tests) is necessary but not
  sufficient for desktop and GUI changes. Record explicitly that
  OS/desktop smoke is deferred and who owns it, rather than treating
  a green gate as verification of record.
- Address all named audit targets in a pass. If two docs cover the
  same topic, reconcile them in the same commit.

## Notes

Terminology drift active during this phase:

- "chan-drive" and "drive" appear in older roadmap and journal files;
  the current name is "chan-workspace" (the crate) and "workspace" (the
  concept). See also: "folder" was replaced by "directory" in prose.
- "Rich Prompt" is the old name for what became "Team Work".
- The "sidecar" or "child-process serving" model refers to the
  abandoned per-drive `chan serve` subprocess approach, replaced by
  the in-process embedded server.

Raw working material (per-author journals, task files, roadmap files,
coordination logs) is preserved in git history under
`docs/journals/phase-10/`; that tree was removed from the working tree
in the phase-15 docs cleanup.
