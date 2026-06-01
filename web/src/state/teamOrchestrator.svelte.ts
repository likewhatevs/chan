// Team Work bootstrap orchestrator.
//
// Lead-first flow: the Team Work Lead terminal ALREADY EXISTS when this
// runs (App.svelte created it at Cmd+P via `createTeamWorkLeadTerminal`,
// and the dialog handed us its tab + pane id). Bootstrap:
//
//   1. Save/update the team config inside the workspace at the
//      dialog's `{teamDir}/config.toml`.
//   2. Launch the LEAD agent FIRST into the existing lead tab, then
//      the workers into new tabs (split-pane or tabs-in-Hybrid per
//      the dialog's real estate).
//   3. Each agent's identity is its `CHAN_TAB_NAME` env var.
//   4. Place the identity prompt into the LEAD's embedded editor.
//   5. Broadcast: deselect ALL terminals first, then enable ONLY the
//      lead + workers set.

import { api, type TeamConfigWire, type TeamMemberWire } from "../api/client";
import { notify } from "./notify.svelte";
import { teamNameFromDir } from "./teamConfigPath";
import {
  allTerminalTabs,
  buildSplitGrid,
  closeTab,
  layout,
  openTerminalInPane,
  primeTeamWork,
  renameTerminalTab,
  setActivePane,
  setTerminalBroadcastEnabled,
  setTerminalBroadcastTarget,
  terminalTabGroup,
  type TerminalTab,
} from "./tabs.svelte";
import type { TeamDialogConfig, TeamMemberDraft } from "./teamDialog.svelte";
import { defaultTabGroupFromPath } from "./teamDialog.svelte";

/// Context the dialog hands the orchestrator: the EXISTING Team Work
/// Lead terminal tab + the pane it lives in. The lead is never
/// respawned; the orchestrator launches the lead agent INTO this
/// tab.
export interface TeamBootstrapContext {
  leadTabId: string;
  leadPaneId: string;
}

/// Parse the dialog's free-form env field ("KEY=VALUE"
/// newline-separated) into a Record. Empty lines + surrounding
/// whitespace stripped. Invalid lines throw so the orchestrator can
/// bail with a message.
export function parseEnvLines(text: string): Record<string, string> {
  const env: Record<string, string> = {};
  if (!text) return env;
  for (const [idx, raw] of text.split(/\r?\n/).entries()) {
    const line = raw.trim();
    if (!line) continue;
    const eq = line.indexOf("=");
    if (eq <= 0) {
      throw new Error(`env line ${idx + 1} must be KEY=value`);
    }
    const key = line.slice(0, eq).trim();
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(key)) {
      throw new Error(`env line ${idx + 1} has an invalid key`);
    }
    env[key] = line.slice(eq + 1);
  }
  return env;
}

/// Compute the handle the way the dialog's `handleOf` helper does:
/// `@@<name>` when `autoPrefix` is on AND the name doesn't already
/// start with `@@`; raw otherwise.
export function memberHandle(member: TeamMemberDraft, autoPrefix: boolean): string {
  if (!autoPrefix) return member.name;
  return member.name.startsWith("@@") ? member.name : `@@${member.name}`;
}

/// Translate the SPA's camelCase `TeamDialogConfig` into the
/// snake_case `TeamConfigWire` shape persisted to the team's
/// config.toml. `created_at` is the call time (ISO 8601 UTC). Each member's env
/// is parsed into a Record; `CHAN_TAB_NAME=<handle>` is auto-injected
/// unless the user supplied an override (the per-tab env var IS the
/// agent's identity inside the PTY).
///
/// Real estate round-trips through the per-member `position`
/// (row/col) field that chan-team.toml already carries: a member in
/// split-cell `i` of an RxC grid gets `position = {row, col}`;
/// tabs-mode members get no position. `wireToDialog` reconstructs the
/// dialog's `realEstate` from these positions, so Load restores the
/// same layout the user saw on save.
export function translateConfig(config: TeamDialogConfig): TeamConfigWire {
  const hostHandle = memberHandle(
    { name: config.hostName, command: "", env: "", isLead: false },
    config.autoPrefix,
  );
  const positions = memberPositions(config);
  const members: TeamMemberWire[] = config.members.map((m, idx) => {
    const env = parseEnvLines(m.env);
    const handle = memberHandle(m, config.autoPrefix);
    if (!Object.prototype.hasOwnProperty.call(env, "CHAN_TAB_NAME")) {
      env.CHAN_TAB_NAME = handle;
    }
    const member: TeamMemberWire = {
      handle,
      command: m.command,
      env,
      is_lead: m.isLead,
    };
    // A shell member ("none", or an unset draft) carries no submit
    // chord, so the wire form omits `agent` entirely; only a named
    // agent round-trips the field.
    if (m.agent && m.agent !== "none") member.agent = m.agent;
    const pos = positions[idx];
    if (pos) member.position = pos;
    return member;
  });
  return {
    team_name: teamNameFromDir(config.teamDir),
    host_name: config.hostName,
    host_handle: hostHandle,
    tab_group: config.tabGroup,
    auto_prefix_at: config.autoPrefix,
    created_at: new Date().toISOString(),
    members,
  };
}

