# Phase 13 round 2
Author: @@Alex
2026-05-28 

More work for the existing lanes or new lanes if needed.

## Clean ups
### Desktop native
Add global chord for New Window Cmd+Shift+N: we currently have a cmd+shift+n which opens a new window of the Workspaces. Instead, what we want is to open a new window of the currently open workspace.

### Rich Prompt
We are going to clean up and revamp the way the Rich Prompt is presented to users, and the way it is implemented.
We are going to rename Rich Prompt -> Team Work, in the UI and in the code.

Clean ups:
- We no longer need the "Spawn agent" dialog and process
- We are deleting the entire code and API related to filesystem watcher for tasks and events between agents
- We are also deleting the templates for Rich Prompt and new team; no archival of prompts either››
- No "Spawn agents" in the menu either, see below for the new process details


New process:
1. When users click Team Work from the Hybrid hamburger menu or hit the Cmd+P chord, we will:
  2. Instantiate a new Team Work *Lead* Terminal, where we place the Markdown Editor at the bottom (what we call Rich Prompt today)
    3. This Editor is instantiated like any other new Editor with a Draft - same as Cmd+N but embedded at the bottom of the Team Work Lead Terminal
  4. Over this terminal, the dialog for "Spawn agents" come up
    5. See the new menu for the Team Work dialog below
  6. If the user dismiss the Team Work dialog, we delete the Team Work Lead Terminal tab, and go back to the previous state
  7. If the user confirms the Team Work dialog (clicks Bootstrap), then we follow the Team Work bootstrap process described below.

Changes to the Team Work dialog.
Current dialog:
![](./image-2.png#w=250)
New dialog:
- Your name: change default from Alex to Neo
  - Renders as `@@name` when joining the team
- [x] Auto-prefix names with `@@` 
- Team configuration (new, replacing Team name):
  - [new] [load] (similar to the toggle we have for "Tabs in current hybrid" <> "Split panes" below)
- If the user picks New:
  - Path to configuration:
  - /tmp/new-team-1/chan-team.toml
  - (info: team management files will be created in `/tmp/new-team-1`)
- If the user picks Load:
  - Path to configuration:
    - User enters path, we auto-validate or reject the path
    - Once we auto-validate, we pre-populate the menu like it was in the original New setup
    - The config toml file should reflect the New dialog and load the same state, accepting an existing dir and config file
    - Before confirming, users should still be able to edit - since they are in a pre-populated "New" dialog
      - In this case we'd update/save the config with the new changes
- New setup:
  - Number of agents: N (remove the slider, let's add dropdown from 1-9)
  - [ Members ] same as today, add rows based on the number of agents
    - Same as today, one of them must be the lead, the this one will land on the terminal with the embedded Editor, the so called Team Work Lead Terminal
    - The other agents will land on new tabs as per the setup
  - Today's (unassigned) button is not very intuitive; should we use (drag-me) ? Because the slots have "drop bot(s) here"
  - Real estate remains as-is, I love it



Again, if during the New setup users dismiss / cancel, we delete the Terminal tab that we just created from Cmd+P. If they click Bootstrap, we update/save the config toml file, instantiate all terminals with their correct names, launch all the agents (including the Lead, which today is incorrectly only instantiated after this verification, it should be before, like the others; in fact, this should be the first! set their names, restart) place the following in the Editor embedded in the lead's terminal:

```
# Team work
We are are team of {N}. Our host is {Host} and the team lead is {Lead}.
You are $CHAN_TAB_NAME. Identify yourself and get ready to work with the rest of the team:
- {Worker1}
- {Worker2}
```

Once the setup is up, these terminals for the group are marked for broadcast. Before doing this, we will cause disruption to the exising user's workflow:
- First of all, the UI will execute the equivalent of "Deselect all" from the Terminal broadcast menu, to ensure no terminal is in broadcast mode
- Then, it will enable the Lead and the Workers (the new team's) terminals for broadcast, enabling only these

The behaviour of the embedded Editor and cmd+Enter is the same. However in this version every time we submit the draft to the lead terminal (cmd+enter), we reset the draft back to empty.

Once this bootstrap is done, the lifecycle of each terminal is just like any other. User may kill, restart, do what they want from here on. No special treatement for these terminals, no updates to the team setup config from this point on.
Rich Prompt's right-click menu, currently:
![](./image-3.png#w=250)

What we want:
- page width, show source code, show style toolbar remains
- spawn agent, spawn agents, copy metadata dir, copy spawn agents config all go, we remove them
- here we add a separator and move 'Bubble stack' and 'Bubble tray' then another separator
- collapse prompt goes here, last option

About the bubbles and stack/tray code:
- Since we are removing filesystem watcher for Rich Prompt and the trigger of those notifications, we no longer need the chan-server APIs for this, at all. Neither the filesystem part or the frontend parts
- However, we will later add equivalent notification bubble functionality, so, for now, I would like to leave only the frontend parts and stub/example code with real visual functionality when we click the Bubble stack and Bubble tray buttons: they should just show an example bubble with the survey modes we support: single or multi-question, the F for follow up, etc; but clicking anything just dismiss the bubble and does nothing else

### Editor
- We are going to change the way our lists are rendered, to match the following style (adapted to our fonts and themes, but using the same glyphs and spacing from the screenshot as reference):
![](./image-1.png#w=250)
- The common chords for editor bold Cmd+B and italic Cmd+I are missing. Let's add them.

### Hybrid hamburger
The current menu: ![](./image-13.png#w=250)

We want to change the shortcuts for split right and split bottom to cmd+/ and cmd+?


