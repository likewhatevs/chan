# Chan pre-release roadmap
This is my listem of items that needs addressing before we can seal the engineering part of the release of Chan.

## How we are going to work here
We have the follow agents working on this project:
* architect: will be coordinating all the work through files in the ./chan-pre-release-phase-1 directory
  * The architect will create a journal.md file there to take notes of the execution progress, add follow ups
  * The architect will record completion, highlights (e.g. found perf win, found bugs and fixed), and low lights (we introduced bugs, caused slowdowns, too much idle time of the agents, agents not picking up on tasks on their own, the architect not keeping up with project management and task distribution, etc)
  * The architect will ensure that all new code has proper testing, end-to-end testing when applicable, and at least 1 hardening session with an sme before commiting
  * Also the architect will distribute the tasks for agents to commit their code as tasks get completed, following our repo standards
  * Once the project is complete, architect will create a summary.md with the summary, highlights and lowlights of the execution
    * We should have a dedicated section ranking the quality and efficacy of the agents, and constructive feedback for them as well
  * Tasks will be distributed as {name}-{n}.md and the agents will pick up from there, and report in there
    * Once the agents complete their task, they execute this procedure:
      * Report back into their {name}-{n}.md task, so that the architect can pick up and update it
      * Create a task back to the architect that the agent is idling and ready for more work; agents can help in adjacent areas of expertise so long they leave a task to the actual sme to review before the architect can pick up
* webdev: our main frontend engineer: does the web development, resilience, and quality of the web stack
* webtest: will run a test web server so that i (the host) can keep eyeballing changes sporadically, and it's ok if they crash
  * webtest will own the loading and reloading of the web test service and web server, via tasks
  * other agents should look into webtest's tasks before making requests, but if we do end up with dups, webtest should consolidate and report back to the other agents via a task update
* rustacean: our main all-things-rust engineer, also manages the cargo builds and quality of tests
* syseng: our systems expert; can take all kinds of tasks related to low-level systems validation, harderning, and so on

And I am Alex, your host. You can come to me when you need help and prompt for direction or permission. Other than that, create your tasks to the architect.

Items that needs addressing:

- [ ] Fresh new to the world: this is the first time that anyone else other than me (and a few colleagues) will see Chan in the wild
  - [ ] Our code has evolved over weeks to this point, and we have things like schema migrations and probably other migration code from our own iterative versions - we don't need none of those
  - [ ] This is the very first canonical version of Chan, so whatever migration code we had from previous versions of Chan itself, are absolutely invalid and must be cleared out
  - [ ] As this happens, we need crystal clear code comments and design documentation on the current decisions, as a snapshot, not a changelog from what happened during the development of Chan

### Search and graph

- [ ] We need a graph-like index for directories and files, sub-directories. We also want to support symlinks and hardlinks in this graph, even if they are broken links - ghosts like other nodes in the graph today.
- [ ] From the File Browser, we should be able to right-click on any file or directory and "Graph this"
  - [ ] In the Graph overlay, we need a SCOPE: Folder (and when whe scope if Folder we could add on convenient Parent Folder in the dropdown selection as well)
  - [ ] When we are graphing scope=file, we should include its Folder in the dropdown as well
- [ ] When the scope if a Folder, we graph the files in that directory. Sub directories and so on. We should start from depth: 1
- [ ] We need a new overlay for the search index
  - [ ] Similar to how we have the Assistant History button next to the SCOPE on the Assistant's overlay, but in our case, for showing a dashboard-like view of the index state
  - [ ] Today we have some of this information in the Inspector of the File Browser, when you use the right click menu and click on the Folder
    - [ ] That click opens a DRIVE item in the inspector with the index information; we will remove that info from there and move over to this new overlay we are creating, the search status dashboard
    - [ ] The search status dashboard should include that button to reset the index, and we should be able to watch it being deleted and re-created
    - [ ] This dashboard should also include information from chan-report and its progress, and at least a breakdown of SLOC per language
- [ ] Use data from chan-reports
  - [ ] Because we already have per-file information about language and so on, we should be able to search those too and reach their files, e.g. language: Python
- [ ]  Search window arrow nav is not scrolling the page down, it should; also recalibrate on window resize, pane resize

### Assistant
- [ ] Assistant chat is not keeping up with the scroll, it should scroll before it adds new objects so there’s always a margin at the bottom of the chat window - and it needs to recalibrate on window resizes, pane resizes
- [ ] The chat bubble sizes (boxes below YOU and ASSISTANT) should be able to stretch to the maximum width of the chat area if there is enough text in them
- [ ] When the assistant is 'thinking' we have our own 'thinking...' with the '...' cycling but we also have the '[orange-dot] thinking' badge which is much nicer, and we should keep only this one
  - [ ] In the file editor tab this orange dot is blinking when the assistant is thinking, but in the assistant's chat window it's not.. it should, and these should be the same - what we do from the file tab should be the new norm

### Command line

- [ ] We need new sub-commands to match the web UI functionality
  - [ ] chan config: all settings we have in the settings overlay today, e.g. chan config {get|set} editor.{theme|layout|appearence}={value}
  - [ ] chan graph: do make queries to the graph, per scope
  - [ ] chan status: overall status of the drive, index, graph, chan-reports, etc
