// chan-team.toml path helpers shared by the Team Work dialog
// (New/Load form) and the bootstrap orchestrator.

/// Default path the New form prefills. The config + its sibling team
/// management files live under `/tmp/new-team-1` by design (app-level
/// dev-orchestration data outside the notes-content sandbox).
export const TEAM_CONFIG_DEFAULT_PATH = "/tmp/new-team-1/chan-team.toml";

/// Derive the directory that holds the config (and the team
/// management files) from a config path. Used by the dialog's info
/// line "team management files will be created in <dir>". Returns the
/// path's parent dir, or the path itself when it has no separator.
export function teamConfigDir(path: string): string {
  const trimmed = path.trim();
  const lastSlash = trimmed.lastIndexOf("/");
  if (lastSlash <= 0) return trimmed;
  return trimmed.slice(0, lastSlash);
}
