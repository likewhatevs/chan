# Phase-15 round-4 plan

Architect decomposition of `round-4-backlog.md` (with @@Host's decisions),
grounded against the current code by the round-4 kickoff exploration. This is
the technical source of truth: file:line anchors + the per-lane approach. Not
an implementation; the lanes own the code.

Quality rules hold: no em dashes, ASCII tables, WHY comments, the full gate
before any push, browser-smoke the runtime-risky bits, no back-compat paths.
Push/tag stays @@Host-only. Round-4 ships as v0.23.0.

## @@LaneB - Linux build tooling (the long pole)

Build ALL linux components from a macOS machine via sdme/lima: chan-desktop
for ubuntu/debian, fedora/centos/almalinux, arch/cachyos + the `gateway/`
components + AppImage packaging + verify a `cs -> chan-desktop` symlink.
Launching the AppImage is OUT (no GUI in the container; qemu later) - the goal
is the BUILD + the `cs` symlink dispatch.

Grounded state:
- NO existing target builds chan-desktop for linux from macOS. `make dev`
  (root Makefile ~191) is the CLI server; `desktop/Makefile build` (~36) is
  `cargo tauri build` native-only. Root Makefile ~51-80 already has CLI linux
  packaging (`linux-deb`, `linux-rpm`, `linux-archpkg`, `linux-packages`) - a
  reference pattern, NOT desktop.
- sdme pattern is documented: `docs/contributing/linux-and-macos.md` ~27-80
  (`limactl shell default sudo sdme ...`; `fs import|build`, `create|start|
  exec|cp`). The gateway ships `.sdme` rootfs templates
  (`gateway/scripts/dev/sdme/chan-psql.sdme`) - the template to copy per
  distro.
- AppImage: Tauri auto-emits it with `targets:"all"`
  (`desktop/src-tauri/tauri.conf.json` ~40-62); no AppImage-specific config
  needed. `release.yml` ~284-347 already has a `linux-desktop-artifacts` job
  (ubuntu x86_64 only) staging `Chan_*_amd64.AppImage` + `.deb` - extend to
  the multi-distro matrix.
- `cs` symlink is DONE + unit-tested (verify-only): argv0 detection
  `crates/chan-shell/src/lib.rs:35-46` (`invoked_as_cs`, `file_stem()=="cs"`);
  AppImage first-run wrapper `desktop/src-tauri/src/cs_install.rs` (drops
  `~/.local/bin/cs` -> `exec -a cs "$APPIMAGE"`); alias tests
  `crates/chan/tests/cs_alias.rs`.

Approach: de-risk ONE distro first (ubuntu, matches CI) - prove `cargo tauri
build` runs inside an sdme rootfs and emits a valid AppImage/.deb, copy it
out to the host, run `cs terminal list` against a server (no GUI). Then add
fedora + arch (.sdme per distro; webkit2gtk dep names differ), then the
gateway linux build, then the CI matrix. New `make` target(s) for
linux-chan-desktop (root + desktop + packaging/linux). Riskiest unknown:
Tauri-bundler-in-sdme + distro dep variants. ~5-7 feature-days; the round's
long pole; subagents for the per-distro matrix.

Files: root `Makefile`, `desktop/Makefile`, `packaging/linux/Makefile`,
`.github/workflows/release.yml`, `scripts/dev/sdme/*.sdme` (new),
`docs/contributing/linux-and-macos.md`.

## @@LaneC - `cs terminal team` CLI (new | load + --script)

CLI equivalent of the Cmd+P team setup/load dialog.

Grounded state:
- Team config: `crates/chan-workspace/src/teams.rs:9-64` - `TeamConfig`
  (team_name, host_name, host_handle, tab_group, auto_prefix_at, created_at,
  members) + `Member` (handle, command, env, is_lead, position?, agent?).
  Validation `routes/team_config.rs:75-109` (1-9 members, exactly one lead,
  non-empty names, agent in claude|codex|gemini|none).
- On-disk (inside the workspace): `{team-dir}/config.toml` (user-editable),
  `bootstrap.md` (server-regenerated), `tasks/ journals/ followups/`. All I/O
  via `Workspace::{read_text,write_text,create_dir}`
  (`routes/team_config.rs:19-26`).
- bootstrap.md generation: `routes/team_config.rs generate_bootstrap_md`
  (~223) - roster table + per-agent poke chords (claude `\x1b[27;9;13~`,
  codex/gemini `\r`) + the 1-liner poke format. REFACTOR to a shared fn so the
  CLI/control-socket path reuses it (no client-side regen).
- HTTP routes: `POST /api/team-config/{read,write}` (`lib.rs` ~911-912).
- Dialog flow (the orchestration to mirror):
  `web/src/state/teamOrchestrator.svelte.ts runTeamBootstrap` ~339-441 -
  write config -> resolve team group (collision-detect, append -N) -> spawn
  LEAD first (into the Cmd+P placeholder pane) -> drop placeholder -> spawn
  workers -> place identity prompt -> seed submit mode -> broadcast.
- `cs terminal` surface: `crates/chan-shell/src/cli.rs:161` `TerminalAction`
  (New/Write/List/Restart/Survey) dispatching `ControlRequest`
  (`wire.rs`) over the control socket; handlers
  `chan-server/src/control_socket.rs handle_request` ~224. No team ops yet ->
  a new `ControlRequest::TerminalTeam` variant + handler.

Approach (build `--script` FIRST - it is the design-driver):
1. `cs terminal team new --script` (+ `load --script`) emits a runnable
   shell script of the WHOLE bootstrap: `mkdir -p {dir}/{tasks,journals,
   followups}`; `cat <<'EOF' > {dir}/bootstrap.md` heredoc with the generated
   bootstrap; then per agent (LEAD first) `cs terminal new --tab-name=<handle>
   --tab-group=<team>` + `cs terminal write --tab-name=<handle>
   --submit=<agent> $'<identity/bootstrap prompt>\x1b[27;9;13~'`. Building this
   first FORCES the public `cs` surface to express team bootstrap end-to-end;
   any gap (missing flag, unscriptable step) is the API to fix.
2. The non-`--script` `new` then RUNS the same sequence (exec the script, or
   the handler does the equivalent) - one source of truth.
3. `load` reads + validates `{dir}/config.toml` (reuse the HTTP read logic),
   then the same spawn path.

Trickiest: the lead-first spawn sequence (placeholder never goes empty);
bootstrap.md is a server-regenerated artifact (never client-side); tab-group
collision detection. ~2-3 days.

Files: `crates/chan-shell/{cli,wire}.rs`,
`crates/chan-server/{control_socket,team_config}.rs`, reuse
`crates/chan-workspace/src/teams.rs`.

## @@LaneD - semantic-search wiring + phase-8 docs cleanup

### Semantic search behind `semantic_enabled` (SMALL)

The probe finding: dense vectors are built + stored every reindex but NEVER
queried - every path is BM25-only. Decision: gate hybrid behind the existing
`semantic_enabled` opt-in.

Grounded state:
- `semantic_enabled` persists in `crates/chan-workspace/src/index/config.rs`
  (~155, default false, per-workspace `config.toml`); read via
  `Workspace::semantic_enabled()`; toggled via `set_semantic_enabled` +
  `/api/index/semantic/{enable,disable}` (`routes/index.rs` ~257-314) + the
  `chan index enable-semantic` CLI.
- `Mode` enum (`facade.rs:78-102`, `#[default] Bm25`, Semantic, Hybrid); the
  Hybrid/RRF path (`facade.rs` ~1088-1111, parallel BM25 + vector ->
  `fusion::rrf`) already works when the feature is on + the model is present.
- The route `routes/search.rs` ~180 hardcodes `SearchOpts {
  ..Default::default() }` = `Mode::Bm25` and NEVER reads the flag; the
  "defaults to Hybrid" comment (~183) is STALE. The empty-query response
  (~171-177) hardcodes `mode:"hybrid"`. The CLI `main.rs cmd_search` ~1779
  also hardcodes default (Bm25).

Approach: in the route, read `workspace.semantic_enabled()` (+ a model-present
check, mirroring `routes/index.rs` ~144 / `embeddings::resolve_model`);
request `Mode::Hybrid` when on, else `Bm25`; fix the stale comment + the
empty-query mode. Mirror in `cmd_search`. Add a unit/integration test (route
requests Hybrid when the flag is on). ~20-30 lines. No facade/config/indexer
change. Files: `crates/chan-server/src/routes/search.rs`,
`crates/chan/src/main.rs`.

### phase-8 docs cleanup (@@Host-sequenced; destructive LAST)

phases 1-7, 9-14 already cleaned (raw dropped, essence READMEs, a930a96f).
phase-8 deferred because `docs/agents/` cites it. Tasks: (1) synthesize the
phase-8 essence README in the phases-1-13 shape; (2) repoint
`docs/agents/desktect.md`'s 3 links (`phase-9-desktop-native-vision.md`,
`event-architect-desktect.md`, `process.md` - already broken, pre-`raw/`
paths) to the essence README (or a git-history note); (3) decide
`docs/agents/bootstrap.md`'s template-phase handling (it uses `phase-8` as an
example path throughout, NOT live cites - leave as an example or bump it);
(4) THEN delete phase-8 `raw/`. Files: `docs/journals/phase-8/`,
`docs/agents/`.

