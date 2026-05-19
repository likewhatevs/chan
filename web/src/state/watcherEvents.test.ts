// @vitest-environment jsdom

import { afterEach, describe, expect, test, vi } from "vitest";

import { api } from "../api/client";
import {
  normalizeStandingOptions,
  parseWatcherEvent,
  readWatcherEvents,
  writeSurveyReply,
} from "./watcherEvents";

afterEach(() => {
  vi.restoreAllMocks();
});

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

  test("writes survey replies through the terminal event-reply endpoint", async () => {
    const writeReply = vi.spyOn(api, "writeTerminalEventReply").mockResolvedValue(undefined);

    await writeSurveyReply(
      "term_123",
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

    expect(writeReply).toHaveBeenCalledWith("term_123", {
      id: "survey/alpha",
      type: "survey-reply",
      from: "@@Alex",
      to: "@@FullStack",
      answers: [{ question_index: 0, key: "1" }],
      scope_grant: "one-shot",
    });
  });

  test("drops unknown event types like the server watcher", () => {
    expect(
      parseWatcherEvent(
        "events/event-future.md",
        JSON.stringify({
          id: "future",
          type: "futuristic-thing",
          from: "@@TestAgent",
          to: "@@Alex",
        }),
      ),
    ).toBeNull();
  });

  test("parses pre-flight event metadata", () => {
    const event = parseWatcherEvent(
      "events/event-preflight.md",
      JSON.stringify({
        id: "preflight-1",
        type: "pre-flight",
        from: "@@Spawner",
        to: "@@Alex",
        note: "authentication required",
        session: "spawn_session",
        tab_label: "@@Pair",
      }),
    );

    expect(event).toMatchObject({
      id: "preflight-1",
      type: "pre-flight",
      note: "authentication required",
      session: "spawn_session",
      tab_label: "@@Pair",
    });
  });

  test("reads pre-flight event files emitted by chan-server", async () => {
    vi.spyOn(api, "list").mockResolvedValue([
      {
        path: "events/pre-flight-f90ed024a46dc89a.md",
        is_dir: false,
        mtime: 1,
        size: 130,
      },
      {
        path: "events/not-an-event.md",
        is_dir: false,
        mtime: 1,
        size: 2,
      },
    ]);
    vi.spyOn(api, "read").mockImplementation(async (path) => ({
      path,
      content: JSON.stringify({
        id: "pre-flight-f90ed024a46dc89a",
        type: "pre-flight",
        from: "@@AuthNeeded",
        to: "HostA",
        note: "please log in",
      }),
      mtime: 1,
    }));

    const events = await readWatcherEvents("events");

    expect(events).toHaveLength(1);
    expect(events[0]).toMatchObject({
      id: "pre-flight-f90ed024a46dc89a",
      type: "pre-flight",
      path: "events/pre-flight-f90ed024a46dc89a.md",
      note: "please log in",
    });
    expect(api.read).toHaveBeenCalledWith("events/pre-flight-f90ed024a46dc89a.md");
    expect(api.read).not.toHaveBeenCalledWith("events/not-an-event.md");
  });
});
