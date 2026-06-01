# Phase-15 round-3 - @@LaneD journal (Desktop + CLI)

## Wave 1 - DONE (gated-green, locally merged)

Scope: cs-shell extraction + per-agent submit map (the FOUNDATION that
unblocks @@LaneC's Wave-2 survey command).

### Delivered

1. **`crates/chan-shell` crate** (new). Lifted the `cs` control-socket
   CLIENT out of the `chan` binary:
   - `cli.rs`: `ShellAction` / `TerminalAction` clap enums + `dispatch`
     (was `cmd_shell`) + `cmd_shell_search` / `cmd_shell_terminal` + the
     `render_*_markdown` helpers.
   - `control.rs`: `OpenEnv` / `open_env` / `open_env_from` /
     `control_socket_env` / `absolutize` / `send_control_request`.
   - `submit.rs`: the per-agent submit map (see below).
   - `wire.rs`: the UNIFIED `ControlRequest` / `ControlResponse`. This
     kills the old client/server duplication: chan-server now does
     `pub use chan_shell::{ControlRequest, ControlResponse}` in
     `control_socket.rs` instead of re-declaring them.
   - `lib.rs`: re-exports + `invoked_as_cs(arg0)` (the `argv[0]=="cs"`
     detection, shared by `chan`'s `parse_cli` now and by chan-desktop in
     Wave 2).

2. **Feature gate.** chan-shell `default = ["client"]`; the `client`
   feature carries clap + tokio + the transport. chan-server depends with
   `default-features = false`, so it links ONLY the serde wire types (no
   clap). Verified with `cargo tree`: chan-shell without `client` pulls
   only serde; with `client` it pulls anyhow/clap/tokio/serde_json. The
   `chan` / `chan-desktop` binaries already link clap, so no new cost
   there.

3. **Per-agent submit-encoding map** (3 consumers, one source each side):
   - `cs terminal write --submit` changed shape from a bare bool to
     `--submit=<agent>` (`claude` | `codex` | `gemini`); unset = pure
     bytes (still the default). `apply_submit_chord(data, Option<Agent>)`.
   - Chords (Rust `submit.rs` SubmitAgent::submit_chord, mirrored in
     TS `web/src/terminal/submitMode.ts` AGENT_SUBMIT_CHORDS):
     claude = `\x1b[27;9;13~` (Cmd+Enter modifyOtherKeys), codex = `\r`,
     gemini = `\r`.
   - `submitMode.ts` `encodeForAgentSubmit(buffer, agent="claude")` gained
     the agent arg with a claude default, so the existing TerminalTab
     caller (which uses the `AGENT_SUBMIT_CHORD` constant directly) keeps
     compiling unchanged. @@LaneC wires the team-config agent field into
     `encodeForAgentSubmit(buf, agentTarget)` in Wave 2.
   - NOTE: the plan referenced a server-side
     `terminal_sessions.rs::SubmitMode::submit_chord` as a third consumer;
     it does not exist in the current code (stale comment). The real
     consumers today are the CLI (chan-shell) and the SPA (submitMode.ts).

### Gate (all green, captured exit codes, no piped-exit masking)

`cargo fmt --check`=0, `cargo clippy --all-targets -D warnings`=0,
`cargo test`=0 (338 tests; chan-shell adds 5), `cargo build
--no-default-features`=0, web `svelte-check`=0 errors, `npm run build`=0,
vitest terminal+TerminalTab 65 passed. chan-desktop compiles clean under
clippy (it does not yet depend on chan-shell; that is Wave 2). gateway
nested workspace untouched (not in scope; release gate only).

### Wire-smoke (the gate-blind risk: serde/clap drift breaks cs at runtime)

Served a throwaway drive from a renamed binary copy (`/tmp/chan-laned`,
`cs` symlink), scoped pkill to my own drive path. Round-tripped EVERY cs
command over the control socket - both the `cs` alias (argv0 rewrite) and
the `chan shell` long form, plus prefix inference (`cs t l`, `cs s`,
`cs o`):
  - terminal list (+ --json, --json --pretty) - OK
  - search (+ --json, --limit, prefix) - OK
  - terminal write (selector validation, server round-trip) - OK
  - terminal restart - OK
  - open / graph / dashboard (--carousel-off, --carousel-index) /
    terminal new - OK (server "request queued" replies)
  - `--submit=bogus` rejected by clap with [claude, codex, gemini]; valid
    agents accepted.
Every reply was a DOMAIN reply, never a serde "invalid control request" -
the unified wire contract holds.

### Real agent submit smoke (@@Host: "we absolutely need this")

Drove a live chan terminal in the browser (a connected SPA window):
  - `cat -v` echo: `--submit=claude` delivered `CLAUDE^[[27;9;13~` -
    the claude chord byte-exact; `--submit=codex`/`gemini` delivered `\r`
    (acted as the line-submit).
  - **gemini**: typed a prompt with NO submit (it sat in the box,
    unsubmitted), then sent a lone `\r` via `--submit=gemini ''` - the
    box cleared and the prompt was accepted (queued, MCP still
    initializing). gemini submits on `\r`: CONFIRMED LIVE (this grounds
    the plan's "gemini = TBD probe live").
  - **codex**: same probe - prompt sat unsubmitted, then a lone `\r` via
    `--submit=codex ''` submitted it and codex replied "PONG". codex
    submits on `\r`: CONFIRMED LIVE.
Torn down: server killed (scoped), drive removed + unregistered, staged
binary/symlink + browser tab cleaned. Other lanes' servers/tabs left
untouched.

## INCIDENT (RESOLVED): a commit-collision with @@LaneB, then a clean split

Sequence (from the reflog):
1. I staged only my explicit paths but split the audit and the commit
   across two shell invocations to eyeball the staged set.
2. In that window @@LaneB ran a blanket `git add`/`commit -a`, which swept
   my staged files into their `adb68241 feat(editor)` commit.
3. @@LaneB caught it and `git reset HEAD~1`, putting both lanes' changes
   back in the working tree, then re-committed JUST their 5 editor files
   as `b273e0b5` (verified: no chan-shell / submitMode / main.rs in it).
4. I then committed my work cleanly as
   `68a2adef feat(shell): extract cs client into chan-shell crate +
   per-agent submit map` - exactly my 14 files, post-commit `git show
   --stat HEAD` audited, no foreign files.

Final history is clean: b273e0b5 (editor, @@LaneB) + 68a2adef (cs-shell,
me) as two separate commits on main, tree clean. No shared-history rewrite
was needed from me; @@LaneB's self-correction did the split.

Lesson (sharpened, now in my standing notes): the chained
`git add && audit && git commit` is still NOT race-proof - the commit
commits the INDEX, so a peer's `git add` in the window contaminates it.
The race-proof form is `git commit -F msg -- <explicit pathspec>` (commit
only those paths regardless of index). My second attempt used
`git add <paths> && <staged-set guard> && git commit`; the guard would
have ABORTED on contamination, and this time the staged set matched
exactly, so it landed clean.

## Touch points

- C<-D: **chan-shell is landed** -> @@LaneC's Wave-2 `cs terminal survey`
  command (a new `TerminalAction` + `ControlRequest` variant) now has its
  home. The survey TRANSPORT (the `TermSurvey` frame + reply round-trip) is
  MY Wave-2 work per the survey contract; @@LaneC routes the payload/reply
  shape through me.
- The submit map (`chan_shell::SubmitAgent` + `submitMode.ts`
  AGENT_SUBMIT_CHORDS) is ready for @@LaneC's team-config per-member agent
  field.

## Deferred to Wave 2 / 3 (not in Wave 1)

- chan-desktop `shell` + `argv[0]=="cs"` (depends on chan-shell).
- Remove `chan open`; move the OS file-association + handoff into
  chan-desktop. (`chan open` still exists, now calling chan_shell helpers.)
- Linux AppImage `cs` story (confirm option with @@Architect; I lean the
  `~/.local/bin` wrapper on first run).
- `cs terminal survey` TRANSPORT + the SPA overlay reply round-trip.
- Full multi-agent submit / team-work plumbing smoke with @@LaneC (Wave 3).

## Wave 2 - IN PROGRESS

### Survey transport - DONE (code-complete; my files clippy+test green)

Built the full D-side of the `cs terminal survey` seam per
`round-3-survey-contract.md` + the 2026-06-01 amendment. Files (all mine):

- `chan-shell/wire.rs`: `ControlRequest::TermSurvey { tab_name, tab_group,
  spec }` (selector inlined like TermWrite/TermRestart, not a sub-struct -
  internal to D, C never sees the request shape). `SurveySpec { surveyId,
  title, bodyMarkdown, options, allowFollowup, followup }` and
  `SurveyFollowup { dir, from, to }`, serde camelCase. `SurveyReply`
  internally tagged on `kind` ("option" | "followup"), camelCase, with a
  `survey_id()` accessor. Nullable fields (`title`, `followup`) serialize
  as `null` (NOT skipped) so the SPA frame matches the contract's
  `string | null` / `{...} | null` literally.
- `chan-shell/cli.rs`: `TerminalAction::Survey` + `cmd_shell_survey` +
  `resolve_followup`. Flags: `--tab-name`/`--tab-group` (>=1 required),
  `--title`, `--option` (1..=4), `--followup` (clap `requires =
  "followup_dir"`), `--followup-dir`, `--from`/`--to` overrides, `--stdin`,
  positional `body`. Followup precedence (amendment): from <- $CHAN_TAB_NAME
  -> --from; to <- --tab-name -> --tab-group -> --to; dir <- --followup-dir.
  Reply prints to STDOUT (the other cs cmds eprintln their queued-ack).
  3 unit tests on `resolve_followup` lock the precedence + the required-field
  bails.
- `chan-server/survey.rs` (NEW, D-owned): `SurveyBus` =
  `Mutex<HashMap<survey_id, oneshot::Sender<SurveyReply>>>` + AtomicU64
  id counter. `register() -> (id, rx)`, `cancel(id)`, `complete_survey(id,
  reply) -> bool`. 4 unit tests.
- `chan-server/control_socket.rs`: `WindowCommand::OpenSurvey { survey }`;
  `handle_request` is now `async` (the one blocking arm); `handle_survey`
  resolves the selector to owning window_id(s), mints + stamps survey_id,
  parks the oneshot, fans the `open_survey` frame to each window, awaits the
  reply, formats stdout. Other arms unchanged + still sync-fast.
- `chan-server/terminal_sessions.rs`: `window_ids_matching(tab_name,
  tab_group) -> Vec<String>` (distinct owning windows; the "survey dispatch"
  piece). A survey is an SPA-window affordance, so the tab selector resolves
  to windows, not PTYs.
- `chan-server/state.rs` + `lib.rs`: `AppState.survey_bus:
  Arc<SurveyBus>`, created in `build_app` before the control socket (passed
  to `control_socket::start` as a new param) and cloned onto AppState.

### THE FRAME SHAPE @@LaneC NEEDS (only detail not already in the contract)

The overlay is delivered as the EXISTING `window_command` envelope, filtered
by `window_id` like every other window command. C's `handleWindowCommand`
adds an `open_survey` case that reads `frame.survey` (a SurveySpec):

    {
      "type": "window_command",
      "window_id": "<owning window>",
      "command": "open_survey",
      "survey": {                       // a full SurveySpec, camelCase
        "surveyId": "survey-3",         // server-minted, echo in the reply
        "title": "..." | null,
        "bodyMarkdown": "...",
        "options": ["A", "B"],          // 1..=4
        "allowFollowup": true,
        "followup": { "dir": "...", "from": "...", "to": "..." } | null
      }
    }

The spec NESTS under `survey` (not flattened) so its camelCase fields do not
mix with the snake_case envelope (type/window_id/command). On [F], C echoes
the whole `survey.followup` object back in the reply POST and creates
`{dir}/followups/followup-{from}-{to}-{n}.md`.

C's side per the contract (unchanged): `POST /api/survey/reply` deserializes
`chan_shell::SurveyReply` and calls `state.survey_bus.complete_survey(id,
reply) -> bool` (false -> 404). C also registers that route in
`lib.rs::router()` - a shared-file edit to sequence with me at the barrier.

### CROSS-LANE seam debt (for @@Architect at the barrier)

- `AppState.survey_bus` field + `SurveyBus::complete_survey` are marked
  `#[allow(dead_code)]` with a pointer comment, because their ONLY consumer
  is C's not-yet-landed reply route. Producer side (register/cancel) is live.
  DROP both allows when C's route lands, else clippy flags the now-used items
  as having a needless allow.
- Adding `AppState.survey_bus` rippled into 4 inline test AppState
  constructors I had to touch (mechanical one-liners): `routes/{index,
  reports_toggle, screensaver, search}.rs`. Flag for the barrier in case a
  peer is mid-edit on those test modules.
- BLOCKED my full-crate clippy: `routes/team_config.rs:339` (C's UNCOMMITTED
  WIP) trips `clippy::type_complexity` (`&dyn Fn(&(String,String,String,
  &str)) -> usize`). Not mine; my own files are clippy+test green
  (`cargo check -p chan-server --all-targets` = 0, chan-shell clippy/test =
  0). C must fix before C's merge; the architect barrier gate on the merged
  HEAD is the authoritative one.

### chan-desktop `cs` dispatch (Theme-2) - DONE (pending desktop build verify)

- `chan-shell/cli.rs`: added `CsCli` (top-level `#[derive(Parser)]`,
  `infer_subcommands`) + `pub async fn run_cs<I>(args)`: parse a full `cs`
  argv (argv[0] included) and dispatch. Exported from lib.rs.
- `desktop/src-tauri/Cargo.toml`: added `chan-shell = { workspace = true,
  features = ["client"] }`. The WORKSPACE dep is `default-features = false`
  (chan-server pulls it wire-only), so desktop MUST opt into `client`
  explicitly; Cargo unifies to one build with client on.
- `desktop/src-tauri/src/main.rs`: `run_as_cs_if_requested()` mirrors
  `run_hidden_mcp_proxy_if_requested` - a pre-GUI argv probe. If
  `chan_shell::invoked_as_cs(argv[0])`, build a current-thread runtime,
  `chan_shell::run_cs(argv)`, exit; else normal GUI launch. Wired into
  `main()` right after the mcp-proxy probe, before init_tracing/GUI.

### DECISION NEEDED from @@Architect (Theme-2: remove `chan open` + file handler)

Grounded facts (read before deciding):
- `chan open` does double duty: (1) inside a chan terminal -> OpenPath over
  the control socket == EXACTLY what `cs open` (ShellAction::Open) already
  does; (2) outside -> registry longest-prefix match (`pick_workspace_root`)
  + `maybe_handoff_to_desktop`. `maybe_handoff_to_desktop` is ALSO used by
  `chan serve` (main.rs:1140), so it STAYS in the CLI; only `cmd_open` +
  `workspace_root_for` + `pick_workspace_root` are open-only.
- tauri.conf.json has NO `bundle.fileAssociations` today. The only OS hook
  is the `chan://` deep-link scheme (auth callbacks). So "double-click a .md
  in Finder -> chan-desktop" does NOT currently work via Tauri; it only
  worked if a separately-installed `chan` CLI was the registered handler ->
  handoff UDS -> desktop. Desktop ships NO `chan` binary, so desktop-only
  users never had Finder file-open.
- Making "the GUI app the file handler directly" (the lane-doc directive)
  therefore requires a NEW capability: `bundle.fileAssociations` for
  `.md`(/`.markdown`) + a `RunEvent::Opened { urls }` handler that resolves
  the owning workspace (pick_workspace_root moves to desktop) and opens it.
- A `.md` fileAssociation is a SYSTEM-WIDE claim: chan-desktop becomes the
  default handler for EVERY markdown file (the current chan-open only opened
  files already inside a registered workspace; a fileAssociation cannot
  pre-filter). And it is macOS-bundle behavior I CANNOT verify in this
  env (needs a built+installed .app + a real Finder double-click; WKWebView,
  not Chrome). It would ship empirically-unverified.

Question (1 topic): file-handler scope for this round?
  (A) Directive as written: wire `.md`+`.markdown` fileAssociations +
      RunEvent::Opened in chan-desktop, delete `chan open`. Ships
      empirically-unverified (per the pre-release-merge-unverified norm:
      merge gated-green, record unverified, re-report if it breaks). Note
      the system-wide-default-.md-handler UX.
  (B) Split: delete `chan open` NOW (cs open covers in-terminal; serve
      handoff unchanged), and carve "desktop becomes OS file handler" into a
      dedicated desktop task with a real .app verify loop (the system-wide
      claim + unverifiability warrant a focused pass, not a blind Wave-2
      bundle).
  (C) Defer both: keep `chan open` this round.

@@LaneD lean: (B). It removes the redundant `chan open` (verifiable: `cs
open` covers the in-terminal case) without making an unverifiable
system-wide file-type claim blind. HELD on this decision; not deleting
`chan open` until @@Architect rules, to avoid a file-open regression.

RESOLVED 2026-06-01: @@Architect ruled OPTION (B). Done:
- `chan open` DELETED from the CLI: the `Command::Open` variant, its
  dispatch arm, `cmd_open`, `workspace_root_for`, `pick_workspace_root`, and
  the `pick_workspace_root_longest_prefix_wins` test. `maybe_handoff_to_desktop`
  KEPT (still used by `chan serve`). `chan open` doc/error references in
  chan-shell cli.rs + control_socket.rs updated to `cs open`.
- The desktop-as-OS-file-handler half (`bundle.fileAssociations` +
  `RunEvent::Opened`) is CARVED to round-4 (unverifiable here + system-wide
  `.md` claim = @@Host call). Not touched this round.

### AppImage `cs` wrapper (Theme-2, option (a)) - DONE

`desktop/src-tauri/src/cs_install.rs` (NEW): on launch FROM an AppImage
(`$APPIMAGE` set), drop `~/.local/bin/cs` = a bash wrapper that
`exec -a cs "$APPIMAGE" "$@"` so argv[0]=="cs" survives AppImage AppRun
(a symlink is unreliable: `current_exe()` inside an AppImage points into the
ephemeral mount + AppRun can reset argv[0]). Best-effort + idempotent: skips
a foreign `cs` (no marker), self-heals a stale target, never fatal. Wired
into `main()` after init_tracing. 5 unit tests on the pure plan/quote/script.
No-op on macOS/dev (no $APPIMAGE). EMPIRICALLY-UNVERIFIED: no Linux AppImage
build in this env (the wrapper content + plan logic ARE unit-tested; the
real AppImage re-exec is a round-4 / @@Host verify on a built AppImage).

## Wave 2 - COMPLETE (gated-green; barrier-sequenced merge to @@Architect)

### Gate (whole workspace, current shared worktree, captured exit codes)
`cargo fmt --check`=0 (after `cargo fmt -p chan-shell`), `cargo clippy
--all-targets -D warnings`=0, `cargo test`=0 (chan 57, chan-shell 13 inc. 5
new survey-wire serde tests + 3 resolve_followup, chan-server lib inc. my
survey:: bus tests + C's routes::survey tests), `cargo build
--no-default-features`=0. NOTE: an earlier full clippy tripped on C's
then-WIP `team_config.rs:339` (type_complexity); C has since fixed it and the
whole-tree clippy is green now. web svelte-check / npm build NOT re-run: I
touched no `web/` files this wave (B/C territory; the barrier covers it).

### Wire-smoke (the gate-blind serde/clap risk)
Built `target/debug/chan` + used the built `chan-desktop`:
- survey clap validation: no --option -> "1..=4"; no selector -> selector
  error; 5 options -> "1..=4"; `--followup` without `--followup-dir` -> clap
  "required". OK.
- existing cs commands vs the LIVE desktop control socket (read-only):
  `terminal list` + `search` round-trip -> the chan-shell refactor
  (run_cs/CsCli + the survey addition) did NOT break the existing wire.
- `cs` argv0 alias via a `cs`->chan symlink: `cs t l` prefix-infers to
  `terminal list`; `cs terminal survey` reachable through the alias.
- TASK-2 core: the built `chan-desktop` symlinked as `cs` runs the control
  client (`cs t l`, `cs search` both rc=0), NOT the GUI. Desktop users get
  `cs` without a `chan` binary: PROVEN.
- 5 serde round-trip tests in wire.rs pin the exact survey JSON (camelCase,
  null title/followup, `kind` tag, `term_survey` request) so a Rust rename
  cannot silently drift the C-facing format.

### EMPIRICALLY-UNVERIFIED (told @@Architect)
- Survey HAPPY-PATH e2e (open_survey overlay renders -> user picks -> C's
  `POST /api/survey/reply` -> bus completes -> blocked CLI prints): needs C's
  SPA overlay + a browser. This is the Wave-3 joint smoke with @@LaneC (an
  explicit Wave-3 item). The transport PIECES are covered (bus unit tests,
  wire serde tests, window_ids_matching shares write_input_matching's tested
  match path, C's routes::survey tests).
- AppImage `cs` wrapper re-exec on a real Linux AppImage (no AppImage build
  here; content + plan unit-tested).

### File inventory (this wave)
Mine (clean): chan-shell/{wire,cli,lib}.rs; chan-server/{control_socket,
state,terminal_sessions}.rs + survey.rs(new); chan/src/main.rs; desktop/
src-tauri/{Cargo.toml,src/main.rs,src/cs_install.rs(new)}; Cargo.lock(+1).
Test-ripple from AppState.survey_bus: chan-server/routes/{index,reports_toggle,
screensaver,search}.rs (one-line survey_bus field each).

### MERGE NOTE for @@Architect (do NOT let me solo-commit this)
The survey feature is a C+D co-merge entangled in SHARED files - committing
my files alone would either sweep C's work or break compile:
- `lib.rs`: CO-EDITED (mine: build_app survey_bus creation + control_socket::
  start param + `mod survey` + AppState field; C's: router `/api/survey/reply`
  + import). My server files won't compile without my build_app edits, and
  those live in the same file as C's router edits.
- `routes/mod.rs`: C-edited (exports `api_survey_reply`).
- `routes/{index,search}.rs`: my one-line survey_bus test-field MAY overlap
  @@LaneA's Wave-2 IDX/search edits - architect has the cross-lane view.
- `AppState.survey_bus` + `SurveyBus::complete_survey` carry `#[allow(dead_code)]`
  (pointer comments); DROP both now that C's reply route consumes them (the
  barrier-sequenced state has the consumer present).
Recommend: @@Architect sequences the chan-server survey integration as one
coherent C+D(+A) barrier merge + gates the final HEAD. chan-shell, chan
main.rs (chan-open removal), and desktop are cleanly D-owned if a split
commit is wanted, but they pair with the chan-server side to compile
coherently. Left ALL gated-green + uncommitted for the barrier (vs Wave-1's
disjoint per-lane commits) precisely because of this entanglement.

## Wave 3 - DONE (verification + smoke only; NO code change)

Scope: the JOINT survey browser smoke with @@LaneC + the desktop verifies that
need a real .app + the multi-agent submit / team-work plumbing smoke. Wave 3 was
verification-only on the merged HEAD 08d7435b: I wrote no product code, so the
Wave-2 barrier gate stands. I rebuilt `chan-desktop` fresh from HEAD purely for
the cs argv0 verify (the pre-existing target/debug binary predated the merge by
2 min). Served a throwaway drive from a renamed binary copy (/tmp/chan-laned,
drive /tmp/chan-laned-w3, :7841), scoped every kill to my own pid/path, torn down
after.

### Task 1 - survey OPTION-PICK e2e: PASS (browser, my :7841)
The full chain, empirically, in Chrome (Blink SPA):
`cs terminal survey --tab-name=Terminal-1 --title=... --option x3 <md-body>` ->
CLI BLOCKS -> the `open_survey` window_command reached the owning SPA window ->
BubbleOverlay rendered the title, the markdown body (a `##` heading AND `**bold**`
both rendered), and the 3 vertically-numbered options, with NO [F] (correct: no
`--followup`) -> clicked [1] "Ship it" -> `POST /api/survey/reply` completed D's
bus oneshot -> the blocked CLI printed `Ship it` to stdout and exited 0 -> overlay
dismissed cleanly back to the terminal. Zero console errors. This single path
exercises the ENTIRE C+D integration: my transport (control_socket handle_survey
+ window_ids_matching) + my bus (register/await/complete) + my CLI stdout + C's
overlay render + C's reply route. The Wave-2 "survey SPA render + bus completion"
empirically-unverified item is now VERIFIED.

### Task 2 - survey [F] followup e2e: COVERED by @@LaneC (joint smoke)
Cross-lane browser COLLISION caught + resolved: @@LaneC was running their own
survey smoke on :7901 while I was on :7841, both driving the SHARED Chrome MCP
tab group. Mid-run the shared tab got navigated from my :7841 to C's :7901,
which orphaned my first `--followup` survey (the overlay was pushed once then
lost to the nav; the blocked CLI never got a reply). I killed that stuck CLI,
verified my server + Terminal-1 survived server-side, and POKED @@LaneC to
deconflict. C replied they had completed the FULL browser smoke on :7901 -
option click + keyboard 1..N + the [F] followup FILE-CREATE - all PASS. Since
C built from the same merged HEAD 08d7435b, their [F] reply round-tripped
through MY survey bus + control_socket `format_survey_reply` ("new follow up
file created: ...") + the blocked CLI's stdout, so their PASS validates D's
followup transport too. Joint coverage split, recorded: D = option-pick e2e on
:7841; C = keyboard + [F]-followup on :7901. No duplicate [F] run by me (C
stood down the browser; re-fighting the shared tab for an already-covered path
was not worth the collision risk). LESSON for the retrospective: the Chrome MCP
tab group is shared across lanes exactly like the worktree is; a JOINT browser
smoke needs the same deconfliction discipline (one driver at a time, or a poke
to claim the browser) as a shared-file commit.

### Task 3 - desktop cs argv0 + chan-open removal: PASS (binary-level)
- `chan open` is GONE: `chan open <path>` -> "unrecognized subcommand 'open'";
  no `open` in `chan --help`. The in-terminal open path is fully covered by
  `cs open` (`chan shell open`, "Open a path in the current window"), retained
  and help-verified.
- chan-desktop cs argv0 dispatch: PASS. Rebuilt chan-desktop fresh from HEAD,
  symlinked it as `cs` (stem must be exactly `cs`; `invoked_as_cs` checks
  `file_stem()=="cs"` - my first attempt named it `cs-desktop` and it correctly
  fell through to the GUI, a TEST-harness error, not a product bug). Invoked as
  `cs` it ran the CONTROL CLIENT and exited (NOT the GUI): `cs terminal list`,
  `cs t l` prefix-infer, and `cs search welcome` (BM25 hit) all rc=0 against my
  live :7841 server. Then proved the FULL desktop-only story: the same
  desktop-binary-as-cs talked to the REAL desktop-embedded control socket
  (@@Host's chan-desktop pid 31335) and listed all four agent terminals
  (@@LaneD/A/C/B). So a desktop-only user (no `chan` binary) gets cs + MCP
  through the chan-desktop binary aliased as `cs`: CONFIRMED.
- AppImage `cs` wrapper: the 5 cs_install unit tests pass on HEAD; the real
  Linux AppImage re-exec stays EMPIRICALLY-UNVERIFIED (no AppImage build in
  this env) -> round-4 / @@Host verify on a built AppImage.
- File-association handoff: the desktop-as-OS-.md-handler half was CARVED to
  round-4 (@@Host's "i do not want that, at all" + the system-wide unverifiable
  claim); `maybe_handoff_to_desktop` is KEPT (still used by `chan serve`,
  main.rs:1235). No tauri.conf.json fileAssociations added. Nothing desktop-only
  to verify here beyond the cs dispatch above.

### Task 4 - multi-agent submit / team-work plumbing smoke: PASS
- Submit-map chord bytes are byte-identical Rust<->TS: claude=`\x1b[27;9;13~`,
  codex=`\r`, gemini=`\r` (submit.rs SubmitAgent::submit_chord ==
  submitMode.ts AGENT_SUBMIT_CHORDS). chan-shell `submit` unit tests pass
  (apply_submit_chord asserts the exact per-agent bytes + trailing-newline
  strip).
- C's bootstrap.md generator renders the per-agent poke chords:
  `bootstrap_roster_shows_agent_and_per_agent_poke_chords` +
  `bootstrap_contains_team_host_lead_and_poke_chord` pass (11 team_config tests).
- Wire-level `--submit` validation on the live server: claude/codex/gemini all
  parse + reach the server well-formed; `--submit=bogus` rejected by clap with
  [claude, codex, gemini].
- Live codex + gemini auto-submit (real agents, `\r`) was CONFIRMED LIVE in
  Wave 1 (codex "PONG", gemini accept; claude chord byte-exact via cat -v); not
  re-run this wave - three independent forms of evidence (unit + wire + Wave-1
  live) cover it, and spinning up live codex/gemini again was disproportionate.

### Empirically-unverified (told @@Architect)
- AppImage `cs` wrapper real re-exec on a built Linux AppImage (no AppImage in
  this env; content + plan are unit-tested).
- Desktop file-association OS-.md-handler: dropped/carved to round-4 (@@Host
  call), not a Wave-3 deliverable.

### Teardown
Killed only my server (pid 6232, :7841); unregistered /tmp/chan-laned-w3; rm'd
the temp drive + staged binaries (/tmp/chan-laned, /tmp/chan-laned-desktop) +
out logs. @@LaneC's :7901 + @@LaneB's :8799 were torn down by those lanes; only
@@Host's desktop (31335) remains, as expected. My Chrome tab was closed by
@@LaneC on their standdown; the remaining :8799 tab is @@LaneB's, left untouched.
No stray chan-desktop-fresh GUI.
