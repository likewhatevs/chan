# Round-3 survey contract (C<->D seam, @@Architect-held)

The `cs terminal survey` feature spans @@LaneD (transport) and @@LaneC (UX +
reply + followup). This pins the shared SHAPE + ownership so the two lanes never
edit one file. Agreed at the Wave-1/2 boundary; chan-shell (68a2adef) has landed,
so both sides can start.

## End-to-end flow (synchronous, blocking CLI)

1. An agent runs `cs terminal survey --tab-name=<target> <spec>` (or a tab
   group). The CLI BLOCKS.
2. chan-shell serializes a `SurveySpec` into a new `ControlRequest::TermSurvey`
   frame over the control socket, carrying the target selector + a server-minted
   `survey_id`.
3. chan-server's TermSurvey handler raises the SPA overlay on the target tab(s)
   via a `WindowCommand` (the existing tab-targeted push), parks a oneshot keyed
   by `survey_id` in a survey bus, and awaits it.
4. The SPA renders the overlay (markdown body + up to 4 options + optional [F]),
   the user picks. The SPA POSTs a `SurveyReply` (echoing `survey_id`) to the
   reply route.
5. The reply route looks the `survey_id` up in the survey bus and completes the
   oneshot. The blocked TermSurvey handler returns the reply over the control
   socket. The CLI prints the result to stdout and exits 0:
   - option pick  -> the chosen option label (or `[N] label`).
   - follow-up    -> `new follow up file created: {team-dir}/followups/...md`.

## Shared SHAPE (pin both sides to this JSON, byte-identical)

Request (CLI/chan-shell -> server -> SPA), serde camelCase:

    SurveySpec {
      surveyId: string,        // server-minted; SPA echoes it back
      title: string | null,    // optional heading
      bodyMarkdown: string,    // the problem description, rendered as markdown
      options: string[],       // 1..=4 option labels; SPA numbers them [1]..[4]
      allowFollowup: boolean,  // render the [F] follow-up affordance
      followup: {              // team context for the [F] path; null unless
        dir: string,           //   --followup was set. See the 2026-06-01
        from: string,          //   amendment at the bottom of this doc.
        to: string
      } | null
    }

Reply (SPA -> reply route -> blocked CLI), serde camelCase + internally tagged:

    SurveyReply =
      | { surveyId: string, kind: "option",   optionIndex: number, optionLabel: string }
      | { surveyId: string, kind: "followup", followupPath: string }

`followupPath` is workspace-relative (`{team-dir}/followups/followup-...md`),
created by @@LaneC before replying, pre-populated per the plan (header/title,
date+time, the "Agents: this is a follow up, not ready; check again later" line,
the original prompt, and @@Host comment placeholders).

## Ownership split (so two lanes never edit one file)

@@LaneD (transport + the shared Rust type + the bus):
  - `SurveySpec` / `SurveyReply` Rust types live in chan-shell `wire.rs` (the
    same crate that owns the unified ControlRequest/ControlResponse). C's TS
    types mirror this doc, not the Rust source.
  - `ControlRequest::TermSurvey { selector, spec }` in control_socket.rs.
  - The `cs terminal survey` command (a new TerminalAction in chan-shell cli.rs).
  - The server-side TermSurvey handler: mint `survey_id`, emit the WindowCommand
    to the target tab(s), park + await the oneshot, return the reply to the CLI.
  - The SURVEY BUS (a `survey_id -> oneshot<SurveyReply>` registry) and its
    AppState field. D exposes `complete_survey(survey_id, reply) -> bool` for C's
    route to call. D owns the bus internals because it is intrinsically the
    blocked-transport side.

@@LaneC (UX + reply route + followup):
  - The SPA survey overlay (replace the gutted BubbleOverlay placeholder):
    render `SurveySpec`, vertically-aligned numbered options + [F], reply
    round-trip.
  - The reply route (`POST /api/survey/reply`, a new routes/ file C owns) that
    deserializes `SurveyReply` and calls D's `state.survey_bus.complete_survey`.
  - On [F]: create the followup file, then reply with `kind:"followup"` +
    `followupPath`.
  - The team-config per-member agent field feeds the bootstrap poke encoding via
    D's landed submit map (`chan_shell::SubmitAgent` / submitMode.ts
    AGENT_SUBMIT_CHORDS).

## Notes

- D corrected a plan inaccuracy in Wave 1: there is no
  `terminal_sessions.rs::SubmitMode::submit_chord`; the real submit consumers are
  the CLI (chan-shell) + the SPA (submitMode.ts). The survey transport plan
  should target the actual code shape, not the stale reference.
