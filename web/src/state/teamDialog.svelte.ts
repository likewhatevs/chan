// `fullstack-a-78`: global Spawn agents dialog request bus.
//
// Mirrors the spawnDialog pattern (`-a-4`): a state singleton +
// open/close helpers, dialog rendered at App root so it's free
// of every parent stacking context (rich-prompt is absolute /
// z-index:20; panes are overflow-hidden; Hybrid Nav adds a
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
///
/// `tabs`: all team terminals spawn as tabs in the host's
/// current Hybrid (single pane).
///
/// `split`: a grid of panes; each pane holds one or more
/// terminals (multi-robot per cell = tabs in that pane). The
/// `grid` field locks the row/col shape; `slots` is a
/// flattened array where `slots[i]` is the list of member
/// indexes (positions in `TeamDialogConfig.members`) assigned
/// to cell `i`. Cell ordering is row-major.
export type TeamRealEstate =
  | { kind: "tabs" }
  | { kind: "split"; grid: GridShape; slots: number[][] };

/// A grid shape (rows × cols) + a human label for the picker.
/// Capacity (`rows * cols`) is what determines how many cells
/// the grid offers; users can leave cells empty (orchestrator
/// drops the empty panes at materialise time) OR stack
/// multiple robots in a single cell.
export interface GridShape {
  rows: number;
  cols: number;
}

/// Available grid shapes for a given team size. Returns a list
/// in display order (most-balanced first). Sizes without a
/// good factor pair fall back to 1×N (e.g. size 5 → just 1×5).
///
/// Capacity ≥ size for every returned shape (the user can
/// leave cells empty). The orchestrator (`-a-79`) drops empty
/// cells at materialise time so a 2×2 grid with only 2 robots
/// assigned reads as a 2-pane layout.
export function gridShapesForSize(size: number): GridShape[] {
  if (size <= 1) return [{ rows: 1, cols: Math.max(1, size) }];
  const shapes: GridShape[] = [];
  const seen = new Set<string>();
  const push = (rows: number, cols: number): void => {
    if (rows * cols < size) return;
    const key = `${rows}x${cols}`;
    if (seen.has(key)) return;
    seen.add(key);
    shapes.push({ rows, cols });
  };
  // Most-balanced shape: smallest perimeter-ish R×C that still
  // fits. Walk R from floor(sqrt(size)) outward and pick the
  // best matching C.
  const root = Math.floor(Math.sqrt(size));
  for (let r = root; r >= 1; r -= 1) {
    const c = Math.ceil(size / r);
    push(r, c);
  }
  // Transpose pairs (so 2×3 + 3×2 both show).
  for (const s of [...shapes]) {
    if (s.rows !== s.cols) push(s.cols, s.rows);
  }
  // Always include the linear fallbacks (1×N + N×1).
  push(1, size);
  push(size, 1);
  return shapes;
}

/// Default grid shape for a given size: most-balanced (the
/// first entry in `gridShapesForSize`). 5 → 1×5; 6 → 2×3; etc.
export function defaultGridForSize(size: number): GridShape {
  const shapes = gridShapesForSize(size);
  return shapes[0] ?? { rows: 1, cols: Math.max(1, size) };
}

/// Empty slots arena sized for `grid.rows * grid.cols`. Each
/// cell starts empty; drag&drop populates the inner arrays.
export function emptySlotsForGrid(grid: GridShape): number[][] {
  const cells = Math.max(1, grid.rows * grid.cols);
  return Array.from({ length: cells }, () => []);
}

