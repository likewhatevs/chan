
# Rich Prompt revamp
Author: @@Alex
2026-05-24
---
Agents: do not edit

This is my list of refinements and enhancements for the Rich Prompt in Chan.
Today users bring up the Rich Prompt through Cmd+P / Cmd+. P and the rule used to be: if the user is already on a terminal, we bring up the Rich Prompt pane from the bottom of the terminal up to halfway its size. If the user hits the shortcut from any other Hybrid application which is not the terminal, we create a Hybrid Terminal and bring up the Rich Prompt.

From now on, we are changing this: hitting Cmd+P / Cmd+. P will always bring up a new Terminal with the Rich Prompt wired in. See below for the setup and teardown procedures.
## User interface
Today the Rich Prompt interface comes up like this (in light mode):
_[screenshot removed: current Rich Prompt composer (light mode)]_

At the top right, we've got:
* New File from here
* Spawn agent
* New team
* Send prompt
* Submit mode toggle between shell and agent
* Collapse / Expand toggle
* Close button

We reserve the left hand side for the style toolbar:
_[screenshot removed: left-side style toolbar reserved for the Rich Prompt editor]_

### New UI
What I want from the user interface is to be similar to Codex's UI:
_[screenshot removed: Codex-style composer, the target UI]_

And the + menu:
_[screenshot removed: Codex plus menu (add files, plan mode, plugins)]_

