import type { FsGraphNode, GraphViewNode } from "../api/types";

export const GRAPH_DEPTH_HARD_MAX = 10;
export const FS_GRAPH_DEPTH_MAX = 6;

type GraphDepthScope =
  | { kind: "file" }
  | { kind: "group"; paths: readonly string[] }
  | { kind: "dir"; path: string }
  | { kind: "workspace" }
  | { kind: "global" }
  | { kind: "tag" }
  | { kind: "git_repo" };

type FsGraphProbe = {
  nodes: readonly Pick<FsGraphNode, "path">[];
  truncated: boolean;
};

export type GraphDepthCapInput = {
  scope: GraphDepthScope | null;
  nodes: readonly GraphViewNode[];
  fsGraph?: FsGraphProbe | null;
  hardMax?: number;
  fsMax?: number;
};

function clampDepth(value: number, max: number): number {
  return Math.max(1, Math.min(max, Math.max(1, value)));
}

function relativeDepth(root: string, path: string): number {
  const cleanRoot = root.replace(/^\/+|\/+$/g, "");
  const cleanPath = path.replace(/^\/+|\/+$/g, "");
  if (!cleanPath) return 1;
  if (!cleanRoot) return cleanPath.split("/").filter(Boolean).length;
  if (cleanPath === cleanRoot) return 1;
  const prefix = `${cleanRoot}/`;
  if (!cleanPath.startsWith(prefix)) return 0;
  return cleanPath.slice(prefix.length).split("/").filter(Boolean).length;
}

function maxDepthFromPaths(root: string, paths: readonly string[]): number {
  let max = 1;
  for (const path of paths) {
    max = Math.max(max, relativeDepth(root, path));
  }
  return max;
}

function filePaths(nodes: readonly GraphViewNode[]): string[] {
  return nodes.flatMap((node) => (node.kind === "file" ? [node.path] : []));
}

function fsPaths(fsGraph: FsGraphProbe): string[] {
  return fsGraph.nodes.map((node) => node.path).filter((path) => path.length > 0);
}

export function graphDepthCap({
  scope,
  nodes,
  fsGraph = null,
  hardMax = GRAPH_DEPTH_HARD_MAX,
  fsMax = FS_GRAPH_DEPTH_MAX,
}: GraphDepthCapInput): number {
  if (!scope) return hardMax;
  if (scope.kind === "file") return 1;
  if (scope.kind === "group") return clampDepth(scope.paths.length, hardMax);
  if (scope.kind === "tag" || scope.kind === "git_repo") return hardMax;
  if (scope.kind === "workspace" || scope.kind === "global") {
    if (!fsGraph) return hardMax;
    if (fsGraph.truncated) return fsMax;
    return clampDepth(maxDepthFromPaths("", fsPaths(fsGraph)), fsMax);
  }
  if (scope.kind === "dir") {
    if (fsGraph) {
      if (fsGraph.truncated) return fsMax;
      return clampDepth(maxDepthFromPaths(scope.path, fsPaths(fsGraph)), fsMax);
    }
    return clampDepth(maxDepthFromPaths(scope.path, filePaths(nodes)), hardMax);
  }
  return hardMax;
}
