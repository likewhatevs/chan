// @vitest-environment jsdom
//
// Pure-helper coverage for the scope-history feature. Tests
// here exercise functions that have no DOM / reactive / network
// dependencies — they're plain inputs → outputs, which is the
// easiest test surface to cover and the highest leverage for
// future refactors.
//
// jsdom is loaded so importing `store.svelte.ts` succeeds (the
// transport layer reads `document` at module load); the tests
// themselves don't touch the DOM.

import { describe, expect, test } from "vitest";
import {
  earliestTurnCreatedAt,
  renderScopeHistoryMarkdown,
  scopeHistoryExportName,
  type AssistantTurn,
  type ScopeHistoryEntry,
} from "./store.svelte";

describe("earliestTurnCreatedAt", () => {
  test("returns undefined for an empty array", () => {
    expect(earliestTurnCreatedAt([])).toBeUndefined();
  });

  test("returns undefined when no turn carries created_at", () => {
    const turns: AssistantTurn[] = [
      { kind: "user", content: "a" },
      { kind: "assistant", content: "b" },
    ];
    expect(earliestTurnCreatedAt(turns)).toBeUndefined();
  });

  test("picks the smallest created_at across mixed turns", () => {
    const turns: AssistantTurn[] = [
      { kind: "user", content: "a", created_at: 3000 },
      { kind: "assistant", content: "b", created_at: 1000 },
      { kind: "user", content: "c" },
      { kind: "assistant", content: "d", created_at: 2000 },
    ];
    expect(earliestTurnCreatedAt(turns)).toBe(1000);
  });

  test("ignores turns without created_at when others have one", () => {
    const turns: AssistantTurn[] = [
      { kind: "user", content: "a" },
      { kind: "assistant", content: "b", created_at: 5000 },
    ];
    expect(earliestTurnCreatedAt(turns)).toBe(5000);
  });
});

describe("scopeHistoryExportName", () => {
  test("file scope strips the extension and prefixes with assistant-", () => {
    const entry: ScopeHistoryEntry = {
      id: "file:notes/intro.md",
      kind: "file",
      title: "notes/intro.md",
      paths: ["notes/intro.md"],
      turn_count: 0,
    };
    expect(scopeHistoryExportName(entry)).toBe("assistant-intro");
  });

  test("file scope handles a basename without extension", () => {
    const entry: ScopeHistoryEntry = {
      id: "file:README",
      kind: "file",
      title: "README",
      paths: ["README"],
      turn_count: 0,
    };
    expect(scopeHistoryExportName(entry)).toBe("assistant-README");
  });

  test("file scope handles a path with no slash", () => {
    const entry: ScopeHistoryEntry = {
      id: "file:notes.md",
      kind: "file",
      title: "notes.md",
      paths: ["notes.md"],
      turn_count: 0,
    };
    expect(scopeHistoryExportName(entry)).toBe("assistant-notes");
  });

  test("file scope falls back to 'scope' when paths is empty", () => {
    const entry: ScopeHistoryEntry = {
      id: "file:?",
      kind: "file",
      title: "?",
      paths: [],
      turn_count: 0,
    };
    expect(scopeHistoryExportName(entry)).toBe("assistant-scope");
  });

  test("group scope names by file count", () => {
    const entry: ScopeHistoryEntry = {
      id: "group:a|b|c",
      kind: "group",
      title: "3 files",
      paths: ["a.md", "b.md", "c.md"],
      turn_count: 0,
      group_key: "a.md|b.md|c.md",
    };
    expect(scopeHistoryExportName(entry)).toBe("assistant-group-3-files");
  });

  test("drive scope is a stable constant", () => {
    const entry: ScopeHistoryEntry = {
      id: "drive",
      kind: "drive",
      title: "Drive",
      paths: [],
      turn_count: 0,
    };
    expect(scopeHistoryExportName(entry)).toBe("assistant-drive");
  });
});

describe("renderScopeHistoryMarkdown", () => {
  test("file scope: heading carries the path, metadata block lists kind / files / timestamps", () => {
    const entry: ScopeHistoryEntry = {
      id: "file:notes/foo.md",
      kind: "file",
      title: "notes/foo.md",
      paths: ["notes/foo.md"],
      created_at: Date.UTC(2026, 0, 1, 12, 0, 0),
      last_touched: Date.UTC(2026, 0, 1, 13, 30, 0),
      turn_count: 2,
    };
    const turns: AssistantTurn[] = [
      { kind: "user", content: "hi" },
      { kind: "assistant", content: "hello" },
    ];
    const md = renderScopeHistoryMarkdown(entry, turns);

    expect(md).toMatch(/^# Assistant conversation — notes\/foo\.md/);
    expect(md).toContain("- kind: file");
    expect(md).toContain("- files: 1");
    expect(md).toContain("  - notes/foo.md");
    expect(md).toContain("- started: 2026-01-01T12:00:00.000Z");
    expect(md).toContain("- last activity: 2026-01-01T13:30:00.000Z");
    expect(md).toContain("- turns: 2");
    expect(md).toContain("## You\n\nhi");
    expect(md).toContain("## Assistant\n\nhello");
  });

  test("drive scope: title reads 'Drive', no file list when paths empty", () => {
    const entry: ScopeHistoryEntry = {
      id: "drive",
      kind: "drive",
      title: "Drive",
      paths: [],
      turn_count: 0,
    };
    const md = renderScopeHistoryMarkdown(entry, []);
    expect(md).toMatch(/^# Assistant conversation — Drive/);
    expect(md).toContain("- kind: drive");
    expect(md).not.toContain("- files:");
  });

  test("group scope: title summarises the file count, body lists each path", () => {
    const entry: ScopeHistoryEntry = {
      id: "group:a|b",
      kind: "group",
      title: "2 files",
      paths: ["a.md", "b.md"],
      turn_count: 0,
      group_key: "a.md|b.md",
    };
    const md = renderScopeHistoryMarkdown(entry, []);
    expect(md).toMatch(/^# Assistant conversation — Group \(2 files\)/);
    expect(md).toContain("- files: 2");
    expect(md).toContain("  - a.md");
    expect(md).toContain("  - b.md");
  });

  test("edit turn renders the proposal path and summary as a fenced block", () => {
    const entry: ScopeHistoryEntry = {
      id: "file:foo.md",
      kind: "file",
      title: "foo.md",
      paths: ["foo.md"],
      turn_count: 1,
    };
    const turns: AssistantTurn[] = [
      {
        kind: "edit",
        edit: {
          toolCallId: "t1",
          path: "foo.md",
          content: "hello world",
          summary: "intro",
          status: "applied",
        },
      },
    ];
    const md = renderScopeHistoryMarkdown(entry, turns);
    expect(md).toContain("## Edit proposal — foo.md");
    expect(md).toContain("> intro");
    expect(md).toContain("```\nhello world\n```");
  });

  test("tool turn collapses to an italic single line", () => {
    const entry: ScopeHistoryEntry = {
      id: "drive",
      kind: "drive",
      title: "Drive",
      paths: [],
      turn_count: 1,
    };
    const turns: AssistantTurn[] = [
      {
        kind: "tool",
        event: {
          tool_call_id: "t1",
          name: "read_file",
          label: "reading foo.md",
          status: "ok",
          result_summary: "12 lines",
          created_at: 0,
        },
      },
    ];
    const md = renderScopeHistoryMarkdown(entry, turns);
    expect(md).toContain("_reading foo.md (ok: 12 lines)_");
  });
});