What we will have in ours:
[ 1 line of input ; the top part needs a resizing picker like we currently have ]
[ + ] [ Events: N in / out ] ... [ agent (none/claude/codex/gemini ] [ mic ] [ submit / stop ]

In the [ + ] menu:
- [ 1 ] Spawn agents
  - Brings up the Spawn agents dialog, which is today's "New team"
  - The details of the workflows for Spawn agents are described below
- [ 2 ] Copy path to metadata dir
  - Does what it says, copies the path to the Rich Prompt's current Draft
- [ 3 ] Copy Spawn agent configuration
  - Copy the current config to the clipboard
- [ 3 ] Collapse / Expand (collapses to the bottom row starting from the [ + ], hides [ mic ] and [ submit / stop ]

We are effectively dropping:
- New File from here / save prompt - no needed, we auto-save the Draft for the session
- Spawn agent and New team become one button
- No Close button - Rich Prompt will piggyback on the closing of the terminal from now on

### The Rich Prompt on/off
This section is about what happens when we bring up and tear down the Rich Prompt.
For reference, the process we used for Chan is in `./docs/agents/bootstrap.md` and we recently made it a more generalised version that may need more polishing. We need to discuss the exact scenarios we are covering and the way forward.

Bringing up the Rich Prompt:
- We create a chan-drive Draft for the Editor which we embed at the bottom of the Terminal
  - This process should not be too different from Cmd+N (new Draft), except that in the Rich Prompt we manage the lifecycle alongside the Hybrid Terminal. What this means:
  - In addition to setting up a Draft, we create a `spool` directory next to `draft.md`
  - In this directory, we place:
    - `process.md` with our process related to the Rich Prompt (this is our existing process with the lead and other workers)
    - `events/` directory where we are going to place the events called event-{from}-{to}.md where from and to are the agent or host (user) names. e.g. we had @@Architect and @@Alex as from and to. We need to standardise how we normalise their names to fit filesystem standards without collision. We need to be friendly to 'tool usage' like grep and sed and so on, we don't want to be too cryptic about this data and where it is.
    - `journals/` directory where agents will place their own append-only journals, like in our current process
    - `tasks/` directory where agents will place their tasks and related information, according to our current process
  - Once this structure is in place, we automatically place the events watcher (fsnotify) on the `events/` directory.

Tearing down the Rich Prompt:
- This is what happens when the user closes the tab, the terminal, or clicks Close [X]
  - The sequence should be:
    - If there is a terminal shell open, notify the user it will be closed, then close it
    - We must then stop the events watcher
    - Then, we move the whole Draft over to the chan-drive's Thrash
    - Then we do the UI cleanup - close windows, etc
  - We should inform the user of what's happening - we can use the main status bar for this

Interop between Terminal and Rich Prompt and Watcher:
- The user must be able to rename the Terminal, restart their shell, and this should not interfere with the events watcher
- When the Rich Prompt comes up, the focus is set to the Editor rather than the Terminal
- The Editor must be able to submit its entire buffer to the terminal, followed by an extra `\n` that we will add if the user didn't.. e.g. they typed "echo hello" in the terminal, in a single line, without pressing enter, but they hit cmd+enter; in this case we add the extra \n and submit
  - If the terminal is running a known agent: claude, codex, gemini, we must be able to send a multi-line input into the agent and only then send the submit key; (TODO: check if this actually matters, to know that an agent is running.. it may be irrelevant) we can probably implement this by using the simple "copy-paste" functionality which already works with all agents, meaning: we copy the buffer from the Editor and paste into the Terminal, followed by the newline or regular Enter key which the agents use to submit the buffer.
  - After submitting the prompt, the contants of `drafts.md` (or the file itself, whatever is cheaper) moves to `prompt-N.md` and a new blank `drafts.md` is presented to the user as the new prompt

## Spawn agent workflow
We already have the New Team widget and workflow today, and we will start by re-branding it to Spawn agents.
We will acept minimum of 1, in which case they have no option but to be the Lead role in our process.
We will now limit the maximum to 9, an arbitrary number that I want to experiment with.
We need a copy/paste for Spawn agent configuration in this dialog. This is so that I can make different setups and copy them without bootstrapping, or paste one of my previous setups and just click Bootstrap right away.
Spaning the agents should come with a pre-flight check:
- Once the Bootstrap button is clicked, we create the new terminals with the right names and commands
- If agents fail to spawn - invalid command, other errors, we show an error to the user and tear down the Rich Prompt + the terminal
- Ask the user to confirm that all agents are up and running
  - Here we should already use one of our survey events as a pre-flight check
- Once the user confirms, we place the broadcast prompt to all Spawned agents: I am {Host}, you are {Agent}. Our lead is {Lead}, read {path-to-bootstrap.md}" which should then link to our `process.md` and configure the agents to our method of operation through tasks and events.
- We want to ensure we have the ENV correctly setup in these Rich Prompt terminals with $CHAN_TAB_NAME and our MCP server wired up as well.

The way we manage the Lead today will remain. The current terminal with the Rich Prompt on is the one that runs the {Lead} role of our process.

## Related work
### The style toolbar
In the current implementation, when the user clicks the toggle to Show Style Toolbar, either in Hybrid Editor or in the Rich Prompt editor area, the style toolbar auto-hides if the user is not clicking on the screen or active with the cursor. We want to remove this functionality and once bring back clarity to the user: the toggle is between Show or Hide, and this is what we will do.
The auto-expand remains, because we want to save real-estate on the screen.
### Hybrid apps and consistency
We have recently updated the way we allow Hybrid apps to have their own individual dark/light setting, e.g. Edit, Terminal, Graph, Infographics, File Browser should all have their own dark/light setting.
We will make this more consistent by placing the dark/light switch ICON like we have in chan-desktop, into the top bar of the config, e.g. where we say "Hybrid Editor" we will have [Title] ... [ ICON ]. And we will no longer have the Appearence block in none of the Hybrid element configs. For reference, the current top bar of the Editor settings tab (flip behind the Editor tab):
_[screenshot removed: editor settings top bar]_
This change should bring consistency across all Hybrid elements with a top bar containing their title and dark/light switch.
At the bottom, they should all have an OK button to flip back to the front and continue using the component.
## Other bugs
- While editing text and landing on a missing image, it is very hard to delete it - you have to figure out how to select the markdown text.. we need a 'delete' button in the preview, at least for missing files like this: _[screenshot removed: image-not-found red placeholder, hard to delete]_
