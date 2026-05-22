# fullstack-a-80 — Load Team flow (FB identifies team dirs + load dialog + duplicate-into-new-name + pre-flight)

Owner: @@FullStackA
Cut: 2026-05-22 by @@Architect
Status: dispatched
Round: addendum-b wave-1
Dependencies: `systacean-30`, `systacean-31`, `fullstack-a-78` (dialog component), `fullstack-a-79` (orchestrator entry points)

## Goal

Surface load-team affordances in the File Browser +
build the load dialog (reuses `-a-78`'s dialog shape
populated with existing config) + handle the
"already up → duplicate into new name" branch.

## Reference

[`../alex/addendum-b.md`](../alex/addendum-b.md)
§"Loading team" + clarification #10 (verbatim copy
+ team name rename only).

## Scope

### FB team-dir identification

* Walk the FB tree under `Drafts/`.
* Any directory matching `team-*` with a
  `config.toml` inside → render with a team-badge
  affordance.
* Right-click context menu adds "Load Team" entry
  on these dirs.

### Load dialog

* Reuses `-a-78`'s TeamDialog component, populated
  with existing config (host name, team name,
  members, real estate).
* User can edit anything: rename / add / remove
  agents / change real estate.
* "Load" button → fires `-a-79`'s orchestrator
  with the edited config.

### Already-up branch

* Check chan-server's `team_list_loaded` IPC from
  `systacean-31`. If the target team is already
  loaded:
  - Show a "this team is already running" notice.
  - Offer "Duplicate into new name" button →
    prompt for new name → call
    `Drive::duplicate_team(source, new_name)` from
    `systacean-30` → open load dialog populated
    with the duplicated config.

### Pre-flight on load

After bootstrap fires (via `-a-79`), the lead runs
the same pre-flight survey as the new-team case.
No special handling here — reuses `-a-79`'s flow.

## Acceptance

1. FB shows team-badge on team dirs.
2. Right-click "Load Team" opens populated dialog.
3. User can edit + Load.
4. Already-loaded teams: notice + Duplicate option.
5. Duplicate produces verbatim copy with team name
   rename.
6. Pre-flight fires post-bootstrap (via `-a-79`).

### Tests

Vitest pins for FB team-dir detection + dialog
population + duplicate flow + already-up branch.

### Gate

`npm test` / `check` / `build` green.

## Coordination

* @@FullStackA SPA primary.
* Depends on `-30` (list/load/duplicate) + `-31`
  (team_list_loaded) + `-78` (dialog component) +
  `-79` (orchestrator).
* Sequencing: pick up after `-78` lands the dialog
  shape; consume `-79`'s entry points when ready.

## Authorization

Yes for SPA FB integration + dialog population +
tests + task tail + outbound.

## Numbering

This is `-a-80`.

## Out of scope

* New team flow (`-a-78` + `-a-79`).
* chan-drive primitives (`-30`).
* chan-server watcher (`-31`).
* Process template (`-a-81`).
