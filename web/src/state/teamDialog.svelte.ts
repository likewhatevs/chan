// `fullstack-a-78`: global "New Team" dialog request bus.
//
// Mirrors the spawnDialog pattern (`-a-4`): a state singleton +
// open/close helpers, dialog rendered at App root so it's free
// of every parent stacking context (rich-prompt is absolute /
// z-index:20; panes are overflow-hidden; Hybrid NAV adds a
// filter to non-focused panes — any of those can clip a
// position:fixed dialog).

import type { TerminalSpawnResponse } from "../api/types";

/// One agent in the team being bootstrapped. Position in
/// `TeamDialogConfig.members` is stable (positional id used by
/// the airplane-grid in slice 2 for drag&drop slot assignment).
export interface TeamMemberDraft {
  /// Display name without the `@@` prefix. If
  /// `TeamDialogConfig.autoPrefix` is true the rendered handle
  /// is `@@<name>`; otherwise raw.
  name: string;
  /// Spawn command + flags (e.g. `claude` / `claude --resume`).
  /// Free-form so users can pick whatever agent runtime.
  command: string;
  /// Additional `KEY=VALUE` env vars (one per line). The
  /// dialog auto-injects `CHAN_TAB_NAME=<name>` separately so
  /// users don't have to remember it (per addendum-b
  /// clarification #8).
  env: string;
  /// Exactly one member must be flagged as lead; the
  /// orchestrator (`-a-79`) uses the lead's terminal as the
  /// rich-prompt host.
  isLead: boolean;
}

/// The pane real-estate strategy for the team's terminals.
/// `-a-78` slice 1 only handles the type; slice 2 lands the
/// airplane-grid drag&drop for the `split` shape.
export type TeamRealEstate =
  | { kind: "tabs" }
  | { kind: "split"; grid?: { rows: number; cols: number }; slots?: number[][] };

export interface TeamDialogConfig {
  hostName: string;
  teamName: string;
  /// Total agents (lead + workers). Min 2, max 16 per
  /// addendum-b clarification #3.
  size: number;
  /// When true, all member names render with `@@` prefix.
  /// Toggled by the host via a checkbox; defaults to true.
  autoPrefix: boolean;
  /// Length must equal `size`. Exactly one member has
  /// `isLead: true`.
  members: TeamMemberDraft[];
  realEstate: TeamRealEstate;
}

export interface TeamDialogRequest {
  /// Terminal session that opened the dialog. The orchestrator
  /// (`-a-79`) uses this as the "host" terminal.
  hostSessionId?: string;
  /// Pre-populated config for the Load Team flow (`-a-80`).
  /// Undefined for the bare New Team flow.
  initial?: Partial<TeamDialogConfig>;
  /// Called when the user clicks Bootstrap with the final
  /// config. The orchestrator (`-a-79`) is the consumer; this
  /// task's scope ends at the handoff.
  onBootstrap: (config: TeamDialogConfig) => void | Promise<void>;
  /// Called after a host terminal is spawned for the new team
  /// (slice 2+ — orchestrator-driven; passed through for
  /// API symmetry with the spawn dialog).
  onSpawned?: (response: TerminalSpawnResponse, name: string) => void;
}

export const teamDialogState = $state<{ request: TeamDialogRequest | null }>({
  request: null,
});

export function openTeamDialog(request: TeamDialogRequest): void {
  teamDialogState.request = request;
}

export function closeTeamDialog(): void {
  teamDialogState.request = null;
}

/// Smallest valid team: lead + 1 worker. Per addendum-b
/// clarification #3 the user is NOT counted; they sit in the
/// rich-prompt host terminal.
export const TEAM_MIN_SIZE = 2;
export const TEAM_MAX_SIZE = 16;

/// Default team config used as the dialog's initial state.
/// Two members; first is the lead. Auto-prefix on. Real estate
/// defaults to tabs (split-pane is opt-in via the selector;
/// slice 2 lands the airplane-grid for split).
export function defaultTeamConfig(): TeamDialogConfig {
  return {
    hostName: "",
    teamName: "",
    size: TEAM_MIN_SIZE,
    autoPrefix: true,
    members: [
      { name: "Lead", command: "claude", env: "", isLead: true },
      { name: "Worker1", command: "claude", env: "", isLead: false },
    ],
    realEstate: { kind: "tabs" },
  };
}

/// Returns the first validation issue with the supplied
/// config, or null when valid. Used by the dialog to
/// enable/disable the Bootstrap button + surface inline
/// errors.
export function validateTeamConfig(
  cfg: TeamDialogConfig,
  existingTeamNames: ReadonlySet<string> = new Set(),
): string | null {
  if (!cfg.hostName.trim()) return "host name required";
  if (!cfg.teamName.trim()) return "team name required";
  if (cfg.size < TEAM_MIN_SIZE) {
    return `team size must be at least ${TEAM_MIN_SIZE}`;
  }
  if (cfg.size > TEAM_MAX_SIZE) {
    return `team size must be at most ${TEAM_MAX_SIZE}`;
  }
  if (cfg.members.length !== cfg.size) {
    return "member count must match team size";
  }
  const leadCount = cfg.members.filter((m) => m.isLead).length;
  if (leadCount === 0) return "one member must be marked as lead";
  if (leadCount > 1) return "exactly one member can be marked as lead";
  if (cfg.members.some((m) => !m.name.trim())) {
    return "every member needs a name";
  }
  if (existingTeamNames.has(cfg.teamName.trim())) {
    return `team name "${cfg.teamName.trim()}" already exists`;
  }
  return null;
}

/// Resize `cfg.members` to match `cfg.size`. Truncates from the
/// end when shrinking; appends fresh `WorkerN` entries when
/// growing. Preserves the lead designation across the resize
/// (the lead always sits in slot 0; growth adds workers
/// beneath).
export function resizeTeamMembers(cfg: TeamDialogConfig): TeamDialogConfig {
  const out = { ...cfg, members: [...cfg.members] };
  while (out.members.length < out.size) {
    const n = out.members.length;
    out.members.push({
      name: `Worker${n}`,
      command: "claude",
      env: "",
      isLead: false,
    });
  }
  while (out.members.length > out.size) {
    out.members.pop();
  }
  // Ensure exactly one lead survives the resize (the original
  // lead may have been popped). Default the lead to slot 0
  // when the resize drops the prior lead.
  if (!out.members.some((m) => m.isLead) && out.members.length > 0) {
    out.members[0] = { ...out.members[0], isLead: true };
  }
  return out;
}
