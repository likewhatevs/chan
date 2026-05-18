// @vitest-environment jsdom

import { describe, expect, test, vi } from "vitest";

import { api } from "../api/client";
import {
  normalizeStandingOptions,
  parseWatcherEvent,
  writeSurveyReply,
} from "./watcherEvents";

describe("watcher event helpers", () => {
  test("parses the locked survey shape and injects the standing comment option", () => {
    const event = parseWatcherEvent(
      "events/event-1.md",
      JSON.stringify({
        id: "s1",
        type: "survey",
        from: "@@Architect",
        to: "@@Alex",
        topic: "round-2",
        questions: [
          {
            header: "Path",
            text: "Pick one",
            options: [
              { key: "1", label: "A" },
              { key: "2", label: "B" },
            ],
          },
        ],
        standing_options: [],
        scope: "topic-session",
      }),
    );

    expect(event?.questions?.[0]?.options).toEqual([
      { key: "1", label: "A" },
      { key: "2", label: "B" },
    ]);
    expect(event?.scope).toBe("topic-session");
    expect(normalizeStandingOptions(event?.standing_options)).toContainEqual({
      key: "C",
      label: "Check my comments first",
    });
  });

  test("writes survey replies by temp create plus rename into the watch dir", async () => {
    vi.spyOn(Date, "now").mockReturnValue(1234);
    const create = vi.spyOn(api, "create").mockResolvedValue(undefined);
    const move = vi.spyOn(api, "move").mockResolvedValue({
      renamed: [],
      rewritten: [],
      conflicts: [],
    });

    const path = await writeSurveyReply(
      "events",
      {
        id: "survey/alpha",
        type: "survey",
        from: "@@FullStack",
        to: "@@Alex",
        path: "events/event-survey.md",
      },
      [{ question_index: 0, key: "1" }],
      "one-shot",
    );

    expect(path).toBe("events/event-reply-survey-alpha.md");
    expect(create.mock.calls[0]?.[0]).toBe("events/.event-reply-survey-alpha-ya.tmp");
    expect(JSON.parse(String(create.mock.calls[0]?.[2]))).toMatchObject({
      id: "survey/alpha",
      type: "survey-reply",
      from: "@@Alex",
      to: "@@FullStack",
      answers: [{ question_index: 0, key: "1" }],
      scope_grant: "one-shot",
    });
    expect(move).toHaveBeenCalledWith(
      "events/.event-reply-survey-alpha-ya.tmp",
      "events/event-reply-survey-alpha.md",
    );
  });
});
