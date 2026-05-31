# Event channel — @@LaneD

Append-only. Completion + checkpoint events from @@LaneD. Newest at the bottom.

---

## 2026-05-31 — bootstrap confirmed

Read bootstrap + round-2-lane-d + part-1 + part-2 + coordination. Confirmed
domain (terminal / cs / keyboard / desktop / Team Work; round-1 Lane-C
continuation), wave plan, and the 2.3 deferral. Starting wave-1 SUBMIT.

## 2026-05-31 — SUBMIT code-complete + statically gated (CK-SUBMIT, smoke pending)

`keymap.ts` `terminalMetaKeyBytes`: Shift+Enter now falls back to a bare LF
(`\n`) when no enhanced keyboard protocol is active (the "agent already
running, never observed negotiating" case round-1 could not cover). Safe for a
plain shell (line discipline submits on `\n`, no stray bytes) and correct for
Claude (`\n` = newline in its draft, per the live-probed AGENT_SUBMIT_CHORD
comment). Scoped to Shift+Enter; Cmd/Ctrl+Enter keep `\r` submit.

Gate (my scope): svelte-check 0/0; keymap.test.ts 19 pass; full vitest 1565
pass / 1 fail. The fail is `dashboardTabAndCarousel.test.ts:158` = @@LaneB's
in-flight A3/A4/A6/A7 work in the shared worktree, NOT mine (I touched only
keymap.ts + keymap.test.ts).

CK-SUBMIT is NOT yet a green light for poke auto-delivery — see the journal:
landing SUBMIT does not make a bare `\n` submit. POKE-2.2 must append
AGENT_SUBMIT_CHORD (`\x1b[27;9;13~`), which already exists from round-1. Flagged
to @@Architect. Real-agent smoke pending a batched wave-1 test server.

## 2026-05-31 — RELOAD code-complete + gated (smoke pending)

Reload remapped to Cmd+R (macOS) / Ctrl+Shift+R (Linux/Windows), never plain
Ctrl+R. One `osChord` resolver in shortcuts.ts diverges the chord per-OS;
shouldEscapeTerminal + labels + help table route through it; App.svelte +
desktop KEY_BRIDGE_JS branch the raw events (metaKey-gated, mirroring Cmd+W).
Regenerated SERVE_LONG_ABOUT (also fixed pre-existing table drift in my own
main.rs). Gate: web svelte-check 0/0 + vitest 1579 pass; Rust fmt/clippy
--all-targets/build --no-default-features clean; desktop bridge test pass.
Terminal smoke batched.

## 2026-05-31 — LINKS code-complete + gated (smoke pending)

Terminal link clicks route through openExternalUrl (editor's path): browser
tab on web, OS browser on chan-desktop, scheme-gated. Replaces the inert
default WebLinksAddon window.open. Gate green. Browser/desktop smoke batched.

## 2026-05-31 — CK-RENAME reached: `cs term` -> `cs terminal`

Renamed the clap group Term/TermAction -> Terminal/TerminalAction +
cmd_shell_terminal + all help/doc strings + cs_alias.rs. Pre-release: dropped
`term`, no alias. Gate: cs_alias 3 pass; fmt/clippy/build --no-default-features
clean. @@LaneC (`cs search`) + the poke/bootstrap docs referencing
`cs term write` should rebase to `cs terminal`. CK-RENAME -> @@LaneC + @@Architect.

## 2026-05-31 — Wave-1 code-complete (SUBMIT, RELOAD, LINKS, CS-RENAME)

All four wave-1 items pass the full static gate. Remaining: ONE batched test
server for the empirical smokes (SUBMIT real-agent newline, RELOAD reverse-
search/reload, LINKS clickable). Need @@Architect: new throwaway drive vs reuse
+ seed (RELOAD wants a busy screen over a shallow clone of this repo; SUBMIT
needs a real claude/codex).

## 2026-05-31 — DECISION NEEDED -> @@Architect: test server for wave-1 smokes

Poked @@LaneA. Question: new throwaway /tmp/chan-test-laned (shallow clone of
this repo + real claude, served from a renamed binary on my own port, scoped
pkills) vs reuse a shared orchestrator server (need path+port)? Note the RELOAD
desktop Ctrl+R *hang* re-tests cleanly only after @@LaneC's CK-INDEX-IDLE lands;
the keymap remap itself (reverse-search/reload chords) smokes independently.
Proceeding into wave-2 code (CS-PREFIX/CS-RESTART/CS-LIST) while awaiting the
answer; holding CS-CAROUSEL for CK-CAROUSEL with @@LaneB.

## 2026-05-31 — FINDING: live server-side repro of CK-INDEX-IDLE (-> @@LaneC)

Setting up the wave-1 smoke server surfaced @@LaneC's CK-INDEX-IDLE bug
SERVER-SIDE, not just poll-only. Built my own binary (HEAD + wave-1 fixes,
provenance verified: `cs terminal` present), served /tmp/chan-test-laned
(shallow clone of this repo) on 8823 --here --standalone.

- `/api/index/status` -> `{state:"building",current:4097,total:4096,
  file:"embedding"}` and stays there indefinitely (identical after 20s, 40s).
  `/api/preflight` -> `{phase:"running",locked:true,...}` forever.
- The drive is NOT semantic (`semantic_search: None` in the registry) yet the
  index runs an "embedding" step that hangs.
- current overshoots total by 1 (4097/4096): the file walk completes, then a
  final "embedding" step wedges and never transitions to Idle.
- An EMPTY drive (0 docs) -> `{state:"idle",indexed_docs:0,...}` and preflight
  `{phase:"ready",locked:false}`. So the hang is the embedding step on any
  drive WITH content, NOT a poll race and NOT size-specific.
- Repro for @@LaneC: `git clone --depth1 file://<this repo> /tmp/x;
  <binary> serve /tmp/x --here --port N --standalone`. This is the common
  root of BOTH round-2-part-2 bugs (stuck status-bar reindex AND the Cmd+R
  preflight hang). The Cmd+R hang is just preflight re-locking on the same
  wedged state after a reload.

Impact on my smokes: the heavy seed blocked preflight, so ALL browser smokes
were blocked (not just the reload-HANG). Pivoted to an EMPTY drive on 8823
(preflight unlocks) to smoke SUBMIT / LINKS / RELOAD-keymap (terminal-only,
no notes needed). The busy-repo reload-HANG re-verify stays deferred to
post-CK-INDEX-IDLE, as @@Architect flagged.

## 2026-05-31 — @@Architect read-trust guidance for wave-2: ACK

@@LaneA ground-truthed @@LaneC's env flakiness: it is output TRUNCATION on
complex commands, NOT byte-fabrication (sha-proved a git blob authentic).
For my wave-2 appends to main.rs + control_socket.rs I will: read regions
with single atomic commands (one `sed -n A,Bp` / one `grep` per call), no
`||` chains, no parallel command storms, and sha-verify each region
(`shasum` disk vs `git show HEAD:file | shasum`) before I Edit. ACK -> reads
are workable; proceeding to wave-2 after the smokes.

## 2026-05-31 — WAVE-1 SMOKE RESULTS (empty drive 8823, my binary)

Browser smokes on the empty drive (preflight unlocked), Chrome on macOS:

- **SUBMIT (CK-SUBMIT) -> EMPIRICALLY VERIFIED, three levels:**
  1. Byte: `stty raw; dd bs=1 count=1 | od -An -tx1`, press Shift+Enter ->
     `0a` (LF). The no-negotiation fallback fires (would be `0d`/CR pre-fix).
  2. Plain shell: `echo X` + Shift+Enter submits cleanly, echoes X, fresh
     prompt, NO stray bytes / no `^M`. Safe-for-shell confirmed.
  3. **Real Claude Code v2.1.158**: typed "LINE_ONE_ABC", Shift+Enter,
     "LINE_TWO_XYZ" -> the draft held BOTH lines (multi-line edit mode,
     "ctrl+g to edit in Vim"), did NOT submit. The exact target behavior.
  => CK-SUBMIT is GREEN. Combined with the directed-wake proof (the
  architect's AGENT_SUBMIT_CHORD poke submitted hands-free into my agent),
  the full agent-keyboard story is verified: `\n` = newline, chord = submit.
- **LINKS -> EMPIRICALLY VERIFIED:** `echo https://example.com/...` -> the
  URL underlines on hover (zoom-confirmed) and a click opened a new browser
  tab "Example Domain". Routes through openExternalUrl (web window.open
  _blank). The hover-highlights-but-click-dead bug is fixed.
- **RELOAD -> runtime NOT browser-smokeable on mac-Chrome.** Chrome captures
  Ctrl+R / Cmd+R at the browser level (the tool fired a page reload, not a
  keystroke to xterm), and the desktop KEY_BRIDGE_JS + the Linux Ctrl+Shift+R
  path aren't reachable from Blink at all (cf. the WebGL/WKWebView note).
  Covered by unit tests (osChord, per-OS App handler, shouldEscapeTerminal,
  desktop bridge pin). Empirically-unverified-at-runtime; the desktop
  reload-HANG re-verify is deferred to post-CK-INDEX-IDLE per @@Architect.
  Incidental: a Chrome Ctrl+R reload of the EMPTY drive recovered cleanly
  (no hang) - consistent with the hang being content-drive/embedding-bound.

Server 8823 (empty drive /tmp/laned-mt, my binary) left up for any wave-2
smokes; scoped to my port/path. Pivoting to wave-2 (CS-PREFIX/RESTART/LIST)
with the sha-verify read discipline; will flag @@Architect at CK-RESTART.

## 2026-05-31 — WAVE-1 MERGED to main (local): commit 1b39832b

Per @@Architect authorization, merged wave-1 to main locally. Guarded atomic
stage: PRE-index-clean check, cargo fmt --check, `git add` my 11 explicit
files, `git diff --staged --stat` audit, `comm` guard (staged set == my files
exactly, else abort+reset), commit, `git show --stat HEAD`. Staged exactly:
main.rs, cs_alias.rs, serve.rs, App.svelte, Pane.svelte (my 2 osChord lines),
TerminalTab.svelte, TerminalTab.test.ts, cmdRWindowReload.test.ts,
shortcuts.ts, keymap.test.ts, keymap.ts (11 files, +207/-65). Commit
`1b39832b` parent = `37d68bef` (@@LaneB part-1 + per-tab autoRotate) - clean
linear, no rebase. Coordination docs + package-lock + a peer's in-flight
walker.ts left dirty (not mine). This clears my cmdRWindowReload/keymap test
noise for @@LaneB + @@LaneC. Did NOT rebuild the live chan-desktop app (the
installed cs/chan stays v0.20.0; my test binary is the separate /tmp/lanedsrv).

## 2026-05-31 — CK-CAROUSEL RESOLVED (<- @@LaneB via @@Architect)

The DashboardTab carousel-rotation field is `tab.autoRotate` (boolean), NOT
`disabledSlots`. So CS-CAROUSEL wires `cs dashboard --carousel-off` ->
`autoRotate: false` on the newly created DashboardTab (default autoRotate
true). A3 needed no Pane.svelte edit, so Pane.svelte is purely my 2 osChord
lines. CS-CAROUSEL is now unblocked; will set autoRotate:false in the
open_dashboard control-socket arm + handleWindowCommand (my region).

## 2026-05-31 — cs surface increment COMMITTED (cf2c8b2c) + handoff to @@LaneC

Per @@Architect's main.rs/control_socket.rs coordination (avoid the round-1
concurrent-edit collision with @@LaneC's cs search), landed my full cs
surface as one gated increment on the clean base, then handed off.

- **CS-PREFIX**: `infer_subcommands(true)` on the Shell + Terminal clap
  groups (not the Cli root - avoids chan-level ambiguity). `cs o/g/d/t` +
  `cs t n/w/l/r` resolve by prefix. cs_alias test `cs t l` -> terminal list.
- **CS-RESTART (CK-RESTART)**: `ControlRequest::TermRestart {tab_name,
  tab_group}` (client in main.rs + server in control_socket.rs) ->
  `Registry::restart_matching` (terminal_sessions.rs): collects matching
  session ids under the lock, drops it, restarts each via the existing
  `Registry::restart(id, None x5)` which preserves spawn command+env, so an
  agent relaunches. Selector required (mirrors term_write). 2 control_socket
  tests. This is the out-of-band path Team Work self-restart needs.
- **CS-LIST**: `cs terminal list` -> markdown table by default
  (render_terminal_list_markdown), `--json` compact, `--json --pretty`
  indented. Server still returns the `{groups:...}` JSON; the client formats.

Gate: fmt clean, clippy --all-targets -D warnings clean, build
--no-default-features clean, chan 58 unit + 4 cs_alias, chan-server 318.
Commit cf2c8b2c (4 files, +250/-6), guarded atomic stage.

HANDOFF: pinged @@LaneC to append cs search on cf2c8b2c. I am HOLDING all
main.rs + control_socket.rs edits until their cs search lands (sequenced, no
concurrent editing). Remaining wave-2 once unblocked: CS-CAROUSEL main.rs
flag + DESKTOP-OPEN (main.rs cmd_open) rebase onto cs search; DESKTOP-SHELL
(chan-desktop). CS-CAROUSEL frontend (store.svelte.ts handleWindowCommand)
+ the empirical CS-RESTART agent-relaunch smoke are doable meanwhile / batched
with wave-3.

## 2026-05-31 — cs search committed for @@LaneC (e10424a5) + green restored

@@LaneC's read-tooling went down mid-cs-search (couldn't build/commit). I
escalated a reassign to @@Architect; @@LaneC recovered briefly then @@Host
cleared me to FINISH it. Ground truth (from @@Architect's reliable tab): the
only red was E0599 - the chan CLI's duplicate Serialize-only ControlRequest
enum (main.rs) missing the Search variant; @@LaneC's server handler +
search_workspace + client ShellAction::Search + cmd_shell_search + render were
all written. I added ONLY that one client `Search { query, limit }` variant,
ran the full gate (fmt, clippy --all-targets, build --no-default-features,
chan 58+4, chan-server 318), and chained-committed control_socket.rs + main.rs
as @@LaneC's authorship (e10424a5, +166). Green build restored; team unblocked.
@@LaneC's CK-INDEX-IDLE fixes also landed (3e54ed3e C-CAP + 326532d9), so my
RELOAD desktop-hang re-verify + heavy-seed smoke should now be testable.

## 2026-05-31 — CS-CAROUSEL committed (7c241370) - WAVE-2 COMPLETE

cs dashboard --carousel-off -> autoRotate:false on the new DashboardTab.
Wiring (grounded in source - the field is tab.autoRotate, default true,
serialized `ar`, from @@LaneB 37d68bef; NOT disabledSlots):
- main.rs: ShellAction::Dashboard gains --carousel-off (one-r); client
  ControlRequest::OpenDashboard gains carousel_off.
- control_socket.rs: server ControlRequest + WindowCommand::OpenDashboard gain
  carousel_off (#[serde(default)] / skip is_false); open_dashboard threads it.
  2 new wire tests.
- store.svelte.ts (my handleWindowCommand region): open_dashboard arm sets
  tab.autoRotate = false when carousel_off (restructured to find the tab once,
  apply carousel_index + carousel_off).
Gate: fmt, clippy --all-targets -D warnings, build --no-default-features,
chan 58+4, chan-server 319, svelte-check 0/0, vitest 1586. Commit 7c241370
(3 files, +55/-12), guarded atomic. Browser smoke (--carousel-off -> rotation
off) pending a fresh server build; the autoRotate reactivity is @@LaneB-smoked.

WAVE-2 COMPLETE for @@LaneD: CS-PREFIX/RESTART/LIST (cf2c8b2c) + CS-CAROUSEL
(7c241370). Remaining: wave-3 (TEAM-SELFSTART root-caused, TEAM-GROUP dialog
-> @@LaneB via @@Architect, TEAM-CONSOLIDATE, POKE-2.2) + DESKTOP-SHELL/OPEN,
with the batched real-agent smokes on a fresh server (index fix should now let
a seeded drive reach Idle).

## 2026-05-31 — IDX fix validated + POKE-2.2 validated live

- Built HEAD, served a small CONTENT drive (3 notes) on 8824: it reaches
  `{state:idle, indexed_docs:3, indexed_vectors:3}` + preflight ready. So
  @@LaneC's CK-INDEX-IDLE fix (C-CAP + per-file drain) resolves the
  embedding hang I found - a content drive now reaches Idle.
- POKE-2.2 validated live: my pokes appending bare `\n` landed as text but
  never SUBMITTED into @@Architect's agent (a newline, not a submit) - that
  was the "delivery race". Re-sending with `text + \x1b[27;9;13~`
  (AGENT_SUBMIT_CHORD) auto-submits. Confirms the POKE-2.2 design: the poke
  MUST append the submit chord, not `\n`.
- Routed the TEAM-GROUP interface (docs/journals/phase-15/team-group-interface.md)
  to @@LaneB via @@Architect (they own the dialog field + default helper; I
  own the -N conflict + orchestrator threading; persist the one TeamConfigWire
  field, coordinated via @@Architect).

## 2026-05-31 — TEAM-SELFSTART SMOKE-DIAGNOSED (hypothesis was WRONG)

Smoked a team-of-1 (agent=claude) on the 8824 drive. The lead tab renamed to
@@Lead (so launchLead RAN, sessionId WAS set -> NOT the early-return I
hypothesized). The lead terminal showed the original shell prompt, then
"session ended (explicit)", then DEAD - no shell, no claude.

ROOT CAUSE: `api.restartTerminal` CLOSES the lead's session (Closed Explicit)
but the SPA never REATTACHES to the restarted agent session. Workers spawn
FRESH sessions (`api.spawnTerminal` -> new id -> openTerminalInPane) so they
attach + launch fine; only the LEAD restarts-in-place and loses its reattach.
So the fix is NEITHER the early-return NOR the command+env override (that code
was never reached) - it is the lead restart-in-place reattach gap.

FIX (recommended, reported to @@Architect): (b) consolidate - make the lead
use the WORKER spawn path (api.spawnTerminal with the agent command + repoint
the lead tab to the fresh session), instead of restart-in-place. That MERGES
TEAM-SELFSTART INTO TEAM-CONSOLIDATE (one create path for lead + workers) and
sidesteps the broken reattach entirely. Next: read api.restartTerminal +
TerminalTab reattach + the worker attach path, implement the consolidated
lead-spawn, re-smoke with real claude. Server torn down (contended cores).

Diagnostic-first paid off: I would have written the wrong fix (command+env
override / await-sessionId) had I not smoked it.

## 2026-05-31 — TEAM-GROUP threading done (combined green, handed to @@LaneB)

@@LaneB finished the TEAM-GROUP dialog side (uncommitted, shared worktree) and
made TeamDialogConfig.tabGroup REQUIRED, which broke svelte-check in MY files.
Did my threading half: teamOrchestrator translateConfig (`tab_group:
config.tabGroup`) + wireToDialog (`tabGroup: wire.tab_group ??
defaultTabGroupFromPath(configPath)`) + the value import + TeamConfigWire
gains `tab_group: string` (api/client.ts). Fixed 8 test literals (LaneB
listed 6 TeamDialogConfig ones; I caught 2 MORE TeamConfigWire literals:
teamLoadFlow.test.ts:72 loadedWire + teamOrchestrator.test.ts:168 wire()).
Combined tree GREEN: svelte-check 0/0, full vitest 1591. Per @@Architect,
@@LaneB commits the COMBINED (their 2 dialog files + my 7), crediting me, so
main is never red - I do NOT commit my own teamOrchestrator changes (avoids a
race on the shared file). This lands the tabGroup DATA plumbing + persistence
only; the FUNCTIONAL -N resolution + threading tab_group into terminal
creation is part of TEAM-CONSOLIDATE, sequenced AFTER @@LaneB's commit (same
teamOrchestrator.svelte.ts file). POKE-2.2 chord used on both pokes.

NEXT (holding for @@LaneB's commit): the consolidate lead-spawn fix
(TEAM-SELFSTART+CONSOLIDATE) on the committed base + the functional tab_group
application + re-smoke with real claude.

## 2026-05-31 — TEAM-SELFSTART + TEAM-CONSOLIDATE LANDED + smoke-verified (fc617e85)

On @@LaneB's clean base (5603403), rewrote launchLead: the lead now uses the
WORKER spawn+mount path - api.spawnTerminal({name,command,env}) +
openTerminalInPane({sessionId,title}) (a FRESH TerminalTab mount bound to the
new session) - then force-closes the Cmd+P placeholder (closeTab
{force:true}, after opening so the pane is never empty). Deleted the dead
api.restartTerminal call + the leadTabIn helper. One create path for lead +
workers (TEAM-CONSOLIDATE). Rewrote the 3 lead tests (teamLeadRestart,
teamLeadPrompt, teamBootstrapOrchestrator) from restart-in-place to
spawn-fresh; removed the obsolete "throws when lead tab missing" test.

RE-SMOKE (real Claude Code, fresh build, 8824, team-of-1): the lead tab
@@Lead LAUNCHES CLAUDE. The EXACT same bootstrap previously dead-ended on
"session ended (explicit)" + a dead shell - so the fix is empirically proven,
A/B. Incidentally confirmed @@LaneB's TEAM-GROUP dialog field renders live
(chan-team default + the -N help text). Gate: svelte-check 0/0, vitest 1589.
Commit fc617e85 (4 frontend files, +105/-109), guarded atomic. Server torn
down (cores).

DIAGNOSTIC-FIRST WIN: my original fix hypotheses (early-return; then
command+env override) were BOTH wrong; only the smoke revealed the real cause
(orchestrator-external restart never reattaches). The consolidate sidesteps
it entirely.

REMAINING wave-3: (a) functional tab_group - terminals JOINING the team group
server-side needs tab_group on TerminalSpawnRequest + the spawn route (small
Rust); (b) POKE-2.2 completion-poke wiring (append AGENT_SUBMIT_CHORD, infra
exists); (c) DESKTOP-SHELL/OPEN.

## 2026-05-31 — FUNCTIONAL tab_group LANDED (020c690c) - TEAM-GROUP complete

Completed the functional half of TEAM-GROUP (the server-side group join),
blessed by @@Architect (chan-server my domain):
- chan-server routes/terminal.rs: CreateTerminalBody gains `group`; the spawn
  handler threads it into CreateOptions.tab_group (normalize_tab_group). New
  test api_create_terminal_joins_the_requested_group asserts a spawned
  terminal lands on the requested registry tab_group.
- web/src/api/types.ts: TerminalSpawnRequest gains `group`.
- teamOrchestrator: resolveTeamGroup(base) resolves config.tabGroup against
  the LIVE groups (allTerminalTabs().map(terminalTabGroup)) with a -N suffix
  on collision; the resolved group threads into BOTH lead + worker
  api.spawnTerminal({group}) AND openTerminalInPane({group}).

So every team terminal now joins the team group server-side ($CHAN_TAB_GROUP
+ cs terminal list grouping) AND SPA-side (group-scoped broadcast). Combined
with @@LaneB's dialog + persistence (5603403) + my wire threading, TEAM-GROUP
is fully done. Gate: fmt, clippy --all-targets -D warnings,
build --no-default-features, chan-server 38, svelte-check 0/0, vitest 1589.
Commit 020c690c (3 files, +59/-2), guarded atomic. The cs-terminal-list
grouping browser smoke is queued for @@LaneC Team Work QA (architect's route).

## 2026-05-31 — POKE-2.2 LANDED (2b9563c7) + DESKTOP scoped (recommend round-3)

POKE-2.2: `cs terminal write --submit` strips trailing newlines + appends
AGENT_SUBMIT_CHORD (apply_submit_chord helper + unit test) so a completion
poke auto-submits into a running agent. Productizes the chord we used all
round. chan 59 + cs_alias 4, clippy/build green. Commit 2b9563c7.

ALL WAVE-3 TEAM WORK DONE: TEAM-SELFSTART (fc617e85, smoke-verified) +
TEAM-GROUP (5603403 + 020c690c) + POKE-2.2 (2b9563c7).

DESKTOP scope read (for @@Architect -> @@Host do-now-vs-defer):
- DESKTOP-SHELL: MEDIUM cross-crate refactor (extract ~400-500 lines of
  cs-shell client to a shared crate both chan + chan-desktop link; risk =
  clap-across-crates + unifying the duplicated client/server ControlRequest).
- DESKTOP-OPEN: MODERATE-SUBSTANTIAL (cmd_open no-terminal fallback: registry
  workspace lookup + reuse the cmd_serve macOS->desktop handoff + reject
  guidance); INDEPENDENT of SHELL.
- BOTH verify only via a real chan-desktop run (NOT Chrome-smokeable, the
  WKWebView/Blink split). RECOMMENDATION: defer BOTH to a focused round-3
  "chan-desktop integration" mini-round (OPEN + SHELL + the ControlRequest
  dedup, with real desktop smokes), since v0.21.0 is already large + the
  reported Team Work bug is fixed+shipped. @@Host's call.

## 2026-05-31 — DESKTOP-OPEN + cs-search polish LANDED -> v0.21.0 BUILD COMPLETE

@@Host call: DESKTOP-SHELL deferred to round-3 (byte-identical-serde-tag risk
near a release cut = the gate-blind wire trap that killed v0.19.0); land
DESKTOP-OPEN now.
- DESKTOP-OPEN (05e9b9eb): cmd_open outside a chan terminal assesses the path
  against the registry (workspace_root_for, longest-prefix) and hands the
  owning workspace to a running chan-desktop (maybe_handoff_to_desktop), else
  guides to chan serve / chan add. pick_workspace_root unit-tested.
  CLI-smoked the guidance path; the handoff branch is unverified-on-desktop
  (not Chrome-smokeable) -> @@LaneC QA.
- cs-search polish (cc076e85): render_search_markdown converts the BM25
  <b>...</b> snippet highlight to markdown **bold** (unit-tested).

@@LaneD v0.21.0 build work COMPLETE. DESKTOP-SHELL is the only round-3
carryover (scoped in task #13: cs-shell -> chan-shell crate extraction +
ControlRequest client/server dedup, with the wire-byte risk noted).

## 2026-05-31 — @@LaneD round-2 RETROSPECTIVE (for @@Architect's round-close)

DONE (all green main, guarded-atomic, authorship credited): wave-1
(SUBMIT/RELOAD/LINKS/CS-RENAME 1b39832b), wave-2 cs (cf2c8b2c + CS-CAROUSEL
7c241370), cs search rescue (e10424a5), TEAM-GROUP (5603403 + 020c690c),
TEAM-SELFSTART+CONSOLIDATE (fc617e85, real-claude smoke-verified), POKE-2.2
(2b9563c7), DESKTOP-OPEN (05e9b9eb), cs-search polish (cc076e85). CK-INDEX-IDLE
found by me + re-validated after @@LaneC's fix.
PENDING (round-3): DESKTOP-SHELL (deferred, scoped).
QA queued (@@LaneC): Team Work surface (grouping/broadcast/dialog), --submit
poke, DESKTOP-OPEN desktop handoff.

Highlights:
- Diagnostic-first paid off twice on TEAM-SELFSTART: the smoke killed TWO
  wrong fix hypotheses (sessionId early-return, then a command+env override
  @@Architect had blessed) before revealing the real cause (orchestrator-
  external restart never reattaches). Writing the fix blind would have
  shipped the wrong thing. The reported bug ended fixed + A/B-smoked.
- Rescued @@LaneC's cs search through their live tool-outage (1-variant E0599
  fix + full gate + chained-commit, crediting them) to restore a green build,
  rather than stall the whole team.
- main stayed green start to finish; every increment guarded-atomic.

Lowlights / honest feedback:
- ME: I under-scoped DESKTOP-OPEN on first read (called it "moderate, land
  now") and corrected only after reading the serve-handoff. I should read the
  reuse target before quoting effort. I also checkpointed for "session
  length" more times than was useful; @@Architect's "proceed" was right every
  time, and I delivered each time - I should trust a decisive architect more
  and self-interrupt less.
- @@Architect (@@LaneA): excellent - decisive, ground-truthed the env
  flakiness from a reliable tab, sequenced the shared-file commits so main
  never went red, and the smoke-first / diagnostic-first discipline you held
  is exactly what caught the wrong fixes. One thing: the command+env-override
  blessing for TEAM-SELFSTART was premature (pre-smoke) - good that we smoked
  first; worth defaulting "bless the fix" to "bless the smoke" for
  lifecycle/race bugs.
- PROCESS: the bare-\n poke stalls (early round) cost real round-trips until
  POKE-2.2's chord. Productizing it as `cs terminal write --submit` closes
  that for v0.21.0; the standing "chord on every poke" rule should be in
  bootstrap.md from round-3's start.

## 2026-05-31 — PROCESS: submit chord on EVERY poke (@@Architect)

Standing rule from @@Architect: every `cs term write` poke MUST append the
AGENT_SUBMIT_CHORD `\x1b[27;9;13~` (= Meta+Enter, hands-free submit), else it
lands in the target's compose box UN-submitted and @@Host has to hit Enter
(that was the cause of the "waiting-on-you" stalls). I bootstrapped before the
CK-SUBMIT recipe update so my early pokes used a bare `\n`; switched to the
chord once POKE-2.2 was validated and now use it on every poke. ACK. Flagged CK-RESTART to @@Architect.
