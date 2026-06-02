import { describe, expect, test } from "vitest";
import { identityPrompt } from "./teamOrchestrator.svelte";

// The lead's identity prompt is now AUTO-DELIVERED to the lead terminal through
// the write queue (the prompt frame), not primed into a Team Work bubble buffer
// (the bubble is gone; the lead is a normal terminal). `identityPrompt` is the
// pure builder for that prompt; these pin its content. The orchestrator's
// delivery of it is exercised in teamBootstrapOrchestrator.test.ts.

describe("identity prompt content", () => {
  const prompt = identityPrompt(
    3,
    "@@Neo",
    "@@Lead",
    ["@@Worker1", "@@Worker2"],
    "new-team-1/bootstrap.md",
  );

  test("opens with the Team work header", () => {
    expect(prompt.startsWith("# Team work")).toBe(true);
  });

  test("names the team size, host, and lead", () => {
    expect(prompt).toContain("We are a team of 3");
    expect(prompt).toContain("Our host is @@Neo and the team lead is @@Lead");
  });

  test("lists the worker bullets", () => {
    expect(prompt).toContain("- @@Worker1");
    expect(prompt).toContain("- @@Worker2");
  });

  test("$CHAN_TAB_NAME stays literal (the lead's shell expands it)", () => {
    expect(prompt).toContain("You are $CHAN_TAB_NAME");
    expect(prompt).not.toContain("\\$CHAN_TAB_NAME");
  });

  test("points the lead at the bootstrap doc", () => {
    expect(prompt).toContain(
      "Read the team process at new-team-1/bootstrap.md",
    );
  });
});
