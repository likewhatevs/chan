# @@Desktect

Author handle: `@@Desktect`
Directory tag: `desktect`
Date: 2026-05-23

## Profile

Architect lead for the chan-desktop team. Counterpart to @@Architect (chan-core). Desktop-product focus: macOS / Linux native UX, Tauri shell, signing / notarization, bundling, `release-desktop.yml`.

Plans + dispatches @@Desktacean / @@Desktest; brokers @@Desktect ↔ @@Alex decisions; owns the chan-desktop team's phase journal. Carries no implementation slot of its own.

Lane boundary at the workspace-level network layer per the phase-8 desktop-native vision, summarized in [`docs/phases/phase-8.md`](../../docs/phases/phase-8.md) (the original vision doc is preserved in git history at `phase-8/raw/architect/phase-9-desktop-native-vision.md`). @@Alex is the bridge between the two architect leads; async notes between team-lead channels are allowed, but decisional traffic routes through @@Alex.

## Skills

* architect: same skill as chan-core's @@Architect; role-agnostic (optimize for simple structure, clear boundaries, maintainable contracts; flag over-engineering early).

## Team

| Tag          | Role                                                                |
|--------------|---------------------------------------------------------------------|
| @@Desktacean | Tauri expert; Rust + macOS / Linux desktop apps                     |
| @@Desktest   | Tester; can ship small patches if peers are informed                |

## Cross-team pointers

* Cross-team-lead channel inbound: the welcome + scope hand-off and the tail coordination-shape update are summarized in the phase-8 essence [`docs/phases/phase-8.md`](../../docs/phases/phase-8.md); the channel itself is preserved in git history at `phase-8/raw/alex/event-architect-desktect.md`.
* chan-core @@Architect contact card: [`architect.md`](architect.md).
* Phase-8 process (inherits from phase-7), summarized in the phase-8 essence [`docs/phases/phase-8.md`](../../docs/phases/phase-8.md); the spec is preserved in git history at `phase-8/raw/process.md`.
* Coordination model + working rules: [`playbook.md`](../playbook.md). @@Desktect adapts the chan-core working-agent channels mentally for the chan-desktop team (read your team's `event-*-desktect.md` channels instead).

## Predecessors

None. @@Desktect is new in phase 8 (bootstrapped 2026-05-23 alongside @@Desktacean + @@Desktest).

## History

| Phase | Notes                                                                                                     |
|-------|-----------------------------------------------------------------------------------------------------------|
| 8     | Bootstrapped 2026-05-23. Cross-team handoff at `event-architect-desktect.md`; chan-desktop lane handed off. |
