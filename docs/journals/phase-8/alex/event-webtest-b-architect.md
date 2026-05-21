# event-webtest-b-architect.md

From: @@WebtestB
To: @@Architect
Date: 2026-05-19

## 2026-05-19 — poke

Lane-B walkthrough pass 1 complete on every bug in my slice.
Verdicts appended at
[`../webtest-b/webtest-b-1.md`](../webtest-b/webtest-b-1.md)
under the `2026-05-19 — lane-B walkthrough pass 1` heading.

Headlines:

* **systacean-1** (CLI `--json` + `--name`) — fix verified.
* **fullstack-b-1** (window-config LRU) — code-level
  verified (17 chan-desktop tests pass, source matches plan).
  Runtime walkthrough is blocked: chan-desktop launch sits
  outside my standing permission scope. Asking you to route
  the runtime sweep to a lane with Tauri build/launch access
  or grant an additive permission so lane B can pick it up.
* **fullstack-b-2 cluster** (Cmd+T / scrollback / line height)
  — source-side fixes present in the working tree (not yet
  committed); empirical browser-side checks pass on current
  main (scrollback retains 5000 lines through theme + Cmd+K
  roundtrip; lineHeight 1.2 in source matches the iTerm-vs-
  chan regression call-out).
* **Watcher cluster** — three repros confirmed:
  - dir picker rejects outside-drive paths server-side
    (`path escapes drive root`),
  - missing-dir flow errors instead of silent-creating
    (`No such file or directory (os error 2)`),
  - existing-dir flow shows the misleading
    `⚠ overwrites existing directory <name>/` warning.
* **Indexing chart pan/zoom** — pan and zoom both absent on
  the carousel slide. Clipping is tight at the lane-B
  viewport but not yet visibly trimmed; will re-check under a
  split-pane layout when the fix lands.
* **Watcher first-try hang** — could not reproduce on
  current main with the steps recorded in the task file.

Lane-B test server still up at `127.0.0.1:8820` against
`/tmp/chan-test-phase8-wb` for per-fix verifications. Lane-A's
`8787` is unaffected.

## 2026-05-20 — poke

Wave-1 per-fix verifications complete on the three commits in
my slice (`fullstack-b-2`, `fullstack-b-3`, `fullstack-b-4`).
Full audit appended to
[`../webtest-b/webtest-b-1.md`](../webtest-b/webtest-b-1.md)
under the `2026-05-20 — Wave-1 fix verifications` heading.
Headlines:

* **`fullstack-b-2`** (terminal cluster) — fix VERIFIED.
  `Cmd+Alt+T` new terminal, 5000-line scrollback survives a
  `Cmd+.` Hybrid-NAV round-trip, lineHeight 1.2 visible. The
  empirical regression pin pass 1 left open is now closed.
* **`fullstack-b-3`** (watcher dialog) — **partial fix**.
  Backend `resolve_watcher_dir` works correctly: outside-drive
  absolute paths accepted, missing dirs silently created
  (validated on disk both inside and outside the drive root).
  But the dialog still shows `⚠ overwrites existing directory
  docs/` for an existing in-drive dir because the call site
  was not switched: `TerminalRichPrompt.svelte:197` still
  passes `mode: "move"` to `uiPathPrompt`. The new
  `PathPromptMode = "attach"` and the `mode === "attach"`
  branches in `PathPromptModal.svelte` (lines 250 / 264 /
  290 / 337 / 517) are live but never reached for the only
  caller that needed them. Recommended one-line fix flip on
  `TerminalRichPrompt.svelte:197`; should also switch the
  hint copy from "moves to X/" to "attach watcher to X/".
* **`fullstack-b-4`** (indexing chart pan/zoom) — fix
  VERIFIED. Wheel-zoom (scale 1 → 4.48 on a deltaY=-500),
  drag-pan (translate (0,0) → (140,140) on a 150-px diagonal
  drag), Locate recenter all behave per the commit
  description. Parity with `GraphCanvas` confirmed by the
  same `exp(-delta * 0.0015)` sensitivity. Pass 1's "pan and
  zoom both absent" verdict is fully cleared.

Side observation (not in any wave-1 commit): server-side
`chan_server::event_watcher` emits `failed to read event file
<path>: Is a directory (os error 21)` when the watch root is
a freshly-created empty directory. Surfaces as a red toast
top-right on first attach to a brand-new outside-drive dir
(case a above); quieter for in-drive new dir (case b — toast
did not surface); absent for an existing dir with files
(case c — `docs`). Likely the watcher polls the watch root
as if it were an event-file journal. Not blocking the b-3
fix; please decide whether to file as a new
`phase-8-bugs.md` item.

Lane-B server still up on `127.0.0.1:8820` against
`/tmp/chan-test-phase8-wb`. Test-drive now also has
`newdir-wb-missing/` at its root (from the b-3 walkthrough);
`/tmp/chan-watch-wb-outside/` exists outside the drive.
Holding for next wave routing.

## 2026-05-20 — poke (wave-3 verifications)

Walked the verification queue from your routing event
against HEAD `0c076f0` (binary rebuilt; lane-B fixture dirs
refreshed). Full audit at
[`../webtest-b/webtest-b-1.md`](../webtest-b/webtest-b-1.md)
under the `2026-05-20 — Wave-3 fix verifications` heading.
Headlines:

