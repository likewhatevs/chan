// @vitest-environment jsdom

import { mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, test } from "vitest";

import BubbleOverlay from "./BubbleOverlay.svelte";
import { surveyState } from "../state/survey.svelte";
import type { SurveySpec } from "../api/client";

// The rebuilt survey overlay renders the singleton `surveyState.active`: a
// markdown body, numbered options, and an optional [F]. It renders nothing
// when no survey is active. These are render assertions only (no clicks, so
// no /api/survey/reply network).

const mounted: Array<Record<string, any>> = [];

function spec(over: Partial<SurveySpec> = {}): SurveySpec {
  return {
    surveyId: "survey-1",
    title: null,
    bodyMarkdown: "Pick a backend",
    options: ["BM25", "Semantic"],
    allowFollowup: false,
    followup: null,
    ...over,
  };
}

afterEach(() => {
  for (const component of mounted.splice(0)) unmount(component);
  document.body.innerHTML = "";
  surveyState.active = null;
  surveyState.busy = false;
});

describe("survey overlay", () => {
  test("renders nothing when no survey is active", async () => {
    const target = document.createElement("div");
    document.body.append(target);
    mounted.push(mount(BubbleOverlay, { target, props: {} }));
    await tick();
    expect(target.querySelector(".survey-overlay")).toBeNull();
  });

  test("renders the body + numbered options for an active survey", async () => {
    surveyState.active = spec({ title: "Search backend" });
    const target = document.createElement("div");
    document.body.append(target);
    mounted.push(mount(BubbleOverlay, { target, props: {} }));
    await tick();

    expect(target.querySelector(".survey-overlay")).not.toBeNull();
    expect(target.querySelector(".survey-title")?.textContent).toBe("Search backend");
    // Markdown body rendered (sanitized) into the body container.
    expect(target.querySelector(".survey-body")?.textContent).toContain("Pick a backend");
    const options = target.querySelectorAll(".survey-option");
    expect(options.length).toBe(2);
    // Options are numbered [1]..[N].
    expect(options[0].querySelector(".survey-option-key")?.textContent).toBe("1");
    expect(options[1].querySelector(".survey-option-key")?.textContent).toBe("2");
    expect(options[1].querySelector(".survey-option-label")?.textContent).toBe("Semantic");
  });

  test("[F] affordance shows only with allowFollowup + a followup context", async () => {
    // allowFollowup true but no context => no [F] (cannot create a file).
    surveyState.active = spec({ allowFollowup: true, followup: null });
    const target = document.createElement("div");
    document.body.append(target);
    mounted.push(mount(BubbleOverlay, { target, props: {} }));
    await tick();
    expect(target.querySelector(".survey-followup")).toBeNull();
  });

  test("[F] affordance renders with allowFollowup + context", async () => {
    surveyState.active = spec({
      allowFollowup: true,
      followup: { dir: "new-team-1", from: "@@LaneC", to: "@@Host" },
    });
    const target = document.createElement("div");
    document.body.append(target);
    mounted.push(mount(BubbleOverlay, { target, props: {} }));
    await tick();
    expect(target.querySelector(".survey-followup")).not.toBeNull();
  });
});
