# TEAM-GROUP interface — @@LaneB (dialog) <-> @@LaneD (orchestrator)

Contract for adding a "Terminal tab group name" to Team Work, split so
@@LaneB builds the dialog in parallel while @@LaneD does the orchestrator
glue + TEAM-SELFSTART/CONSOLIDATE. Routed via @@Architect.

## The shared field

Add one field to `TeamDialogConfig`
(`web/src/state/teamDialog.svelte.ts:111`):

```ts
export interface TeamDialogConfig {
  ...
  configPath: string;
  /// Terminal tab-group every team terminal joins ($CHAN_TAB_GROUP).
  /// Default derived from the config filename; the orchestrator resolves
  /// a -N suffix at Bootstrap if it collides with a live group.
  tabGroup: string;
  ...
}
```

## @@LaneB owns (the dialog)

1. **The input.** A "Terminal tab group name" text field in `TeamDialog.svelte`,
   next to "Path to configuration". Bound to `config.tabGroup`.
2. **Default-from-filename.** A helper in `teamDialog.svelte.ts`:
   ```ts
   export function defaultTabGroupFromPath(configPath: string): string {
     // /tmp/new-team-1/chan-team.toml -> "chan-team"
     const base = configPath.split("/").pop() ?? "";
     return base.replace(/\.toml$/i, "") || "chan-team";
   }
   ```
   Seed `tabGroup` from this when a config is created AND keep it in sync as
   the user edits `configPath` UNLESS they have hand-edited the group (same
   "dirty" pattern you already use for any derived field; if none, simplest
   is: re-derive while the field still equals the previous default).
3. **Persistence.** Thread `tabGroup` through the New/Load round-trip the
   same way `configPath`/`hostName` go (translateConfig / restore via
   teamConfigToDialog). If it should live in `chan-team.toml`, add a
   `tab_group` to `TeamConfigWire` (coordinate with me; I own
   teamOrchestrator's translateConfig but will take a clean `tabGroup`
   field). If it is dialog-only (not persisted), say so and skip the wire.
4. The `-N` CONFLICT SUFFIX is NOT the dialog's job — it is resolved at
   Bootstrap by the orchestrator (me), against the LIVE groups, so the
   dialog just shows the desired base name.

## @@LaneD owns (the orchestrator glue, mine)

1. Read `config.tabGroup` in `runTeamBootstrap` (teamOrchestrator.svelte.ts).
2. **`-N` conflict resolution** at Bootstrap against the live groups
   (`allTerminalTabs()` + `terminalTabGroup`, the SPA mirror of the registry
   `cs terminal list` reads): if `tabGroup` already names a live group,
   append `-2`, `-3`, ... until unique. Resolve ONCE, use for the whole team.
3. Thread the resolved group as `tab_group` into every terminal creation
   (lead restart + worker spawns), which today pass only `{name, command,
   env}`. This is part of TEAM-CONSOLIDATE (one create-with-group path).

## Boundary

@@LaneB: `teamDialog.svelte.ts` (the field + default helper) + `TeamDialog.svelte`
(the input). @@LaneD: `teamOrchestrator.svelte.ts` (read + resolve + thread).
No file overlap. The only shared touch is `TeamConfigWire`
(`api/client.ts`) IF we persist the group — coordinate that one field through
@@Architect.