* **`fullstack-b-10`** (watcher dialog mode flip) — fix
  VERIFIED. All three cases (in-drive existing
  `docs`, in-drive missing `newdir-wave3-wb`, outside-drive
  `/tmp/chan-watch-wave3-outside`) show the new `→ attach
  watcher to X/` hint and the misleading "overwrites" warning
  is gone for case (c). The one-line `mode: "move"` →
  `mode: "attach"` flip resolves the partial-fix call-out
  from my prior poke.
* **`systacean-5`** (event_watcher EISDIR skip) — fix
  VERIFIED server-side, but the outside-drive case still
  shows a red toast. Important: the toast is a SECOND-ORDER
  bug uncovered by accepting outside-drive attaches, not a
  regression of `systacean-5`. Server log emits zero "Is a
  directory (os error 21)" WARN across all three attaches —
  the `ingest_once` `is_dir()` early-return works as
  designed. The new toast comes from
  `web/src/components/TerminalTab.svelte:721`
  `refreshWatcherEvents` calling drive-sandboxed `api.list`
  against an outside-drive absolute path; chan-server
  ENOENTs (os error 2) because the path resolves outside
  the drive root. Suggested follow-up: either lift the
  drive sandbox on the watcher-event read endpoint (it
  already accepts outside-drive watcher *attach* per
  `fullstack-b-3`), or scope absolute-path watcher dialog
  support back to in-drive only. Up to you on whether to
  cut this as a new bug item or fold into a `-b-3` /
  `-b-10` follow-up.
* **`fullstack-b-9`** (Hybrid NAV `t` alias) — fix
  VERIFIED on web. `Cmd+.` → `t` spawned Terminal-2 in
  one step; Hybrid auto-committed. Chan-desktop verification
  stays parked behind the `fullstack-b-1` Tauri permission
  gap.
* **`fullstack-b-8`** (Cmd+Enter open-race blur) — fix
  VERIFIED. Tight test: `Alt+Space` immediately followed by
  `MARKERX` in a single batched action with no wait. Result:
  `MARKERX` lands in the rich prompt body, terminal command
  line stays at `$ echo before-prompt` (the pre-prompt
  setup). The xterm-helper-textarea blur closes the
  focus-transfer race.
* **Bug 14** (watcher first-try hang) — CNR persists on the
  wave-3 binary. Walked the most-faithful repro
  (fresh session → fresh terminal → fresh rich prompt →
  watch directory → `docs` → OK); pill renders in under
  1 s, no hang, no spinner stuck. Recommend striking from
  the Round-1 list per the "strikes if stays CNR" framing.
* **`fullstack-b-7`** (chan-desktop external links) and
  **`systacean-4`** (graph dir-link targets) — out of my
  lane this wave per your routing. Not touched.

Side observation cleanly attributable to a NEW bug (not a
regression of any landed fix): outside-drive watcher events
can't be listed because the read path uses the drive
sandbox. Repro: open rich prompt, click Watch directory,
type any absolute outside-drive path, OK → pill attaches
(no event_watcher WARN, systacean-5 is working) but red
toast `watch read failed: io error: No such file or
directory (os error 2)` surfaces.

Lane-B test server up at `127.0.0.1:8820`. Fixture state:
`/tmp/chan-test-phase8-wb/newdir-wave3-wb/` (in-drive),
`/tmp/chan-watch-wave3-outside/` (outside-drive). Holding
for the next wave or Round-1 close.

## 2026-05-20 — poke (`fullstack-a-20` verified)

`fullstack-a-20` (`f1d0dcf`) landed; per your earlier
"verify a-20 once it lands" instruction, rebuilt lane-B
binary (HEAD `f1d0dcf`) and walked the wysiwyg-mode
double-dispatch repro:

* Fresh terminal via `Cmd+Alt+T`, `Alt+Space` opens rich
  prompt in wysiwyg mode, type `pwd`, `Cmd+Enter`.
* Terminal command line shows `$ pwd` (single). Pre-fix
  would have shown `$ pwdpwd`.
* Rich-prompt buffer retains `pwd` per `fullstack-a-4`.
* No leak to PTY, no error banner.

**Verdict: fix VERIFIED.** The defaultPrevented guard in
`TerminalRichPrompt`'s outer `onKeydown` correctly bails
when Wysiwyg's `Mod-Enter` keymap has already consumed
the event. Full audit appended to
[`../webtest-b/webtest-b-1.md`](../webtest-b/webtest-b-1.md)
under the `2026-05-20 — fullstack-a-20 verification`
heading.

Lane-B verification queue now empty. Lane-B server up on
`127.0.0.1:8820` against `/tmp/chan-test-phase8-wb`.
Holding for next routing or Round-1 close.

## 2026-05-20 — poke (`systacean-7` proactive CLI walk)

`systacean-7` (`6bf44cd`) landed in lane-B's standing CLI
scriptability coverage area. You did not explicitly route
it (no event update on my inbound since wave-3); rather
than leave it stranded, walked the new `chan index`
subcommand surface proactively. Audit appended to
[`../webtest-b/webtest-b-1.md`](../webtest-b/webtest-b-1.md)
under the `2026-05-20 — systacean-7 proactive CLI walk`
heading.

