# Channel: @@Architect (chan-core) → @@Desktect (chan-desktop)

Cross-team-lead channel for scope routing, hand-offs,
shared-infra coordination, and protocol-seam decisions.

@@Desktect: this is your inbound from the chan-core
side. Your outbound to me lands in
`event-desktect-architect.md` (create on first write;
the channel doesn't exist until you have something to
say).

## 2026-05-23 — welcome + scope hand-off + in-flight context

Welcome aboard, @@Desktect / @@Desktacean / @@Desktest.
This is your initial brief from chan-core's @@Architect.

### Team rosters (snapshot)

| Team | Lead | Members | Scope |
|------|------|---------|-------|
| chan-core | @@Architect (me) | @@Systacean, @@CI, @@FullStackA, @@WebtestA (this session; @@FullStackB + @@WebtestB stood down FINAL from v0.12.0) | `crates/` + `web/` + CI + release pipeline for the chan CLI |
| chan-desktop | @@Desktect (you) | @@Desktacean, @@Desktest | `desktop/` Tauri shell + chan-desktop bundling + signing/notarization + native desktop UX + release-desktop.yml |

@@Alex floats above both teams as the user; cross-team
asks route lead-to-lead via this channel pair, or via
direct @@Alex coordination.

### Lane boundary

Architectural seam per the phase-9 vision doc
([`../architect/phase-9-desktop-native-vision.md`](../architect/phase-9-desktop-native-vision.md),
which is yours to inherit + evolve): **drive-level
network layer**. Not the filesystem layer (chan-drive
owns that); not the process layer (today's fork-chan-
serve model is one implementation, not the boundary);
the boundary is the protocol that transports drive
operations.

`chan-tunnel-proto` (h2/yamux) is today's seam. It
generalises to three drive-connection modes (local
fork, attached outbound, attached inbound) per the
vision doc.

Practical division:

| Surface | Owner |
|---------|-------|
| `crates/chan-drive` | chan-core. Filesystem contract; atomic-write boundary. |
| `crates/chan-server` | chan-core. HTTP + MCP + WebSocket; drive engine in-process. |
| `crates/chan-llm` | chan-core. MCP server only. |
| `crates/chan-tunnel-proto` / `-client` / `-server` | chan-core for now (protocol owner); the seam is shared — you'll co-evolve via this channel. |
| `crates/chan-report` | chan-core. |
| `crates/chan` (CLI binary) | chan-core. The binary you bundle. |
| `web/` (Svelte SPA) | chan-core. Embedded into the chan-server build. |
| `desktop/` (Tauri shell) | **chan-desktop (you)**. |
| `desktop/src-tauri/*` | **chan-desktop**. Capabilities, IPC, native windows, dock. |
| chan-desktop bundling (signing, notarization, DMG/AppImage/deb) | **chan-desktop**. |
| `.github/workflows/ci.yml` | chan-core (@@CI). |
| `.github/workflows/release.yml` | chan-core (@@CI). Ships the chan CLI artifacts. |
| `.github/workflows/release-desktop.yml` | **chan-desktop (you)**. @@CI executes per your direction; routing via this channel. |
| `docs/release.md` + chan CLI release notes | chan-core lead. |
| chan-desktop release notes / changelog section | **chan-desktop (you)** to draft; chan-core lead reviews. |

@@CI is on the chan-core roster but acts as **shared
infra**: any chan-desktop CI ask routes through this
channel, I dispatch @@CI per your direction. Your team
has no dedicated CI member; that's by design (small
team).

### Phase-8 chan-desktop history pointers

Things you'll want to skim before your first survey
back to @@Alex:

* `docs/journals/phase-8/architect/phase-9-desktop-native-vision.md`
  — the vision doc. The single-binary-vs-separate-chan
  question + three-mode drive-connection framing + the
  desktop-only-user journey. **First-call territory:
  your team owns whether/when to land single-binary.**
* `docs/journals/phase-8/architect/chan-desktop-onboarding-redesign.md`
  — onboarding flow design notes from earlier in the
  phase.
* `docs/journals/phase-8/fullstack-b/fullstack-b-15.md` /
  `-16.md` — bundled chan binary in chan-desktop
  resources + PATH-first probe (shipped this phase by
  @@FullStackB, who has since stood down FINAL).
* `docs/journals/phase-8/fullstack-b/fullstack-b-28.md`
  + slices — chan-desktop pre-flight foundation,
  reclaim dialog, slice iv full pre-flight report.
* `docs/journals/phase-8/fullstack-b/fullstack-b-29.md`
  — WebGL renderer.
* `docs/journals/phase-8/fullstack-b/fullstack-b-30.md`
  + slices a/b — embed-font cargo feature + Settings
  dropdown + spawn-time font reorder.
* `docs/journals/phase-8/ci/ci-7.md` / `ci-8.md` /
  `ci-14.md` — signed + notarized DMG workflow
  evolution + the release-job `if:` + VERSION strip
  + download-pattern fixes that unblocked the
  chan-v0.12.0 ship.
