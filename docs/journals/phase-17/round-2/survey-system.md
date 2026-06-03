# Task: survey system v2 (reach team terminals + F/Dismiss on every survey)

For @@LaneB (lead), with @@LaneD available to pair on Part C (survey overlay
UI). Identify from `$CHAN_TAB_NAME`. @@LaneA owns Part B (bootstrap template),
so do NOT touch `team_config.rs` bootstrap generation.

## Why

`cs terminal survey` is becoming the preferred channel for an agent to request
the Host's attention. Two gaps to close.

## Part A - surveys must reach team-created terminals (BUG, diagnosed)

Today `cs term survey --tab-name=@@X` to a Team-Work-dialog terminal replies
"no live terminal session matched" even though the terminal exists (writes to
it work). Root cause (verified by @@LaneA):

- The survey resolves its target via `registry.window_ids_matching` which only
  counts sessions whose `window_id` is `Some` (terminal_sessions.rs:591; the
  comment at :562 wrongly assumes every session has one).
- Team-dialog terminals are spawned via `POST /api/terminal`, whose handler
  HARDCODES `window_id: None` (routes/terminal.rs:260), then are merely
  ATTACHED over `/ws` - and `get_or_create_for_ws` does NOT rebind window_id on
  attach. So they keep `window_id = None` -> survey finds no window.
- Normal terminals are created BY the `/ws` connect (no prior POST), which sets
  window_id from the query -> survey works. That is the asymmetry.

Fix (thread the window through the team spawn):
- `web/src/api/client.ts`: add `window_id?` to `TerminalSpawnRequest`.
- `web/src/state/teamOrchestrator.svelte.ts`: pass `windowId: sessionWindowId()`
  in BOTH `api.spawnTerminal` calls (~289 lead, ~392 workers).
- `crates/chan-server/src/routes/terminal.rs`: the spawn handler honors
  `body.window_id` (normalize it) instead of hardcoding `None`.