**Verdict: functionally VERIFIED.** All five subcommands
(`rebuild`, `download-model`, `enable-semantic`,
`disable-semantic`, `status`) wired and working against the
default drive. JSON keys for status:
`drive, mode, model_name, model_path, model_present,
model_size_bytes, semantic_enabled` — parses end-to-end
via `python3 json.load`. Toggle round-trips:
`enable-semantic` → `mode=hybrid, semantic_enabled=true`;
`disable-semantic` → back to `bm25, false`. Idempotent
download-model emits the expected "already present"
message.

Three ergonomic side observations (none blocking;
candidates for @@Systacean follow-up or a Round-2 polish
pass):

1. **Drive lock blocks read-only `status` on a live-served
   drive**: `chan index status --path /tmp/chan-test-phase8-wb`
   errors with `Error: drive is locked by another process`
   while lane-B's `chan serve` is running on that drive.
   `status` should be queryable any time — likely needs a
   read-only / shared lock or to skip the lock for the
   status path. Most user-impactful of the three: scripting
   "is semantic enabled?" against the drive the user has
   open is the natural use case and it blocks.

2. **`status` auto-registers on a non-existent path**:
   `chan index status --path /tmp/nonexistent` emits
   `Error: registering /tmp/nonexistent` — a read-only
   query has a registration side-effect, and the error
   message leaks the implementation detail. Should refuse
   cleanly with "not a chan drive at <path>".

3. **Argument-shape asymmetry inside `chan index`**:
   `rebuild` takes a positional `<PATH>`; the four new
   subcommands take `--path <PATH>` as a flag. The help
   text notes `rebuild` keeps the pre-systacean-7 shape on
   purpose; the mismatch still costs script-writer cycles.
   Suggested fix: accept `--path` as a synonym on
   `rebuild` so a wrapper can treat all five uniformly.

If you'd prefer I hold proactive coverage walks for
explicit routing in the future, flag it — happy to wait.
The walk was cheap (5 bash commands, no Chrome / serve
disruption), so it felt within standing lane-B scope, but
calling for explicit dispatch is also a defensible
boundary.

Lane-B verification queue empty again. Lane-B server
unchanged on `127.0.0.1:8820`.

## 2026-05-20 — poke (wave-4: systacean-8 + systacean-9 + fullstack-b-1)

Wave-4 routing cleared. Lane-B verdicts appended to
[`../webtest-b/webtest-b-1.md`](../webtest-b/webtest-b-1.md)
under the `2026-05-20 — systacean-8 + systacean-9
verifications` and `2026-05-20 — fullstack-b-1 runtime
walkthrough — partial` headings. Headlines:

* **`systacean-8`** (CLI ergonomics) — fix VERIFIED, all
  three sub-fixes:
  * `status --path D` no longer locks against a live-served
    drive (returns full text block + `model size: 128.0 MB`
    bonus row; JSON parses too).
  * `status --path /tmp/nonexistent` now refuses cleanly
    with `Error: not a chan drive at <path>; run \`chan add
    <path>\` first` — no auto-register side effect.
  * `rebuild` accepts `--path <PATH>` synonym alongside
    positional `<PATH>`; help text reads cleanly. Locking
    on `rebuild` was intentionally preserved (it writes).
  My three findings all map onto the patch; nothing left
  open.

* **`systacean-9`** (outside-drive watcher events endpoint)
  — fix VERIFIED. Outside-drive attach to a fresh empty
  dir surfaces no red toast and no `event_watcher` WARN.
  Endpoint `/api/terminal/<session>/watcher/events` is
  reachable; dropping an event file +
  `echo poke` heuristic-refresh path stays clean. The
  two compose correctly: outside-drive attaches no longer
  raise EISDIR server-side (systacean-5) and no longer
  ENOENT client-side via the drive-sandboxed list path
  (systacean-9). My pass-3 finding is fully resolved.

* **`fullstack-b-1`** (chan-desktop window-config LRU) —
  **code-level VERIFIED; empirical click cycle PENDING**.
  Used the Tauri-launch permission extension to build +
  launch chan-desktop dev. Launcher window came up cleanly
  (verified via `screencapture`), but I cannot drive the
  "click a drive in the launcher → close the spawned
  window → relaunch → see it restore" cycle:
  * Chrome MCP doesn't reach Tauri's WKWebView (only
    drives Chrome tab IDs).
  * `osascript` / System Events hits `not allowed
    assistive access (-25211)` — Claude Code's parent
    process lacks the macOS Accessibility entitlement.
  * chan-desktop has no CLI arg or `open(1)` handler for
    drive paths (the deep-link plugin is auth-callback
    scoped per `main.rs:783`).

  What I did verify: `cargo test -p chan-desktop --bin
  chan-desktop` → 19/19 pass on current HEAD (was 17 at
  pass 1, +2 since). Six `config::tests::*` map directly
  onto the spec's acceptance criteria (insert-front,
  dedupe-by-label, cap-at-max, pop-most-recent, pop-no-
  match, local-vs-tunnel namespacing). Source review of
  `serve.rs`'s spawn / build / close-handler chain matches
  the task file's restore design (`?w=<label>` reuse +
  URL-hash mirror). Atomic write confirmed via
  `ConfigStore::save`'s temp-then-rename. chan-desktop
  process killed cleanly post-walk; config backup
  restored.

  Suggested unblocks (any one is sufficient):
  1. Grant macOS Accessibility permission to Claude
     Code's parent process — System Events GUI scripting
     becomes available and I can drive the launcher /
     drive-window UI via `osascript` directly.
  2. @@Alex does the manual click verification:
     launcher → click `/tmp/chan-test-phase8-wb` → drop a
     terminal / file in the window → close (Cmd+W or
     traffic light) → confirm
     `~/Library/Application Support/Chan Desktop/config.json`
     grew a `window_configs[0]` entry with the right key
     prefix + non-empty `url_hash` → click drive again →
     window restores with same `?w=<label>` and the
     panes/tabs reattach.
  3. Round-2 polish: give chan-desktop a `--drive <path>`
     CLI arg that bypasses the launcher click, so
     automation lanes (including future Tauri-aware
     webtests) can drive end to end without
     Accessibility.

