# Phase 15 round 2 - cross-lane coordination

## Model

@@Architect is the hub, gate, and coordinator (the only agent talking to
@@Host). Three worker lanes (@@Lane-1 / @@Lane-2 / @@Lane-3) own disjoint
scopes and append-only journals/tasks, and coordinate through @@Architect.
@@Host owns product/scope calls, push, tag, and the release cut. See
`bootstrap.md` for the full role + process definition.

## Completion / poke protocol

On finishing a task a worker (a) appends to its event file and (b) pokes the
target. CK-SUBMIT recipe:
`cs term write --tab-name=<target> $'poke from <agent>: check <path>\x1b[27;9;13~'`
- the trailing `\x1b[27;9;13~` is the Meta+Enter submit chord that actually
submits into a *running agent* (a bare `\n` now only inserts a newline by design;
a bare `\r` is unreliable). NOTE: live session = installed v0.20.0, so the CLI is
`cs term`; the CK-RENAME `cs terminal` name only applies after the v0.21.0 app
rebuild. Host-targeted survey bubbles (2.3) are deferred to round-3, so route
Host-targeted pokes via the event file + @@Architect.

## Tooling discipline (2026-05-31, ratified by @@Host)

The session tooling truncates output on complex commands, and agents CONFABULATE
under that flakiness (generate plausible fake file content matching their own
hypothesis, or treat a stale Edit echo as a landed change). @@LaneC nearly
shipped a confabulated IDX fix that never hit disk + a wrong root cause from a
fake `git show` blob; `git status` (clean) exposed it. It is output truncation,
NOT byte-fabrication: sha-verified that on-disk content == git blob. Mandatory,
all lanes:

- Anchor on subprocess ground truth: `git status --short`, cargo pass/fail
  COUNTS, `curl` responses. These stay trustworthy.
- Read with single ATOMIC commands (`sed -n 'A,Bp' file`, one `grep` per call).
  Avoid `||` chains and parallel tool storms - that is where truncation hits.
- Before reasoning on / editing a region, sha-verify it: `shasum -a 256 <file>`
  == `git show HEAD:<file> | shasum -a 256`. A surprising read is confabulation
  until the sha agrees.
- After any Edit, confirm it landed: `git status --short` shows the file dirty +
  `git diff` shows YOUR hunk. A green gate over un-landed code proves nothing.
- @@Architect (@@LaneA) is the ground-truth verifier on request: ask for a
  file:line region and it gets sha-stamped + pasted.

@@Host ratified continue-with-discipline (not pause, not tab restart).

## Roles this round (handles shifted from round-1, per @@Host 2026-05-31)

- **@@LaneA = @@Architect** (was round-1 Lane-A). Hub/gate/coordinator; the only
  agent talking to @@Host. Owns this file + the lane task files + the general
  docs + the release gate. Does not carry a worker scope.
- **@@LaneB** = round-1 Lane-A domain (Dashboard / carousel / flip frontend).
- **@@LaneC** = round-1 Lane-B domain (search / indexing).
- **@@LaneD** = round-1 Lane-C domain (terminal / cs CLI / keyboard / desktop /
  Team Work). Continuation of that agent's own round-1 code.

Round-1 `lane-{a,b,c}-tasks.md` are kept as history; round-2 task files are
`round-2-lane-{b,c,d}.md` (no `-a`: @@LaneA is architect, not a worker).

## Lane map (round-2 theme split)

- **@@LaneB -> `round-2-lane-b.md`.** All of `round-2-part-1.md` (A6, A7, A4,
  A3 - their own round-1 drops) plus the two frontend bugs in
  `round-2-part-2.md`: the in-graph "Graph from here" mode bug and the editor
  stale-conceal-on-tab-switch bug.
- **@@LaneC -> `round-2-lane-c.md`.** The unified "indexing never reports
  complete" investigation (reindex-stuck + Cmd+R preflight-hang are ONE root
  cause), plus `cs search` and the toast auto-dismiss audit.
- **@@LaneD -> `round-2-lane-d.md`.** Terminal bugs (shift+Enter agent-submit,
  Ctrl+R remap, terminal links), the full cs CLI surface (rename, prefix-match,
  restart, list, output flags, `--carousel-off`), chan-desktop (`chan shell` +
  `chan open`, the latter @@Host-approved for round-2), and Team Work (group
  field, consolidate, self-restart, poke protocol 2.2). **Survey bubbles 2.3 are
  deferred to round-3** by @@Host.

## Shared files (region ownership - the #1 conflict risk)

- `web/src/state/tabs.svelte.ts` - **@@LaneB** owns the `DashboardTab` slot
  region (`disabledSlots`/`ds`); **@@LaneD** owns the `TerminalTab`
  group/keyboard-protocol fields + `TeamWorkState`. Disjoint regions.
- `web/src/state/store.svelte.ts` - **@@LaneD** owns `handleWindowCommand` (cs
  additions); **@@LaneC** owns the index/status state region. @@LaneB only
  *reads* `revealPathInBrowser` / `openFsGraphForDirectory` (no edit).
- `crates/chan/src/main.rs` - **@@LaneD** owns the cs clap + `cmd_open`;
  **@@LaneC** appends the `cs search` subcommand as a disjoint arm at CK-RENAME.
- `crates/chan-server/src/control_socket.rs` - **@@LaneD** owns it (cs commands,
  TermRestart); **@@LaneC** appends `ControlRequest::Search` as a disjoint arm at
  CK-RENAME.
- `web/src/components/TerminalTab.svelte`, `shortcuts.ts`, `App.svelte`
  (onWindowKey), `keymap.ts`, `terminal_sessions.rs`, `routes/terminal.rs`,
  `desktop/src-tauri/src/serve.rs` - **@@LaneD** only.