export interface TeamDialogConfig {
  hostName: string;
  teamName: string;
  /// Total agents (lead + workers). Phase 9 uses min 1 and max 9.
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
  /// Undefined for the bare Spawn agents flow.
  initial?: Partial<TeamDialogConfig>;
  /// Called when the user clicks Bootstrap with the final
  /// config. The orchestrator (`-a-79`) is the consumer; this
  /// task's scope ends at the handoff.
  onBootstrap: (config: TeamDialogConfig) => void | Promise<void>;
  /// Called after a host terminal is spawned for the new agent group
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

/// Phase 9 Spawn agents limits: one agent is allowed and is the
/// lead by definition. The user is not counted; they sit in the
/// rich-prompt host terminal.
export const TEAM_MIN_SIZE = 1;
export const TEAM_MAX_SIZE = 9;

/// Default team config used as the dialog's initial state.
/// One lead member. Auto-prefix on. Real estate
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
  // `fullstack-a-80` slice 2: copy fix per @@WebtestA's
  // round 41 note. The validator message used "host name"
  // but the dialog labels the field "Your name"; the
  // mismatch reads as a stale error reference. Use the
  // dialog-visible labels so the user knows which input to
  // fix.
  if (!cfg.hostName.trim()) return "Your name required";
  if (!cfg.teamName.trim()) return "Team name required";
  if (cfg.size < TEAM_MIN_SIZE) {
    return `agent count must be at least ${TEAM_MIN_SIZE}`;
  }
  if (cfg.size > TEAM_MAX_SIZE) {
    return `agent count must be at most ${TEAM_MAX_SIZE}`;
  }
  if (cfg.members.length !== cfg.size) {
    return "member count must match agent count";
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

export function exportTeamDialogConfig(cfg: TeamDialogConfig): string {
  return JSON.stringify(cfg, null, 2);
}

export function importTeamDialogConfig(text: string): TeamDialogConfig {
  let raw: unknown;
  try {
    raw = JSON.parse(text);
  } catch (err) {
    throw new Error(`config is not valid JSON: ${(err as Error).message}`);
  }
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
    throw new Error("config must be a JSON object");
  }
  const obj = raw as Record<string, unknown>;
  const rawMembers = Array.isArray(obj.members) ? obj.members : [];
  const requestedSize =
    typeof obj.size === "number" && Number.isFinite(obj.size)
      ? Math.trunc(obj.size)
      : rawMembers.length || TEAM_MIN_SIZE;
  const size = Math.max(TEAM_MIN_SIZE, Math.min(TEAM_MAX_SIZE, requestedSize));
  const base = defaultTeamConfig();
  const members = rawMembers
    .slice(0, size)
    .map((member, idx) => normalizeImportedMember(member, idx));
  const cfg: TeamDialogConfig = {
    ...base,
    hostName: stringField(obj.hostName, base.hostName),
    teamName: stringField(obj.teamName, base.teamName),
    size,
    autoPrefix:
      typeof obj.autoPrefix === "boolean" ? obj.autoPrefix : base.autoPrefix,
    members: members.length > 0 ? members : base.members,
    realEstate: importedRealEstate(obj.realEstate, size),
  };
  return resizeTeamMembers(cfg);
}

function stringField(value: unknown, fallback: string): string {
  return typeof value === "string" ? value : fallback;
}

function normalizeImportedMember(value: unknown, idx: number): TeamMemberDraft {
  const base: TeamMemberDraft = {
    name: idx === 0 ? "Lead" : `Worker${idx}`,
    command: "claude",
    env: "",
    isLead: idx === 0,
  };
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return base;
  }
  const obj = value as Record<string, unknown>;
  return {
    name: stringField(obj.name, base.name),
    command: stringField(obj.command, base.command),
    env: stringField(obj.env, base.env),
    isLead: typeof obj.isLead === "boolean" ? obj.isLead : base.isLead,
  };
}

function importedRealEstate(value: unknown, size: number): TeamRealEstate {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return { kind: "tabs" };
  }
  const obj = value as Record<string, unknown>;
  if (obj.kind !== "split") return { kind: "tabs" };
  const gridObj = obj.grid;
  if (!gridObj || typeof gridObj !== "object" || Array.isArray(gridObj)) {
    const grid = defaultGridForSize(size);
    return { kind: "split", grid, slots: emptySlotsForGrid(grid) };
  }
  const gridRecord = gridObj as Record<string, unknown>;
  const rows = positiveInt(gridRecord.rows);
  const cols = positiveInt(gridRecord.cols);
  const grid = rows && cols ? { rows, cols } : defaultGridForSize(size);
  const slots = importedSlots(obj.slots, grid, size);
  return { kind: "split", grid, slots };
}

function positiveInt(value: unknown): number | null {
  if (typeof value !== "number" || !Number.isFinite(value)) return null;
  const n = Math.trunc(value);
  return n > 0 ? n : null;
}

function importedSlots(value: unknown, grid: GridShape, size: number): number[][] {
  const cells = grid.rows * grid.cols;
  if (!Array.isArray(value)) return emptySlotsForGrid(grid);
  return Array.from({ length: cells }, (_, cellIdx) => {
    const rawCell = value[cellIdx];
    if (!Array.isArray(rawCell)) return [];
    return rawCell
      .map((idx) => (typeof idx === "number" ? Math.trunc(idx) : -1))
      .filter((idx) => idx >= 0 && idx < size);
  });
}

