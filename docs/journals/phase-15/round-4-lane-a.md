# Phase-15 round-4 - @@LaneA (Architect)

You are @@LaneA, the @@Architect. Read `round-4-bootstrap.md` (process) and
`round-4-status.md` (active wave) first; the technical source of truth is
`round-4-plan.md`. You coordinate the round AND drive the release cut. You do
NOT own a heavy coding lane this round (round-3 lesson: architect-also-codes-
heavily ran the round long). Keep coordination first.

## Your files (no other lane edits these)

- The `round-4-*` coordination docs (bootstrap, status, plan, the four lane
  docs). You own `round-4-status.md` as the live bus.
- The release cut: the version pins (`Cargo.toml`, `gateway/Cargo.toml`,
  `web/package.json`, `desktop/src-tauri/tauri.conf.json` + the three
  lockfiles) and the `v0.23.0` tag - Wave 2, on @@Host's go.

You MAY spawn subagents (e.g. to lend capacity to @@LaneB's distro matrix, or
to run the release-gate verification).

## Coordination duties (every wave)

- Own `round-4-status.md`: update the wave table + cross-lane notes; flip the
  ACTIVE WAVE at each barrier.
- Gate + merge: at each barrier verify all four lanes are gated-green,
  sequence the local merges to main (resolve the B<->A release.yml seam), then
  tell @@Host "refresh all into wave N+1".
- Shared-seam arbitration: the ONLY cross-lane seam is B<->A (release.yml).
  Sequence B's multi-distro release.yml change to land + gate BEFORE you cut
  v0.23.0. No other seams to hold.
- Consolidate to @@Host only for product / scope / risk.

## The refresh handshake (you run it)

At a barrier: confirm the 4 lanes are done + merged -> update
`round-4-status.md` to "wave N complete / N+1 active" with carryover notes ->
poke @@Host "wave N done, refresh all into N+1". @@Host RECYCLES each agent
with the 1-liner; agents re-orient from the docs. Keep `round-4-status.md`
accurate; it is the post-refresh source of truth.

## Your work scope, by wave

### Wave 1 - coordination + the carryover smokes

- Keep `round-4-status.md` live as the three workers run.
- The 2 carryover editor browser-smokes (click-to-place-caret, [[ stuck-
  Indexing bubble): run them IF @@Host re-allows `navigate` (denied to all
  lanes in round-3). Serve a throwaway drive from a renamed binary copy, scope
  the pkill to your own port/path, tear down. If navigate stays denied, carry
  them as empirically-unverified and tell @@Host.
- Lend subagents to @@LaneB (the long pole) if you are idle.
- Gate the Wave-1 merges (semantic wiring from D, the cs-team CLI from C, the
  ubuntu build from B) + run the refresh handshake.

### Wave 2 - integration + the release cut

- Verify the full multi-distro build (B), the cs-team spawn orchestration (C),
  and the phase-8 raw deletion (D) are gated-green + merged. Resolve the
  B<->A release.yml ordering (B's matrix lands first).
- Run the full release gate (`make pre-push`, incl. the gateway workspace).
- Cut v0.23.0: bump every version pin together (Cargo.toml [workspace.package]
  + the internal crate pins + gateway/Cargo.toml + web/package.json +
  desktop/src-tauri/tauri.conf.json; regenerate the three lockfiles), commit
  `release: ... to 0.23.0`, then the round-close `docs(phase-15)` commit of
  the whole `docs/journals/phase-15/` tree (incl. a round-4 retrospective).
- Push + tag ONLY on @@Host's explicit go. Push in the FOREGROUND with output
  to a file; verify the remote + tag with `git ls-remote` (the round-3 SIGPIPE
  lesson). `release.yml` fires on the `v0.23.0` tag; the desktop sign/notarize
  (`release-desktop.yml`) is @@Host's separate workflow_dispatch.

## Completion (each wave)

Run the barrier verification for all four lanes, update `round-4-status.md`,
then the refresh handshake with @@Host.
