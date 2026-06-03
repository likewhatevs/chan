# task-LaneB-LaneA-7: R2-3 per-terminal survey - STOP + route (contract + C/D)

From: @@LaneB  To: @@LaneA  Re: task-LaneA-LaneB-6 (R2-3)

## STOP + route, per the task. Per-terminal surveys need a CONTRACT change
   across @@LaneD (transport) + @@LaneC (SPA) - bigger than one file.

I recon'd the full survey flow against HEAD + read round-3-survey-contract.md
(the @@Architect-held C<->D seam) before editing. The blocker is structural:
the `open_survey` frame carries NO target tab, so the SPA cannot attach a
survey to a specific terminal. Today surveys are WINDOW-targeted (the server
pushes to the window owning `--tab-name`; the frame itself has no tab). Making
them PER-TERMINAL (@@Alex: "each terminal could have their own survey, not
impact each other") requires the target tab to ride the frame -> a change to
the survey contract SHAPE.

## The flow (grounded)

`cs terminal survey --tab-name=X` -> ControlRequest::TermSurvey (carries X) ->
chan-server control_socket.rs TermSurvey handler (HAS X, ~372) -> pushes
`WindowCommand::OpenSurvey { survey }` (~91, NO tab) -> SPA store.svelte.ts
open_survey handler (~1013) -> survey.svelte.ts `surveyState` (SINGLETON
`active`) -> BubbleOverlay.svelte (App-root, window-wide centered modal).

## What per-terminal needs (cross-lane / cross-crate)

1. CONTRACT (round-3-survey-contract.md, yours): add a target-tab field to the
   open_survey frame, e.g. `OpenSurvey { survey, tab_name: Option<String> }`
   (snake_case wire) / `{ command: "open_survey", survey, tabName }` (SPA). One
   field; the survey content (SurveySpec) is untouched, so the reply path +
   surveyId round-trip are unchanged. You hold the contract - please ratify the
   SHAPE.
2. TRANSPORT (@@LaneD, chan-server control_socket.rs survey region ~91 + ~372):
   put the already-known `tab_name` into the OpenSurvey push. ~2 lines; the
   handler already has it. NOT my pane-exec region; @@LaneD's.
3. SPA routing (store.svelte.ts open_survey handler ~1013, @@LaneC): pass the
   tab to showSurvey.
4. SURVEY STORE (survey.svelte.ts): key state by tab id instead of a singleton
   (`byTab: Record<tabId, {active, busy}>`), exactly the B1 rich-prompt pattern.
   Not in any lane's explicit owned list; it is the C<->D UX side.
5. RENDER (BubbleOverlay.svelte [MINE] + TerminalTab mount): render each
   terminal's own survey anchored over that terminal (like RichPrompt), not one
   window-wide modal. @@Alex's image-3 shows the survey over its terminal.

## Decision for you (you hold the contract)

This is an @@Architect-held C<->D contract change, so it is yours to orchestrate.
My recommendation (minimizes @@LaneD, gives me the bulk SPA/UX work):
- YOU ratify the 1-field contract amendment (tab on the open_survey frame).
- @@LaneD: the ~2-line transport tweak (tab into the OpenSurvey push). It is
  their crate region + the seam.
- ME: own the entire SPA side - survey.svelte.ts (key by tab), store.svelte.ts
  open_survey handler routing (authorize me that region, like B4's
  applyPaneExec), BubbleOverlay.svelte + TerminalTab per-terminal render, and
  the api/client.ts SurveySpec/frame mirror. I'll land the SPA atomically once
  the contract + transport land.
Alternative: authorize me to ALSO do the chan-server transport tweak (I touched
control_socket.rs for B4's wire; I can do the survey region too if you'd rather
not split to @@LaneD). Your call - it is @@LaneD's region + the contract seam.

The reply route (POST /api/survey/reply, @@LaneC) + the survey bus (@@LaneD) +
surveyId are UNCHANGED - this is purely additive (one routing field), so it
should not disturb the existing C<->D reply contract.

## Status

Holding R2-3 edits pending your ratification of the contract SHAPE + the
ownership split. BubbleOverlay (mine) per-terminal render is ready to go the
moment the frame carries the tab. This is my last round-2 item.
