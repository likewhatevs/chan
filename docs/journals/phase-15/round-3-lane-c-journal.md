# round-3 @@LaneC journal

## Wave 1 - team in the workspace + delete the rich-prompt  [DONE, locally merged]

Commit `8eb99391` on main (local). Plan + contract:
`round-3-part-1-lane-c.md`. Split backend (Rust) vs frontend (TS) via
two sub-agents against the shared API contract; integration + smoke +
commit done by me.

### What landed
- Team config moved OUT of the outside-sandbox `/tmp` absolute path and
  INTO the workspace under a user-chosen RELATIVE `{team-dir}/`, written
  through `Workspace::{read_text,write_text,create_dir}` (sandbox +
  atomic). Tree: `config.toml`, generated `bootstrap.md`, and the
  `tasks/ journals/ followups/` dirs.
- `POST /api/team-config/{read,write}` payloads changed from absolute
  `path` to workspace-relative `dir`. Route paths unchanged (no lib.rs
  edit). Handlers now take `State<AppState>` + `state.workspace()`.
- `chan_workspace::TeamConfig` grew `tab_group` (was carried by the TS
  wire but silently dropped on the Rust read). `#[serde(default)]` so a
  hand-edited config that predates the field still reads.
- Backend `validate_team_config` (<=9 members, exactly one lead,
  non-empty team/host/handle) runs on BOTH read and write -> 400 with a
  human-readable message the Load flow surfaces inline.
- `generate_bootstrap_md(team_dir, config)`: roster ASCII table,
  host/lead reveal, the "How we work" hold-for-Lead flow, the poke
  1-liner with the `\x1b[27;9;13~` submit chord, the Files reference.
  Tool-owned; regenerated on every write.
- SPA: dialog field is now "Team directory (in workspace)" (relative),
  hint "Team files will be created in <workspace>/{teamDir}/". Validation
  rejects absolute paths. Orchestrator writes via the new dir API and
  appends "Read the team process at {teamDir}/bootstrap.md" to the
  identity prompt.
- Deleted the dead bubble-stub / rich-prompt: `bubbleStub.svelte.ts`
  removed, `BubbleOverlay.svelte` gutted to a no-op placeholder (kept a
  valid no-prop mount so TerminalTab.svelte was NOT touched), Team Work
  bubble-mode menu entries removed. Collapse/Expand kept (live composer
  feature; Wave-2 survey bubbles want the room).

### Gate (scoped to my files; full-tree fmt fails only on @@LaneD's
### in-flight chan-shell, not mine)
- cargo fmt/clippy/test on chan-server + chan-workspace: GREEN (9 new
  team_config tests pass).
- web svelte-check: 0 errors / 0 warnings. vitest on my 11 touched test
  files: 116/116 green.
- cargo build -p chan: ok.

### Empirical smoke (renamed binary /tmp/chan-lanec-bin, drive
### /tmp/chan-lanec-w1, port 7799; torn down after)
- write -> 200; on disk: `new-team-1/{config.toml,bootstrap.md,tasks,
  journals,followups}` INSIDE the workspace. read round-trips incl.
  tab_group.
- 400 paths confirmed: absolute dir, zero leads, missing config.
- SPA mounted with ZERO console errors (no stale-import break from the
  deleted bubbleStub / gutted overlay). The team dir showed in the file
  tree. Team Work dialog rendered the reworked relative-dir field + hint;
  Cancel clean, no console errors. Preflight did not lock (small drive).

### Notes / contention
- Shared working tree: committed ONLY my explicit 21-path set; verified
  the staged stat + post-commit `git show --stat HEAD`. The explicit-path
  add correctly excluded a concurrently-added LaneB file
  (editor/widgets/wikilink.ts) and all other lanes' in-flight edits.
- teams.rs lives in chan-workspace (not formally listed in any lane's
  owned set). I edited it for the `tab_group` field since it is the Team
  Work config schema (my domain) and unrelated to @@LaneA's index/search
  scope. Flagging for the architect's awareness.

## Wave 2 - survey rebuild + team-config agent field  [CODE-COMPLETE, gated-green]

Plan: `round-3-part-2-lane-c.md`. Split: a background subagent did the
mechanical team-config-agent-field frontend; I did the Rust backend, the
reactive survey overlay/store, and all the cross-lane seam wiring + the gate +
the smoke. NOT committed yet (see "Commit sequencing" below - the survey
backend is a hard C+D coupled unit @@Architect must sequence at the barrier).

### Seam escalation -> approved (followup needs team context)
Found D's just-landed `SurveySpec` carried NO team-dir/from/to, but `[F]` must
write `{team-dir}/followups/...`. Escalated to @@Architect; the 2026-06-01
survey-contract AMENDMENT approved the full `followup { dir, from, to }` shape
on `SurveySpec`, populated by `cs terminal survey --followup-dir` (from <-
$CHAN_TAB_NAME, to <- the survey target). D added the wire field + flags; I
mirror it.