- Verify the `cs terminal team` path too (control_socket spawn_team already
  sets window_id from the caller's `$CHAN_WINDOW_ID` - confirm it survives).
- Test: serve a workspace, spawn a team via the dialog, then
  `cs terminal survey --tab-name=@@<member> ...` -> the overlay must appear in
  that window (not "no live terminal session matched").

## Part C - every survey offers options + F-followup + Dismiss

The Host may want to follow up or dismiss instead of picking an option. Make
those STANDARD on every `cs terminal survey` (not just the opt-in
`allowFollowup`): the overlay always renders the survey options PLUS an `F`
follow-up affordance AND a `Dismiss`. A dismiss returns a distinct "dismissed"
reply (not an option index) so the asking agent can tell. Investigate the
survey overlay component (web) + the survey spec/wire (chan-shell
`SurveySpec` / the reply shape) + the route. Coordinate with @@LaneD on the UI.

## Acceptance

- A survey to a Team-Work-dialog terminal shows the overlay (Part A).
- Every survey overlay shows options + F (follow up) + Dismiss; the reply
  distinguishes option / followup / dismissed (Part C).
- Pure ASCII / no em dashes in committed text.
- Own-gate green (cargo fmt/clippy -Dwarnings/test for Rust; make web-check for
  the SPA). Report to @@LaneA; @@LaneA owns the full-tree gate. Do NOT push.

## Part C contract + ownership (proposed by @@LaneD; @@LaneB confirm/counter)

Clean non-overlapping split so we both build in parallel, the only shared
thing being the reply JSON shape:

- @@LaneD owns ALL of `web/` for Part C: `components/BubbleOverlay.svelte`,
  `state/survey.svelte.ts`, the `SurveyReplyRequest` type in `api/client.ts`,
  and the two tests (`BubbleOverlay.test.ts`, `state/survey.svelte.test.ts`).
  I keep web/ self-consistent + green via `make web-check`.
- @@LaneB owns ALL of `crates/` for Part C: `chan-shell/src/wire.rs`
  (`SurveyReply`), `chan-server/src/{survey.rs,routes/survey.rs}`,
  `chan-shell/src/cli.rs` (CLI print), plus Part A. You keep crates/ green via
  cargo. (This mirrors round-3, where the TS wire mirror lived in the web lane
  and the Rust was the other lane; the contract doc is the source of truth.)

The single coordination point is the reply JSON (SPA POSTs to
`/api/survey/reply`). Three kinds, camelCase, internally tagged on `kind`:

    { surveyId, kind: "option",    optionIndex, optionLabel }
    { surveyId, kind: "followup",  followup: <ctx|null>, title, bodyMarkdown }
    { surveyId, kind: "dismissed" }                                  <- NEW

- `dismissed` carries ONLY `surveyId` (no option index): the asking agent must
  be able to tell a dismiss apart from an answer.
- `SurveyReply` (server -> blocked CLI, `chan-shell/src/wire.rs`): add a
  `Dismissed { survey_id }` variant (serde tag "dismissed"), wire it into
  `survey_id()`, and have the CLI print a clear `survey dismissed` line so the
  agent sees it. The route (`routes/survey.rs`) completes the parked oneshot
  with that variant.

Two changes Part C needs on the SurveySpec / followup side (@@LaneB):

1. F is now STANDARD (shown on every survey), so the SPA no longer gates F on
   `allowFollowup`. RECOMMEND: the server now ALWAYS populates `followup`
   context (derive `from` = surveying agent `$CHAN_TAB_NAME` or a sane default,
   `to` = target tab/group, `dir` = team dir or the workspace `followups/`), so
   F always writes a paper-trail file and stays cleanly distinct from Dismiss
   (F = defer-with-file; Dismiss = dropped, no file). `allow_followup` then
   becomes vestigial and can be dropped (pre-release, no back-compat).
   FALLBACK if some surveys genuinely have no context: the SPA will send
   `followup: null` and the route must accept it as a plain deferral (no file).
   The SPA side is built to tolerate either; your call which the route does.

I am building the web side now against this shape. If you want a different
`kind` name, F-without-context behavior, or ownership boundary, say so and I
will adjust; the web slice is small.

### Part C web slice status (@@LaneD) - DONE, own-gate green

Built against the contract above. Files (all web/, no crates/ overlap):
- `api/client.ts`: `SurveyReplyRequest` += `{ surveyId, kind: "dismissed" }`;
  the `followup` variant now allows `followup: SurveyFollowupContext | null`.
  (@@LaneA-ratified DROP now DONE: `allowFollowup` removed from `SurveySpec` +
  both my test fixtures + the stale comments, in lockstep with @@LaneB's Rust
  drop (wire+cli+control_socket). Re-gated GREEN: svelte-check 0 errors, vitest
  1670/1670, build clean. No `allowFollowup` left anywhere in web/src.)
- `state/survey.svelte.ts`: `requestFollowup` no longer gates on
  allowFollowup/context (sends `followup: spec.followup ?? null`); new
  `dismissSurvey(slot)` posts `{ kind: "dismissed" }`, clears the slot, busy-
  guarded, notifies on failure.
- `components/BubbleOverlay.svelte`: F + Dismiss render on EVERY survey in a
  `.survey-actions` row (F dashed = soft defer, takes width; Dismiss solid =
  firm drop, right). Keys: 1..N option, f/F followup, Escape dismiss (a real
  reply now, so no hang-on-silent-close; handled keys stopPropagation so Escape
  does not bubble out to close other overlays).
- Tests updated: `survey.svelte.test.ts` (followup-without-context posts null;
  dismiss posts + clears only that slot; failed-dismiss keeps survey + clears
  busy) and `BubbleOverlay.test.ts` (F + Dismiss render on the bare default
  spec).

Gate: svelte-check 0 errors (1 pre-existing RichPrompt warning, not mine);
full vitest 1670/1670 (incl the real-Svelte jsdom mount of the overlay); vite
build clean. The web side compiles + unit-passes independently of the Rust.

INTEGRATION SMOKE (deferred to the joint pass, needs your route): raise a real
survey, confirm the overlay shows options + F + Dismiss, click/Escape Dismiss
-> asking agent receives a "dismissed" reply (not an option). The dismiss
round-trip can't complete until `routes/survey.rs` handles `kind:"dismissed"`,
so that smoke is the natural joint step once your crates/ slice lands.

### Part C crates/ slice + ownership CONFIRMED (@@LaneB) - DONE, own-gate green

Contract + ownership split ACCEPTED as proposed. The reply JSON shape (3 kinds,
camelCase, `dismissed` = surveyId only, `followup` context nullable) is exactly
what landed in Rust. Files (all crates/, no web/ overlap):
- `chan-shell/src/wire.rs` `SurveyReply`: `followup_path` -> `Option<String>`
  (skip when None); new `Dismissed { survey_id }` (serde tag "dismissed");
  `survey_id()` arm added. Wire tests pin all three kinds + the no-path
  deferral.
- `chan-server/src/routes/survey.rs` `SurveyReplyRequest`: `followup` ->
  `Option<FollowupContext>`; new `Dismissed`. Route: Some(ctx) -> create file ->
  `Followup{Some(path)}`; None -> `Followup{None}` (no file); Dismissed ->
  `SurveyReply::Dismissed`. Tests pin null-context + dismissed deserialization.
- `chan-server/src/survey.rs` (bus): Followup test updated to `Some(..)`.
- `chan-server/src/control_socket.rs` `format_survey_reply`: distinct stdout
  lines per kind - option label / "new follow up file created: PATH" /
  "host deferred; no follow up file created" / "survey dismissed; no answer".
  The CLI prints this verbatim, so an asking agent can tell answer vs defer vs
  dismiss from stdout. (No cli.rs change: it already prints the server message.)

DECISION on F-without-context: I took the FALLBACK (route accepts
`followup: null` as a plain deferral, no file), NOT the "server always populates
context" recommendation. Auto-deriving `dir = workspace followups/` would write
a file into the user's workspace for EVERY deferred survey (incl. non-team
ones); the CLI already opts in to a file via `--followup --followup-dir`, so
F-with-context writes a file and F-without is a bare defer. @@LaneD's SPA
tolerates either, so no web change needed.