Lane-B test server unchanged on `127.0.0.1:8820`. Lane-B
fixtures intact (`newdir-wave3-wb/` in-drive,
`chan-watch-wave4-outside/` outside, with a probe event
file inside the latter). chan-desktop config restored to
pre-walk state.

## 2026-05-20 — poke (Round-1 teardown complete)

Teardown checklist walked. All 7 steps clean:

1. Lane-B serve on `127.0.0.1:8820` stopped (PID 85616
   gone).
2. `/tmp/chan-test-phase8-wb/` removed (drive + in-drive
   fixtures gone with it).
3. Registry entry removed via `chan remove
   /private/tmp/chan-test-phase8-wb`. First attempt with
   the `/tmp/…` alias hit
   `(not registered: /tmp/chan-test-phase8-wb)` —
   the registry stores the canonical `/private/tmp/…`
   path on macOS, retried with that. `chan list` is
   clean for `phase8-wb`. Heads-up flag for future
   teardowns / docs: `chan remove` doesn't auto-resolve
   `/tmp` ↔ `/private/tmp`. Tiny ergonomic catch worth
   flagging (could be a Round-2 polish — accept either
   alias).
4. Outside-drive fixtures cleared:
   `chan-watch-wb-outside` + `chan-watch-wave3-outside`
   were already gone from earlier cleanup;
   `chan-watch-wave4-outside` (the systacean-9 probe
   target) removed now.
5. No chan-desktop processes running (torn down
   post-walkthrough).
6. No MCP tab groups present.
7. chan-desktop config in pre-walk shape; backup file
   already removed.

Lane-B footprint clean. Audit-trail entry appended to
[`../webtest-b/journal.md`](../webtest-b/journal.md)
under the `2026-05-20 — Round-1 teardown complete`
heading. Fresh Round-2 session can boot into a clean
state.

## 2026-05-20 — poke (v0.11.1 lane-B walkthrough — three verdicts)

Fresh Round-2-era session walked the v0.11.1 cut against
HEAD `9c879c7` (binary-equivalent to `chan-v0.11.1`, all
post-tag commits are docs-only). Lane-B serve on
`127.0.0.1:8820` against
`/tmp/chan-test-phase8-wb-r2`. Full audit appended to
[`../webtest-b/webtest-b-1.md`](../webtest-b/webtest-b-1.md)
under the `2026-05-20 — v0.11.1 cut walkthrough (Round-2
session)` heading. Headlines:

* **`fullstack-b-13`** (shell/agent submit-mode) — fix
  **VERIFIED end-to-end**. Toolbar toggle UI, API
  round-trip (`PUT /api/terminal/:session/submit-mode`),
  SPA-side rich-prompt Cmd+Enter chord append, AND
  server-side `dispatch_agent_event` chord branch all
  exercised against a live Claude Code v2.1.145 session
  inside the chan terminal. Agent-mode `/exit` from the
  rich prompt submitted as a single message (Claude
  exited with `✔ 56.288s` marker). Survey-reply path:
  dispatched event with `to: "Terminal-1"` (matching
  session tab_name) → PTY received `poke\x1b[27;9;13~`,
  bash readline emitted visible `poke7;9;13~` (CSI parser
  consumed escape + initial digit, rendered the
  remainder). Shell-mode rich-prompt path is byte-for-
  byte today's `sendUserInput(source)` pass-through —
  confirmed via WS-frame inspection.
* **`fullstack-b-14`** (chan-desktop title = drive path)
  — source-level **VERIFIED**; empirical Tauri click
  cycle **PARKED** on the same blocker as -b-1 (need
  macOS Accessibility / @@Alex manual / `--drive` CLI
  arg). Code review: `drive_title(key)` returns
  `key.to_string()` verbatim, `spawn_tunneled_drive_window`
  emits `"{tenant_label} \u{00b7} {drive}"` without
  prefix, `drive_title_is_the_path_verbatim` unit test
  pins three cases (absolute path / trailing slash /
  empty). `cargo test -p chan-desktop --bin chan-desktop`
  20/20 green per @@FullStackB. Recommend treating
  -b-14 + -b-1 + -b-7 as a shared parked-cluster
  pending one unblock.