## @@LaneA - architect + release

Coordination (status, gating, merges, the wave refreshes) + the release cut
(v0.23.0). Coding-light: the 2 carryover editor browser-smokes (click-caret,
[[ stuck bubble) when @@Host re-allows `navigate`; lends subagents to @@LaneB
when idle. The B<->A release.yml seam: sequence B's multi-distro release.yml
change to land + gate BEFORE the cut.

## Recommended waves

- Wave 1 (de-risk + quick wins): B ONE distro (ubuntu) building + AppImage +
  cs verify; C the `--script` + CLI + control-socket handler; D the semantic
  wiring (lands) + phase-8 essence README + citation handling; A coordinate +
  editor smokes (if navigate) + gate.
- Wave 2 (bulk + ship): B full matrix (fedora, arch) + gateway linux + CI
  matrix; C lead-first spawn orchestration + tests + smoke; D delete phase-8
  raw; A full smoke + release gate (incl. gateway) + docs(phase-15) commit +
  cut v0.23.0.

## Verification (per lane, end-to-end)

See `round-4-bootstrap.md` for the gate + the gated-push SIGPIPE rule. Per
lane: B = `make <linux-desktop>` builds per distro via sdme + the AppImage
runs `cs terminal list` (no GUI) + gateway builds + CI matrix green. C =
`--script` emits a runnable script that reproduces the same team as direct
`new` (diff them) + `load` validates + lead-first smoke. D = live search
probe (semantic OFF -> mode=bm25, ON+model -> mode=hybrid) + CLI parity;
phase-8 graph shows no ghost nodes + desktect.md links resolve + raw deleted.
A = the 2 editor smokes + the full release gate + the v0.23.0 cut.
