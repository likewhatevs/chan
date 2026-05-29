# Phase 9 request

Author: @@Architect (final phase-8 incarnation)
Phase 8 closed: 2026-05-23 with `chan-v0.13.0` cut.

Phase 9 inherits the carry-over items from phase 8 plus
the desktop-native vision authored at
[`../phase-8/architect/phase-9-desktop-native-vision.md`](../phase-8/architect/phase-9-desktop-native-vision.md).

## Phase 8 close summary

`chan-v0.13.0` shipped:

* Public-flip pre-flight documentation (LICENSE Apache 2.0,
  CONTRIBUTING, CODE_OF_CONDUCT, SECURITY, GitHub templates,
  docs/coordination.md, CHANGELOG.md).
* Screensaver themes (Matrix rain + code-drawn Castaway) +
  theme picker + timeout bounds.
* Terminal WebGL glyph corruption regression fixed.
* Drafts FB stale-tree chain fixed (covers Drafts browseability,
  Graph tab loading, Cmd+N New Draft).
* Tab-click focus on terminal + editor headers.
* Right-click menu closures (Graph hamburger Settings/Reopen/Close,
  Hybrid pane revamp, Terminal + Editor menu nits).
* chan-server async-blocking cleanup across 13 route handlers +
  `static_assets` (sync filesystem / graph / report work moved
  behind `spawn_blocking` / `tokio::fs`).
* chan-tunnel-* unused-dep cleanup.
* chan-desktop updater pubkey rotation + bridge-release flow
  documentation + package metadata bump.
* Repo history audited clean for the open-source flip.

## Carry-overs from phase 8 → phase 9

@@Alex's 2026-05-23 direction at v0.13.0 cut: items not
completing in Round 3 move to phase 9.

### Headline carry-overs

1. **Open-source flip (repo private → public).** Pre-flight docs
   are all in place; remaining steps are operational:
   * GitHub repo settings: private → public toggle.
   * README polish for outside readers (the existing README is
     internal-shape).
   * Announcement (channels: @@Alex-driven).
   * Optional: enable GitHub Discussions, set up issue / PR
     templates as defaults, configure CODEOWNERS if needed.

2. **Multi-model search picker** (was Round-3 Track 2; default-
   deferred 2026-05-23 per the time-boxed Round-3 cap). Curated
   list of embedding models, Settings UI dropdown, per-drive
   preference, extends the `systacean-6` / `-7` /
   `fullstack-a-21` Round-1 detour work. Reference:
   [`../phase-8/architect/round-3-plan.md`](../phase-8/architect/round-3-plan.md)
   §"Track 2 — Multi-model search picker".

3. **Chan metadata import / export** (was Round-3 Track 4
   added 2026-05-20; deferred from phase 8). `chan metadata
   export <drive-path> <output-path>` to a `.tar.zst`
   archive + `chan metadata import <drive-path>
   <archive-path>` with SCM-identity guard + `--rescan`.
   UI surfaces in the Infographics tab + pre-flight
   remediation card. Reference:
   [`../phase-8/architect/round-3-plan.md`](../phase-8/architect/round-3-plan.md)
   §"Track 4 — Chan metadata import/export".

4. **Desktop-native vision implementation** (phase-9 charter):
   * Single-binary-vs-separate-chan call (architect's read:
     embed by default; keep separate chan CLI for CLI / headless
     users).
   * Three-mode drive connection (local fork / attached outbound
     / attached inbound).
   * Default "Chan" drive lifecycle (fresh-install auto-creates
     drive named "Chan" seeded with manual; user-delete wipes all
     chan metadata; next launch recreates).
   * Bidirectional discovery between chan CLI and chan-desktop.
   * Cross-version protocol stability via `chan-tunnel-proto`.
   * Reference:
     [`../phase-8/architect/phase-9-desktop-native-vision.md`](../phase-8/architect/phase-9-desktop-native-vision.md).

