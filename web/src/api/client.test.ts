// @vitest-environment jsdom

import { afterEach, describe, expect, test, vi } from "vitest";
import { api, sessionPath, sessionWindowId } from "./client";

afterEach(() => {
  vi.restoreAllMocks();
  window.history.replaceState(null, "", "/");
  window.sessionStorage.clear();
});

describe("sessionWindowId", () => {
  test("uses per-tab sessionStorage without a window id", () => {
    window.history.replaceState(null, "", "/?t=token");

    window.sessionStorage.setItem("chan.session.window", "tab-a1b2c3d4");

    expect(sessionWindowId()).toBe("tab-a1b2c3d4");
    expect(sessionPath()).toBe("/api/session?w=tab-a1b2c3d4");
  });

  test("generates and reuses a per-tab sessionStorage id", () => {
    window.history.replaceState(null, "", "/?t=token");

    const first = sessionWindowId();
    const second = sessionWindowId();

    expect(first).toMatch(/^[0-9a-f]{8}$/);
    expect(second).toBe(first);
  });

  test("uses the chan-desktop window id from the URL", () => {
    window.history.replaceState(null, "", "/?t=token&w=drive-notes-7");

    expect(sessionWindowId()).toBe("drive-notes-7");
    expect(sessionPath()).toBe("/api/session?w=drive-notes-7");
  });

  test("encodes unusual window labels before calling the session API", () => {
    window.history.replaceState(null, "", "/?w=tunnel%20a/drive%201");

    expect(sessionWindowId()).toBe("tunnel a/drive 1");
    expect(sessionPath()).toBe("/api/session?w=tunnel%20a%2Fdrive%201");
  });
});

describe("file read streaming", () => {
  test("parses meta, chunks, progress, and done from NDJSON", async () => {
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        const enc = new TextEncoder();
        controller.enqueue(enc.encode(
          [
            '{"type":"meta","path":"CHANGELOG.md","size":10,"mtime":1,"mtime_ns":"100","writable":true}',
            '{"type":"chunk","content":"hello","bytes":5}',
            '{"type":"chunk","content":"world","bytes":5}',
            '{"type":"done"}',
            "",
          ].join("\n"),
        ));
        controller.close();
      },
    });
    const fetchMock = vi
      .spyOn(globalThis, "fetch")
      .mockResolvedValue(new Response(body, { status: 200 }));
    const chunks: Array<{ chunk: string; loaded: number; total: number | null }> = [];

    const file = await api.readStream("CHANGELOG.md", {
      onChunk(chunk, progress) {
        chunks.push({
          chunk,
          loaded: progress.loadedBytes,
          total: progress.totalBytes,
        });
      },
    });

    expect(fetchMock.mock.calls[0][0]).toContain("/api/files/CHANGELOG.md?stream=1");
    expect(file.content).toBe("helloworld");
    expect(file.mtime_ns).toBe("100");
    expect(chunks).toEqual([
      { chunk: "hello", loaded: 5, total: 10 },
      { chunk: "world", loaded: 10, total: 10 },
    ]);
  });

  test("turns stream error events into ApiError failures", async () => {
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(new TextEncoder().encode(
          '{"type":"meta","path":"a.md","size":1,"mtime":1}\n{"type":"error","error":"bad read"}\n',
        ));
        controller.close();
      },
    });
    vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response(body, { status: 200 }));

    await expect(api.readStream("a.md")).rejects.toThrow("bad read");
  });
});

describe("relationship streaming", () => {
  test("parses report file stream events", async () => {
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(new TextEncoder().encode(
          [
            '{"type":"meta","path":"CHANGELOG.md"}',
            '{"type":"report","stats":{"language":"Markdown","code":10,"comments":0,"blanks":2,"complexity":0}}',
            '{"type":"done"}',
            "",
          ].join("\n"),
        ));
        controller.close();
      },
    });
    const fetchMock = vi
      .spyOn(globalThis, "fetch")
      .mockResolvedValue(new Response(body, { status: 200 }));
    const seen: string[] = [];

    const report = await api.reportFileStream("CHANGELOG.md", {
      onReport(stats) {
        seen.push(stats.language);
      },
    });

    expect(fetchMock.mock.calls[0][0]).toContain(
      "/api/report/file?path=CHANGELOG.md&stream=1",
    );
    expect(report?.language).toBe("Markdown");
    expect(seen).toEqual(["Markdown"]);
  });

  test("parses backlinks stream edges as they arrive", async () => {
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(new TextEncoder().encode(
          [
            '{"type":"meta","path":"b.md"}',
            '{"type":"edge","edge":{"src":"a.md","dst":"b.md","kind":"link","anchor":null}}',
            '{"type":"done"}',
            "",
          ].join("\n"),
        ));
        controller.close();
      },
    });
    vi.spyOn(globalThis, "fetch").mockResolvedValue(new Response(body, { status: 200 }));
    const edges: string[] = [];

    const result = await api.backlinksStream("b.md", {
      onEdge(edge) {
        edges.push(edge.src);
      },
    });

    expect(result).toHaveLength(1);
    expect(edges).toEqual(["a.md"]);
  });

  test("parses graph stream batches with node upserts and edge dedupe", async () => {
    const body = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(new TextEncoder().encode(
          [
            '{"type":"meta","scope":"file","path":"a.md","depth":1}',
            '{"type":"nodes","nodes":[{"kind":"file","id":"file:a.md","label":"a.md","path":"a.md"}]}',
            '{"type":"nodes","nodes":[{"kind":"file","id":"file:a.md","label":"A","path":"a.md"}]}',
            '{"type":"edges","edges":[{"source":"file:a.md","target":"tag:x","kind":"tag","rank":1}]}',
            '{"type":"edges","edges":[{"source":"file:a.md","target":"tag:x","kind":"tag","rank":1}]}',
            '{"type":"done"}',
            "",
          ].join("\n"),
        ));
        controller.close();
      },
    });
    const fetchMock = vi
      .spyOn(globalThis, "fetch")
      .mockResolvedValue(new Response(body, { status: 200 }));
    const partialNodeCounts: number[] = [];

    const graph = await api.graphStream(
      { scope: "file", path: "a.md", depth: 1 },
      {
        onNodes(_nodes, view) {
          partialNodeCounts.push(view.nodes.length);
        },
      },
    );

    expect(fetchMock.mock.calls[0][0]).toContain(
      "/api/graph?scope=file&path=a.md&depth=1&stream=1",
    );
    expect(graph.nodes).toEqual([
      { kind: "file", id: "file:a.md", label: "A", path: "a.md" },
    ]);
    expect(graph.edges).toHaveLength(1);
    expect(partialNodeCounts).toEqual([1, 1]);
  });
});
