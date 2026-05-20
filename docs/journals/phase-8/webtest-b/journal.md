# @@WebtestB's phase-8 journal

Author: @@WebtestB
Date: 2026-05-19

Authoritative walkthrough lane B. Same profile as @@WebtestA;
operates in parallel for coverage breadth.

Append-only. New entries go at the bottom under a dated heading.

## 2026-05-19 — bootstrap

Fresh phase-8 session. Read:

* Contact card + webdev skill guide.
* phase-8 process (deltas from phase-7) and inherited phase-7
  process for the event protocol.
* `request.md`, `phase-8-bugs.md`.
* `webtest-b-1.md` (baseline walkthrough task — my coverage
  slice: native window-config persistence, terminal cluster,
  watcher dialog cluster, indexing-chart pan/zoom, CLI
  scriptability).

No incoming events from @@Architect or @@Alex to me yet. Working
tree dirty with carry-over phase-7 edits + the new phase-8
scaffolding — not mine to touch.

Filed permission request at
[`../alex/event-webtest-b-alex.md`](../alex/event-webtest-b-alex.md)
for standing approval covering terminal exec (cargo build,
chan serve lifecycle on a lane-B drive distinct from @@WebtestA's)
and Chrome MCP sessions for the Round-1 bug walkthrough.

Holding here until written approval lands.

## 2026-05-19 — lane-B walkthrough pass 1

Standing approval granted (transcribed by @@Architect). Built
chan from current `main` (HEAD `97b82df`), seeded
`/tmp/chan-test-phase8-wb` with the chan repo, served on
`127.0.0.1:8820`. Lane-A on `8787` left alone.

Walked every bug in the lane-B slice. Full per-bug audit
appended to
[`webtest-b-1.md`](webtest-b-1.md). Top-level results:

* CLI `--json` + `--name` (systacean-1): fix verified.
* chan-desktop window-config LRU (fullstack-b-1): code-level
  verified (17 tests pass); runtime walkthrough blocked on
  Tauri-launch permission — poked @@Architect.
* Cmd+T / scrollback / line-height (fullstack-b-2 cluster):
  source-side fixes already staged; empirical browser checks
  pass — 5000-line scrollback survives theme + Cmd+K
  roundtrips; lineHeight 1.2 reading well.
* Watcher dir picker / create-dir / overwrite warning: three
  symptoms reproduced cleanly.
* Indexing chart pan/zoom: confirmed missing on the carousel
  slide.
* Rich-prompt watcher first-try hang: NOT REPRODUCED.

Server kept up for per-fix verifications later in the round.
Poke filed at
[`../alex/event-webtest-b-architect.md`](../alex/event-webtest-b-architect.md).

## 2026-05-20 — Wave-1 fix verifications

Resumed lane B for per-fix verifications on the three wave-1
commits in my slice: `fullstack-b-2`, `fullstack-b-3`,
`fullstack-b-4`. Lane-B binary rebuilt (`npm run build` →
`cargo build -p chan`) against HEAD at the time
(`041de34`); the staged `systacean-2` graph.rs change was in
the working tree at build time so it's effectively included
(committed post-build as `4a04917`). Lane-B server restarted
on `127.0.0.1:8820` against `/tmp/chan-test-phase8-wb`.

Full per-fix verdicts appended to
[`webtest-b-1.md`](webtest-b-1.md) under the `2026-05-20 —
Wave-1 fix verifications` heading. Headline results:

* `fullstack-b-2` (terminal cluster) — **fix verified**.
  `Cmd+Alt+T` spawns, 5000-line scrollback retained through
  `Cmd+.` Hybrid-NAV round-trip, lineHeight 1.2 visible.
* `fullstack-b-3` (watcher dialog) — **partial**. Backend
  fix works (any path accepted, missing dirs silently
  created). Frontend "overwrites existing directory <name>/"
  warning still appears on existing in-drive dirs because
  `TerminalRichPrompt.svelte:197` passes `mode: "move"`
  instead of the new `mode: "attach"` that the same commit
  added. One-line fix at the call site.
* `fullstack-b-4` (indexing chart pan/zoom) — **fix
  verified**. Wheel-zoom + drag-pan + Locate recenter all
  work, parity with Graph view confirmed by the transform
  formula matching.
* Side observation: `chan_server::event_watcher` warns
  "failed to read event file <path>: Is a directory (os
  error 21)" when watching a freshly-created empty dir; red
  toast appears in UI on the first attach. Out of
  `fullstack-b-3`'s scope; flagged for @@Architect to
  decide whether to file in `phase-8-bugs.md`.