### What landed (my files)
TEAM-CONFIG AGENT FIELD (independent of the survey transport):
- `chan_workspace::teams::Member.agent: Option<String>` ("claude"|"codex"|
  "gemini"; None = shell). serde(default, skip_if None). Stored as String to
  avoid a chan-workspace -> chan-shell layering dep.
- `team_config.rs`: validates the agent value on read+write (unknown -> 400);
  bootstrap.md roster grew an `agent` column; the poke section now teaches the
  agent-correct `cs terminal write --submit=<target-agent>` form with a
  per-agent chord list (claude=ESC[27;9;13~, codex/gemini=CR) built from the
  roster. +2 tests (unknown-agent reject, roster/poke-chord render).
- SPA (subagent): `TeamMemberDraft.agent` + `agentForCommand()` sniff +
  per-member picker in TeamDialog; translateConfig/wireToDialog round-trip
  (omit "none"); the lead tab's `teamWork.agentTarget`/`submitMode` set from
  the lead member's agent so the composer submits with the right chord. The
  field is OPTIONAL not required (3 unowned test files build TeamDialogConfig
  literals; a required field would break them) - producers always populate it.

SURVEY (the C side of the C+D seam):
- `routes/survey.rs` (NEW): `POST /api/survey/reply` deserializes a C-owned
  `SurveyReplyRequest` (option | followup) and calls D's
  `state.survey_bus.complete_survey(id, chan_shell::SurveyReply) -> bool`
  (false -> 404). On followup it first creates the file via
  `create_followup_file(workspace, dir, from, to, title, body)` - mints `n` by
  scanning `{dir}/followups/`, sanitizes @@-handles for the filename, writes
  through the Workspace sandbox, pre-populates header/created-ts/from/to/the
  "not ready, check later" line/original prompt/@@Host comment placeholder. 7
  unit tests.
- `routes/mod.rs`: `mod survey;` + `pub use survey::api_survey_reply;`.
- `lib.rs`: route mount + import (CO-EDITED with D - see sequencing).
- `web/src/api/client.ts`: `SurveySpec` + `SurveyFollowupContext` +
  `SurveyReplyRequest` TS mirrors of D's wire + `api.surveyReply`; plus
  `TeamMemberWire.agent`.
- `web/src/state/survey.svelte.ts` (NEW): `surveyState` singleton +
  `showSurvey`/`pickOption`/`requestFollowup`. Non-dismissable (CLI is blocked;
  [F] is the defer path). 7 store tests.
- `BubbleOverlay.svelte`: REBUILT from the gutted placeholder into the real
  overlay - markdown body via DOMPurify-sanitized renderMarkdown, <=4 vertical
  numbered options, [F] (only with allowFollowup + context), keyboard 1..N + F,
  focus-steal on show. 4 jsdom render tests.
- Mount MOVED from per-terminal-tab (TerminalTab.svelte) to ONCE at App root:
  D's `open_survey` frame is window-targeted (no tab id), so a single
  window-level modal is correct, not N per-pane copies.
- `store.svelte.ts`: `handleWindowCommand` gained the `open_survey` case ->
  `showSurvey(frame.survey)`, per D's documented frame shape.

### Gate (on the integrated working tree, coherent moment)
- cargo build -p chan / -p chan-server / --no-default-features (whole ws): 0.
- clippy -p chan-server -p chan-workspace --all-targets -D warnings: 0.
  (D's `#[allow(dead_code)]` on complete_survey/survey_bus did NOT error once my
  route used them - confirmed; I left D's files untouched.)
- cargo test chan-server + chan-workspace: pass (survey 7, team_config 11).
- fmt: my files clean (only D's chan-shell/cli.rs has diffs - D's to fix).
- svelte-check 0/0; full vitest 162 files / 1619 tests pass; npm run build: 0.

### Empirical smoke (live server, scoped + torn down)
Browser NAV was denied by the harness (Wave-1 precedent: @@Host re-allows), so
the live overlay-render + reply-round-trip in a real browser is recorded
empirically-unverified-by-me and folds into the JOINT Wave-3 smoke with
@@LaneD (the lane doc already scopes that). Instead I curl-smoked the FULL C
backend against a fresh standalone server (/tmp/chan-lanec-smoke binary,
:7866, scoped pkill, registry entry removed, drive+tab torn down): option reply
-> 404 with the right body (route mounted, bus says not-parked); followup reply
-> the route created `test-team/followups/followup-LaneC-Host-1.md` on disk via
the real Workspace sandbox with the exact pre-populated content. So route +
generator + Workspace write are EMPIRICALLY confirmed on a running server;
only the SPA-render + bus-completion (D-verified) await the joint browser smoke.

### Known benign edge (flagged)
The reply route creates the followup file BEFORE complete_survey, so a stale /
duplicate followup POST (unknown survey_id) leaves an orphan file then 404s. In
normal use the SPA only POSTs while the overlay is up (CLI blocked, survey
parked), so it won't fire. Tightening would need a bus `is_parked(id)` peek
(D's side) - low priority, raised for awareness.

### Commit sequencing (NOT committed - needs @@Architect at the barrier)
The survey backend is a HARD C+D coupled unit and cannot be locally merged
alone:
- `routes/survey.rs` imports `chan_shell::SurveyReply` (D's UNCOMMITTED wire.rs)
  and `crate::survey::SurveyBus` (D's UNTRACKED survey.rs).
- `lib.rs` is co-edited: my route mount/import + D's survey_bus wiring + handoff
  mod. A pathspec commit of lib.rs by either lane sweeps the other's hunks. D
  already flagged lib.rs as "sequence at the barrier".
So the survey landing is one coordinated C+D commit @@Architect sequences on a
coherent HEAD (D's chan-shell + survey.rs + state.rs + control_socket +
terminal_sessions, plus my routes/survey.rs + mod.rs + lib.rs mount + client.ts
+ the SPA files). The team-config agent field is independent and could be its
own commit; I left it uncommitted too so @@Architect sequences the whole wave
coherently (Cargo.lock is shared - do not let it ride a lane commit). My exact
file manifest is in the poke + above.

## Wave 3 - JOINT survey browser smoke  [DONE - all paths PASS]

The Wave-2 browser-unverified item (survey SPA overlay render + reply
round-trip; navigate was denied to C that wave) is now empirically verified
end-to-end in a real browser against the committed HEAD (08d7435b). NO code
changes - this wave was pure verification.

### Setup (scoped + torn down)
- `npm run build` (web) BEFORE `cargo build -p chan` so the served bundle has
  my survey SPA (stale-web/dist guard). Binary copied to /tmp/chan-lanec-w3-bin
  (v0.21.0), throwaway drive /tmp/chan-lanec-w3, served on :7901, control socket
  chan-control-6309. Browser tab on my :7901 only; I scoped strictly to my own
  port/socket and never touched @@LaneD's :7841 server.
- Opened a Terminal tab ("Terminal-1") in the SPA so the survey selector had a
  live session to target, then ran `cs terminal survey --tab-name Terminal-1`
  from my shell with CHAN_CONTROL_SOCKET pointed at my socket.

### Results (3 blocking surveys, real overlay, real reply)
1. OPTION PICK via CLICK: title + body rendered with markdown (**bold** bold,
   `code` monospace via DOMPurify renderMarkdown), 3 vertically-numbered
   options, NO [F] (correct - no --followup), modal dimmed backdrop. Clicked
   [2] -> overlay dismissed -> blocked CLI printed `Option Bravo`, exit 0.
2. OPTION PICK via KEYBOARD: 4-option survey (the cap) rendered all 4 vertical.
   Pressed `4` -> dismissed -> CLI printed `Four`, exit 0. (window-routed key
   handler does NOT leak into the terminal PTY underneath - confirmed.)
3. [F] FOLLOWUP via CLICK: --followup --followup-dir myteam --from @@LaneC.
   Overlay showed the 2 options PLUS the dashed "[F] Follow up later" button.
   Clicked [F] -> dismissed -> CLI printed
   `new follow up file created: myteam/followups/followup-LaneC-Terminal-1-1.md`
   -> the file exists on disk (created through the Workspace sandbox; myteam/
   appeared live in the file tree via the watcher) with the exact pre-populated
   content: `# Follow up: Followup smoke`, ISO `Created:`, `From: @@LaneC`,
   `To: Terminal-1`, the "not ready, check later" line, `## Original prompt` +
   the body, and the `## Terminal-1 comments` placeholder. No em dashes.

`to` resolved to the survey TARGET (Terminal-1, the --tab-name), not my
`--to @@Host` override - correct per D's CLI + the contract ("to <- the survey
target ... fallback --to"); --to/--from are fallbacks, and `from` used --from
only because $CHAN_TAB_NAME was unset in my ad-hoc shell. In real team use
from=$CHAN_TAB_NAME (surveying agent) and to=surveyed tab.

### Cross-lane note (shared-Chrome collision, resolved)
@@LaneD and I share ONE Chrome (the extension drives a single instance, same
tab-group). On my FIRST navigate my tab briefly drifted to D's :7841 (D was
driving a tab concurrently); re-navigating to :7901 was stable and did NOT
auto-drift, and a careful re-run kept :7901 throughout. D poked mid-smoke asking
who covers [F]; I replied I had just finished ALL three paths on :7901 so the
joint smoke is fully covered (D had option-pick PASS on :7841) and stood down
the browser. No edits to D's environment.

### Conclusion
Survey feature (the C side: SPA overlay + reply route + [F] followup generator)
is empirically verified end-to-end on a running server + real browser. The
Wave-2 "survey SPA render+reply" browser-unverified item is CLEARED. No
carryover; no code changes this wave.
