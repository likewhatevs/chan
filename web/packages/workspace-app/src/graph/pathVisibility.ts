/// True when `nodePath` should be visible under a path-scoped expanded
/// directory tree.
///
/// `rootPath` is the graph scope root. Ancestors above that root stay visible
/// so a directory-scoped graph remains attached to the workspace-root spine.
/// Descendants below the root are gated by the expanded directory map.
export function ancestorsExpanded(
  rootPath: string,
  nodePath: string,
  expanded: Record<string, boolean>,
): boolean {
  const root = rootPath.replace(/\/+$/, "");
  const node = nodePath.replace(/\/+$/, "");
  if (!node || node === root) return true;
  if (root && root.startsWith(`${node}/`)) return true;
  if (root && !node.startsWith(`${root}/`)) return false;

  const rel = root ? node.slice(root.length + 1) : node;
  const parts = rel.split("/");
  let prefix = root;
  for (let i = 0; i < parts.length - 1; i += 1) {
    prefix = prefix ? `${prefix}/${parts[i]}` : parts[i];
    if (!expanded[prefix]) return false;
  }
  return true;
}
