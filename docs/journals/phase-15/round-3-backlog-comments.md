# Comments on Phase-15 round-3 backlog
Following are my comments regarding [[docs/journals/phase-15/round-3-backlog-comments.md]] .
## Survey bubbles
First of all, we need to update the Team Work setup and encorce that the team configuration file lives inside the workspace. We will create a directory structure for the team and this structure must be inside the workspace. The team directory will contain:
- ./path/to/{team-name}
- {team-name}/config.toml
  - Contains all fields from the setup. Users can edit.
  - We must validate the fields on reload, e.g. no more than 9 members
- {team-name}/bootstrap.md (team-wide information about how to work)
- {team-name}/tasks/task-{from}-{to}-{n}.md (owned by "to", N atomic increment, append only)
- {team-name}/journals/journal-{member}.md (owned by each member, append only)
- {team-name}/followups/followup-{from}-{to}-{n}.md (owned by "to", N atomic increment)

The bootstrap.md file will contain the process for all members, and will describe everyone in the team. It will reveal the @@Host and the @@Lead, and explain to all workers how they will hold on and wait for the @@Lead to distribute the tasks and poke them using the standard 1-liner "poke from @@Lead: check your task {path-to-task}.md"; after completing their task, they cut a task back to the @@Lead and poke back in the same format, e.g. "poke from @@LaneA: check your task {path-to-task}.md".

These are done through `cs terminal write` and next up we are going to introduce `cs terminal survey` to manage the bubbles.

We will rebuild the survey bubbles, and we need the following:
- We no longer need that "Rich prompt" bottom widget that we had previously, we can delete all of that code and those menus if they are still around
- About the bubbles:
  - We will implement the `cs terminal survey` command to bring up the bubbles over a specific tab or a group fo tabs
  - We will instruct the agents to use this command when they need to contact the @@Host, however they should remain focusing on requesting permission and tasks and most communication through the @@Lead, and the Lead will aggregate the requests for the @@Host.
  - We will support the following survey modes:
    - Single question, where the user is prompted with a problem description formatted as markdown, and has the vertically aligned options to pick from (no more than 4):
      - [1] Description of the option
      - [N] ...
      - [F] Follow up
    - When the user picks a survey option, the tool returns that option
    - When the user picks F, we create a follow up file in the {team-name}/followups/followup... file (described above) and returns that to the agent: new follow up file created: {path/to/file}.md
    - We pre-populate the document a header and title, date and time, a header "**Agents**: this is a follow up, not ready; check again later",  the original prompt, and then the places where the @@Host is expected to put their comments

## IDX Option B: embeddings as a proper background job
Let's do.
## IDX bg-embed chip clobber
Let's do.
## IDX embed in-flush chip freeze
Let's do. Related to freeze/movefast: we had previously commented out the use of Metal on macOS because it was hanging the indexing. Let's create a follow up item to investigate the hang and re-enable.
## Desktop verifies
- bug-editor: remind me how to test
- reload: ctrl+r works on terminal, cheers!
- desktop-open: it works!!

About the desktop-open: I would like to *remove* the `chan open` subcommand and move it over to the `chan-desktop` if that is possible, the one in `/Applications/Chan.app/Contents/MacOS/chan-desktop` which is soon getting its own `shell` and the `argv[0]="cs"` code path so that users of chan-desktop don't need to have `chan` installed.

TODO: We need to make sure we can do the same on Linux, where we run with the AppImage on chan-desktop: how could they get the `cs` symlink to work? If not, they must then have the `chan` binary and that's an odd dependency since we ship both tools.

## DESKTOP-SHELL: cs-shell extraction to a shared crate
Yes, let's do. Craft this properly.

## Per-agent submit-encoding map
We need proper smoke tests for the team work plumbing here.

## Follow up / new items / fixes
- The markdown links ON DISK must always be relative markdown links, not true wiki-links.
  - When we are editing a file that already has wiki-links, we will keep it
  - Whenever we create a new file, we will produce markdown relative links
    - This applies for images, documents, anything that we started from the `[[` command
    - When we hit `[[` in a given file that hasn't been indexed, we should prioritise this file's direcotry
    - As I type this sentence and enter the wikilinks `[[` it gets stuck showing this bubble until I press enter ![](./image-2.png#w=250)  or paste an image
- As part of this clean up, we will set up a separate agent to REALLY clean up our old data in the docs/journals, all of the raw data will be deleted - it's in the git history anyway
  - The phases still missing summarisation and transcription of images / deletion, will also be part of this new plan; our very phase 15 will be summarised and cleaned up with images transcribed and the docs containing the essence of the work
  - When we use chan's own source code as a workspace and plot the graph, we no longer want to see ghost nodes and an excessive amount of documentation that is not really useful.. it was ok to produce this up to this point but since the essence is captured in the summaries we can roll with just that.. I want to include hashtags in these documents resembling their outcomes: #reliability, #features, #bugfixes, etc
- What happened to link to markdown sections???
  - We used to support the links like Obsidian: ![](./image-3.png#w=250)
    - We need the 'Use # to link to heading, ^ to link blocks' 
    - Our search used to understand these as well
- The search overlay does not understand paths, mentions
  - We must be able to index and search @@mention, path/to/file, .md and other extensions
    - Maybe we already are with semantic search, and if that's the case, no new work here
- Clicking on a line with mouse, to start editing, is very difficult - we often try to click at the end of a line (over or after the text, in the blank space) with the intention of placing the cursor at the end of the last word in that line, especially on lines that end on a given row. Being able to click anywhere on the row to place the cursor over it (from beginning to end of text) and also at the end of text on empty space to place the cursor at the last character