/// `fullstack-a-78` slice 2: switch the real-estate strategy
/// while preserving any previously-configured split grid (so
/// the user can toggle tabs ↔ split without losing their
/// arrangement). When switching INTO `split`, picks the
/// default grid for the team size + empty slots.
export function switchRealEstate(
  cfg: TeamDialogConfig,
  kind: TeamRealEstate["kind"],
): TeamDialogConfig {
  if (kind === "tabs") return { ...cfg, realEstate: { kind: "tabs" } };
  if (cfg.realEstate.kind === "split") return cfg;
  const grid = defaultGridForSize(cfg.size);
  return { ...cfg, realEstate: { kind: "split", grid, slots: emptySlotsForGrid(grid) } };
}

/// Reshape the active split grid + reset slots to empty.
/// Called from the airplane-grid picker when the user clicks
/// an alternative shape (e.g. 1×4 ↔ 2×2 for size 4).
export function reshapeSplitGrid(
  cfg: TeamDialogConfig,
  grid: GridShape,
): TeamDialogConfig {
  if (cfg.realEstate.kind !== "split") return cfg;
  return {
    ...cfg,
    realEstate: {
      kind: "split",
      grid,
      slots: emptySlotsForGrid(grid),
    },
  };
}

/// Assign a member index to a split grid cell. If the member
/// was previously assigned elsewhere, remove from the prior
/// cell first. Same-cell drop is a no-op (idempotent).
/// Multiple members per cell are allowed (they materialise as
/// tabs in that pane).
export function assignMemberToCell(
  cfg: TeamDialogConfig,
  memberIdx: number,
  cellIdx: number,
): TeamDialogConfig {
  if (cfg.realEstate.kind !== "split") return cfg;
  const slots = cfg.realEstate.slots.map((cell, i) => {
    const filtered = cell.filter((m) => m !== memberIdx);
    if (i === cellIdx && !filtered.includes(memberIdx)) {
      return [...filtered, memberIdx];
    }
    return filtered;
  });
  return {
    ...cfg,
    realEstate: { kind: "split", grid: cfg.realEstate.grid, slots },
  };
}

/// Remove a member from every split-grid cell. Used by the
/// member-row "unassign" affordance + by `resizeTeamMembers`
/// to clean up after a shrink removed the member entirely.
export function unassignMember(
  cfg: TeamDialogConfig,
  memberIdx: number,
): TeamDialogConfig {
  if (cfg.realEstate.kind !== "split") return cfg;
  const slots = cfg.realEstate.slots.map((cell) =>
    cell.filter((m) => m !== memberIdx),
  );
  return {
    ...cfg,
    realEstate: { kind: "split", grid: cfg.realEstate.grid, slots },
  };
}

/// Resize `cfg.members` to match `cfg.size`. Truncates from the
/// end when shrinking; appends fresh `WorkerN` entries when
/// growing. Preserves the lead designation across the resize
/// (the lead always sits in slot 0; growth adds workers
/// beneath).
export function resizeTeamMembers(cfg: TeamDialogConfig): TeamDialogConfig {
  let out = { ...cfg, members: [...cfg.members] };
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
  // `fullstack-a-78` slice 2: when `realEstate.kind === "split"`,
  // re-pick the default grid for the new size + drop slot
  // assignments referencing now-removed members. Keep the
  // split mode; the user explicitly picked it.
  if (out.realEstate.kind === "split") {
    const grid = defaultGridForSize(out.size);
    const memberCount = out.members.length;
    const slots = emptySlotsForGrid(grid);
    // Migrate prior assignments that still reference valid
    // members; drop assignments for members beyond the new
    // count. Row-major flattened indexing means the
    // re-pick takes the first N members of the prior
    // assignment in order, capped at the new grid's
    // capacity.
    let cellCursor = 0;
    for (const cell of out.realEstate.slots) {
      for (const memberIdx of cell) {
        if (memberIdx >= memberCount) continue;
        if (cellCursor >= slots.length) break;
        slots[cellCursor].push(memberIdx);
      }
      cellCursor = Math.min(cellCursor + 1, slots.length - 1);
    }
    out = { ...out, realEstate: { kind: "split", grid, slots } };
  }
  return out;
}
