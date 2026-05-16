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

  it("uses the drive fs graph probe for drive and global scopes", () => {
    expect(
      graphDepthCap({
        scope: { kind: "drive" },
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
});