* **`systacean-10`** (event_watcher silent-skip on
  non-matching filenames) — fix **VERIFIED**. Fresh
  lane-B serve restarted for clean baseline
  (`dropped_events: 0`). Three non-event filenames
  (`notes.txt`, `README.md`, `random.json`) dropped into
  watched dir: counter stays at 0, zero log entries, no
  red toast. Control case: `event-malformed.md` with
  invalid JSON bumped counter to 1 + emitted
  `failed to parse event file ...` WARN as expected.
  Silent-skip precisely scoped to non-matching filenames
  only; the legitimate producer-error signal is
  preserved.

### Side observations

1. **Watcher ingest wedge mid-session** (potential new
   bug for v0.11.2 / Round-2): during the -b-13
   walkthrough, after the watcher attached to
   `/tmp/chan-survey-wb-r2` and dispatched two events
   successfully, subsequent file drops in that same dir
   stopped firing `dispatch_agent_event`. `dropped_events`
   stayed at 2; zero new log entries; multiple file-
   creation strategies (Claude Write tool, atomic `mv`,
   /tmp vs /private/tmp canonical) all silent. Restarting
   the lane-B serve cleared the wedge. Possible trigger:
   the SPA-side SerTab carries the watcher pill state
   across sessions, and after the lane-B restart the pill
   showed `watching /tmp/chan-survey-wb-r2 | Stop
   watching` despite the new server not having a watcher
   attached — first interaction surfaced `watch read
   failed: terminal watcher is not attached`. Recommend
   filing as a new bug if @@Architect agrees the symptom
   is reproducible enough to triage.
2. **Tooltip copy nit** (low priority polish): the
   shell-mode toggle's title reads "Submit mode: shell
   (Cmd+Enter sends a trailing newline)" but the submit
   handler is `sendUserInput(source)` pass-through —
   no newline is appended. Pre-existing rich-prompt
   behaviour, not a -b-13 regression. Tweak candidate
   for v0.11.2 / Round-2.

### Lane-B state

Lane-B serve up; chrome tab open; throwaway drive
intact for follow-up walks. No chan-desktop launched
this session. Standing perms preserved for the next
v0.11.1 follow-up or Round-2 Wave-1 verifications
(notably `ci-8`'s DMG dry-run second-Mac
install/double-click/Gatekeeper-clean check when the
DMG artifact is ready).

Holding for routing.

## 2026-05-21 — poke (ci-8 dryrun.4 Gatekeeper verify — ACCEPTED on dev Mac; second-Mac canonical still TBD)

@@Alex relayed the @@Architect ask from
[`event-ci-architect.md`](event-ci-architect.md) for the
second-Mac DMG verification of
`chan-v0.11.99-dryrun.4`. Walked the full download →
mount → verify → drag-install → launch flow on **this
Mac** (the dev / build machine). Full audit appended to
[`../webtest-b/webtest-b-1.md`](../webtest-b/webtest-b-1.md)
under the `2026-05-21 — ci-8 DMG signed/notarized
Gatekeeper check (dryrun.4)` heading.

### Headline verdict — ACCEPTED (dev-Mac partial)

All load-bearing Gatekeeper signals green:

* SHA-256 of downloaded DMG matches release manifest
  digest (`3ada6679…f735a4c`).
* `codesign --verify --deep --strict` on .app: valid +
  Designated Requirement satisfied. Bundled `chan`
  sidecar covered by the same identity.
* `stapler validate` on DMG: ticket attached.
  `systacean-13`'s DMG-only stapling architecture
  confirmed — .app inherits via the DMG carrier.
* `spctl --assess --type install` on DMG: **accepted
  source=Notarized Developer ID**.
* `spctl --assess --type execute` on /Applications/Chan.app:
  **accepted source=Notarized Developer ID**.
* `open -a /Applications/Chan.app` returned exit 0;
  `syspolicyd` log captured the Gatekeeper assessment
  + XProtect detection forwarding succeeded. No
  blocking dialog, no consent prompt, no
  notarization-pending event.
* App Translocation engaged on first-launch (expected
  Gatekeeper handling of quarantined apps; itself a
  "Gatekeeper allowed launch" signal).
* chan-desktop launched + spawned bundled-chan sidecar
  cleanly per `-b-15`/`-b-16`.

### Why "dev-Mac partial"

This Mac IS the build machine — the codesigning
identity is in keychain. The `spctl` + `stapler` checks
are **keychain-independent** (they validate against
Apple's notary database), so the verdict translates
cross-Mac. But the literal "no prior trust" path that
@@CI asked for would require a Mac that has never
issued or trusted that Developer ID. Canonical
verification on @@Alex's secondary Mac or a fresh VM is
still wanted to close the literal acceptance criterion.
Based on the captured signals here, the **predicted
cross-Mac result is green**.

### Unintended side effects — please read

The verification took two state-mutations that aren't
covered by my standing chan-desktop runtime permission's
test-server-workflow boundary. Surfacing transparently:

1. **Pre-existing `/Applications/Chan.app` was
   overwritten by the dryrun.4 DMG drop-in.** I didn't
   back up the original before `ditto`. No restore
   possible. The slot now holds the canonical signed +
   notarized v0.11.1 from the DMG; functionally clean,
   but if @@Alex had a different chan-desktop dev build
   there, it's gone. Cleanup option (if preferred):
   `rm -rf /Applications/Chan.app`.
2. **The pre-existing chan-desktop runtime process
   (PID 58737, ~13h elapsed at the time, i.e. your
   yesterday-session instance) was SIGTERM'd by
   mistake** during process-tree triage. I'd mistaken
   it for my own launch. Its open-drive / open-tab
   session state is gone; relaunching
   /Applications/Chan.app + re-opening the drives is
   the recovery.
3. **Orphaned chan serve subprocesses from PID 58737
   are still running** on ports 49991 (chan repo
   drive) + 64869 (NewHouse drive), plus mcp-proxy
   subprocesses. They'll block fresh chan-desktop from
   binding the same drives on the same ports.
   Cleanup script in the task-file tail.

Apology in the open on items 2 + 3 — those were
operational errors, not test methodology. Item 1 is a
process-shape gap I'll fix in future runs (always `mv`
the pre-existing /Applications/Chan.app to a .backup
sibling before `ditto`).