- Keep the reply route + the bus on opposite sides of a stable API
  (`complete_survey`) so C's routes/ file and D's control_socket/state files do
  not overlap. If C needs a different bus signature, raise it through
  @@Architect; do not edit D's bus directly.

## AMENDMENT 2026-06-01 (@@Architect): followup carries team context

@@LaneC escalated a seam gap (round-3-part-2-lane-c.md): the `[F]` followup must
land at `{team-dir}/followups/followup-{from}-{to}-{n}.md`, but `SurveySpec`
carried no team context, and neither the SPA nor the reply route can re-derive
the team-dir for an arbitrary survey (a workspace may hold several teams). The
context originates with the surveying agent (it read bootstrap.md and knows its
own `$CHAN_TAB_NAME`), so it must flow command -> SurveySpec -> SPA -> echo back
-> C's reply route.

DECISION (approved, full shape, not the minimal fallback - it matches the
established `followup-{from}-{to}-{n}.md` naming and D is mid-flight so the wire
add is cheap now vs a retrofit):

`SurveySpec` gains `followup: { dir, from, to } | null`, populated by
`cs terminal survey` ONLY when `--followup` is set (otherwise null).

@@LaneD (wire + command):
  - `wire.rs`: add `followup: Option<SurveyFollowup>` to `SurveySpec`;
    `SurveyFollowup { dir: String, from: String, to: String }`, serde camelCase.
  - `cli.rs`: new flag `--followup-dir <team-dir>`, REQUIRED when `--followup`
    is set (clean clap error if absent, so followups are always team-scoped).
    Derive `from` <- `$CHAN_TAB_NAME` (fallback `--from`); `to` <- the survey
    target: the `--tab-name` value, or the `--tab-group` name when surveying a
    group (fallback `--to`). When `--followup` is unset, emit `followup: null`.

@@LaneC (SPA + reply route):
  - The overlay echoes the whole `followup` object back in the `[F]` POST body.
  - The reply route creates `{dir}/followups/followup-{from}-{to}-{n}.md`
    (n minted by scanning `{dir}/followups/`), pre-populates it per the plan,
    then replies `{ kind:"followup", followupPath }` to D's bus.

Ownership unchanged: D owns the wire type + command; C owns the SPA + reply
route + followup generator. C's TS mirrors this doc, not D's Rust. Everything
except the [F] end-to-end stays parallel and unblocked.

## AMENDMENT 2026-06-03 (@@Architect): open_survey frame carries the target tab

Phase-17 R2-3 (@@Alex: "survey must be per terminal, not window-wide; each
terminal could have their own survey and they should not impact each other").
Today the `open_survey` WindowCommand carries the survey but NO target tab, so
the SPA can only render one window-wide modal. To make surveys per-terminal the
frame must carry which terminal it was raised on. Purely ADDITIVE: SurveySpec,
the reply path, survey_id, and the bus are all UNCHANGED, so the existing
reply/followup contract is not disturbed.

RATIFIED SHAPE (one optional field on the frame):
  - Rust (control_socket.rs WindowCommand::OpenSurvey): add
    `tab_name: Option<String>`, serialized to the SPA as `tabName` (camelCase;
    pin the wire string with serde(rename) so a green compile can't hide a
    mismatch).
  - SPA frame: `{ command: "open_survey", survey, tabName?: string | null }`.
  - Semantics: `tabName = Some(X)` when the survey targets a specific terminal
    (`--tab-name=X`); the SPA attaches the survey to terminal X only. `None` for
    a `--tab-group` broadcast (or no specific tab) -> the SPA keeps the current
    window-wide fallback. `--tab-name` is the primary case (it is how the lead
    surveys @@Alex), so per-terminal lands there; group-broadcast stays
    window-wide.

PHASE-17 OWNERSHIP (the SPA survey overlay moved to @@LaneB this phase, who owns
BubbleOverlay.svelte; the reply route + bus are untouched so their owners are
not involved):
  - @@LaneD (transport, ~2 lines): control_socket.rs TermSurvey handler already
    knows the selector (~372); put `tab_name` into the OpenSurvey push (~91).
  - @@LaneB (SPA, atomic once the frame lands): survey.svelte.ts state keyed by
    tab id (`byTab`, the B1 rich-prompt pattern) instead of a singleton;
    store.svelte.ts open_survey handler (~1013) routes the tab to showSurvey;
    BubbleOverlay.svelte + TerminalTab render each terminal's own survey
    anchored over that terminal; api/client.ts mirrors the frame field.
    AUTHORIZED to edit the store.svelte.ts open_survey-handler region + the
    unowned survey.svelte.ts + the api/client.ts survey frame for this.
