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

## 2026-05-23 — slice 1 (FB team-dir badge + Load Team menu + Duplicate flow)

SPA-only. Backend gap on `GET /api/teams/{name}/config`
blocks the full dialog-from-config path; slice 1
ships everything that doesn't depend on it.

### Shape applied

**Team-dir detection**

* `TEAM_DIR_RE = /^Drafts\/team-([^/]+)$/` matches
  the workspace shape `systacean-30` writes.
* `teamNameFromPath(path) → string | null` extracts
  the `{name}` group; `isTeamDir(path)` returns
  the boolean.
* False-positive defense: any path matching the
  shape but missing a `config.toml` falls through
  to the chan-server `team_events_dir` not-found
  error which the caller surfaces.

**Team badge in the tree**

* FileTree's dir-icon block renders the lucide
  `Users` icon for team dirs (overrides the
  default `Folder` / `FolderOpen` swap).

**Load Team menu entry**

* Gated on `menu.isDir && isTeamDir(menu.path)`,
  so only team dirs surface the entry.
* Wired to `loadTeamFromMenu(path)` which:
  1. Walks `api.teamListLoaded()`.
  2. If the team is in the loaded set: notify +
     `uiPrompt("Team '{name}' is already running.
     Duplicate into new name:", "{name}-copy")` →
     `api.teamDuplicate(name, trimmed)` on submit.
  3. Otherwise: `api.teamLoad(name)` spins up
     the watcher + notify ("Loaded team '{name}';
     Slice 2 will wire the dialog-from-config flow").

### Backend gap (scope-poked separately)

The full Load Team flow per addendum-b §"Loading
team" calls for the dialog to open populated with
the persisted config (members, real estate, etc.).
That requires a `GET /api/teams/{name}/config`
endpoint that reads
`Drafts/team-{name}/config.toml` and returns the
`TeamConfig` shape. `chan-drive` exposes
`teams::load(drafts_dir, team_name) → TeamConfig`
but chan-server only surfaces load/unload/loaded
+ create/duplicate (per `-31` + `-41`).
**Scope-poke filed on the architect event channel
as the slice-2 unblocker.**

### Files touched

* `web/src/components/FileTree.svelte`
  * Imports: `Play`, `Users` lucide icons;
    `uiPrompt` from store; `api` client.
  * `TEAM_DIR_RE`, `teamNameFromPath`,
    `isTeamDir` helpers.
  * `loadTeamFromMenu(path)` handler.
  * Dir-icon branch for team-* dirs.
  * Ctx menu Load Team entry.
* `web/src/components/teamLoadFlow.test.ts`
  (new) — 11 architectural pins for the
  detection / badge / menu entry / handler
  shape.

### Decisions

* **Path-shape detection** (not config.toml
  read) — keeps the helper cheap + reactive
  to `tree.entries`. The server's not-found
  error catches stray `team-*` dirs without
  configs.
* **`uiPrompt` for duplicate name** — matches
  the existing rename / new-file pattern;
  doesn't require a new modal component.
* **Slice 1 ships the not-loaded path as a
  bare teamLoad** rather than blocking on
  the backend. Spinning up the watcher is
  the only thing the user can do today
  short of the dialog; teaching them
  through notify keeps the surface
  truthful.

### Gate

* `svelte-check` → 0/0.
* `vitest` → +11 new pins; intermittent flake
  on 1-2 pre-existing terminal-renderer tests
  (jsdom WebGL stub instability, unrelated to
  this slice — reproduces on isolated runs of
  the same tests too, depending on test
  ordering).
* `npm run build` → clean.
* `cargo fmt --check` + `clippy --all-targets
  -- -D warnings` → clean (no Rust delta).

### Suggested commit subject

```
File Browser: team-dir badge + Load Team menu + Duplicate flow (fullstack-a-80 slice 1)
```

### Files (per-path)

* `web/src/components/FileTree.svelte`
* `web/src/components/teamLoadFlow.test.ts` (new)
* `docs/journals/phase-8/fullstack-a/fullstack-a-80.md`

Autonomous-commit mode. No clearance held.
Slice 2 blocked on chan-server config-GET
scope-poke. Picking up `-a-79` slice 2 next
(template placement + lead pre-flight survey
+ split-pane real estate) per the original
addendum-b sequence.
