# Item 5 — survey-first host comms + X dismiss key + template

Lane: @@TeamFlow. Small. Line numbers from main @ 3ebee587.

## Part A — X key dismisses the survey overlay

`web/src/components/BubbleOverlay.svelte`:
- Keyboard today (~42-70): 1..9 pick (~50-57), F/f follow-up
  (~59-63), Escape dismiss (~65-69) — all preventDefault +
  stopPropagation, scoped to the focused card. Dismiss button
  (~124-131, class `survey-dismiss`) → `dismissSurvey(slot)`.
- Add `x`/`X` alongside Escape → `void dismissSurvey(slot)` (same
  guard pattern). Keep Escape working.
- Update the line-6 comment ("Picking an option (click or 1..N), [F]
  (click or F), or Dismiss (click, X, or Escape)") and surface the key
  on the button label the way [F] is shown (e.g. `[X] Dismiss`) —
  match the existing option/[F] label styling.
- vitest: `web/src/state/survey.svelte.test.ts` covers
  dismissSurvey() behavior (~108-126); add a source pin for the X
  binding in the overlay (repo ?raw style).

No wire/server changes: the reply path
(`api.surveyReply` → POST /api/survey/reply →
`SurveyReply::Dismissed`, routes/survey.rs ~77-140, wire.rs ~335) and
the CLI's "survey dismissed; no answer" line
(control_socket.rs format_survey_reply ~983-997) already exist.

## Part B — bootstrap template: survey-first + key documentation

`crates/chan-server/src/routes/team_config.rs`,
`generate_bootstrap_md()` (~244-362), "Reaching the host" section
(~303-325). Rewrite to say:

1. The LEAD uses `cs terminal survey` for host communication
   **whenever possible** — decisions, status checks, smoke requests —
   not only when a decision is needed. (Keep: workers never survey the
   host directly; no TUI/AskUserQuestion surveys; consolidate/sequence
   rather than firing many tiny surveys; one decision, up to 4
   options.)
2. Document the host's keys: pick an option with 1..N (or click),
   F = follow-up (paper-trail under {team_dir}/followups/),
   X = dismiss (or Escape/click).
3. Keep the existing command example and the
   `--tab-name` guidance; note the host is reached via a tab the
   host's window owns (the lead's own tab) when the host has no
   member tab.

Constraints: ASCII-only (test asserts no em dashes); keep
`{host_handle}`/`{lead_handle}`/`{team_dir}` interpolation style.

Tests (same file): extend
`test_bootstrap_contains_team_host_lead_and_poke_chord` (~756-771) to
assert "whenever possible" and the X/F key documentation; keep the
ASCII assertion; check the other template tests (~816-949) still pass.

## Process note (already in force)

Round-1-plan.md makes survey-first binding for THIS round regardless
of the generated bootstrap text (this team's bootstrap predates the
fix). Do NOT restart the live serving binary to pick the template up
mid-round (kills every team PTY). Verify Part A+B on a throwaway
`chan serve --standalone` workspace: run a survey, exercise 1..N, F,
X from the keyboard, and regenerate a bootstrap to read the new text.
