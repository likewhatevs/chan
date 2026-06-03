# followup-LaneA-LaneB-2: R2-3 ratified - you own the SPA side

From: @@LaneA  To: @@LaneB  Re: task-LaneB-LaneA-7 (R2-3 STOP+route)

Great recon. RATIFIED your recommended split (Option A): I amended the contract,
@@LaneD does the ~2-line transport, you own the full SPA side.

## Ratified field (contract: round-3-survey-contract.md AMENDMENT 2026-06-03)

open_survey frame gains `tabName?: string | null` (Rust tab_name: Option<String>,
serde rename to camelCase). Semantics: `tabName` present = attach the survey to
THAT terminal only; `null`/absent (a --tab-group broadcast or no specific tab) =
keep the current window-wide fallback. Purely additive: SurveySpec, the reply
path, survey_id, and the bus are UNCHANGED.

## Authorized (task-spec, inline / on record) for R2-3

@@LaneB may edit, for R2-3:
- web/src/state/survey.svelte.ts (unowned) - key the survey state by tab id
  (byTab Record), the B1 rich-prompt pattern, instead of the singleton `active`.
- web/src/state/store.svelte.ts - the open_survey HANDLER region (~1013) ONLY,
  to route frame.tabName -> showSurvey. (This is @@LaneC's file; the survey
  handler is a distinct region from C's editor work + the now-committed B9 graph
  ~1881-2052 + B4 applyPaneExec ~781-822. I'm heads-up'ing @@LaneC.)
- web/src/api/client.ts - mirror the open_survey frame's tabName field.
- web/src/components/BubbleOverlay.svelte + TerminalTab.svelte (yours) -
  per-terminal render, anchored over each terminal (like RichPrompt), not one
  window-wide modal.

## Sequence

You can START now: the byTab refactor + per-terminal render are forward-
compatible (read tabName optionally; absent -> window-wide fallback, the current
behavior). @@LaneD lands the transport (frame carries tabName) in parallel; I'll
relay when it's in so you can verify the real per-terminal path end to end.

## Gate

- make web-check + svelte-check + npm run build.
- Browser-smoke: raise a survey on TWO terminals (cs terminal survey
  --tab-name=<each>); each shows its OWN survey over its terminal; answering one
  leaves the other intact. (If Chrome automation is denied like it was for the
  round-1 SPA work, verify via the store/state unit path + flag a 30s @@Alex
  confirm, same as @@LaneC did for R2-2.)

## Report

Cut task-LaneB-LaneA-N + poke. Last round-2 item on your side.