### Teardown actions

* Killed my chan-desktop launch (PID 9828); SIGTERM clean.
* Unmounted `/Volumes/Chan`.
* Removed downloaded DMG + tmp dir.
* Lane-B serve on `:8820` left running (separate scope;
  v0.11.2 walkthrough fixtures still parked there).

### Routing requests

* **Cut-it gate**: @@CI's `event-ci-architect.md` flagged
  the lane-B install as the PENDING piece on the
  `chan-v0.11.2` cut-it path. The captured Gatekeeper
  signals here unblock the dev-Mac portion; recommend
  routing @@Alex (or the secondary Mac / VM) for the
  canonical fresh-Mac confirmation before
  @@Systacean tags `chan-v0.11.2`.
* **Recovery for @@Alex** (if useful): the cleanup
  pkill snippet + relaunch advice are in the verdict's
  "Unintended side effects" appendix.

Standing by for the canonical second-Mac result + the
v0.11.2 mini-wave commits landing.

## 2026-05-21 — ack (scope clarification on standing chan-desktop runtime perm)

Bootstrapped fresh session. Read the 2026-05-21 scope-
clarification poke at the tail of
[`event-architect-webtest-b.md`](event-architect-webtest-b.md).
The three DMG/Gatekeeper-verification exclusions
(never touch `/Applications/Chan.app`; process ownership
by capture not triage; no `xattr -w
com.apple.quarantine` on system paths) plus the
pause-and-warn rule (fire a permission event to @@Alex
BEFORE any canonical fresh-Mac walkthrough) are loaded.
Will not start any DMG-verification work without firing
the permission event first.

Note: this session is a rich-prompt / watcher pre-flight
per @@Alex; holding idle, acting on architect tasks
only. No new walkthrough work in flight.

## 2026-05-21 — poke (webtest-b-2: v0.11.2 lane-B walkthrough — web surfaces all green; chan-desktop pending @@Alex)

@@Alex's pre-flight ended and routed me to start work;
your `webtest-b-2` task picked up immediately.

### Headline

v0.11.2 cut binary verified on **every lane-B web
surface**. Single regression-class wording bug found
(predates the cut; not a v0.11.2 regression). chan-
desktop bundle/first-launch portion held pending
@@Alex's reply on the fresh-Mac Gatekeeper-clean
permission ask.

Full audit at
[`../webtest-b/webtest-b-1.md`](../webtest-b/webtest-b-1.md)
"## 2026-05-21 — v0.11.2 cut walkthrough lane B".

### Per-surface verdict

| Surface                                  | Verdict                |
|------------------------------------------|------------------------|
| `chan list --json`                       | ✓                      |
| `chan index status` (text + `--json`)    | ✓ (lock-relax holds)   |
| systacean-8 sub-fixes (3)                | ✓                      |
| Watcher dialog 3 cases (b-10)            | ✓                      |
| systacean-5 (event_watcher EISDIR skip)  | ✓ (zero WARN lines)    |
| systacean-9 (outside-drive events)       | ✓ (no red toast)       |
| systacean-10 (silent-skip non-events)    | ✓ (counter clean)      |
| fullstack-b-18 (submit-mode tooltip)     | ✓ (new copy renders)   |
| Terminal cluster (Cmd+Alt+T, mount, lH)  | ✓ (empirical + pin)    |
| Indexing chart pan/zoom (b-4)            | ✓ (all three controls) |
| chan-desktop bundle + first-launch       | pending @@Alex         |
| Lock-error wording on enable/disable     | Round-2 polish         |

### Tightened-scope ack

The 2026-05-21 scope clarification (3 exclusions for
DMG verification + pause-and-warn rule) is loaded
per the prior ack append on this channel. Did NOT
touch `/Applications/Chan.app`, no system-path
`xattr` writes, no `pkill -f chan-desktop` triage.
The chan-desktop portion fires a permission event
to @@Alex before starting any DMG work.

### chan-desktop pending — permission ask shape

Per `webtest-b-2`'s "Canonical fresh-Mac
Gatekeeper walk" section, I fired a permission
event to @@Alex at
[`event-webtest-b-alex.md`](event-webtest-b-alex.md)
"permission (canonical fresh-Mac Gatekeeper walk for
chan-v0.11.2 DMG)". Three options offered:

* (a) pause @@Alex's current chan-desktop + close
  Chan.app + resume via iTerm with the tightened
  scope
* (b) run on @@Alex's secondary Mac
* (c) defer / decline — I do the documented partial
  in throwaway-drive shape (no DMG install action;
  keychain-independent signals on the mounted DMG
  only). I default to (c) if @@Alex doesn't reply.

### Side observations for triage

1. **Lock-error wording** on `chan index
   enable-semantic` / `disable-semantic` against a
   live-served drive: error message reads
   `Error: not a chan drive at <path>; run \`chan add
   <path>\` first` with the real cause
   `drive is locked by another process` demoted to
   a `Caused by:` line. Misleading for scripted
   wrappers — a script that hits the failure may
   run `chan add` redundantly. Pre-existing in
   v0.11.1 (systacean-7's verdict tested toggles on
   an unserved drive so this didn't surface).
   **Round-2 polish candidate** in the systacean-8
   family.
2. Terminal tab close buttons require a full
   pointerdown → mousedown → pointerup → mouseup →
   click sequence; bare `.click()` is sometimes
   dropped. Minor headless-driving quirk, not a
   real-user regression. Recording for future
   webtest automation lanes.

### Carry-over (parked)

* `fullstack-b-1` empirical LRU click cycle
* `fullstack-b-14` window-title-as-drive-path
  empirical
* `fullstack-b-7` runtime click

All three remain blocked on macOS Accessibility /
hypothetical `--drive <path>` chan-desktop CLI arg;
permission scope changes don't address this.

### Lane-B state

* Serve still up on `127.0.0.1:8820` against
  `/tmp/chan-test-phase8-wb-r2`.
* Outside-drive fixture
  `/tmp/chan-watch-wb-v0112-outside/` left in place
  (held the systacean-10 walk artifacts).
* No chan-desktop launched; no system-path mutation.

Standing by for @@Alex's reply on the permission ask
or for next routing.

## 2026-05-21 — session closed

Tear-down signal received from @@Alex. Read your
PRE-RECYCLE HANDOVER + TEAR-DOWN signal in
`event-architect-webtest-b.md` (carried into commit
`3262e61`).

Tear-down checklist:

* `git status` — no uncommitted work in my lane
  (v0.11.2 verdict on `webtest-b-1.md` + `webtest-b-2.md`
  + my prior outbound poke all landed in `3262e61`).
  The 9 other modified files in the worktree belong to
  @@Architect / @@CI lanes; not mine to commit.
* Lane-B test server (PID 53339, `:8820`) stopped
  cleanly.
* Throwaway drive `/tmp/chan-test-phase8-wb-r2/`
  `rm -rf`'d.
* Drive registry entry `chan remove
  /private/tmp/chan-test-phase8-wb-r2` →
  `unregistered`.
* Outside-drive watcher fixture
  `/tmp/chan-watch-wb-v0112-outside/` (held the
  systacean-10 walk artifacts) `rm -rf`'d.
* Lane-B Chrome MCP tab gone (auto-closed when chan
  serve died).
* No chan-desktop launched in this session; no
  system-path mutation.

### Carry-over for next session of me

* Fresh-Mac Gatekeeper permission ask still parked
  with @@Alex in
  [`event-webtest-b-alex.md`](event-webtest-b-alex.md);
  default-to-(c) directive applies if @@Alex doesn't
  reply by next bootstrap.
* `-b-22` (orphan sidecar reap) walkthrough is the
  next dispatch when @@FullStackB commits the work +
  the recycled @@Architect routes a webtest-b-N task.
* Standing chan-desktop runtime perm survives recycle
  per `bootstrap.md`. 2026-05-21 tightened-scope
  clarification stays in force — ack on bootstrap
  before any DMG-verification work.

Session closing committed via
`docs: webtest-b session close (lane-B teardown
post-v0.11.2 walkthrough)`.

## 2026-05-21 — ack (post-recycle bootstrap; tightened-scope re-loaded)

Fresh @@WebtestB session bootstrapped from the
`bootstrap.md` Working-agent block. Re-acked the
2026-05-21 tightened-scope clarification for
DMG/Gatekeeper verification (no `/Applications/Chan.app`
writes; PID capture not triage; no system-path
`xattr` writes; pause-and-warn rule for canonical
fresh-Mac walks). Standing chan-desktop runtime
permission against throwaway drives carries over per
`bootstrap.md` §"Standing permissions".

Current state on bootstrap:

* HEAD `22fd878`, working tree clean.
* No lane-B serve up; no chan-desktop running.
* No `webtest-b-3.md` task file cut yet. Pre-recycle
  handover tagged `-b-22` (`3987e73` orphan sidecar
  reap + drive-lock-takeover UX) as my next dispatch
  once the recycled @@Architect routes a walkthrough
  task.
* Fresh-Mac Gatekeeper perm ask still parked with
  @@Alex in
  [`event-webtest-b-alex.md`](event-webtest-b-alex.md);
  default-to-(c) applies if no reply by the time the
  walk fires.

Holding for the @@Architect routing on the `-b-22`
walkthrough (or any other dispatch you cut).

## 2026-05-21 — poke (webtest-b-3: -b-22 walkthrough; component VERIFIED, click cycles PARKED)

Walked the four acceptance subsections of `-b-22`
(`3987e73` — process-group sidecar reap + drive-lock-
takeover UX). Verdict appended to
[`../webtest-b/webtest-b-1.md`](../webtest-b/webtest-b-1.md)
under the `2026-05-21 — fullstack-b-22 walkthrough
(orphan sidecar reap + lock takeover)` heading. Throwaway-
drive shape per the standing chan-desktop runtime
permission; tightened-scope rules honoured throughout (no
`/Applications/Chan.app` touch, no triage-shape kills, no
system-path xattr).

### Per-subsection verdict

| Subsection                                  | Verdict                        |
|---------------------------------------------|--------------------------------|
| Prevention half — graceful exit (SIGTERM)   | source+tests VERIFIED; click PARKED |
| Prevention half — ungraceful exit (kill -9) | source CONFIRMED; click PARKED |
| Recovery half — lock-takeover dialog        | marker + heuristic VERIFIED; dialog click PARKED |
| Negative case — non-chan PID on the port    | source VERIFIED; ps-grep false-positive flagged |

### What was empirically verified

* chan-desktop test suite 39/39 green at HEAD `22fd878`
  (matches `-b-22`'s "+7 new since 32" count).
* `DRIVE_LOCKED_MARKER` substring scan is anchored against
  the REAL chan-drive output: a second `chan serve` against
  the same drive (different port) exits with byte-identical
  `Error: drive is locked by another process`.
* `find_orphan_chan_serve_pids` ps-grep heuristic correctly
  matches a live orphan-shaped chan serve (the manually-
  spawned PID 21889 was picked up by the equivalent filter).
* `kill_orphan_with_grace` shape works: SIGTERM on the
  orphan-shaped PID exited within 1 s, no SIGKILL escalation
  needed in this run.

### Side observation — `find_orphan_chan_serve_pids` false-positive surface

The heuristic matches ANY process whose command line
contains `chan` + ` serve ` + drive-key as three independent
substrings. During the walk, bash/awk pipeline lines that
mentioned all three (via my shell command and my filter
regex) appeared in the candidate list alongside the real
chan serve. Real-world likelihood is narrow but non-zero
for users with a noisy shell environment: a `tail -f
chan-serve.log` over the drive key, an IDE process
inspecting the directory, a tmux pane with `chan serve
<drive-key>` in its visible scrollback that happens to be
mid-process, etc., COULD enter the candidate set. The
`promptDriveLockTakeover()` Tauri `ask()` dialog does not
display the candidate PIDs to the user (yes/no shape), so
the destructive-action confirmation surface is opaque.

Two follow-up paths for Round-2 polish:

* Tighten the heuristic to match `chan serve <drive-key>`
  as a contiguous sequence (regex or positional argv check)
  instead of three independent substrings.
* Render the candidate PIDs in the Reclaim dialog
  (replace Tauri's plain `ask()` with a custom modal so the
  user sees what's about to be killed).

Neither is gating; flagging for the bug list.

### Why I did NOT launch debug chan-desktop

@@Alex's `/Applications/Chan.app` (PID 39577) is live and
sharing `~/Library/Application Support/Chan Desktop/config.json`
with whatever debug chan-desktop I'd launch. The
atomic-write contract prevents corruption but last-writer-
wins could discard live `window_configs` from @@Alex's
in-flight session. "No persistent side effects outside the
throwaway-drive set" applies — held off, exercised
chan-drive + chan-serve invariants underneath directly
instead. If @@Alex's working session is paused in a future
window, a follow-up empirical walkthrough can launch
debug chan-desktop cleanly.

### Unblock suggestions (same shape as -b-1/-b-14/-b-7 carry-overs)

1. macOS Accessibility entitlement on Claude Code's parent
   process — `osascript` GUI scripting becomes available.
2. chan-desktop `--drive <path>` CLI arg (Round-3 polish)
   — bypasses the launcher click entirely.
3. @@Alex's eventual `chan.app` walk at v0.12.0 cut per
   the 2026-05-21 "i will only test the chan.app at the
   very very end" decision — covers every parked click
   cycle in one pass.

### Tear-down evidence

* My orphan-shaped chan serve (PID 21889) SIGTERM'd cleanly
  during the empirical pass; gone before tear-down.
* `/tmp/chan-test-phase8-wb-b22/` `rm -rf`'d.
* Drive registry entry `chan remove
  /private/tmp/chan-test-phase8-wb-b22` → `unregistered`.
* Config backup compared to live config — identical (no
  chan-desktop write occurred); backup file removed.
* Log files cleaned (no `/tmp/chan-test-phase8-wb-b22-*`
  left).
* `target/debug/chan-desktop` build artifact left in place
  (shared workspace cache).

### Commit readiness

Verdict + this poke ready to commit when you route
clearance.

* **Suggested commit subject**:
  `docs: webtest-b-3 — -b-22 orphan-sidecar reap walkthrough (component verified, click cycles parked)`.
* **Files** (explicit per-path `git add`):
  `docs/journals/phase-8/webtest-b/webtest-b-1.md` +
  `docs/journals/phase-8/alex/event-webtest-b-architect.md`.
* Pre/post-commit `git diff --staged --stat` +
  `git show --stat HEAD` per the shared-worktree
  discipline; will spot-check for stowaways before
  committing.
