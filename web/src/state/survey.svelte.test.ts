import { afterEach, describe, expect, test, vi } from "vitest";

import { api, type SurveySpec } from "../api/client";
import {
  surveyState,
  showSurvey,
  pickOption,
  requestFollowup,
} from "./survey.svelte";

// The survey store holds the active survey and round-trips the reply through
// api.surveyReply. We spy on that method so no network is hit.

function spec(over: Partial<SurveySpec> = {}): SurveySpec {
  return {
    surveyId: "survey-7",
    title: "T",
    bodyMarkdown: "the question",
    options: ["Yes", "No"],
    allowFollowup: false,
    followup: null,
    ...over,
  };
}

afterEach(() => {
  surveyState.active = null;
  surveyState.busy = false;
  vi.restoreAllMocks();
});

describe("survey store", () => {
  test("showSurvey sets the active survey", () => {
    showSurvey(spec());
    expect(surveyState.active?.surveyId).toBe("survey-7");
    expect(surveyState.busy).toBe(false);
  });

  test("pickOption posts the option reply and dismisses", async () => {
    const reply = vi.spyOn(api, "surveyReply").mockResolvedValue(undefined as never);
    showSurvey(spec());
    await pickOption(1);
    expect(reply).toHaveBeenCalledTimes(1);
    expect(reply).toHaveBeenCalledWith({
      surveyId: "survey-7",
      kind: "option",
      optionIndex: 1,
      optionLabel: "No",
    });
    expect(surveyState.active).toBeNull();
  });

  test("pickOption out of range is a no-op", async () => {
    const reply = vi.spyOn(api, "surveyReply").mockResolvedValue(undefined as never);
    showSurvey(spec());
    await pickOption(9);
    expect(reply).not.toHaveBeenCalled();
    expect(surveyState.active).not.toBeNull();
  });

  test("requestFollowup posts the followup reply with the echoed context", async () => {
    const reply = vi.spyOn(api, "surveyReply").mockResolvedValue(undefined as never);
    showSurvey(
      spec({
        allowFollowup: true,
        followup: { dir: "new-team-1", from: "@@LaneC", to: "@@Host" },
      }),
    );
    await requestFollowup();
    expect(reply).toHaveBeenCalledWith({
      surveyId: "survey-7",
      kind: "followup",
      followup: { dir: "new-team-1", from: "@@LaneC", to: "@@Host" },
      title: "T",
      bodyMarkdown: "the question",
    });
    expect(surveyState.active).toBeNull();
  });

  test("requestFollowup is a no-op without a followup context", async () => {
    const reply = vi.spyOn(api, "surveyReply").mockResolvedValue(undefined as never);
    showSurvey(spec({ allowFollowup: true, followup: null }));
    await requestFollowup();
    expect(reply).not.toHaveBeenCalled();
    expect(surveyState.active).not.toBeNull();
  });

  test("a failed reply keeps the survey up and clears busy", async () => {
    vi.spyOn(api, "surveyReply").mockRejectedValue(new Error("boom"));
    showSurvey(spec());
    await pickOption(0);
    // Still showing so the user can retry; not wedged in busy.
    expect(surveyState.active).not.toBeNull();
    expect(surveyState.busy).toBe(false);
  });
});
