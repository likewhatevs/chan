# Phase-15 round-3 part-2 - @@LaneC Wave-2 plan (Team Work + Survey)

Wave-2 implementation plan, grounded against the current tree (HEAD 68a2adef +
@@LaneD's in-flight uncommitted chan-shell survey types). Read after the lane
doc + the survey contract.

## State of the seam (as observed 2026-06-01)

@@LaneD's survey TRANSPORT is mid-flight in the working tree (uncommitted):
  - `chan-shell/src/wire.rs`: `SurveySpec` + `SurveyReply` + `ControlRequest::
    TermSurvey` ALL landed (camelCase, internally-tagged reply on `kind`).
  - `chan-shell/src/cli.rs`: `cs terminal survey` command landed
    (--tab-name/--tab-group, --title, repeated --option 1..=4, --followup bool,
    --stdin, positional body).
NOT yet landed (still D's Wave-2): the server-side TermSurvey handler, the
`open_survey` WindowCommand variant, the survey BUS + `complete_survey`, and the
AppState field.

D's SurveySpec shape (the source of truth my TS mirrors):

    SurveySpec { surveyId, title?, bodyMarkdown, options[1..=4], allowFollowup }

D's SurveyReply shape:

    { kind:"option",   surveyId, optionIndex, optionLabel }
    { kind:"followup", surveyId, followupPath }

## SEAM GAP (escalated to @@Architect): followup needs team context

`[F]` must create `{team-dir}/followups/followup-...md`. But D's SurveySpec
carries NO team-dir / from / to, and the SPA does not know the team-dir for an
arbitrary survey raised over a tab. So C's reply route cannot compute the
followup path. The team-dir originates with the AGENT that raised the survey (it
read bootstrap.md and knows its own `$CHAN_TAB_NAME`), so the context must flow
command -> SurveySpec -> SPA -> echo back -> C's route.

RECOMMENDATION (lean, faithful to the spec; D is mid-flight so cheap to add
now rather than retrofit): SurveySpec gains one optional sub-object, populated by
`cs terminal survey` only when `--followup` is set:

    SurveySpec {
      ...,
      followup: { dir: string, from: string, to: string } | null
    }

Command-side population (one NEW flag, two free derivations):
  - dir  <- new `--followup-dir <team-dir>` (required with --followup; clean
            error if absent so followups are always team-scoped).
  - from <- `$CHAN_TAB_NAME` (the surveyor's handle; falls back to `--from`).
  - to   <- the `--tab-name` target being surveyed (falls back to `--to`).

The SPA echoes `followup` back in the [F] POST body; C's reply route creates
`{dir}/followups/followup-{from}-{to}-{n}.md` (n minted by scanning the dir),
pre-populates it, then replies `{ kind:"followup", followupPath }` to D's bus.
Fallback if @@Architect prefers minimal: just `followupDir: string|null` and
C names the file `followup-{n}.md`. Either way C only needs the team-dir from D.

This blocks ONLY the [F] end-to-end. Everything else proceeds in parallel.

## Phased build (what is independent vs D-gated)

INDEPENDENT (build + gate + commit now; no D dependency):
  1. Team-config per-member AGENT field (consumes D's landed submit map):
     - chan-workspace `teams::Member.agent: Option<String>` ("claude"|"codex"|
       "gemini"; None = shell). serde(default) for hand-edited configs.
     - `TeamMemberWire.agent?` (client.ts) + `TeamMemberDraft.agent`
       (teamDialog.svelte.ts) + the dialog per-member picker (TeamDialog.svelte,
       mirroring TeamWork's none/claude/codex/gemini select).
     - teamOrchestrator: round-trip agent in translateConfig/wireToDialog; set
       the lead tab's `teamWork.agentTarget` from the lead member's agent so the
       embedded composer submits with the right chord.
     - team_config.rs bootstrap.md: add an `agent` column to the roster table +
       a per-agent submit-chord note in the poke section (claude=ESC[27;9;13~,
       codex/gemini=CR), so pokes use the right encoding per target.
  2. SPA survey OVERLAY (rebuild BubbleOverlay) + `survey.svelte.ts` store:
     render SurveySpec (markdown body, <=4 vertical numbered options, [F]),
     keyboard 1..4 + F, reply round-trip via POST /api/survey/reply. The store
     entry point `showSurvey(spec)` is what store.svelte.ts will call from the
     `open_survey` WindowCommand branch (wired at integration, held for D).
     Browser-smoke via a mock dispatch (call showSurvey from the console).
  3. Followup-file GENERATOR (chan-server, standalone pub fn + unit tests):
     `create_followup_file(workspace, dir, from, to, title, body) -> rel path`,
     minting `n`, pre-populating header/title, date+time, the "Agents: this is a
     follow up, not ready; check again later" line, the original prompt, and
     @@Host comment placeholders.

D-GATED (integrate once D's transport commits):
  4. Reply route `POST /api/survey/reply` (new routes/survey.rs I own): on
     "option" call `state.survey_bus.complete_survey`; on "followup" call the
     generator (3) then complete_survey with the path. Needs D's bus + AppState
     field. + router mount (lib.rs) + mod.rs re-export.
  5. store.svelte.ts `open_survey` WindowCommand branch -> survey store
     showSurvey (one dispatch branch in a shared file; flagged to @@Architect).
  6. End-to-end + REAL-agent smoke: survey raised over a running claude tab, the
     option returns to the blocked CLI, [F] writes the followup file.

## Commit discipline (shared worktree, D editing concurrently)

Pathspec commits only: `git commit -F msg -- <explicit paths>`. Pre-commit
`git diff --staged --stat`, post-commit `git show --stat HEAD`. Never sweep D's
uncommitted chan-shell edits. Scoped gates per crate to tolerate mid-wave flicker.
</content>
</invoke>
