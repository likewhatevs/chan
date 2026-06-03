// Survey overlay state + reply round-trip for `cs terminal survey`.
//
// An agent runs `cs terminal survey --tab-name=<target>`; the server mints a
// survey id, pushes an `open_survey` window command carrying a SurveySpec +
// (R2-3) the target `tabName` to the owning window, and BLOCKS the CLI on a
// oneshot. store.svelte.ts routes that frame to `showSurvey`; the overlay
// (BubbleOverlay.svelte) renders it and the user picks an option or [F]. The
// reply POSTs to /api/survey/reply, which completes the oneshot and unblocks
// the CLI.
//
// PER-TERMINAL (R2-3, @@Alex): surveys are keyed by terminal tab id (the B1
// rich-prompt pattern), so two terminals can each show their own survey without
// colliding - answering/dismissing one does not touch the other. A survey with
// no resolvable target (`tabName` absent/unmatched, or a --tab-group broadcast)
// falls back to a single window-wide slot, the pre-R2-3 behavior.
//
// Part C (R2, @@LaneD): every survey overlay now offers its options PLUS an [F]
// follow-up AND a Dismiss. The host can defer (F: writes a followup file and
// unblocks) or dismiss (a distinct "dismissed" reply, no file, so the asking
// agent can tell). Both are real replies that unblock the CLI, so a stray
// Escape/backdrop close is no longer the hang risk it was: Escape now maps to
// the explicit Dismiss reply rather than a silent close.

import { api, type SurveySpec, type SurveyReplyRequest } from "../api/client";
import { notify } from "./notify.svelte";

/// One in-flight survey + its reply guard. `busy` gates the reply buttons so a
/// double-click / double-keypress cannot fire two replies for the same oneshot
/// (the second would 404, but the guard keeps the UI honest).
type SurveyEntry = { spec: SurveySpec; busy: boolean };

/// A survey's slot: a terminal tab id (per-terminal) or `null` (the window-wide
/// fallback). The reply functions + BubbleOverlay take this so they act on
/// exactly one survey.
export type SurveySlot = string | null;

/// Active surveys: one per terminal (keyed by tab id) plus a single window-wide
/// fallback. Two terminals answer independently.
export const surveyState = $state<{
  byTab: Record<string, SurveyEntry>;
  windowWide: SurveyEntry | null;
}>({ byTab: {}, windowWide: null });

function entry(slot: SurveySlot): SurveyEntry | null {
  return slot === null ? surveyState.windowWide : (surveyState.byTab[slot] ?? null);
}

function clear(slot: SurveySlot): void {
  if (slot === null) surveyState.windowWide = null;
  else delete surveyState.byTab[slot];
}

/// The active survey spec for a slot, or null. BubbleOverlay/TerminalTab gate
/// the render on this.
export function surveyFor(slot: SurveySlot): SurveySpec | null {
  return entry(slot)?.spec ?? null;
}

/// Whether a slot's reply is in flight (disables its buttons).
export function surveyBusy(slot: SurveySlot): boolean {
  return entry(slot)?.busy ?? false;
}

/// Raise a survey on a slot. A new survey replaces a showing one in the same
/// slot; the server mints distinct ids. `slot` null = window-wide fallback.
export function showSurvey(spec: SurveySpec, slot: SurveySlot = null): void {
  if (slot === null) surveyState.windowWide = { spec, busy: false };
  else surveyState.byTab[slot] = { spec, busy: false };
}

/// Reply with the option at `index` (0-based; the overlay numbers them
/// [1]..[N]) for the survey on `slot`. The chosen label round-trips to the
/// blocked CLI's stdout.
export async function pickOption(slot: SurveySlot, index: number): Promise<void> {
  const e = entry(slot);
  if (!e || e.busy) return;
  const label = e.spec.options[index];
  if (label === undefined) return;
  e.busy = true;
  const reply: SurveyReplyRequest = {
    surveyId: e.spec.surveyId,
    kind: "option",
    optionIndex: index,
    optionLabel: label,
  };
  try {
    await api.surveyReply(reply);
    clear(slot);
  } catch (err) {
    e.busy = false;
    notify(`survey reply failed: ${(err as Error).message ?? err}`);
  }
}

/// Reply with [F] for the survey on `slot`. F is standard on every survey now
/// (Part C), so it is no longer an opt-in affordance: when the survey carries
/// followup context the route creates
/// `{dir}/followups/followup-{from}-{to}-{n}.md` and unblocks with that path;
/// when it does not (`followup: null`), the route treats it as a plain deferral
/// (no file) and still unblocks.
export async function requestFollowup(slot: SurveySlot): Promise<void> {
  const e = entry(slot);
  if (!e || e.busy) return;
  e.busy = true;
  const reply: SurveyReplyRequest = {
    surveyId: e.spec.surveyId,
    kind: "followup",
    followup: e.spec.followup ?? null,
    title: e.spec.title ?? null,
    bodyMarkdown: e.spec.bodyMarkdown,
  };
  try {
    await api.surveyReply(reply);
    clear(slot);
  } catch (err) {
    e.busy = false;
    notify(`followup create failed: ${(err as Error).message ?? err}`);
  }
}

/// Dismiss the survey on `slot` (Part C). Unlike [F], a dismiss creates no
/// file: it sends a distinct "dismissed" reply that carries only the surveyId,
/// so the asking agent can tell the host dropped the survey rather than
/// answering or deferring it. Still a real reply, so it unblocks the CLI.
export async function dismissSurvey(slot: SurveySlot): Promise<void> {
  const e = entry(slot);
  if (!e || e.busy) return;
  e.busy = true;
  const reply: SurveyReplyRequest = {
    surveyId: e.spec.surveyId,
    kind: "dismissed",
  };
  try {
    await api.surveyReply(reply);
    clear(slot);
  } catch (err) {
    e.busy = false;
    notify(`survey dismiss failed: ${(err as Error).message ?? err}`);
  }
}
