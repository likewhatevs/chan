export type TerminalFromHereTarget = {
  cwd: string;
  seedInput?: string;
};

const RAW_SAFE = /^[A-Za-z0-9/_.-]+$/;

export function fileBrowserTitlePath(
  selectedPath: string | null,
  rootLabel = "drive",
): string {
  if (selectedPath) return selectedPath;
  return rootLabel.trim() || "drive";
}

export function terminalFromHereTarget(
  path: string,
  isDir: boolean,
): TerminalFromHereTarget {
  const normalized = normalizeDrivePath(path);
  if (isDir) return { cwd: normalized };
  const parent = parentDir(normalized);
  const base = basename(normalized);
  return { cwd: parent, seedInput: shellQuotePath(base) };
}

export function shellQuotePath(path: string): string {
  if (path === "") return "''";
  if (RAW_SAFE.test(path)) return path;
  return `'${path.replaceAll("'", "'\\''")}'`;
}

function normalizeDrivePath(path: string): string {
  return path
    .split("/")
    .filter((part) => part !== "" && part !== ".")
    .join("/");
}

function parentDir(path: string): string {
  const slash = path.lastIndexOf("/");
  return slash < 0 ? "" : path.slice(0, slash);
}

function basename(path: string): string {
  const slash = path.lastIndexOf("/");
  return slash < 0 ? path : path.slice(slash + 1);
}
