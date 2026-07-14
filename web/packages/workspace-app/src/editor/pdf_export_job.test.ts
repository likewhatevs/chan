// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { api } from "../api/client";
import { type PageSnapshot } from "./pdf_snapshot";
import { respondExportJob } from "./pdf_export";

vi.mock("../api/client", async (importOriginal) => {
  const mod = await importOriginal<typeof import("../api/client")>();
  return {
    ...mod,
    api: {
      ...mod.api,
      read: vi.fn(),
      create: vi.fn(),
      replaceFile: vi.fn(),
      windowReply: vi.fn(),
    },
  };
});

vi.mock("./mermaid_render", () => ({
  renderMermaid: vi.fn(async () => ({ ok: true, svg: "<svg></svg>" })),
}));
vi.mock("./excalidraw_render", () => ({
  renderExcalidraw: vi.fn(async () => ({ ok: true, svg: "<svg></svg>" })),
  renderExcalidrawFile: vi.fn(async () => ({ ok: true, svg: "<svg></svg>" })),
}));

const TINY_PNG = Uint8Array.from(
  atob(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
  ),
  (c) => c.charCodeAt(0),
);

const SEAMS = {
  rasterize: async (): Promise<PageSnapshot> => ({
    png: TINY_PNG,
    widthPx: 2,
    heightPx: 2,
  }),
};

const JOB = {
  id: "job-1",
  path: "notes/doc.md",
  format: "pdf",
  out: "notes/doc.pdf",
};

beforeEach(() => {
  vi.mocked(api.read).mockResolvedValue({
    path: "notes/doc.md",
    content: "# Title\n\nbody\n",
    mtime: null,
  });
  vi.mocked(api.create).mockResolvedValue(undefined);
  vi.mocked(api.replaceFile).mockResolvedValue({
    path: "notes/doc.pdf",
    size: 1,
  });
  vi.mocked(api.windowReply).mockResolvedValue(undefined);
});

afterEach(() => {
  vi.clearAllMocks();
  document.body.innerHTML = "";
});

describe("respondExportJob", () => {
  test("renders, uploads to out, and replies ok with the request id", async () => {
    await respondExportJob(JOB, "light", SEAMS);

    expect(api.read).toHaveBeenCalledWith("notes/doc.md");
    expect(api.replaceFile).toHaveBeenCalledTimes(1);
    const [file, out] = vi.mocked(api.replaceFile).mock.calls[0]!;
    expect(out).toBe("notes/doc.pdf");
    expect((file as File).name).toBe("doc.pdf");
    expect((file as File).type).toBe("application/pdf");
    expect(api.windowReply).toHaveBeenCalledWith({
      requestId: "job-1",
      payload: { ok: true, out: "notes/doc.pdf" },
    });
  });

  test("an unknown format replies ok:false without reading the file", async () => {
    await respondExportJob({ ...JOB, format: "docx" }, "light", SEAMS);

    expect(api.read).not.toHaveBeenCalled();
    expect(api.windowReply).toHaveBeenCalledWith({
      requestId: "job-1",
      payload: { ok: false, error: "unknown export format: docx" },
    });
  });

  test("a missing out target is created, then the replace retries", async () => {
    vi.mocked(api.replaceFile)
      .mockRejectedValueOnce(new Error("not found: notes/doc.pdf"))
      .mockResolvedValueOnce({ path: "notes/doc.pdf", size: 1 });

    await respondExportJob(JOB, "light", SEAMS);

    expect(api.create).toHaveBeenCalledWith("notes/doc.pdf", false);
    expect(api.replaceFile).toHaveBeenCalledTimes(2);
    expect(api.windowReply).toHaveBeenCalledWith({
      requestId: "job-1",
      payload: { ok: true, out: "notes/doc.pdf" },
    });
  });

  test("when creation cannot repair the upload, the original error reports", async () => {
    vi.mocked(api.replaceFile).mockRejectedValue(new Error("replace exploded"));
    vi.mocked(api.create).mockRejectedValue(new Error("create denied"));

    await respondExportJob(JOB, "light", SEAMS);

    expect(api.windowReply).toHaveBeenCalledWith({
      requestId: "job-1",
      payload: { ok: false, error: "replace exploded" },
    });
  });

  test("a render failure replies ok:false with the message", async () => {
    await respondExportJob(JOB, "light", {
      rasterize: async () => {
        throw new Error("raster blew up");
      },
    });

    expect(api.replaceFile).not.toHaveBeenCalled();
    expect(api.windowReply).toHaveBeenCalledWith({
      requestId: "job-1",
      payload: { ok: false, error: "raster blew up" },
    });
  });

  test("a stale reply id (404) is swallowed", async () => {
    const err = Object.assign(new Error("gone"), { status: 404 });
    vi.mocked(api.windowReply).mockRejectedValue(err);
    await expect(respondExportJob(JOB, "light", SEAMS)).resolves.toBeUndefined();
  });
});