Server stays up; will reuse for the next wave's
verifications. Poke filed at
[`../alex/event-webtest-b-architect.md`](../alex/event-webtest-b-architect.md).

## 2026-05-20 — Wave-3 fix verifications

@@Architect routed the wave-3 verification queue via
`event-architect-webtest-b.md`. Rebuilt lane-B binary
against HEAD `0c076f0` (`npm run build` + `cargo build -p
chan`), restarted serve on `127.0.0.1:8820`. Replaced the
pass-1 fixture dirs with fresh ones
(`/tmp/chan-watch-wave3-outside/` outside the drive,
`/tmp/chan-test-phase8-wb/newdir-wave3-wb/` in-drive) so
`systacean-5` gets exercised on a guaranteed-fresh empty
dir.

Per-fix verdicts appended to
[`webtest-b-1.md`](webtest-b-1.md) under the `2026-05-20 —
Wave-3 fix verifications` heading. Headlines:

* `fullstack-b-10` (watcher dialog mode flip) — **fix
  verified**. New "attach watcher to X/" copy renders for
  all three cases (existing in-drive, missing in-drive,
  outside-drive absolute). The misleading "overwrites
  existing directory" warning from pass 1 is gone.
* `systacean-5` (event_watcher EISDIR skip) — **fix
  verified server-side; new ENOENT surface for outside-
  drive case**. Server log emits zero "Is a directory (os
  error 21)" WARN across the three watcher attaches. In-
  drive new-dir case (b) is clean end-to-end. Outside-drive
  case (a) still shows a red toast, but root cause is
  independent: `web/src/components/TerminalTab.svelte:721`
  `refreshWatcherEvents` calls drive-sandboxed `api.list`,
  which ENOENTs on absolute outside-drive paths. New
  finding; out of `systacean-5`'s scope.
* `fullstack-b-9` (Hybrid NAV `t` alias) — **fix
  verified**. `Cmd+. → t` spawns Terminal-N via the new
  mnemonic alias. Native verification still parked per the
  Tauri permission gap.