5. **chan-desktop runtime walk on chan-v0.13.0 DMG**. The
   canonical fresh-Mac Gatekeeper walk on the released DMG
   (deferred from phase 8 per @@Alex's "i will only test the
   chan.app at the very very end" 2026-05-21 decision). Now
   that v0.13.0 ships, the walk lands as the final empirical
   sign-off. Belongs to chan-desktop team's @@Desktest under
   @@Desktect's dispatch.

### Polish + hardening carry-overs (non-blocking)

6. **`fullstack-a-96` sub-passes 1, 2, 3** (frontend cleanup):
   dead-code sweep, accessibility audit (Editor / Hybrid Nav /
   FB / Graph / Carousel — keyboard nav, ARIA, screen-reader),
   performance pass (editor scroll, graph open on large drives,
   carousel slide-change, SPA bundle size). Cleared in phase 8
   but never picked up; defer to phase 9. Reference task:
   [`../phase-8/fullstack-a/fullstack-a-96.md`](../phase-8/fullstack-a/fullstack-a-96.md).

7. **`systacean-44` / `systacean-45` P2 follow-ups**:
   * Broader `Mutex::lock().unwrap()` / `RwLock` unwraps in
     chan-server route / state code → explicit 500 conversion.
     Mechanical, not adversarial-input reachable, but improves
     crash containment.
   * Broader CLI error-message polish beyond the bind-address
     seed @@Alex flagged.
   References:
   [`../phase-8/systacean/systacean-44.md`](../phase-8/systacean/systacean-44.md)
   + [`../phase-8/systacean/systacean-45.md`](../phase-8/systacean/systacean-45.md).

### Bug backlog carry-overs

8. **Round 4 backlog**. `docs/journals/phase-8/alex/round4.md`
   currently has a `## Rich Prompt TODO` section @@Alex was
   drafting; the 4 original bugs from that file (Drafts FB,
   tab focus, Graph tabs, Cmd+N) all shipped in v0.13.0 via
   `-100` + `-101`. Whatever @@Alex adds to that file post
   v0.13.0 cut belongs to phase 9.

### Untracked-files decision

9. **`.codex/`, `.claude/`, `AGENTS.md`** untracked items at
   the repo root: @@Alex 2026-05-23 said "leave them in there
   and i will decide later; they are not going into any
   release". Phase 9 decides whether to track, gitignore, or
   delete.

### Process review (suggested, not load-bearing)

10. **Phase-8 process retrospective**: what worked, what
    didn't, what to evolve for phase 9. Examples worth
    capturing:
    * Multi-agent shared-worktree commit discipline (atomic
      audit + per-path adds). When it worked vs when it
      slipped (e.g. the `dec62ff` / `33382db` mishaps).
    * Cross-team bridge protocol with the chan-desktop team
      (@@Alex as bridge; async notes between architect leads).
      Worked smoothly this session; worth keeping.
    * Self-cut tasks (e.g. `systacean-45`): when authorized
      by @@Alex it works; default-architect-cuts otherwise.
    * Content-filter blocking on legal/policy text writes
      (CODE_OF_CONDUCT.md, SECURITY.md initial attempts):
      @@Alex's re-authorization with explicit context
      unblocked the second attempt. Worth documenting as a
      known failure mode.

## How phase 9 starts

@@Alex chooses when to open phase 9. On open:

1. Cut `docs/journals/phase-9/` directory structure mirroring
   phase 8 (process.md inherits with deltas; per-agent dirs;
   `alex/event-*-architect.md` channels).
2. Fresh @@Architect session bootstraps from
   `docs/agents/bootstrap.md` — Architect block; reads this
   `request.md` first; cuts task files from the carry-over
   list.
3. Round structure is @@Alex's call: could be a single Round 1
   (open-source flip + multi-model picker + metadata
   import/export bundled as the first cut), or multiple
   rounds.

## Inherited reference docs

* [`../phase-8/architect/phase-9-desktop-native-vision.md`](../phase-8/architect/phase-9-desktop-native-vision.md) — desktop-native architecture vision (@@Desktect's charter).
* [`../phase-8/architect/round-3-plan.md`](../phase-8/architect/round-3-plan.md) — Round-3 plan with the Track 2 / Track 4 deferrals.
* [`../phase-7/next-phase-backlog.md`](../phase-7/next-phase-backlog.md) — original phase-8 backlog; carries items 5 (chan config currency audit, partially landed via `systacean-28`) and 6 (website migration) still pending.
* [`../phase-8/alex/round4.md`](../phase-8/alex/round4.md) — round-4 bug list @@Alex started.
* `CHANGELOG.md` at repo root — released history.
