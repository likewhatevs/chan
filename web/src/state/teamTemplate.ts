/// `fullstack-a-81` Team-process template substitution helper.
///
/// Bootstraps new chan-Drafts teams by reading parameterised
/// templates from `docs/templates/team-process/` + substituting
/// the team's actual handles. `-a-79`'s orchestrator calls this
/// after the team's config + watcher land.
///
/// Substitution variables follow the addendum-b clarification:
/// process happens between Host ↔ Lead and Lead ↔ Workers. Chan
/// agents themselves are a special-case team where
/// `{host-handle} = @@Alex`, `{lead-handle} = @@Architect`,
/// workers = @@FullStackA / @@Systacean / etc.

export interface TeamTemplateVars {
  /// The team's host handle (e.g. `@@Alex`). Receives the
  /// initial prompt + grants decisions; doesn't run inside the
  /// agent loop.
  hostHandle: string;
  /// The team's lead handle (e.g. `@@Architect`). Orchestrates
  /// the worker agents, dispatches tasks, holds the queue.
  leadHandle: string;
  /// Worker handles in dispatch order (e.g. ["@@FullStackA",
  /// "@@Systacean", "@@CI"]). Substituted into the template as
  /// `{worker-1-handle}`, `{worker-2-handle}`, etc. — the
  /// template can reference any subset; missing indexes leave
  /// the placeholder as-is so the template author sees the
  /// gap.
  workerHandles: string[];
  /// Optional team name (e.g. `team-alpha`). Substituted as
  /// `{team-name}`. Falls back to "team" if unset so the
  /// template still renders.
  teamName?: string;
  /// `fullstack-a-81` slice 4: phase-slug for the team's
  /// working-directory layout. Substituted as `{phase-slug}`
  /// everywhere the source template hardcoded chan's own
  /// `phase-8` path / prose form. Chan-internal substitution
  /// stays `phase-8`; new teams typically start at `phase-1`
  /// or omit phases entirely (orchestrator's call).
  ///
  /// Falls back to "phase-1" if unset so a fresh new-team
  /// substitution doesn't render `{phase-slug}` literally on
  /// the user's first read. Empty string skips the
  /// substitution + leaves `{phase-slug}` so the orchestrator
  /// sees a gap.
  phaseSlug?: string;
}

/// Substitute `{host-handle}` / `{lead-handle}` /
/// `{worker-N-handle}` / `{team-name}` tokens in `template`
/// using the supplied vars. Unknown tokens (typos, gaps in the
/// worker list) are left as-is so the template author sees the
/// gap rather than getting an empty silent substitution.
///
/// Token grammar: `{<kebab-case-name>}`. The helper only
/// recognises the documented tokens above; arbitrary
/// `{anything-else}` strings are preserved verbatim. The
/// kebab-case convention is the addendum-b shape; CamelCase
/// or snake_case variants are NOT recognised (so a future
/// `{TeamName}` typo gets caught at audit time).
export function substituteTeamTemplate(
  template: string,
  vars: TeamTemplateVars,
): string {
  const teamName = vars.teamName ?? "team";
  const phaseSlug = vars.phaseSlug ?? "phase-1";
  return template.replace(
    /\{(host-handle|lead-handle|worker-(\d+)-handle|team-name|phase-slug)\}/g,
    (_, token: string, workerIdx: string | undefined) => {
      if (token === "host-handle") return vars.hostHandle;
      if (token === "lead-handle") return vars.leadHandle;
      if (token === "team-name") return teamName;
      if (token === "phase-slug") return phaseSlug;
      const idx = Number(workerIdx) - 1;
      if (
        Number.isInteger(idx) &&
        idx >= 0 &&
        idx < vars.workerHandles.length
      ) {
        return vars.workerHandles[idx];
      }
      // Gap in the worker list — leave the placeholder so the
      // template author / orchestrator sees the missing
      // worker. Better than silently rendering an empty
      // handle.
      return `{worker-${workerIdx}-handle}`;
    },
  );
}

/// Chan's own substitution vars — the special-case "team" the
/// chan project itself runs as. `-a-79`'s orchestrator can
/// reuse this for the chan-internal substitution path; new
/// teams supply their own vars at bootstrap.
export const CHAN_INTERNAL_TEAM_VARS: TeamTemplateVars = {
  hostHandle: "@@Alex",
  leadHandle: "@@Architect",
  workerHandles: [
    "@@FullStackA",
    "@@FullStackB",
    "@@Systacean",
    "@@CI",
    "@@WebtestA",
    "@@WebtestB",
  ],
  teamName: "chan",
  // `fullstack-a-81` slice 4: chan's own working directory is
  // under `docs/journals/phase-8/` — the templated bootstrap
  // doc references that path 43 times via `{phase-slug}`.
  // Substituting with `phase-8` here gives chan's own agents
  // an identical bootstrap to what they read pre-`-81` slice 4
  // (templated render is byte-equivalent for the chan case).
  phaseSlug: "phase-8",
};
