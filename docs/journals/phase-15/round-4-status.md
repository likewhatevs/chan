# Phase-15 round-4 status (architect-owned, live)

ACTIVE WAVE: Wave 1 (kickoff). Round-4 opened on the v0.22.0 base (HEAD at
kickoff = the round-3 docs(phase-15) commit). Awaiting @@Host bootstrap of the
four agents.

On (re)start, read this first to learn the active wave, then do your lane
doc's section for that wave. @@Architect updates this file at every barrier;
it is the single source of "where are we" after a refresh.

## Wave status

```
legend: -- todo  ~~ in progress  GG gated-green  MM merged
        VV verified (verification-only, no code to merge)
+------+--------+--------+
| Lane | Wave 1 | Wave 2 |
+------+--------+--------+
| A    |   --   |   --   |
| B    |   --   |   --   |
| C    |   --   |   --   |
| D    |   --   |   --   |
+------+--------+--------+
```

## Wave-1 scope per lane (lanes re-orient from their own lane-doc Wave-1 sections)

- A (architect): write + maintain these coordination docs; coordinate; gate
  the Wave-1 merges + sequence them; run the editor browser-smokes IF @@Host
  re-allows `navigate` (else carry as empirically-unverified); lend subagents
  to @@LaneB. NO heavy coding this wave.
- B: de-risk the build long pole. Get ONE distro (ubuntu, matches CI) building
  chan-desktop from macOS via sdme + emit a valid AppImage + verify the `cs ->
  chan-desktop` symlink dispatches (`cs terminal list` against a server, no
  GUI). New `make` target(s). This is the riskiest unknown; prove it first.
- C: build the `cs terminal team` CLI surface. `--script` FIRST (the
  design-driver): `new --script` / `load --script` emit a runnable shell
  script of the whole bootstrap. Then the `new`/`load` control-socket handler
  (config write/read + server-side bootstrap.md regen via the refactored
  shared fn). Spawn orchestration is Wave 2.
- D: (1) land the semantic wiring (small, self-contained): the route + CLI
  request Mode::Hybrid when `semantic_enabled` (+ model present), else Bm25;
  fix the stale comment. (2) Start phase-8 docs: synthesize the essence
  README + resolve the docs/agents citation handling (do NOT delete raw yet).

## Wave-2 scope (preview; lanes re-orient at the Wave-2 refresh)

- B: full multi-distro matrix (fedora, arch) + the gateway linux build + the
  CI matrix (release.yml).
- C: the lead-first terminal-spawn orchestration + tests + a live smoke (the
  emitted script reproduces the direct `new`).
- D: delete phase-8 `raw/` (after the citation repoint).
- A: full smoke + the release gate (incl. gateway) + the docs(phase-15)
  round-4 commit + cut v0.23.0.

## Touch points this wave (@@Architect-held)

- B<->A (release.yml): the ONLY cross-lane seam. @@LaneB edits
  `.github/workflows/release.yml` for the multi-distro matrix; the architect's
  release cut USES release.yml. Sequence B's release.yml change to land + gate
  BEFORE @@LaneA cuts v0.23.0 (Wave 2). No file collision within a wave (B owns
  release.yml; A only reads it for the cut).
- No other seams: C's chan-server files (control_socket/team_config) are
  disjoint from D's (routes/search); D's CLI edit (main.rs cmd_search) is
  disjoint from C's chan-shell.

## Carryover from round-3 (tracked for the architect)

- 2 editor browser-smokes (click-to-place-caret, [[ stuck-Indexing bubble):
  gated-green + source-tested, shipped empirically-unverified in v0.22.0
  because `navigate` was denied to ALL lanes. Re-run when @@Host re-allows
  navigate; A owns.
- The chip-clobber fix (round-3 41e7908e) was partially confirmed live (a
  server showed embedding:{done,total} during a real background embed); the
  full edit-during-embed transition is locked by the set_idle_reattaches unit
  test. No round-4 action.

## Cross-lane notes (latest at top)

- Round-4 opened. Coupling is LOW (disjoint lanes), so expect fewer
  barrier stalls than round-3. The gated-push SIGPIPE rule
  (round-4-bootstrap.md) is standing: foreground + file-redirect + verify with
  git ls-remote.
