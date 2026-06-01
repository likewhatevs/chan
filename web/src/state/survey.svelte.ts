// Survey overlay state + reply round-trip for `cs terminal survey`.
//
// An agent runs `cs terminal survey`; the server mints a survey id, pushes an
// `open_survey` window command carrying a SurveySpec to the owning window(s),
// and BLOCKS the CLI on a oneshot. store.svelte.ts routes that frame to
// `showSurvey`; the overlay (BubbleOverlay.svelte) renders it and the user
// picks an option or [F]. The reply POSTs to /api/survey/reply, which
// completes the oneshot and unblocks the CLI.
//
// The overlay is intentionally NOT dismissable without an answer: the CLI is
// blocked on the reply, so a stray Escape/backdrop close would hang it. [F]
// (follow up) is the defer path - it writes a followup file and unblocks.

import { api, type SurveySpec, type SurveyReplyRequest } from "../api/client";
import { notify } from "./notify.svelte";

/// The active survey for this window (one at a time). `busy` gates the reply
/// buttons so a double-click / double-keypress cannot fire two replies for
/// the same oneshot (the second would 404, but the guard keeps the UI honest).
export const surveyState = $state<{ active: SurveySpec | null; busy: boolean }>({
  active: null,
  busy: false,
});

/// Raise a survey. A new survey replaces a showing one; the server mints
/// distinct ids, and a window answers one survey at a time in practice.
export function showSurvey(spec: SurveySpec): void {
  surveyState.active = spec;
  surveyState.busy = false;
}

function dismiss(): void {
  surveyState.active = null;
  surveyState.busy = false;
}

/// Reply with the option at `index` (0-based; the overlay numbers them
/// [1]..[N]). The chosen label round-trips to the blocked CLI's stdout.
export async function pickOption(index: number): Promise<void> {
  const active = surveyState.active;
  if (!active || surveyState.busy) return;
  const label = active.options[index];
  if (label === undefined) return;
  surveyState.busy = true;
  const reply: SurveyReplyRequest = {
    surveyId: active.surveyId,
    kind: "option",
    optionIndex: index,
    optionLabel: label,
  };
  try {
    await api.surveyReply(reply);
    dismiss();
  } catch (err) {
    surveyState.busy = false;
    notify(`survey reply failed: ${(err as Error).message ?? err}`);
  }
}

/// Reply with [F]: the route creates `{dir}/followups/followup-{from}-{to}-{n}.md`
/// from the echoed-back context + the original prompt, then unblocks the CLI
/// with that path. No-op when the survey carried no followup context.
export async function requestFollowup(): Promise<void> {
  const active = surveyState.active;
  if (!active || surveyState.busy) return;
  if (!active.allowFollowup || !active.followup) return;
  surveyState.busy = true;
  const reply: SurveyReplyRequest = {
    surveyId: active.surveyId,
    kind: "followup",
    followup: active.followup,
    title: active.title ?? null,
    bodyMarkdown: active.bodyMarkdown,
  };
  try {
    await api.surveyReply(reply);
    dismiss();
  } catch (err) {
    surveyState.busy = false;
    notify(`followup create failed: ${(err as Error).message ?? err}`);
  }
}
