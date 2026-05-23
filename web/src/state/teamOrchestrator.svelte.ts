// `fullstack-a-79` slice 1: Team Bootstrap orchestrator.
//
// Walks the steps the addendum-b spec lays out for the New Team
// dialog's Bootstrap action. Slice 1 lands the core chain:
//
//   1. Persist config — `api.teamCreate(name, configWire)`.
//   2. Load the watcher — `api.teamLoad(name)`.
//   3. Spawn one terminal per non-lead member with
//      `CHAN_TAB_NAME=<handle>` env + the agent's command. Each
//      worker's TerminalTab is seeded with the identity prompt so
//      the agent reads it on first mount.
//   4. Surface a notify() on success so the user sees the team
//      came up.
//
// Slice 2+ items deferred:
//
//   * Process-template placement (copying `-a-81`'s parameterised
//     docs into `Drafts/team-{name}/docs/`).
//   * Lead-side pre-flight survey trigger.
//   * Split-pane real estate (slice 1 routes everything through
//     tabs-in-current-Hybrid; the split-pane branch is just a
//     scope-poke today).
//   * `dispatch_agent_event`-driven identity prompts (slice 1
//     uses `seedInput` for the in-process delivery; the
//     event-channel path is wired in `-a-79` slice 2 when
//     `systacean-21`'s rich-poke flow consumes a team channel).

import { api, type TeamConfigWire, type TeamMemberWire } from "../api/client";
import { notify } from "./notify.svelte";
import { openTerminalInActivePane } from "./tabs.svelte";
import type {
  TeamDialogConfig,
  TeamMemberDraft,
} from "./teamDialog.svelte";

/// `fullstack-a-79`: parse the dialog's free-form env field
/// ("KEY=VALUE" newline-separated) into a Record. Empty lines +
/// surrounding whitespace stripped. Invalid lines surface as
/// thrown errors so the orchestrator can bail with a message.
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

/// `fullstack-a-79`: compute the handle the way the dialog's
/// `handleOf` helper does — `@@<name>` when `autoPrefix` is on
/// AND the name doesn't already start with `@@`; raw otherwise.
/// Mirrors `TeamDialog.svelte`'s helper so the persisted config
/// matches what the user saw at submit time.
export function memberHandle(member: TeamMemberDraft, autoPrefix: boolean): string {
  if (!autoPrefix) return member.name;
  return member.name.startsWith("@@") ? member.name : `@@${member.name}`;
}

/// `fullstack-a-79`: translate the SPA's camelCase
/// `TeamDialogConfig` into the chan-drive snake_case
/// `TeamConfigWire` shape that `Drive::create_team` accepts.
/// `created_at` is set to the call time (ISO 8601 UTC) so the
/// server gets a sortable timestamp. Member env strings are
/// parsed into Records; CHAN_TAB_NAME gets auto-injected (per
/// addendum-b clarification #8) so users don't have to type it.
export function translateConfig(config: TeamDialogConfig): TeamConfigWire {
  const hostHandle = memberHandle(
    { name: config.hostName, command: "", env: "", isLead: false },
    config.autoPrefix,
  );
  const members: TeamMemberWire[] = config.members.map((m) => {
    const env = parseEnvLines(m.env);
    const handle = memberHandle(m, config.autoPrefix);
    // Auto-inject CHAN_TAB_NAME=<handle> unless the user
    // already supplied an override. The per-tab env-var IS the
    // agent's identity inside the PTY.
    if (!Object.prototype.hasOwnProperty.call(env, "CHAN_TAB_NAME")) {
      env.CHAN_TAB_NAME = handle;
    }
    return {
      handle,
      command: m.command,
      env,
      is_lead: m.isLead,
    };
  });
  return {
    team_name: config.teamName,
    host_name: config.hostName,
    host_handle: hostHandle,
    auto_prefix_at: config.autoPrefix,
    created_at: new Date().toISOString(),
    members,
  };
}

/// `fullstack-a-79`: assemble the identity prompt addendum-b
/// clarification #4 calls for. `$CHAN_TAB_NAME` is intentionally
/// NOT escaped — the worker's shell expands it to the env-var
/// value when the agent reads the prompt. The host-handle
/// substitutes in literally.
export function identityPrompt(hostHandle: string): string {
  return (
    `I'm ${hostHandle}. You're $CHAN_TAB_NAME. ` +
    `Identify yourself, and then read docs/agents/bootstrap.md`
  );
}

/// `fullstack-a-79` slice 1: run the bootstrap chain. Throws
/// (returns rejected promise) on any step's failure so the
/// dialog can surface the error inline. The caller closes the
/// dialog on success.
export async function runTeamBootstrap(
  config: TeamDialogConfig,
  hostSessionId?: string,
): Promise<void> {
  const wire = translateConfig(config);
  if (config.realEstate.kind === "split") {
    // Slice 1 routes everything through tabs-in-current-Hybrid.
    // Split-pane real estate (paneSplit + per-cell assignment)
    // is slice 2. Flag explicitly so callers can decide whether
    // to bail or continue with the tab fallback; today the
    // dialog only offers tabs by default, and slice 2 wiring
    // lands before users can reach this branch.
    notify(
      "Split-pane real estate not yet wired — falling back to tabs (slice 1).",
    );
  }
  // 1. Persist config.
  await api.teamCreate(wire.team_name, wire);
  // 2. Load watcher.
  await api.teamLoad(wire.team_name);
  // 3. Spawn worker terminals (lead is the host session — see
  //    addendum-b clarification #1).
  const prompt = identityPrompt(wire.host_handle);
  for (const m of wire.members) {
    if (m.is_lead) continue;
    try {
      const response = await api.spawnTerminal({
        name: m.handle,
        command: m.command,
        env: m.env,
        ...(hostSessionId ? { orchestrator_session: hostSessionId } : {}),
      });
      openTerminalInActivePane({
        sessionId: response.session,
        title: response.tab_label,
        seedInput: prompt,
      });
    } catch (err) {
      notify(
        `spawn failed for ${m.handle}: ${(err as Error).message ?? err}`,
      );
    }
  }
  notify(`Team "${wire.team_name}" bootstrapped.`);
}
