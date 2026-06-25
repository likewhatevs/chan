// Team-dir helpers shared by the Team Work dialog (New/Load form) and
// the bootstrap orchestrator.

/// Default team directory the New form prefills. The config + its
/// sibling team management files live under this workspace-RELATIVE
/// directory (e.g. `<workspace>/new-team-1/`), written through the
/// Workspace sandbox (atomic, special-file refusal).
export const TEAM_DIR_DEFAULT = "new-team-1";

/// Derive the team name from a workspace-relative team directory: the
/// last path segment (basename). Used to keep the persisted config
/// self-describing without re-adding a "Team name" field to the
/// dialog. Strips a trailing slash; falls back to "team" when the dir
/// has no usable basename.
export function teamNameFromDir(dir: string): string {
  const trimmed = dir.trim().replace(/\/+$/, "");
  const lastSlash = trimmed.lastIndexOf("/");
  const base = lastSlash >= 0 ? trimmed.slice(lastSlash + 1) : trimmed;
  return base.trim() || "team";
}