/// Map each member index to its split-grid `{row, col}` position.
/// Returns an array aligned with `config.members`; entries are
/// undefined in tabs mode (no position persisted).
function memberPositions(
  config: TeamDialogConfig,
): (TeamMemberWire["position"] | undefined)[] {
  const out: (TeamMemberWire["position"] | undefined)[] = config.members.map(
    () => undefined,
  );
  if (config.realEstate.kind !== "split") return out;
  const { grid, slots } = config.realEstate;
  for (let cellIdx = 0; cellIdx < slots.length; cellIdx += 1) {
    const row = Math.floor(cellIdx / grid.cols);
    const col = cellIdx % grid.cols;
    for (const memberIdx of slots[cellIdx] ?? []) {
      if (memberIdx >= 0 && memberIdx < out.length) {
        out[memberIdx] = { row, col };
      }
    }
  }
  return out;
}

/// Inverse of `translateConfig`: map the snake_case wire shape back
/// into the dialog's camelCase `TeamDialogConfig` so the Load flow
/// opens the dialog populated from the team's config.toml. The user
/// edits, hits Bootstrap, and the config is re-saved with their
/// changes.
///
/// `env` Records serialise back to "KEY=VALUE\n" lines;
/// `CHAN_TAB_NAME` is dropped from the visible env field
/// (`translateConfig` re-injects it on save, so showing it would
/// create a duplicate on round-trip). Real estate is reconstructed
/// from member positions.
export function wireToDialog(
  wire: TeamConfigWire,
  dir: string,
): TeamDialogConfig {
  const members: TeamMemberDraft[] = wire.members.map((m) => {
    const envText = Object.entries(m.env)
      .filter(([k]) => k !== "CHAN_TAB_NAME")
      .map(([k, v]) => `${k}=${v}`)
      .join("\n");
    return {
      name: m.handle,
      command: m.command,
      env: envText,
      isLead: m.is_lead,
      // An omitted wire `agent` is a shell member; read it back as
      // "none" so the dialog's picker shows the right selection.
      agent: m.agent ?? "none",
    };
  });
  const size = Math.max(members.length, 1);
  return {
    hostName: wire.host_name,
    configMode: "load",
    teamDir: dir,
    tabGroup: wire.tab_group ?? defaultTabGroupFromPath(dir),
    size,
    autoPrefix: wire.auto_prefix_at,
    members,
    realEstate: realEstateFromWire(wire, size),
  };
}

/// Rebuild the dialog's `realEstate` from member positions. When no
/// member carries a position, the team is tabs-in-current-Hybrid.
/// Otherwise derive the grid from the max row/col seen + map each
/// positioned member into its row-major cell.
function realEstateFromWire(
  wire: TeamConfigWire,
  size: number,
): TeamDialogConfig["realEstate"] {
  const positioned = wire.members
    .map((m, idx) => ({ pos: m.position, idx }))
    .filter((e): e is { pos: NonNullable<TeamMemberWire["position"]>; idx: number } =>
      Boolean(e.pos),
    );
  if (positioned.length === 0) return { kind: "tabs" };
  let maxRow = 0;
  let maxCol = 0;
  for (const { pos } of positioned) {
    maxRow = Math.max(maxRow, pos.row);
    maxCol = Math.max(maxCol, pos.col);
  }
  const grid = { rows: maxRow + 1, cols: maxCol + 1 };
  const slots: number[][] = Array.from(
    { length: grid.rows * grid.cols },
    () => [],
  );
  for (const { pos, idx } of positioned) {
    if (idx >= size) continue;
    const cellIdx = pos.row * grid.cols + pos.col;
    if (cellIdx >= 0 && cellIdx < slots.length) slots[cellIdx].push(idx);
  }
  return { kind: "split", grid, slots };
}

