import { describe, expect, it } from "vitest";
import { FS_GRAPH_DEPTH_MAX, graphDepthCap } from "./depth";
import type { FsGraphNode, GraphViewNode } from "../api/types";

function file(path: string): Extract<GraphViewNode, { kind: "file" }> {
  return { kind: "file", id: path, label: path, path };
}

function fsNode(path: string): Pick<FsGraphNode, "path"> {
  return { path };
}

describe("graphDepthCap", () => {
  it("pins file scopes to depth 1", () => {
    expect(graphDepthCap({ scope: { kind: "file" }, nodes: [] })).toBe(1);
  });

  it("caps group scopes at the number of files up to the hard max", () => {
    expect(
      graphDepthCap({
        scope: { kind: "group", paths: ["a.md", "b.md", "c.md"] },
        nodes: [],
      }),
    ).toBe(3);
    expect(
      graphDepthCap({
        scope: {
          kind: "group",
          paths: Array.from({ length: 20 }, (_, i) => `${i}.md`),
        },
        nodes: [],
      }),
    ).toBe(10);
  });

  it("derives directory depth from loaded content graph file paths", () => {
    expect(
      graphDepthCap({
        scope: { kind: "dir", path: "notes" },
        nodes: [
          file("notes/today.md"),
          file("notes/projects/chan/todo.md"),
          file("outside/deep/file.md"),
        ],
      }),
    ).toBe(3);
  });

  it("derives directory depth from the loaded fs graph and honors truncation", () => {
    expect(
      graphDepthCap({
        scope: { kind: "dir", path: "notes" },
        nodes: [],
        fsGraph: {
          truncated: false,
          nodes: [
            fsNode("notes"),
            fsNode("notes/projects"),
            fsNode("notes/projects/chan/todo.md"),
          ],
        },
      }),
    ).toBe(3);
    expect(
      graphDepthCap({
        scope: { kind: "dir", path: "notes" },
        nodes: [],
        fsGraph: { truncated: true, nodes: [fsNode("notes/projects")] },
      }),
    ).toBe(FS_GRAPH_DEPTH_MAX);
  });

  it("uses the workspace fs graph probe for workspace and global scopes", () => {
    expect(
      graphDepthCap({
        scope: { kind: "workspace" },
        nodes: [],
        fsGraph: {
          truncated: false,
          nodes: [fsNode("a.md"), fsNode("notes/projects/chan/todo.md")],
        },
      }),
    ).toBe(4);
    expect(
      graphDepthCap({
        scope: { kind: "global" },
        nodes: [],
        fsGraph: { truncated: true, nodes: [fsNode("notes/projects")] },
      }),
    ).toBe(FS_GRAPH_DEPTH_MAX);
  });

  it("keeps tag and git repo scopes on the hard max", () => {
    expect(graphDepthCap({ scope: { kind: "tag" }, nodes: [] })).toBe(10);
    expect(graphDepthCap({ scope: { kind: "git_repo" }, nodes: [] })).toBe(10);
  });

  // GI-7: the depth slider snapped back to 1 because the cap was derived
  // from the fs-graph LOADED AT THE CURRENT DEPTH. At depth 1 only the
  // depth-1 layer is loaded, so the cap collapsed to 1 even when deeper
  // structure exists. These cases prove the cap depends on WHICH fs-graph
  // is fed in: the shallow loaded slice caps at 1, but a full-depth probe
  // of the same directory exposes the real reachable depth. GraphPanel's
  // fix is to feed `graphDepthCap` the full-depth `dirDepthProbe` for a
  // dir scope rather than the shallow loaded slice.
  it("caps a dir at 1 from a depth-1 loaded slice but at the true depth from a full-depth probe", () => {
    const dir = { kind: "dir", path: "journals" } as const;
    // Slice loaded at depth 1: only the directory's depth-1 child dir.
    const shallowSlice = {
      truncated: false,
      nodes: [fsNode("journals"), fsNode("journals/phase-11")],
    };
    expect(graphDepthCap({ scope: dir, nodes: [], fsGraph: shallowSlice })).toBe(1);
    // Full-depth probe of the same dir reaches journals/phase-11/lane-a/
    // journal.md (relative depth 3 from `journals`).
    const fullProbe = {
      truncated: false,
      nodes: [
        fsNode("journals"),
        fsNode("journals/phase-11"),
        fsNode("journals/phase-11/lane-a"),
        fsNode("journals/phase-11/lane-a/journal.md"),
      ],
    };
    expect(graphDepthCap({ scope: dir, nodes: [], fsGraph: fullProbe })).toBe(3);
  });
});
