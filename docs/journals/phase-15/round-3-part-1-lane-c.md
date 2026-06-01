# round-3 @@LaneC Wave-1 plan + contract (team in workspace + delete rich-prompt)

Author: @@LaneC. Scope: Wave 1 only (no @@LaneD dependency). Two work
units split backend (Rust) vs frontend (TS/Svelte) against the shared
API contract below.

## Goal

Move the Team Work config OUT of the outside-sandbox `/tmp` path and
INTO the workspace under a user-chosen `{team-dir}/` directory, written
through `Workspace::{read_text,write_text}` (sandbox + atomic). Generate
`bootstrap.md`. Delete the dead bubble-stub / rich-prompt code (the real
survey overlay is rebuilt in Wave 2).

## Target on-disk structure (inside the workspace root)

```
{team-dir}/                      e.g. new-team-1  or  teams/alpha
  config.toml                    the TeamConfig (users may hand-edit)
  bootstrap.md                   generated team-wide process doc
  tasks/                         task-{from}-{to}-{n}.md  (owned by `to`, append-only)
  journals/                      journal-{member}.md      (owned by each member, append-only)
  followups/                     followup-{from}-{to}-{n}.md (owned by `to`)
```

`{team-dir}` is workspace-RELATIVE. `team_name` = its last path segment.
`config.toml` is `FileClass::Text` (editable, not indexed). The `.md`
files are indexed + graphed (intended: team docs become workspace
content).

## API contract (the seam between the two work units)

Keep the existing route paths so `lib.rs` route table is untouched:
`POST /api/team-config/read` and `POST /api/team-config/write`. Change
the payloads from an absolute `path` to a workspace-relative `dir`.

Wire type (TS `TeamConfigWire` <-> Rust `chan_workspace::TeamConfig`).
Add `tab_group` to the Rust struct so it round-trips (TS already carries
it; today the Rust struct drops it silently on read). Fields:

```
team_name: string        // = basename of {team-dir}
host_name: string
host_handle: string
tab_group: string        // NEW on the Rust side; terminal tab group
auto_prefix_at: bool
created_at: string       // ISO-8601 UTC, set by the SPA on save
members: TeamMember[]     // { handle, command, env:Record, is_lead, position? }
```

### write  `POST /api/team-config/write`  body `{ dir, config }`
1. Validate `dir`: non-empty, NOT absolute (reject leading `/`), the
   Workspace sandbox refuses `..` traversal already.
2. Validate `config` structurally (see Validation below); 400 on fail.
3. `create_dir({dir})`, `{dir}/tasks`, `{dir}/journals`, `{dir}/followups`.
4. `write_text("{dir}/config.toml", toml_pretty(config))`.
5. `write_text("{dir}/bootstrap.md", generate_bootstrap_md(&config))`
   (regenerated from config on every write; tool-owned artifact).
6. 200 `{}` on success.

### read  `POST /api/team-config/read`  body `{ dir }`
1. `read_text("{dir}/config.toml")` -> parse TOML -> validate.
2. 200 `TeamConfig` on success; 400 on missing / invalid TOML /
   failed structural validation (the Load flow surfaces the message).

### Validation (backend, on BOTH read+write)  -- "<=9 cap + structural"
- `1 <= members.len() <= 9`
- exactly one member with `is_lead == true`
- non-empty `team_name`, `host_name`, `host_handle`
- every member has a non-empty `handle`
Return the first failure as a plain string (becomes the 400 body).

## bootstrap.md generated content (Rust generator inside team_config.rs)

Markdown, no em dashes, ASCII only. Substitute from `TeamConfig`:

- `# {team_name} - team bootstrap`
- created line: `Generated for the {team_name} team. created_at: {created_at}.`
- `## Who we are`: reveal @@Host (`{host_handle}` / human `{host_name}`)
  and @@Lead (the `is_lead` member's handle). One line each.
- `## Roster`: an ASCII table `handle | command | role` (role = lead /
  worker), one row per member.
- `## How we work`: the process for all members:
  - Workers HOLD and wait for @@Lead to distribute tasks.
  - @@Lead cuts a task file `{team-dir}/tasks/task-{from}-{to}-{n}.md`
    (owned by `to`, N atomic increment, append-only) and pokes the
    target with the standard 1-liner.
  - On completion a worker cuts a task back to @@Lead and pokes back in
    the same format.
  - Most worker<->host communication routes THROUGH @@Lead; @@Lead
    aggregates requests for @@Host.
- `## The poke 1-liner`: the lean-bus format + the Meta+Enter submit
  chord, fenced:
  ```
  cs terminal write --tab-name=<target> $'poke from <me>: <1-line>; read <path>\x1b[27;9;13~'
  ```
  Note: the trailing `\x1b[27;9;13~` is the submit chord; a bare newline
  parks the poke unsubmitted. (Per-agent submit encodings land in Wave 2.)
- `## Files`: document the four dirs + naming + ownership + append-only.
  Task / followup filenames use the member's BARE name (handle without
  the `@@`) to keep paths clean, e.g. `tasks/task-Lead-LaneA-1.md`.

