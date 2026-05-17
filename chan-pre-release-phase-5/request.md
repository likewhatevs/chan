# Request
Start by learning about our 


This will be a multi-step move, starting from big clean up and than drilling into smaller bugs and feature requests.

I will decide when we close the session and tell the @@Architect when to dispatch the tasks to release everyone. This is because even after we finish with my original request, usualy during my manual test I end up finding more issues and reporting before we close off the session.

This means: keep up with your reports, house keeping, and so on. You have to be ready to do work, to switch profiles depending on capacity, and so on.

About the team tweaks for this session:
* Web test A and B, because I'm anticipating capacity for testing like previous session
* Introducing @@Systacean, a mix of @@Syseng and @@Rustacean for the @@Architect to address directly

### Clean up
- [ ] Let's make a plan to merge my work from ../chan-term onto main here, before anything else
- [ ] Let's remove the Agent and Agent history overlay from the frontend, and all backend - docs must be on point afterwards
- [ ] Let's remove everything from chan-llm and global configs (e.g. settings, settings page) that is backing the Agent and history backends and the agent Overlay
  - iirc there's nothing left? or maybe the MCP server is there? We definitely want to preserve the MCP server and agent access to the DRIVE
### Enhancements
- [ ] The embedded terminal should set ENV variables for common agents (claude, codex, gemini) to use chan's MCP server
- [ ] Fine tune the boot and in-flight resource utilisation
  - [ ] Prioritise building and rebuilding the graph and chan-reports over the search index
  - [ ] Contain the search index with a configurable know for how aggressive to be
  - [ ] We may need to detect sudden underlying filesystem changes (e.g. git checkouts)
    - [ ] If we know the DRIVE is a git or hg repo, we should know index the graph and search in a way that supports this properly
    - [ ] We need end-to-end tests here and benchmarks; we also extremely need correctness tests
    - [ ] Handling indexing interruptions and resuming needs to be hardened to support these sudden fs changes
- [ ] Reloading windows with terminal tabs will likely kill the tabs; we need to integrate with tmux’s -CC protocol natively; this means the backend server will require tmux to exist (or if there is a decent rust implementation of tmux CC (we want to be compatible, too) we could use it instead of forking; other than that, the idea is that every tab we create is backed by a tmux -CC tab as well
### Bug fixes
- [ ] Closing tabs with files unsaved or terminal sessions must require confirmation - reload should be fine
- [ ] In chan-desktop, we have to distinct each window to not change the panes and tab layout of the other… reloading those windows should bring back the exact same state…
- [ ] When the page is exactly the size of the screen, and the user is trying to type on the top of the page, the editor keeps scrolling because it wants to add that lower marging that we actually need.. we need a solution for this, e.g. if the cursor is at the top, don't scroll - or, only scroll when the cursor is at the bottom, even better? you tell me
## Closing of this round
We need end-to-end hardening sessions for all these workflows described here and confirmation they work as spec'd.
If anything comes up and needs UX and capacity decisions, ping me - Alex.
This is where we may continue or wrap depending on the outcome.