/// Build the `# Team work` identity prompt placed in the lead's
/// embedded editor. `$CHAN_TAB_NAME` is intentionally NOT escaped:
/// the lead's shell expands it to the env-var value when the agent
/// reads the prompt. The team size, host handle, lead handle, and
/// worker handles substitute in literally. The trailing line points
/// every agent at the generated team bootstrap doc (`bootstrapPath`,
/// e.g. `{teamDir}/bootstrap.md`) so they read the shared process
/// before starting.
export function identityPrompt(
  size: number,
  hostHandle: string,
  leadHandle: string,
  workerHandles: string[],
  bootstrapPath: string,
): string {
  const bullets =
    workerHandles.length > 0
      ? workerHandles.map((h) => `- ${h}`).join("\n")
      : "- (no other agents)";
  return (
    `# Team work\n` +
    `We are a team of ${size}. Our host is ${hostHandle} and the team lead ` +
    `is ${leadHandle}.\n` +
    `You are $CHAN_TAB_NAME. Identify yourself and get ready to work with\n` +
    `the rest of the team:\n` +
    bullets +
    `\n` +
    `Read the team process at ${bootstrapPath} before you start.`
  );
}

/// Launch the lead agent by spawning a FRESH session running the agent
/// into the lead's pane and dropping the Cmd+P placeholder shell - the
/// SAME spawn+mount path the workers use (one create path, TEAM-CONSOLIDATE).
///
/// We do NOT restart the placeholder in place. The orchestrator's
/// external `api.restartTerminal` closes the lead's session but never
/// flips the lead `TerminalTab` to "connecting" (only a component-
/// initiated restart does, e.g. the UI restart button), so the SPA shows
/// "session ended (explicit)" and never reattaches - smoke-confirmed: the
/// lead agent never came up while workers (fresh spawns) did. A fresh
/// spawn yields a fresh `TerminalTab` mount bound to the new session, so
/// the lead agent launches exactly like a worker's.
/// Resolve the team's tab group against the LIVE terminal groups at
/// Bootstrap, appending `-N` until unique so a new team never collides
/// with an existing group. `allTerminalTabs().map(terminalTabGroup)`
/// mirrors what the registry / `cs terminal list` reads.
function resolveTeamGroup(base: string): string {
  const live = new Set(allTerminalTabs().map(terminalTabGroup));
  if (!live.has(base)) return base;
  for (let n = 2; n < 1000; n += 1) {
    const candidate = `${base}-${n}`;
    if (!live.has(candidate)) return candidate;
  }
  return base;
}

async function launchLead(
  ctx: TeamBootstrapContext,
  lead: TeamMemberWire,
  group: string,
): Promise<TerminalTab> {
  const response = await api.spawnTerminal({
    name: lead.handle,
    command: lead.command,
    env: lead.env,
    group,
  });
  const tab = openTerminalInPane(ctx.leadPaneId, {
    sessionId: response.session,
    title: response.tab_label,
    group,
  });
  if (!tab) throw new Error("failed to open the lead terminal");
  renameTerminalTab(tab, lead.handle);
  // Drop the Cmd+P placeholder shell + its session. Force-close so no
  // confirm modal blocks the bootstrap; done AFTER opening the fresh lead
  // so the lead pane is never momentarily empty.
  await closeTab(ctx.leadPaneId, ctx.leadTabId, { force: true });
  return tab;
}

/// Resolve the target pane for each worker (by member index) +
/// confirm the lead's pane. Tabs mode: every worker shares the lead's
/// pane (stacked as tabs). Split mode: build the RxC grid rooted at
/// the lead's pane (cell 0 = lead) and map each worker to its
/// assigned cell; unassigned workers fall back to the lead's pane.
function resolveWorkerPanes(
  config: TeamDialogConfig,
  leadPaneId: string,
): (string | undefined)[] {
  if (config.realEstate.kind === "tabs") {
    return config.members.map(() => leadPaneId);
  }
  const { grid, slots } = config.realEstate;
  const cells = buildSplitGrid(leadPaneId, grid.rows, grid.cols);
  const fallback = cells[0] ?? leadPaneId;
  const panes: (string | undefined)[] = config.members.map(() => undefined);
  for (let cellIdx = 0; cellIdx < slots.length; cellIdx += 1) {
    for (const memberIdx of slots[cellIdx] ?? []) {
      if (memberIdx < 0 || memberIdx >= panes.length) continue;
      panes[memberIdx] = cells[cellIdx] ?? fallback;
    }
  }
  for (let i = 0; i < panes.length; i += 1) {
    if (!panes[i]) panes[i] = fallback;
  }
  return panes;
}