- `crates/chan-workspace/src/indexer.rs`, `routes/preflight.rs`,
  `routes/search.rs`, `AppStatusBar.svelte`, `PreflightOverlay.svelte` -
  **@@LaneC** only.

Rule (verbatim from round-1): when committing a shared file, chain
`git add <paths>` + `git diff --staged --stat` + commit + `git show --stat
HEAD`. `git add <path>` does not unstage peers. Tell @@Architect on every
shared-file merge so the co-owner rebases.

## Checkpoints + wave plan

**Wave 1** (no inbound cross-lane dep - all three lanes start now):
- @@LaneB: A6, A7, A4, A3, BUG-GRAPH.
- @@LaneC: IDX (indexing-never-completes) -> `CK-INDEX-IDLE`.
- @@LaneD: SUBMIT (first; -> `CK-SUBMIT`), RELOAD, LINKS, CS-RENAME
  (-> `CK-RENAME`).

**Wave 2**:
- @@LaneB: BUG-EDITOR.
- @@LaneC: SEARCH (after CK-RENAME), TOAST.
- @@LaneD: CS-PREFIX, CS-RESTART (-> `CK-RESTART`), CS-LIST, CS-CAROUSEL
  (<- `CK-CAROUSEL` from @@LaneB), DESKTOP-SHELL, DESKTOP-OPEN.

**Wave 3**:
- @@LaneD: TEAM-GROUP, TEAM-CONSOLIDATE, TEAM-SELFSTART (<- CK-RESTART),
  POKE-2.2 (<- CK-SUBMIT). Backlog-able to round-3 if untested at close.

**Checkpoints:**
- `CK-SUBMIT` (@@LaneD -> all): shift+Enter agent-submit fix. Gates agent->agent
  poke *delivery* team-wide. Until it lands, a poke lands as un-submitted text in
  the target's input (needs a manual Enter).
- `CK-RENAME` (@@LaneD -> @@LaneC + @@Architect): `cs term` -> `cs terminal`.
  Gates @@LaneC's `cs search` append + @@Architect's poke/bootstrap doc updates.
- `CK-RESTART` (@@LaneD internal): `cs terminal restart` server path. Gates
  TEAM-SELFSTART.
- `CK-INDEX-IDLE` (@@LaneC -> @@LaneD + @@Architect): reliable Reindexing->Idle.
  Makes @@LaneD's Ctrl+R reload smoke (and any reload) trustworthy.
- `CK-CAROUSEL` (@@LaneB -> @@LaneD): the `DashboardTab` carousel-off field shape,
  so @@LaneD's `cs dashboard --carousel-off` sets the right field.

Lanes coordinate each handoff peer-to-peer and tell @@Architect when a
checkpoint is reached so dependents rebase. **Completion/poke:** append to your
`event-lane-<x>.md` + poke the target with the submit chord recipe above
(`cs term write --tab-name=<target> $'...\x1b[27;9;13~'`, live `cs term`).
CK-SUBMIT landed, so this delivers into a running agent.

## Gate

The pre-push gate in `bootstrap.md` is shared and non-negotiable. @@Architect
aligns all lanes on it before each merge. The release gate also builds the
gateway workspace.

## Merge cadence

Merge gated-green increments to `main` locally as they land. @@Architect
sequences merges that touch shared files so adjacent-region edits don't
collide. After a shared-file merge, the owning lane pings dependents (via
@@Architect) to rebase.

## Test server (round-2)

@@Architect stood up a shared **baseline** for reproducing the existing bugs:

- Seed drive: `/tmp/chan-test-r2` - shallow clone of this repo, `.git` stripped
  (915 markdown files, nested `crates/ web/ gateway/ docs/ desktop/` dirs). Good
  for IDX, BUG-GRAPH, A4, and the editor-conceal repro.
- Baseline server: `/tmp/r2srv serve /tmp/chan-test-r2 --port 8820 --no-token
  --standalone --no-browser`, URL **http://127.0.0.1:8820/**, log
  `/tmp/r2-server.log`. Binary is a renamed copy of current-HEAD `chan` so a
  scoped `pkill` can't hit it. This serves the **current bundle** - use it to
  reproduce the bug, not to smoke your own fix.
- **Smoking YOUR fix:** rust-embed bakes the bundle at build time, so a frontend
  change needs its own rebuild+serve. Copy the seed
  (`cp -r /tmp/chan-test-r2 /tmp/chan-test-<lane>`), build your own renamed
  binary, serve on **your own port**, and **scope every `pkill` to your own
  drive path / port** - never `pkill chan serve` broadly (it kills the shared
  baseline and peers). Tear your own server + drive down at the end.
- Backend-only fixes (e.g. parts of IDX) can be smoked by rebuilding your own
  renamed binary against a copy of the seed; do not rebuild `/tmp/r2srv` in
  place (that disrupts the shared baseline).
- **HEAVY SEED WEDGES PREFLIGHT (confirmed 2026-05-31).** The shallow-repo-clone
  seed (~4096 embed chunks) triggers the CK-INDEX-IDLE wedge: the embedding phase
  never reaches `Idle`, so the preflight gate never unlocks and the whole UI is
  blocked server-side. Therefore: the heavy clone is **only** for @@LaneC's IDX
  repro; **all other lanes smoke on EMPTY or SMALL drives** (a few nested dirs
  for BUG-GRAPH / A4 directory inspectors; nothing special for A3/A7/SUBMIT/
  RELOAD/LINKS), which index fast and unlock preflight. The busy-repo
  reload-HANG re-verify (@@LaneD) is deferred to post-CK-INDEX-IDLE. IDX is on
  the critical path for every empirical smoke that needs the heavy drive.
