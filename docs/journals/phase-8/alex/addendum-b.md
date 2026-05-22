# Addendum B
The Rich Prompt and the Team feature.
The feature we refer to by "the watcher" today is gaining a new make up, and we will call it Team. The workflow I envision for working with Team is this:
1. You bring up the Rich Prompt, which either enables it in the current Terminal or creates a new Terminal with Rich Prompt enabled
2. The button which today we click to create the watcher will now open a new dialog, which I'm defining below
## The Team Feature
We start from 2 premises:
1. We can set up and bootstrap a new team
2. We can load a previously bootstrapped team
### New Team
* All new teams are initialised with a directory in chan-drive's Drafts/team-{name}/config.json
    * This way we can detect duplicated teams easily
    * Users can copy/move their Draft teams over to their drive later if they want to
  * What they need to input to create a team:
    * Your name: who are you, how we introduce you to the team
    * Team Name: will be used in the config; during new team we also use this name to create the draft subdir for the team
    * Size: starts at 2, the user + 1 agent; maximum 16 for now
    * [ ] checkbox for: Automatically prefix team member names with '@@' for markdown compatibility
       -  WHen selected, shows '@@' before each team member
    * For each team member in a row:
      * [robot-icon] [ name ]  [ command [--flags] ]  [ env k=v ] (separated by space)
      * One of them must be marked as leader, which is equivalent to our @@Architect of today
    * Real estate: how to create the terminals during team bootstrap
      * a) As tabs in the current Hybrid (just add them to the current pane)
      * b) Split pane: user picks a shape that works with the number of assistants, e.g. if they picked 4, they can have 1x4, 2x2
        * These will split out of the current terminal window and form their own real estate on the screen
    * Users should be able to drag&drop more than 1 robot on the same cell to make them tabs of the same Hybrid/pane
When they click Bootstrap, what do we do:
- Place a config in the Drafts/team-{name}/config.json (or toml, maybe be even better)
- Prepare the screen real estate, execute the splits
  - Then we spawn the terminals, name them properly (restart to pick up), then run command for the agent with the ENV for each
- Once all terminals are up and running the agent, we place a watcher inside the team's dir, under events/
- We then place our process inside the team's dir as well, in docs/
  - During this, we place the Host name as {user-name} in the process template
  - For now we use the Chan's process, here we focus only on the watcher and events process for how to work on tasks and report.
    - This is about our journal + task method, event pokes and so on
    - Not so much about Chan's current team (architect, webtests, fullstacks, etc) and more about the process itself (having a host, communicating through a coordinator)
- We then prompt all agents with:
  - I'm {user-name}. You're \$CHAN_TAB_NAME. Identify yourself, and then read docs/agents/bootstrap.md
- The coordinator should then do a pre-flight check and send a survey to the user, if they can confirm the agents are up and running, and explain what would be the next step: how to delegate an intiial task.

## Loading team
From the file browser we should be able to identify directories that contain a team config and offer to load them up. If they are already up, we can offer to duplicate them into a new directory with a new team name.
During the load process, we should load the existing team setup in a dialog similar to the one when you create a team, so that users could rename, add/remove agents, and load/morph that existing team into something new.
Once the team is loaded, the coordinator also runs a pre-flight check to kick off the session.

## Clarifications (2026-05-22)

Resolution of @@Architect's review gaps:

1. **Coordinator = lead = architect — same entity.** One terminal in the team is the lead; the lead's terminal IS the user's rich-prompt terminal (the one where Cmd+P was pressed to open the New Team dialog).

2. **Per-team isolated watcher.** Each loaded team gets its OWN watcher rooted at `Drafts/team-{name}/events/`. Architecturally lands as multiple `WatchRoot` entries per `systacean-25`'s primitive. Lifecycle: watcher comes up at team load, tears down at team close.

3. **Team size = agent count, user NOT included.** Spec fix: "starts at 2" means the lead + 1 worker (minimum); max 16 agents. The USER (Host) is separate — they sit in the rich-prompt terminal that hosts the lead. The terminal where Cmd+P was pressed IS the lead's terminal after bootstrap.

4. **Bootstrap message escaping.** The `\$CHAN_TAB_NAME` in the spec was doc-escape for markdown. The actual prompt sent to agents prints UNESCAPED — agents read `$CHAN_TAB_NAME` as a literal env-var reference. Chan sets this env var as part of the terminal's name.

5. **Spawn-with-name preferred.** When possible, spawn the terminal WITH the correct `CHAN_TAB_NAME` env from the get-go (this works today via the 1-agent spawn button). Fallback restart-to-pick-up only if the spawn path can't set env at process-creation.

6. **Process template generalisation.** Current Chan process docs name @@Alex (host) + @@Architect (lead) + @@FullStackA / @@Systacean / @@CI etc. as workers. **Process is actually between Host↔Lead and Lead↔team-of-generic-workers** — chan-specific worker handles shouldn't be in the template. Template parameterises:
   * `{host-handle}` (e.g. `@@Alex`, `@@Bruno`)
   * `{lead-handle}` (e.g. `@@Architect`, or user-chosen)
   * Worker handles (`@@Worker1` ... `@@WorkerN` OR user-chosen names with `@@` auto-prefix)
   The lead automates the pokes that @@Alex does manually today (sweeping channels, routing, etc.).

7. **Promotion via FB copy/move.** Should be enough — all paths inside `Drafts/team-{name}/` are RELATIVE so the team workspace stays portable. Copy/move moves the team's `events/` + `docs/` + `config.toml` to the new location and it just works.

8. **ENV conflict resolved by auto-populate.** `CHAN_TAB_NAME=<name-field-input>` is auto-populated by chan-desktop from the per-member name input. User can't conflict because they don't manually type `CHAN_TAB_NAME` — it derives from the name field.

9. **Real estate picker = visual airplane-style grid.** Drag&drop robot icon → slot. Dropping on the same slot = new tab in that pane (multi-robot per pane is allowed). Picker shows the available shapes for the chosen team size (e.g. team of 4 → 1x4 / 2x2 grids).

10. **Load-team duplicate = verbatim copy with team name change only.** All paths inside the team workspace are relative, so a verbatim copy + team-name rename in `config.toml` (and the dir name) is sufficient. No path-rewriting machinery needed.

### Architect-side smaller nits resolved

* **config.toml** (not .json) for consistency with phase-8 .toml usage.
* **Single-user disclaimer**: chan is single-user per CLAUDE.md; Host = the one user driving the session. Multi-user (multiple humans sharing a team) is out of scope.
* **Naming**: "team member" = "agent" = an entity with a `@@<handle>` and a terminal. In the config the schema field is `members: [...]` with each entry carrying `handle`, `command`, `env`, `is_lead`.
* **Pre-flight survey format**: uses the existing survey/survey-reply event shape (one-question multi-choice "are all agents up and running?" with a follow-up step description).