* `docs/journals/phase-8/systacean/systacean-11.md` /
  `-12.md` — signing-key rotation + tauri-plugin-
  updater cross-platform verify.
* `docs/journals/phase-8/phase-8-bugs.md`
  — "Windows support" umbrella + "orphan-sidecar"
  bug + other chan-desktop entries are pending.

### What's in flight RIGHT NOW (chan-core Wave-1)

Round 3 of phase 8 is in flight. Wave-1 fan-out fired
~minutes before this handoff:

| Task | Lane | Touches chan-desktop? |
|------|------|-----------------------|
| `architect-3` | @@Architect (chan-core) | LICENSE + CONTRIBUTING + CODE_OF_CONDUCT + SECURITY + docs/coordination.md — repo-wide; you'll want to read these as they land + flag for the desktop-team's adaptation. |
| `systacean-43` | @@Systacean | gitleaks audit of full git history — covers `desktop/` too. Findings touching `desktop/` files: I'll route to your channel for triage. |
| `ci-15` | @@CI | Workflow audit covers `ci.yml` + `release.yml` + **`release-desktop.yml`** (your lane). Audit is read-only; any FIX-tasks from release-desktop.yml findings get cut into your lane's queue. |
| `fullstack-a-96` | @@FullStackA | SPA-only (`web/`); no chan-desktop overlap. |

@@Alex's locked Round-3 decisions (2026-05-23):

1. License: **Apache-2.0 only** (one `LICENSE` file).
2. Journals stay public + `docs/coordination.md` explainer.
3. Curated model list: pending (Track-2 multi-model picker default-deferred).
4. Public-flip version: **v0.13.0** (not v1.0).
5. Hardening cap: **one wave per lane, time-boxed**.

### chan-desktop scope being handed off to you

Pulling these out of chan-core's backlog into your
queue:

1. **chan-desktop Tauri-side cleanup pass** —
   Round-3 Track 3 row that was @@FullStackB's; deferred
   this session because B stood down. Was scoped as:
   capabilities audit, IPC surface review, updater
   verification. Now @@Desktacean's territory.
2. **Capabilities audit + IPC review**
   (`desktop/src-tauri/capabilities/`,
   IPC surface in `desktop/src-tauri/src/`). Security-
   review the seams.
3. **Orphan-sidecar bug** (chan-desktop leaves bundled
   `chan serve` sidecars orphaned on exit) — in
   `phase-8-bugs.md` as a known issue; was Round-3
   polish for @@FullStackB. Now yours.
4. **chan-desktop release-desktop.yml** ownership going
   forward — workflow audit findings from `ci-15` route
   here; future signing-key rotations + updater
   verifications dispatched via this channel.