## Frontend rework (TS/Svelte)

- `teamConfigPath.ts`: `TEAM_CONFIG_DEFAULT_PATH` -> `TEAM_DIR_DEFAULT =
  "new-team-1"`. Replace `teamConfigDir` with a `teamNameFromDir(dir)`
  basename helper (or inline). No absolute-path assumptions left.
- `teamDialog.svelte.ts`: rename `configPath` -> `teamDir` on
  `TeamDialogConfig`. `validateTeamConfig`: teamDir non-empty and NOT
  absolute (reject leading `/`); keep the `<=9` cap. `defaultTeamConfig`
  uses `TEAM_DIR_DEFAULT`. `defaultTabGroupFromPath` -> derive from the
  team-dir basename.
- `teamOrchestrator.svelte.ts`: `translateConfig` sets `team_name` =
  basename(teamDir), `tab_group` from `config.tabGroup`. `runTeamBootstrap`
  calls `api.writeTeamConfig(config.teamDir, wire)`. `identityPrompt`
  gains a line telling agents to read `{teamDir}/bootstrap.md`.
  `wireToDialog(wire, teamDir)`. `teamNameFromPath` -> `teamNameFromDir`.
- `client.ts`: `readTeamConfigFile(path)` -> `readTeamConfig(dir)` body
  `{ dir }`; `writeTeamConfigFile(path, config)` -> `writeTeamConfig(dir,
  config)` body `{ dir, config }`. `TeamConfigWire` already has
  `tab_group`.
- `TeamDialog.svelte`: field label "Path to configuration" -> "Team
  directory (in workspace)", placeholder `new-team-1`. Hint (New): "Team
  files will be created in <workspace>/{teamDir}/". Load: enter team dir,
  read config.toml. Drop the "must be absolute" copy.

## Delete the rich-prompt / bubble-stub (Wave-1 deletion; rebuilt Wave 2)

- Delete `web/src/state/bubbleStub.svelte.ts`.
- Gut `BubbleOverlay.svelte` to a minimal shell that renders NOTHING
  (drop the static EXAMPLES payload, the bubbleStub import, the
  stack/tray demo). Keep the `<BubbleOverlay />` mount in
  TerminalTab.svelte UNTOUCHED (not our file) so we avoid a cross-lane
  edit; the gutted component just renders nothing until Wave 2 rebuilds
  the real reply-capable overlay.
- `TeamWork.svelte`: remove `setBubbleMode`, the `showBubbleStub` import,
  the two "Bubble stack" / "Bubble tray" menu entries + their separator,
  and any now-unused imports (e.g. `Layers`). KEEP the Collapse/Expand
  affordance (live composer feature; survey bubbles in Wave 2 still want
  the room). Leave `api.setBubbleOverlayMode` + the `bubble_overlay_mode`
  preference in place (Wave-2 layout persistence; not stub code, and
  keeps us out of preferences.rs / types.ts).
- Tests: rewrite `BubbleOverlay.test.ts` to mount the gutted component
  and assert it renders nothing (drop bubbleStub imports). Update
  `teamWorkFollowUp.test.ts`: keep the "absent" assertions
  (surveyAsQuoteMarkdown / quoteSurveyToPrompt / onQuoteToPrompt /
  quoteIntoTeamWork gone, `<BubbleOverlay />` mounted) and drop the
  now-false "follow-button / <kbd>F</kbd> present" assertions. Remove the
  setBubbleMode/showBubbleStub case in `TeamWork.test.ts`. Adjust any
  team-config test that referenced an absolute path / the old API names.

## Gate

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`, `cargo build --no-default-features`; web: svelte-check +
`npm run build` + vitest. Then build `-p chan`, serve a throwaway drive,
browser-smoke the dialog, confirm the `{team-dir}/` tree (config.toml +
bootstrap.md + the three dirs) lands INSIDE the workspace.