* `fullstack-b-8` (Cmd+Enter open-race blur) — **fix
  verified**. Pre-fix race: type immediately after
  `Alt+Space` and the first chars leak to the PTY. Post-fix:
  the `MARKERX` test string lands entirely in the rich
  prompt body; Terminal-2's command line stays at `$ echo
  before-prompt`. xterm-helper-textarea blur on prompt-open
  closes the leak window.
* Bug 14 (watcher first-try hang) — **CNR persists**.
  Walked the most-faithful repro (fresh session, fresh
  terminal, fresh rich prompt, watch dialog, type, OK) on
  the wave-3 binary. Clean attach, no hang, under 1 s end
  to end. Recommend striking from Round-1 as CNR.

Server stays up; next-wave verifications will reuse the same
drive. Poke filed at
[`../alex/event-webtest-b-architect.md`](../alex/event-webtest-b-architect.md).

## 2026-05-20 — `fullstack-a-20` verification

`fullstack-a-20` landed as `f1d0dcf`; per @@Architect's
earlier instruction ("verify a-20 once it lands"), rebuilt
lane-B against the new HEAD and walked the wysiwyg-mode
double-dispatch repro.

Verdict appended to
[`webtest-b-1.md`](webtest-b-1.md) under the `2026-05-20 —
fullstack-a-20 verification` heading. Headline:
**`fullstack-a-20` fix VERIFIED**. Wysiwyg-mode `Cmd+Enter`
dispatches `pwd` exactly once to the terminal (pre-fix would
have shown `pwdpwd`). Rich-prompt buffer retained per
`fullstack-a-4`.

Wave-3 set is now fully cleared from my lane. Verification
queue empty pending next wave / Round-1 close. Poke filed
at
[`../alex/event-webtest-b-architect.md`](../alex/event-webtest-b-architect.md).

## 2026-05-20 — `systacean-7` proactive CLI walk

`systacean-7` (`6bf44cd`) landed after the wave-3
verification queue. CLI scriptability sits in lane-B's
standing coverage (per the original task split), so walked
the new subcommand surface proactively without explicit
routing rather than leave it stranded.

Verdict appended to
[`webtest-b-1.md`](webtest-b-1.md) under the `2026-05-20 —
systacean-7 proactive CLI walk` heading. Headline:
**functionally verified** — all five subcommands
(`rebuild`, `download-model`, `enable-semantic`,
`disable-semantic`, `status`) work end-to-end against the
default drive; JSON shape stable + machine-parseable;
toggle round-trips. Three ergonomic findings filed as side
observations for @@Architect to triage:

* `status` errors with "drive is locked by another process"
  when `chan serve` is running on the queried drive
  (read-only query shouldn't take an exclusive lock).
* `status --path <nonexistent>` tries to register the path
  instead of refusing with "not a chan drive".
* Argument shape asymmetry: `rebuild <PATH>` (positional)
  vs the new subcommands (`--path <PATH>` flag); hurts
  scripting consistency.

Lane-B verification queue empty again. Poke filed at
[`../alex/event-webtest-b-architect.md`](../alex/event-webtest-b-architect.md).

## 2026-05-20 — wave-4: `systacean-8` + `systacean-9` + `fullstack-b-1`

@@Architect's wave-4 routing cleared three items:

* `systacean-8` (CLI ergonomics) — fix VERIFIED, all
  three sub-fixes (lock-free read-only `status`, refuse
  non-existent path without auto-register, `--path`
  synonym on `rebuild`).
* `systacean-9` (outside-drive watcher events endpoint)
  — fix VERIFIED. Re-attached watcher to fresh outside-
  drive fixture; no red toast, no event_watcher WARN.
  `echo poke` trigger refresh path also clean.
* `fullstack-b-1` (chan-desktop window-config LRU) —
  **code-level VERIFIED; empirical click cycle PENDING**.
  Tauri-launch permission extension used to build and
  launch chan-desktop dev; launcher window came up
  cleanly but I cannot drive the "click drive → close
  window → reopen" cycle without macOS Accessibility
  permission for System Events. Chrome MCP doesn't reach
  Tauri's WKWebView, and chan-desktop has no CLI arg /
  open-handler for drive paths. Cargo tests (19/19 pass,
  six covering LRU + namespace) and the source review
  match the spec exactly. Filed an unblock note: either
  grant Accessibility to Claude Code's parent process,
  @@Alex does the manual click, or chan-desktop grows a
  drive-path CLI arg for automation lanes.

Full audit at
[`webtest-b-1.md`](webtest-b-1.md) under the `2026-05-20
— systacean-8 + systacean-9 verifications` and
`2026-05-20 — fullstack-b-1 runtime walkthrough —
partial` headings.

Lane-B server stays up on `127.0.0.1:8820`. chan-desktop
process killed cleanly; config backup restored. Poke
fired at
[`../alex/event-webtest-b-architect.md`](../alex/event-webtest-b-architect.md).

## 2026-05-20 — Round-1 teardown complete

@@Architect routed the 7-step teardown checklist via
`event-architect-webtest-b.md` ("Round-1 teardown
checklist before recycle"). Walked all steps:

1. Stopped lane-B serve on `127.0.0.1:8820` (PID 85616
   killed, gone).
2. `rm -rf /tmp/chan-test-phase8-wb` — directory removed
   (included `newdir-wave3-wb/` and any in-drive
   artifacts).
3. `chan remove /private/tmp/chan-test-phase8-wb` →
   `unregistered: …`. First attempt used the `/tmp/…`
   alias and got `(not registered)` because the
   registry stores the canonical `/private/tmp/…` path
   on macOS; retried with the canonical form.
   `chan list` confirms no `phase8-wb` entry remains.
4. Outside-drive fixtures: `/tmp/chan-watch-wb-outside`
   and `/tmp/chan-watch-wave3-outside` were already
   gone from prior cleanup; `/tmp/chan-watch-wave4-outside`
   removed (held the systacean-9 probe event file).
5. No chan-desktop processes were running (already torn
   down post the `fullstack-b-1` walkthrough).
6. No MCP tab groups present (`tabs_context_mcp` reports
   none) — lane-B Chrome tab was already closed.
7. chan-desktop config at
   `~/Library/Application Support/Chan Desktop/config.json`
   already restored to its pre-walk shape
   (`sidecar + tunnel` keys, no `window_configs` field);
   `.webtest-b-backup` sibling was removed at the end of
   the `fullstack-b-1` walkthrough.

Lane-B footprint is now clean. Fresh Round-2 session can
boot into a clean state. Final teardown poke filed at
[`../alex/event-webtest-b-architect.md`](../alex/event-webtest-b-architect.md).

