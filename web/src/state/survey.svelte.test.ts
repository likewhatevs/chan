import { afterEach, describe, expect, test, vi } from "vitest";

import { api, type SurveySpec } from "../api/client";
import {
  surveyState,
  showSurvey,
  surveyFor,
  surveyBusy,
  pickOption,
  requestFollowup,
} from "./survey.svelte";

// The survey store holds active surveys keyed by slot (a terminal tab id, or
// `null` for the window-wide fallback) and round-trips the reply through
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
  surveyState.byTab = {};
  surveyState.windowWide = null;
  vi.restoreAllMocks();
});

describe("survey store", () => {
  test("showSurvey sets the window-wide survey when slot is null", () => {
    showSurvey(spec(), null);
    expect(surveyFor(null)?.surveyId).toBe("survey-7");
    expect(surveyBusy(null)).toBe(false);
  });

  test("per-terminal surveys are independent: two tabs do not collide", () => {
    showSurvey(spec({ surveyId: "survey-a" }), "t1");
    showSurvey(spec({ surveyId: "survey-b" }), "t2");
    expect(surveyFor("t1")?.surveyId).toBe("survey-a");
    expect(surveyFor("t2")?.surveyId).toBe("survey-b");
    expect(surveyFor(null)).toBeNull();
  });

  test("pickOption posts the option reply and dismisses ONLY that slot", async () => {
    const reply = vi.spyOn(api, "surveyReply").mockResolvedValue(undefined as never);
    showSurvey(spec({ surveyId: "survey-a" }), "t1");
    showSurvey(spec({ surveyId: "survey-b" }), "t2");
    await pickOption("t1", 1);
    expect(reply).toHaveBeenCalledTimes(1);
    expect(reply).toHaveBeenCalledWith({
      surveyId: "survey-a",
      kind: "option",
      optionIndex: 1,
      optionLabel: "No",
    });
    // t1 cleared; t2 untouched (independence).
    expect(surveyFor("t1")).toBeNull();
    expect(surveyFor("t2")?.surveyId).toBe("survey-b");
  });

  test("pickOption out of range is a no-op", async () => {
    const reply = vi.spyOn(api, "surveyReply").mockResolvedValue(undefined as never);
    showSurvey(spec(), null);
    await pickOption(null, 9);
    expect(reply).not.toHaveBeenCalled();
    expect(surveyFor(null)).not.toBeNull();
  });

  test("requestFollowup posts the followup reply with the echoed context", async () => {
    const reply = vi.spyOn(api, "surveyReply").mockResolvedValue(undefined as never);
    showSurvey(
      spec({
        allowFollowup: true,
        followup: { dir: "new-team-1", from: "@@LaneC", to: "@@Host" },
      }),
      "t1",
    );
    await requestFollowup("t1");
    expect(reply).toHaveBeenCalledWith({
      surveyId: "survey-7",
      kind: "followup",
      followup: { dir: "new-team-1", from: "@@LaneC", to: "@@Host" },
      title: "T",
      bodyMarkdown: "the question",
    });
    expect(surveyFor("t1")).toBeNull();
  });

  test("requestFollowup is a no-op without a followup context", async () => {
    const reply = vi.spyOn(api, "surveyReply").mockResolvedValue(undefined as never);
    showSurvey(spec({ allowFollowup: true, followup: null }), null);
    await requestFollowup(null);
    expect(reply).not.toHaveBeenCalled();
    expect(surveyFor(null)).not.toBeNull();
  });

  test("a failed reply keeps the survey up and clears busy", async () => {
    vi.spyOn(api, "surveyReply").mockRejectedValue(new Error("boom"));
    showSurvey(spec(), "t1");
    await pickOption("t1", 0);
    // Still showing so the user can retry; not wedged in busy.
    expect(surveyFor("t1")).not.toBeNull();
    expect(surveyBusy("t1")).toBe(false);
  });
});