`allow_followup` DROPPED (per @@LaneA's ratification: pre-release, no
back-compat, F is standard). Removed from the Rust `SurveySpec` (wire.rs), the
CLI `SurveySpec` construction + the three after-help example JSONs + the
`--followup` flag help (the flag still gates whether `followup` CONTEXT is
attached, it just no longer sets a render flag), and the control_socket test
fixture. crates/ re-gated green. @@LaneD's half of the synchronized drop: remove
`allowFollowup` from `client.ts` `SurveySpec` + the 2 test fixtures (the overlay
already ignores it). My Rust drop does NOT break the web gate in the meantime:
the field just goes unpopulated at runtime and nothing reads it. Updated blobs:
chan-shell/wire.rs + cli.rs + chan-server/control_socket.rs (re-fingerprint at
commit time).

Own-gate GREEN:
- crates: cargo fmt --check + clippy -p chan-shell -p chan-server --all-targets
  -D warnings + test (chan-server 402, chan-shell 45, 0 failed).
- web (Part A touches types.ts + teamOrchestrator.svelte.ts; ran the whole
  gate): make web-check GREEN (svelte-check 0 errors, vitest 1670/1670, build
  clean).

### Part A status (@@LaneB) - DONE, own-gate green

Implemented the diagnosed fix:
- `chan-server/src/routes/terminal.rs`: `CreateTerminalBody` += `window_id`
  (serde default); `api_create_terminal` now binds
  `body.window_id.as_deref().and_then(normalize_window_id)` instead of the
  hardcoded `None`.
- `web/src/api/types.ts`: `TerminalSpawnRequest` += `window_id?: string`.
- `web/src/state/teamOrchestrator.svelte.ts`: import `sessionWindowId`; both
  `api.spawnTerminal` calls (lead + workers) pass `window_id: sessionWindowId()`.
- Confirmed the `cs terminal team` path is unaffected: `control_socket`
  `spawn_team` still binds `window_id` from the caller's `$CHAN_WINDOW_ID`
  (control_socket.rs ~500). The fix targets the team-DIALOG (POST
  /api/terminals) path specifically, which was the `window_id = None` source.

EMPIRICAL SMOKE (Part A + the joint Part C smoke) is the natural @@LaneC step
once the server is rebuilt: serve a workspace, spawn a team via the dialog, then
`cs terminal survey --tab-name=@@<member> ...` -> overlay appears in that window
(was "no live terminal session matched"); plus the dismiss round-trip.

Blob fingerprints (my product files):
  routes/terminal.rs    6e310170   routes/survey.rs   d67baa97
  control_socket.rs     d23a9690   survey.rs (bus)    d0cd0ceb
  chan-shell/wire.rs    7e795db9   api/types.ts       d9579f92
  teamOrchestrator.svelte.ts       cd5462af

### Part C integration smoke (@@LaneD) - build + WIRE green (live visual deferred)

Both slices are in this shared tree, so I smoked the INTEGRATION at the level
the sandbox allows (cs can't connect cross-process here, a known sandbox UDS
limitation, so the human-loop visual is deferred - see below).

- Combined build: `cargo build -p chan` GREEN (17.4s, 0 errors) - my embedded
  web bundle + @@LaneB's crates link. My `make web-check` (svelte-check 0,
  vitest 1670/1670, build clean) already covered the combined web tree (it
  includes @@LaneB's Part A `types.ts` + `teamOrchestrator.svelte.ts` edits;
  no file overlap with my Part C web files).
- WIRE smoke (served a throwaway workspace from a renamed binary on private
  port 4717, POST /api/survey/reply with a bearer token, then torn down -
  port free, temp dir removed, registry entry unregistered): all THREE Part C
  reply shapes deserialize cleanly into the route ->
      {kind:"dismissed", surveyId}                  -> 404 "no survey parked" (parsed OK)
      {kind:"followup", followup:null, bodyMarkdown} -> 404 "no survey parked" (parsed OK)
      {kind:"option", optionIndex, optionLabel}      -> 404 "no survey parked" (parsed OK)
  Control: {kind:"bogus"} -> 422 "unknown variant `bogus`, expected one of
  `option`, `followup`, `dismissed`" - proves the route's accepted variant set
  is EXACTLY the three my SPA sends. The followup:null path (LaneB's bare-defer
  fallback) deserializes, confirming no web change is needed for that decision.
- DEFERRED (sandbox cs UDS limitation, same as B5): the live human-loop visual
  - raise a real `cs terminal survey`, see the overlay render options + F +
  Dismiss, pick Dismiss/Escape, confirm the asking agent's CLI prints `survey
  dismissed` (and Part A: the overlay reaches a team-dialog terminal). Needs a
  real machine or a webtest lane with a working cross-process `cs`. Everything
  up to that final human loop (render via the real-Svelte mount test; wire via
  the curl smoke; build) is empirically green.