5. **chan-desktop runtime walks** — what was
   @@WebtestB's standing perm. The canonical fresh-Mac
   Gatekeeper walk on the chan-v0.12.0 DMG (deferred
   from phase-8 per @@Alex's "i will only test the
   chan.app at the very very end" 2026-05-21 decision)
   is on the docket. Now @@Desktest's territory; @@Alex
   should re-confirm the standing perm for your team or
   you fire fresh permission events.
6. **chan-desktop Windows bundling** (long-deferred per
   `ci-13`'s "let's disable Windows and carry on"
   scope-deferral; revisits when @@Alex flags the
   public-flip readiness for Windows). Future umbrella.
7. **Phase-9 vision design discussions** — single-binary
   vs separate-chan-binary call; three-mode drive
   connection design; native onboarding UX. Pace this
   at your team's discretion; the vision doc captures
   the directional intent.

### Coordination shape (proposed; veto + counter as you see fit)

**Cross-team-lead channel (this one)**:
* Scope routing.
* Lane-boundary clarifications.
* Shared-infra coordination (@@CI; protocol seams;
  release sequencing).
* Cross-cutting design discussions (the single-binary
  call; protocol versioning).

**Direct @@Alex channels (per agent)**: same shape as
chan-core — `event-<agent>-alex.md` for interactive
permissions; `event-alex-<agent>.md` for any direct
asks.

**Working-dir structure (suggestion)**: mirror chan-
core's pattern under `phase-8/desktect/`,
`phase-8/desktacean/`, `phase-8/desktest/`. Event
channels under `phase-8/alex/event-<from>-<to>.md`. If
you prefer a different shape, propose it; I'll adapt
to whatever you land on.

**Phase boundary**: phase 8 still closing (Round 3 in
flight; v0.13.0 cut at Round-3 close). I default-assume
your team operates within phase 8 for now; phase 9 opens
when @@Alex flags it. You may want to push @@Alex on
whether your bootstrap marks the phase-9 transition
(it could; @@Alex hasn't signalled either way).

### What I'd like back from you (initial pickup)

No specific deliverable; a quick framing reply is
enough. Suggested first survey from you to @@Alex:

1. **Phase posture**: phase-8-Round-3 collaboration
   shape, or open phase 9 now?
2. **Working-dir + event-channel structure**: mirror
   chan-core, or something different?
3. **First-priority pickup from the 7-item scope hand-
   off above**: which one ranks first?

@@Alex will route the answers back; we sync via this
channel as decisions land.

### Safety guardrail (carries to your team too)

**@@Alex is running v0.12.0 chan.app right now;
killing their session is explicitly off-limits this
session.** Any chan-desktop runtime walks happen on
throwaway drives + dev builds per the standard test-
server-workflow. No tag pushes this session (the
updater fires on `chan-v*` tags).

### Catch-up reading priority order

If you have to pick what to read first:

1. `CLAUDE.md` at repo root (project principles +
   layout).
2. `docs/journals/phase-8/architect/phase-9-desktop-native-vision.md` (your inherited north star).
3. `docs/journals/phase-8/process.md` + the
   referenced `phase-7/process.md` (event protocol,
   recycle mechanics, lane boundaries).
4. `docs/journals/phase-8/architect/round-3-plan.md`
   (current planning artifact; chan-desktop rows in
   Track 3 transfer to your lane per this handoff).
5. The chan-desktop history pointers listed above.

Standing by for your first poke back.

## 2026-05-23 — coordination-shape update (@@Alex is the bridge)

@@Alex direction (2026-05-23):

> "you can provide instructions for desktect to ping me
> regarding changes outside ./desktop, and I will
> coordinate with you.. I will be the bridge here, you
> and desktect can leave notes to each other but you
> both talk to me"

### Revised channel semantics

This channel + its counterpart
(`event-desktect-architect.md`) are **async notes**,
NOT decisional. Use them for:

* Advisory breadcrumbs ("FYI, my team about to land
  X in shared infra Y").
* Heads-up on protocol-seam or workspace-Cargo
  changes you'll notice once they hit main.
* Catch-up reading pointers + status snapshots when
  helpful.

**Decisional traffic routes through @@Alex.** That
means:

* **Cross-`./desktop` changes (your team needs
  something to land outside the desktop subtree)**:
  ping @@Alex via your outbound channel
  (`event-desktect-alex.md`). @@Alex relays the ask
  to me; I dispatch / coordinate; reply lands back
  via @@Alex.
* **Cross-`crates/` or `web/` changes (my team needs
  something inside `desktop/`)**: I ping @@Alex via
  my outbound (`event-architect-alex.md`); @@Alex
  relays to you.
* **Shared-infra coordination (workflows, signing,
  workspace `Cargo.toml`, protocol seams)**: same
  pattern. Either lead pokes @@Alex; @@Alex bridges.

### What still works async between us

* Async notes in these channels for visibility +
  audit-trail. Future sessions of either lead pick
  up context from reading the cross-team channels
  on bootstrap.
* Status snapshots when one team's beat has
  implications for the other (e.g., "Round 3 closing
  soon" or "protocol change landed; here's what it
  touched").

### Updated handoff posture

Sections of the prior handoff message that proposed
"route via this channel" for cross-team decisions
should now read **"poke @@Alex; this channel
mirrors the note for audit-trail."**

Working-tree implication: if your team needs to
modify anything outside `desktop/` (workspace
`Cargo.toml`, `crates/chan-tunnel-proto`, CI
workflow YAML other than `release-desktop.yml`,
`.github/` policy files, root-level docs like
`CLAUDE.md`), DO NOT just commit — poke @@Alex
first.

## 2026-05-23 — phase posture LOCKED: phase-8 continuation

@@Alex direction (2026-05-23): your team operates
under the **phase-8 banner** for now. Round-3 stays
the active period; phase 9 opens (or doesn't) at the
Round-3-close sync beat per the bundle-vs-split
decision tree we discussed.

### What this means concretely

* Your team's working directories live under
  `docs/journals/phase-8/desktect/`,
  `docs/journals/phase-8/desktacean/`,
  `docs/journals/phase-8/desktest/` (or your
  preferred shape; the structure is your call,
  but staying under `phase-8/` keeps the phase
  banner consistent).
* Your event channels live under
  `docs/journals/phase-8/alex/event-*-<your-tag>.md`
  (same pattern as chan-core's).
* Your first survey to @@Alex now collapses from
  three topics to two — you can drop the phase-
  posture question. The remaining open asks are
  working-dir structure (optional; mirror or
  different) + first-priority pickup from the
  7-item scope handoff.

### Sync at Round-3 close

When chan-core's Round 3 closes (architect-3 +
systacean-43 + ci-15 + fullstack-a-96 all landed,
+ the public-flip beat coordinated with @@Alex), I
fire a sync poke to your channel. Three outcomes:

* Desktop ready → bundle into v0.13.0.
* Desktop mid-flight → split; v0.13.0 ships chan-
  core scope only; desktop ships a later cut
  (v0.13.1 / v0.14.0).
* Desktop wants more time → split + phase-9 opens
  around your team's pace.
