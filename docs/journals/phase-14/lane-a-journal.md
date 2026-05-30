# @@LaneA journal - Phase 14 (round 3 + addendum-1 carryovers)

Backend hot-paths + pre-flight lane. Rust only (`chan-server`,
`chan-workspace`, `chan-desktop`). Worktree `../chan-p14-lane-a`,
branch `phase-14-lane-a`. Status board + decisions in `lane-a-plan.md`;
this is the round-close narrative + retrospective.

## What shipped (branch `phase-14-lane-a`)

| Commit | Item |
|--------|------|
| `cd1d625` | A1/A2 cursor-paged `/api/fs-graph` delivery |
| `20db58d` | A4 draft-banner backend stress test |
| `b864328` | A5 de-flake serial-lock + macOS-PTY tests |
| `ed21c91` | A5 drop vestigial `team-work-N` convention |
| `0f727ff` | A3 new-workspace pre-flight endpoints |
| `82d54dd` | A5 de-flake drafts-subtree watcher test (3rd offender) |

Contracts §1 + §2 proposed -> Lane B confirmed -> PINNED. Lane gate
(`make ci-linux` Rust half) green: fmt --check + clippy --all-targets
-D warnings + test --all-targets (all pass) + build --no-default-features.
No frontend touched.

## Key decisions

- **Graph transport = pull-based cursor paging over HTTP on the
  fs-graph spine; `/ws` bus untouched.** The bus is a per-socket
  broadcast for server-initiated watcher/progress; graph data is
  request-scoped and pull-paging gives exact backpressure for free.
  Lane B confirmed the spine/overlay split was already the existing
  shape. Whole-scope (no params) stayed byte-identical so the depth
  probe + CLI didn't move.
- **Pre-flight is DERIVED from live state, not stored.** No AppState
  field (which would have churned every test harness) and no first-boot
  flag to persist/reset: `index` step reads the indexer's IndexStatus,
  `locked = phase != ready`. A fresh large workspace reads locked while
  it builds and flips to ready when settled; an indexed one is ready at
  once. The "skip" model decision sticks via the existing
  `semantic_enabled` flag.
- **De-flakes by intent, not by bigger timeouts.** Serial-lock: the
  150ms was a latency budget masquerading as a deadlock backstop ->
  reframed + made generous. macOS-PTY + drafts-subtree: skip when the
  environment capability (device tty / OS-watcher delivery) demonstrably
  didn't happen, keep the assertion when it did - mirroring the addendum's
  endorsed pattern.

## Highlights

- A1 landed matching the eventual pinned §1 exactly, so Lane B has zero
  rework on the graph seam.
- Caught a THIRD indexer-flake offender (`writes_to_drafts_subtree…`)
  only because I ran the full `cargo test --all-targets` gate, not just
  per-crate tests - it dropped an FSEvents event under parallel load.
  Fixed it without losing coverage (the deterministic reindex test still
  guards the product path).
- The mid-round "blank remote window" turned out to be a one-line
  user-facing flag-name bug, and chasing it surfaced the SAME error in
  three more places (gateway UI + docs) now handed to Lane C.

## Lowlights / honest feedback

- **On me:** I over-committed to the SSH-forward diagnosis for the blank
  window. The `administratively prohibited` SSH error was real, but the
  ACTUAL fix @@Alex already knew was the `--tunnel-workspace` ->
  `--tunnel-workspace-name` flag typo in the desktop snippet. I should
  have grepped the desktop's `chan serve` command construction for flag
  correctness FIRST (cheap, 30s) before building the SSH-transport
  theory + reproducing in a browser. The empirical repro (SPA loads fine
  over loopback) was still useful, but I spent effort proving "not chan's
  HTML" when the snippet flag was the obvious first suspect for a
  desktop-launched serve. Lesson: for "desktop launches X and X is
  broken," diff the exact command/flags the desktop emits against the
  CLI's clap defs before theorizing about transport.
- **On A3 scope:** I shipped the server core but deferred the desktop
  relocation + made a unilateral product call on the model-prompt policy
  (prompt only when semantic-enabled-but-model-missing, not every new
  workspace). It's a defensible local-first default and I flagged it, but
  it IS a product decision @@Alex should ratify - calling it out loudly
  rather than burying it in code.
- **On the round:** the third flake should have been a known item up
  front - the addendum named the "family" but scoped A5 to two
  offenders. Running the full gate (not per-crate) earlier would have
  surfaced it sooner. Worth a standing rule: de-flake work always
  validates against the full parallel `--all-targets` run, since that's
  the load profile that flakes.

## Carryovers / handoffs

- **A3 desktop relocation** (`default_workspace.rs`/`serve.rs`/`main.rs`):
  deferred. The server pre-flight is additive; rewiring the desktop's
  default-workspace dialog is best done once Lane B's locked OverlayShell
  exists so the flow can be verified end-to-end in WKWebView (couples to
  A5d).
- **Model-prompt policy:** product call for @@Alex (see lane-a-plan A3).
- **A5d WKWebView walk:** human verify only (agents can't repro WKWebView);
  `make macos-chan-dmg` + the Cmd+Shift+N / Cmd+I / Cmd+P / self-upgrade
  walk.
- **Lane C:** `--tunnel-workspace` -> `--tunnel-workspace-name` docs/copy
  sweep (`event-lane-a-lane-c.md`); one target is in @@LaneB's gateway
  frontend tree.
- **Lane B UX gap:** chan-desktop shows a BLANK white window when the
  webview can't reach the server (refused connection / bad flag / server
  down). A "couldn't reach the workspace at <url>" load-failure state
  would make this class of failure self-diagnosing. Filed here for the
  frontend lane.

## Out-of-round fix (this session, @@Alex-raised)

- `desktop/src-tauri/src/main.rs` `build_snippets()`: the tunnel
  listen-panel snippet emitted `chan serve … --tunnel-workspace={name}`,
  which clap rejects (real flag `--tunnel-workspace-name`), so a pasted
  command failed and the desktop tunnel webview opened blank. One-line
  fix; no test pinned the string. NOT yet committed (separate from the
  round work; @@Alex to sequence).
