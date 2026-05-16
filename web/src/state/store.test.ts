// @vitest-environment jsdom
//
// Reducer coverage for assistant lifecycle websocket frames. These
// tests drive the same frame handler used by the real /ws watcher,
// without mounting Svelte components or starting chan-server.

import { afterEach, describe, expect, test } from "vitest";
import {
  assistantConversations,
  assistantStream,
  bareToolName,
  beginAssistantStream,
  endAssistantStream,
  onWatchEvent,
} from "./store.svelte";

function resetStream(): void {
  assistantStream.sessionId = null;
  assistantStream.contextId = null;
  assistantStream.text = "";
  assistantStream.toolResults = {};
  assistantStream.status = null;
  assistantStream.lastHeartbeatAt = null;
  assistantStream.activity = [];
  assistantStream.userRequest = null;
  assistantStream.error = null;
}

afterEach(() => {
  resetStream();
  assistantConversations.drive = null;
  assistantConversations.byFile = {};
  assistantConversations.byGroup = {};
});

describe("assistant lifecycle websocket frames", () => {
  test("status updates assistantStream and stamps heartbeat time", () => {
    beginAssistantStream("s1", "drive");
    const before = Date.now() - 100;

    onWatchEvent({
      type: "llm.status",
      session_id: "s1",
      status: { kind: "heartbeat", backend: "claude_cli", idle_ms: 2000 },
    });

    expect(assistantStream.status?.kind).toBe("heartbeat");
    expect(assistantStream.status).toMatchObject({
      backend: "claude_cli",
      idle_ms: 2000,
    });
    expect(assistantStream.lastHeartbeatAt).toBeGreaterThanOrEqual(before);
  });

  test("activity buffers lifecycle events in order", () => {
    beginAssistantStream("s1", "drive");

    onWatchEvent({
      type: "llm.activity",
      session_id: "s1",
      activity: {
        kind: "tool_started",
        backend: "claude_cli",
        id: "t1",
        name: "read_file",
        parent_id: null,
      },
    });
    onWatchEvent({
      type: "llm.activity",
      session_id: "s1",
      activity: {
        kind: "tool_finished",
        backend: "claude_cli",
        id: "t1",
        name: "read_file",
        output: { ok: true },
        is_error: false,
        parent_id: null,
      },
    });

    expect(assistantStream.activity.map((activity) => activity.kind)).toEqual([
      "tool_started",
      "tool_finished",
    ]);
  });

  test("tool activity is folded into persistent tool turns when a conversation exists", () => {
    assistantConversations.drive = { messages: [], turns: [] };
    beginAssistantStream("s1", "drive");

    onWatchEvent({
      type: "llm.activity",
      session_id: "s1",
      activity: {
        kind: "tool_started",
        backend: "claude_cli",
        id: "t1",
        name: "Grep",
        parent_id: null,
      },
    });
    onWatchEvent({
      type: "llm.activity",
      session_id: "s1",
      activity: {
        kind: "tool_args_delta",
        backend: "claude_cli",
        id: "t1",
        partial_json: "{\"pattern\":\"Cargo\"}",
        parent_id: null,
      },
    });
    onWatchEvent({
      type: "llm.tool_call",
      session_id: "s1",
      call: { id: "t1", name: "Grep", args: { pattern: "Cargo", path: "." } },
    });
    onWatchEvent({
      type: "llm.tool_result",
      session_id: "s1",
      result: { id: "t1", output: { hits: [{ path: "Cargo.toml" }] } },
    });

    expect(assistantStream.activity).toEqual([]);
    expect(assistantConversations.drive.turns).toHaveLength(1);
    const turn = assistantConversations.drive.turns[0];
    expect(turn?.kind).toBe("tool");
    if (turn?.kind !== "tool") throw new Error("expected tool turn");
    expect(turn.event).toMatchObject({
      tool_call_id: "t1",
      name: "Grep",
      status: "ok",
      args: { pattern: "Cargo", path: "." },
      output: { hits: [{ path: "Cargo.toml" }] },
      result_summary: "1 hits",
    });
    expect(turn.event.partial_args).toBe("{\"pattern\":\"Cargo\"}");
    expect(turn.event.started_at).toBeTypeOf("number");
    expect(turn.event.finished_at).toBeTypeOf("number");
  });

  test("activity history keeps the most recent 32 entries", () => {
    beginAssistantStream("s1", "drive");

    for (let i = 0; i < 40; i++) {
      onWatchEvent({
        type: "llm.activity",
        session_id: "s1",
        activity: {
          kind: "agent_note",
          backend: "claude_cli",
          text: `note ${i}`,
          parent_id: null,
        },
      });
    }

    expect(assistantStream.activity).toHaveLength(32);
    expect(assistantStream.activity[0]).toMatchObject({ text: "note 8" });
    expect(assistantStream.activity.at(-1)).toMatchObject({ text: "note 39" });
  });

  test("user_request survey is stored intact", () => {
    beginAssistantStream("s1", "drive");

    onWatchEvent({
      type: "llm.user_request",
      session_id: "s1",
      request: {
        kind: "survey",
        backend: "claude_cli",
        id: "survey-1",
        parent_id: null,
        questions: [
          {
            question: "Choose a path",
            header: "Decision",
            multi_select: false,
            options: [
              { label: "A", description: "first" },
              { label: "B", description: "second" },
            ],
          },
        ],
      },
    });

    expect(assistantStream.userRequest?.kind).toBe("survey");
    const request = assistantStream.userRequest as {
      id: string;
      questions: Array<{
        question: string;
        header?: string | null;
        multi_select: boolean;
        options: Array<{ label: string; description?: string | null }>;
      }>;
    } | null;
    expect(request?.id).toBe("survey-1");
    expect(request?.questions).toHaveLength(1);
    expect(request?.questions[0]).toMatchObject({
      question: "Choose a path",
      header: "Decision",
      multi_select: false,
    });
    expect(request?.questions[0]?.options).toEqual([
      { label: "A", description: "first" },
      { label: "B", description: "second" },
    ]);
  });

  test("frames for other sessions are ignored", () => {
    beginAssistantStream("s1", "drive");

    onWatchEvent({
      type: "llm.status",
      session_id: "s2",
      status: { kind: "thinking", backend: "claude_cli", status: "busy" },
    });
    onWatchEvent({
      type: "llm.activity",
      session_id: "s2",
      activity: {
        kind: "agent_note",
        backend: "claude_cli",
        text: "wrong stream",
        parent_id: null,
      },
    });
    onWatchEvent({
      type: "llm.user_request",
      session_id: "s2",
      request: {
        kind: "survey",
        backend: "claude_cli",
        id: "wrong",
        parent_id: null,
        questions: [],
      },
    });

    expect(assistantStream.status).toBeNull();
    expect(assistantStream.lastHeartbeatAt).toBeNull();
    expect(assistantStream.activity).toEqual([]);
    expect(assistantStream.userRequest).toBeNull();
  });

  test("unknown llm frame variants do not throw or mutate stream state", () => {
    beginAssistantStream("s1", "drive");

    expect(() => {
      onWatchEvent({
        type: "llm.future_thing",
        session_id: "s1",
        payload: { whatever: true },
      });
    }).not.toThrow();

    expect(assistantStream.status).toBeNull();
    expect(assistantStream.lastHeartbeatAt).toBeNull();
    expect(assistantStream.activity).toEqual([]);
    expect(assistantStream.userRequest).toBeNull();
    expect(assistantStream.text).toBe("");
  });

  test("endAssistantStream clears lifecycle fields", () => {
    beginAssistantStream("s1", "drive");
    onWatchEvent({
      type: "llm.status",
      session_id: "s1",
      status: { kind: "heartbeat", backend: "claude_cli", idle_ms: 2000 },
    });
    onWatchEvent({
      type: "llm.activity",
      session_id: "s1",
      activity: {
        kind: "agent_note",
        backend: "claude_cli",
        text: "working",
        parent_id: null,
      },
    });
    onWatchEvent({
      type: "llm.user_request",
      session_id: "s1",
      request: {
        kind: "survey",
        backend: "claude_cli",
        id: "survey-1",
        parent_id: null,
        questions: [],
      },
    });

    endAssistantStream("s1");

    expect(assistantStream.status).toBeNull();
    expect(assistantStream.lastHeartbeatAt).toBeNull();
    expect(assistantStream.activity).toEqual([]);
    // userRequest survives endAssistantStream so the survey panel
    // stays visible after the agent process exits on
    // AskUserQuestion; the next beginAssistantStream resets it.
    expect(assistantStream.userRequest).not.toBeNull();
  });

  test("existing delta frames still append streamed text", () => {
    beginAssistantStream("s1", "drive");

    onWatchEvent({ type: "llm.delta", session_id: "s1", text: "hello " });
    onWatchEvent({ type: "llm.delta", session_id: "s1", text: "world" });

    expect(assistantStream.text).toBe("hello world");
  });

  test("delta frames normalize glued sentence boundaries into paragraphs", () => {
    beginAssistantStream("s1", "drive");

    onWatchEvent({ type: "llm.delta", session_id: "s1", text: "One." });
    onWatchEvent({ type: "llm.delta", session_id: "s1", text: "Two." });

    expect(assistantStream.text).toBe("One.\n\nTwo.");
  });
});

describe("bareToolName", () => {
  test("strips claude-cli mcp__chan__ prefix", () => {
    expect(bareToolName("mcp__chan__write_file")).toBe("write_file");
    expect(bareToolName("mcp__chan__read_file")).toBe("read_file");
  });

  test("strips gemini-cli mcp_chan_ prefix", () => {
    expect(bareToolName("mcp_chan_write_file")).toBe("write_file");
    expect(bareToolName("mcp_chan_list_files")).toBe("list_files");
  });

  test("strips codex-cli chan:: prefix", () => {
    expect(bareToolName("chan::write_file")).toBe("write_file");
    expect(bareToolName("chan::list_files")).toBe("list_files");
  });

  test("passes through already-bare names", () => {
    expect(bareToolName("write_file")).toBe("write_file");
    expect(bareToolName("read_file")).toBe("read_file");
  });

  test("passes through unrelated tools so non-chan callers stay legible", () => {
    expect(bareToolName("list_directory")).toBe("list_directory");
    expect(bareToolName("update_topic")).toBe("update_topic");
    expect(bareToolName("mcp__other__do_thing")).toBe("mcp__other__do_thing");
    expect(bareToolName("other::do_thing")).toBe("other::do_thing");
  });
});