/// Run the lead-first bootstrap chain. Throws on a step's failure so
/// the dialog can surface the error inline; the caller closes the
/// dialog on success.
export async function runTeamBootstrap(
  config: TeamDialogConfig,
  ctx: TeamBootstrapContext,
): Promise<void> {
  const wire = translateConfig(config);

  // 1. Save/update the team config inside the workspace at
  //    `{teamDir}/config.toml`. The backend writes it (plus the
  //    generated bootstrap.md + the tasks/journals/followups dirs)
  //    through the Workspace sandbox: atomic, path-sandboxed,
  //    special-file refusal (see api.writeTeamConfig).
  await api.writeTeamConfig(config.teamDir, wire);

  const leadEntry = wire.members.find((m) => m.is_lead);
  if (!leadEntry) throw new Error("config has no lead member");
  const workerEntries = wire.members.filter((m) => !m.is_lead);

  // Resolve the team's tab group once (with a -N suffix on collision)
  // so every team terminal - lead + workers - joins the same group
  // server-side ($CHAN_TAB_GROUP + cs terminal list) and SPA-side
  // (group-scoped broadcast).
  const group = resolveTeamGroup(config.tabGroup);

  // 2a. Launch the LEAD FIRST: spawn a fresh agent session into the
  //     lead's pane and drop the Cmd+P placeholder (same path as workers).
  const leadTab = await launchLead(ctx, leadEntry, group);

  // 2b. Resolve real estate + spawn the workers into new tabs.
  const workerPanes = resolveWorkerPanes(config, ctx.leadPaneId);
  const workerTabs: TerminalTab[] = [];
  for (let i = 0; i < config.members.length; i += 1) {
    const m = wire.members[i];
    if (m.is_lead) continue;
    try {
      const response = await api.spawnTerminal({
        name: m.handle,
        command: m.command,
        env: m.env,
        group,
      });
      const paneId = workerPanes[i] ?? ctx.leadPaneId;
      const tab = openTerminalInPane(paneId, {
        sessionId: response.session,
        title: response.tab_label,
        group,
      });
      if (tab) {
        renameTerminalTab(tab, m.handle);
        workerTabs.push(tab);
      }
    } catch (err) {
      notify(`spawn failed for ${m.handle}: ${(err as Error).message ?? err}`);
    }
  }

  // 3 + 4. Place the identity prompt in the lead's embedded editor.
  //    `$CHAN_TAB_NAME` is each agent's identity (env var, step 3).
  //    The prompt's trailing line points agents at the generated
  //    `{teamDir}/bootstrap.md`; strip any trailing slash so the path
  //    reads cleanly.
  const bootstrapPath = `${config.teamDir.replace(/\/+$/, "")}/bootstrap.md`;
  const prompt = identityPrompt(
    wire.members.length,
    wire.host_handle,
    leadEntry.handle,
    workerEntries.map((m) => m.handle),
    bootstrapPath,
  );
  primeTeamWork(leadTab, prompt);

  // Seed the lead tab's Team Work composer submit mode from the lead
  // member's agent. The composer is the surface that pokes/drives the
  // lead agent, so it must submit with THAT agent's chord (claude's
  // Cmd+Enter CSI vs codex/gemini's plain CR); a shell lead ("none")
  // stays in shell mode (plain Enter). `primeTeamWork` above guarantees
  // `leadTab.teamWork` exists, so we mutate it directly (the
  // orchestrator already owns this tab's reactive state). Mirrors
  // TeamWork.svelte's setAgentTarget: agentTarget !== "none" => "agent".
  const leadAgent = leadEntry.agent ?? "none";
  if (leadTab.teamWork) {
    leadTab.teamWork.agentTarget = leadAgent;
    leadTab.teamWork.submitMode = leadAgent === "none" ? "shell" : "agent";
  }

  // Restore focus to the lead's pane so the editor lands there.
  setActivePane(ctx.leadPaneId);

  // 5. Broadcast membership. First force-clear EVERY terminal's
  //    broadcast (the spec's "Deselect all" equivalent) so no
  //    pre-existing broadcast group leaks into the new team. Then
  //    enable ONLY the lead + workers set. We use the
  //    setTerminalBroadcast* primitives (force-clear+set), not the
  //    toggle helper, so the final membership is deterministic.
  for (const tab of allTerminalTabs()) {
    setTerminalBroadcastEnabled(tab, false);
  }
  setTerminalBroadcastEnabled(leadTab, true);
  for (const tab of workerTabs) {
    setTerminalBroadcastTarget(leadTab, tab.id, true);
  }

  notify(`Team "${wire.team_name}" bootstrapped.`);
}
