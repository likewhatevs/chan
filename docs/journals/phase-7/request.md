# Phase 7 requests
## Setup
We are running slighly different than previous sessions. Here's the new setup:
- @@Architect remains on the same role, except that from here on (pls go and update the process if needed), you will cut tasks to me in {alex}/{task}-{n}.md
  - Different from the other agents, I will not cut tasks back to you. You will poke me to look at them, and I will edit the task and append my notes, and poke you back to carry on
  - When the other agents finish their tasks, they will request me to poke you by placing a {alex}/poke-{from}-{to}.md for me; this file should link to their latest update
- @@WebtestA and @@WebtestB remain our workhorses for running test servers and the test waves
  - At the completion of a round, make sure to include the URL of ONE test server that I can access and click around - yet, do this through a task to @@Architect and the poke through me
    - They may decide to add that to a later round or if I am not replying this shouldn't be blocking anyone's progress
- @@FullStack is our new profile, a mix of @@Backend and @@Frontend
- @@Systacean remains our mix of @@Syseng and @@Rustacean: owner of the overall project and code quality, the build and CI, dependencies, and so on
## Round 1: maintenance
### Project hygieneystaceanys * We are going to create `./docs/journals` and move our `phase-*` directories from the root of the project over there
  * We are going to normalise the format and always refer to the agents via their `@@{name}`, except for the files which remain `{workdir}/{agent}/[journal|task-N]}.md`
  * We need to backfill our earlier sessions because we didn't apply this standard despite having the same/similar agent names 
  * The journals that we pre-created from the previous sessions should be updated as well
    * These are missing the Date: 2026-05-18 (I use `@today` to populate them from the editor); I added to mine and to @@Architect's and we need on all
  * Before we even start, we are going to create `./docs/agents` and create their Contact.md files
    * We need to refer to the skills that we've been using during this development: i think you can copy the skills (from my `fiorix/dotfiles` repo) into `./docs/agents/{name}/{skills.md}`  and link from their contact
    * In each of the agent's contacts we will place their `@@{name}` so that we can graph it later.. the outcome should be that we can graph `./docs/journal` and see the previous work correctly linked and tagged
    * We will use this to start curating our project's development logs over time and enhance the way we work with agents and so on
  * Process
    * Before we start on this round, let's be clear that we must tidy up the process and then start the work!
### Enhancements
* The file browser needs a button that allows it to stick to the left or right hand side of the screen in a vertical pane, top to bottom - it lives *outside* of the main pane where we host editor and terminal tabs
  * The user should be able to stick one on each side, and still bring up the file browser overlay
  * The look-and-feel should be familiar (ours already is), and these side-attached file browsers should be inspired by github's: ![](./image.png#w=250)
  * When there is already something in the Find buffer and we didn't close it, we can't press Cmd+F again - does nothing: ![](./image-4.png#w=250); if I close it with the mouse and press Cmd+F again, it opens..
  * The style toolbar for the file editor and the rich prompt must be the exact same, the one from the file editor; we need to upgrade the icon set - are the correct? i dont think so.. judging by the 'insert image' link.. nono
    * Are we missign any important items there? the <hr> bit?
    * We must ensure to always open external links in the default system browser; it would be nice if we could render decent bubbles with preview, but because we're not the system browser we may not have the right cookies etc for links that need auth.. still could show preview bubbles if possible
    * The split left, right, and settings buttons from the terminal menu should resemble the file menu with the sections and separators, same for sequence of items: ![](./image-11.png#w=250)
 vs ![](./image-12.png#w=250)
 * When we change the terminal's name, we should should show under the name an indicator if the name has changed but the terminal hasn't restarted; the restart button should ask for confirmation as well because the session gets reset
   * _Addendum (2026-05-18 21:30 BST):_ on Enter in the rename input, don't just close the edit silently — immediately offer to restart right there. The out-of-sync indicator is fine as a secondary affordance, but the rename moment is the right time to ask "restart now?" so the user doesn't have to discover the indicator + dig back into the menu.
* `chan open <file-or-directory>` from inside a chan-spawned terminal (detected via `$CHAN_TAB_NAME`): for `.md` files load a new file tab (create if missing); for directories or non-`.md` files load the file browser at that path. Shell completion would be nice. May need to export `$CHAN_DRIVE_NAME` (or a session identifier) and rely on that instead of/alongside `$CHAN_TAB_NAME` so the right window receives the open. Must respect cross-window restrictions (the chan CLI talks to *this* window's chan-server, not some other window's).
* Activity indicator on terminal tabs: visual cue (small dot / pulse / colored icon) showing when a terminal has produced output since the user last looked at it, vs when it's idling. Helps spot which long-running terminals just finished a build, which one is silently waiting, etc. Clears when the tab is focused.
* Auto-configure external agents (claude, codex, gemini) to talk to our chan MCP server. Today we export `CHAN_MCP_*` env vars; agents need the standard, un-prefixed MCP env names (or the per-agent config-file shape, whichever they actually read) to auto-discover the server without manual setup. Two hard requirements: (a) coexist with any existing MCP config the user already has - append our server rather than overwrite, (b) don't be left out - make sure our descriptor lands in the place each agent actually reads. Investigate per-agent discovery shapes (file vs env) before designing the wire. This enables the deeper UX win below (fs-aware editor) since agents will move files through our server's tools instead of raw shell.
* Pane menu reorganization: today's right-click pane menu surfaces "Reload" (and toggle web inspector). Move those two "developer / page-level" actions to the pane hamburger menu where they're less in the way. Right-click on the pane should instead show the *pane-structural* actions: Split (left/right/up/down) + Close. Keeps everyday-frequent actions on the gesture that matches them (right-click for pane structure, hamburger for less-common dev/reload commands). Pairs with B15 (left-click should not open the right-click menu).
  * _Revision (2026-05-19 01:45 BST):_ swap back. Right-click on the pane = Reload + toggle web inspector (the original placement). Hamburger = structural actions. Also drop Split left and Split up — I only asked for **Split right** and **Split down**; navigation left/right between panes is the separate `Cmd+[` / `Cmd+]` binding. So hamburger contents = Split right + Split down + Close + Next pane (`Cmd+]`) + Previous pane (`Cmd+[`) + focus-border color. Right-click contents = Reload + Toggle web inspector.
* Pane focus highlight color: the current blue focus border around the active pane is too subtle when several terminals are visible — hard to tell at a glance which one your input goes to. Add a per-pane color option in the same right-click menu (alongside Split + Close from the bullet above): switch the focus border between green and pink. Persists per pane through preferences. Default stays blue; this is opt-in punch.
* Next / previous pane navigation: add "Next pane" + "Previous pane" entries to the pane right-click menu, bound to cmd+] / cmd+[ on chan.app native. Browsers reserve cmd+[ and cmd+] for back/forward navigation (same problem as cmd+t), so the web variant needs cmd+alt+] / cmd+alt+[. Detect at runtime and register accordingly. Folds into the pane menu reorg task.


### Bugfixes
* Pressing shift-tab in a list brings the item 1 indentation back, which is correct; however pressing shift-tab anywhere else in the doc takes the cursor to the pane's hamburger: let's not do that; we should just block shift-tab when the cursor is not in a list, and if the item belongs to a list it de-indent until it's no longer in a the list and will get blocked
* When we are in a list (bullet, number, etc) and we paste an image, we are currently pushing the cursor to the next line BOL; we should instead just add +1 space after the image, so the writing is fluid for the user; *if* the user press enter, however, and did not use the space we added, we take it back - we don't want trailing spaces out there and in fact we need a menu button in the file editor's Find function (the cmd+f) to:
  * a) Find and highlight training space
  * b) Toggle code blocks (only applies to markdown)
  * c) Remove training space (with a tick to do this automatically on save and auto-save); and this should *never* move the cursor except 
* When we are indexing for auto-completion, and the user is trying to use `[[` we should indicate that the index is running instead of silence or No Matches
* I keep getting eventual timeouts (failed to write after 10s - also, how come? indexing or something else in the way of writing my tiny .md file??? not acceptable), got while writing this very document: ![](./image-1.png#w=250)
* Also while writing the previous bullet point my cursor jumped down to Wo[here]rktree out of the blue..
* When we switch from source code to rendered, we need to place the cursor correctly; if the source code is a markdown and the cursor is on an image, for example, the rendered switch will select the image and vice-versa
* When we've got no matches we should at least indicate that we tried: searched N documents (or, we're indexing, be patient) and in the case of indexing add a spinner before giving up; it should be above the separator which looks weird when empty results return: ![](./image-2.png#w=250)
* And iirc we have the same issue in the image search dialog triggerd by `![`  - yes ![](./image-3.png#w=250)
* We should at least say 'empty search, type something' to make UX more fluid
* The rich input on terminals is missign the right-click button on the rich input area
  * Should have the following: toggle source code, toggle style toolbar, prompt width (similar to page width, but for prompt - this was hard to get right last time when we did in the previous 'Assistant OverlayShell' code, you can prob. find the details in previous commits
  * The current toolbar that we show in the prompt editor should not include that button to toggle source code, that should be only the menu option, and it should resemble the one from file editor ![](./image-6.png#w=250)
except that there's no outline or details buttons; we should tho have a button 'Link to File' and then we prompt the new file menu, and switch the prompt buffer  to 'read [[link-to-new-file]] so the prompt still works
* Editing tet at the end of the page is proven to be very difficult, the page keeps scrolling as I type: ![](./image-7.png#w=250)
* Typing on a list moves the cursor before the marker: ![](./image-8.png#w=250)![](./image-9.png#w=250)
* switching tabs between doc and terminal results in the editor tab not rendering until click or cursor move: ![](./image-10.png#w=250)
* Clicking on an empty pane with the left click opens the right click menu: this is wrong.. same for the pane tab's click: we should no longer have any logic to open the menu with the left click, only right click; left click simply selects the tab, or the pane - this is blocking me from drag & dropping tabs!
* The cmd+\` is nice for opening new terminal but conflicts with macOS shortcut to cycle windows; we will switch to using cmd+t on chan.app native. The browser-served chan needs a variant because browsers reserve cmd+t for "new browser tab" and the page can't intercept it. Proposed: cmd+alt+t as the web binding (free in chrome/safari/firefox). Bind both on native so muscle memory carries over for users who switch between native and web. Detect at runtime which platform we're on (web vs Tauri) and register accordingly.
* The cmd+shift+I for toggle should always toggle all tabs; after the toggle the user can still go and on/off individual tabs; this will always preserve the MUTE status of each tab
* The tab's [BCAST] -> broadcast icon from the menu! can't click mute maybe because 
  * _Clarification (2026-05-18 18:50 BST, repro on 6-terminal stress):_ after toggling BCAST on/off plus muting/unmuting some terminals, state drifts out of sync across tabs. Two concrete symptoms: (a) the per-tab `[BCAST]` text pill should be replaced by the broadcast (radio) icon used in the membership chip area, so the tab strip is visually consistent with the menu, and (b) ticking/unticking a terminal in the BCAST membership menu must affect only that terminal — currently other tabs flip too. Spec: select-all / deselect-all are bulk operations that preserve each tab's individual pre-existing MUTE state; the BCAST membership and per-tab MUTE are independent axes. Same wave-2 cluster as B17/B18; not promoted ahead of `fullstack-6`.
* When opening a doc with images, some thumbnails render and some don't (partial render); preference is all-or-nothing - render all consistently, or render none at all rather than the current half-rendered state
* Markdown tables don't render in the editor: the table area shows as empty space (with section chevrons still visible) and content below the table is also affected; repro doc with a pipe-style table currently in `docs/journals/phase-7/alex/setup-1.md` (Q3 skill mapping)
* External fs moves of open files surface as "i/o error file not found" in the tab - technically correct but bad UX. Detect file-moved-while-open (inotify-equivalent on the open path), and either auto-follow if the new location is unambiguous (inode survives the move on same fs) or show a clear "this file was moved or deleted" state with a re-open / find / close affordance instead of the raw i/o error. Long-term fix is to route external-agent moves through our MCP server (see the auto-config enhancement) so we never lose visibility.
* After a browser reload (observed in chan.app desktop), terminal tabs that had agents running show blank panes with disabled input and no output. Only "Restart" from the tab menu recovers, which resets the PTY session — too heavy as default. The WebSocket reconnect path needs to re-attach to the live PTY session instead of leaving it orphaned. Also: prior interaction with BCAST on/off + mute may have contributed to the stuck state; suspect they share state with the input-enable path. Related cluster: B17 (cmd+shift+I mute toggle), B18 (BCAST mute can't click), B14 (doc/terminal tab switch leaves editor blank).
* Light-mode terminal: lighter glyphs (pale text, dim ANSI colors) lack sufficient contrast against the light background. Bump foreground contrast in light mode so faint output is readable without straining. Dark mode unaffected.
* "Graph from here" on a directory returns nothing — empty graph view (0/4 nodes, 0/3 edges) with no errors and no progress indication, even though the Details panel correctly reports 246 files / 26 subdirectories / 228 documents under `docs/`. The graph either failed silently or is still indexing without surfacing state. Two fixes needed: (1) actually return the nodes the directory contains (or fix whatever scope-filter mismatch is producing the empty result), (2) surface a clear state — "indexing… N of M scanned" with a spinner, or "no matching nodes for scope filter X" if the filter chips are excluding everything. The Find UX bullets above (B3/B4/B8/B9) have the same root cause family: empty-state ambiguity. Treat consistently.




New bug report: ![](./image-13.png#w=250)


Got this after (I think) I clicked to Copy Path of a directory... I needed to reload the whole screen by usign the pane's left-click button ![](./image-14.png#w=250)
 which I thought we had already swapped with the  pane's hamburger ![](./image-15.png#w=250)
 but it's ok if we haven't yet. Let's make sure to do that, took me a while to find the reload button

## Round 2: features
> Before we start here, let's commit round 1 and create a new patch release, then push. We can ignore CI because the repo is still private and we haven't setup keys, but we should run the pre-hook checks like if it was on CI - although for now only on macOS.
> 

This is already done ^^

I will be elaborating on the new features while we execute on round 1.
### Updated protocol beween agents and I
Now the agents can send these events and I can reply back to them. From now on we will use survey style so that I can pick from 1, 2, 3  and please also include an option for subsequent approvals for the same topic/kind of work, not specific per tool execution; we want to optimise for throughput here and can grant permissions on a per-topic basis
I also want to support 4x3-style survey with 2-4 topics and each having 1-3 options; this is because the @@Architect may accumulate enough info to survey me and dispatch larger scopes

### Notification system over the rich prompt
When we open the rich prompt, we should have an option put a watcher on a specific directory using the same 'new file' dialog with the complete and so on.. for our test, this will be the directory of the events for me + ./events-watch, so that we can pick up and show chat bubbles like a chatsapp chat (similar to what we had previously in the Assistant OverlayShell) with my messages going on left hand side (and into the terminal), and the notifications floating over the top part of the terminal... id like to see them like floating over (bubbles do have background but bubbles are floating over the terminal), where i am presented with the text and links (e.g. docs to open in new tab with details to append to a journal) then / button to reply to the survey; one of the options will always be "check my comments first" for when i udpate the doc and need a follow up

Hiding the rich prompt hides this whole thing - the prompt and the bubbles

When there is a watcher on the terminal tab, we should place one of those bullets like we have for file saves, and blink them when there are replies from the assistant so the user knows when to open the rich prompt if they are looking at the regular terminal

Because we never leave state outside of disk, closing the session and terminal should be fine because agents can pick up when they restart.

About how I envision the pokes working:

For now, all we want is to trigger an assistant via their terminal when a notification file is created. So, first of all, we need to tell all assistants that they must create files atomically because partial content can cause issues for us - but for us (looking at you @@Syseng) we need to actually support partial updates and perhaps do checks a few times because we can take action on "new file on disk". How we're going to use this:
When the user brings up the rich prompt and sets a directory to watch, this watcher is placed in the chan-server and tied to the terminal session. Closing the terminal or restarting means dropping the watcher - we do not re-create them if the terminal exits. When there is a change, we look at the content, see the destination @@{name}, find the matching tab, and write just 'poke\n' to them, for now. Leave a TODO so that later we can consider doing `/clear` and other more exotic automation like `/effort` or `/fast` and so on depending on the agent. This will come at a cost - the agent may not be running, may be stuck in some other prompt, etc; there will be issues but the happy path is really really #movefast. Will deal with all the corner cases as I come across them later.

_Engineering addendum (2026-05-18 18:30 BST, agreed with @@Architect):_ writers do temp + rename for every event file, so the watcher reads each one once on fsnotify and never sees a partial - no defensive multi-read on the chan-server side. And chan-server's reaction to a watched event must never write back into the watched directory: poke goes to the PTY, not to disk. Any chan-server-emitted artifact (acks, status mirrors, logs) lands in a sibling dir outside the watch root. If we ever do need to write inside a watched dir later, reuse `crates/chan-server/src/self_writes.rs` for notify suppression. Per-language temp+rename examples (bash `mv`, python `os.replace`, rust `std::fs::rename`, JS `fs.renameSync`) land in the orchestration SKILL for external agent authors.


### Session setup
I want to be able to ask from the rich prompt for an agent to setup a session, and specify who'll work on it - which profiles, etc.. if the user already have a setup similar ot mine this will be a breeze, but if they dont, we should provide the SKILL (as part of our codebase, here in docs/) for creating their own setup and orchestrating Chan
Via a distinct mechanism (http from the agent to chan serve? this is better than mcp imo, especially if we make it simple (how to handle chan-server's token vs --no-token token?)), we want to allow agents (in our process, this will be only the @@Architect) to create new terminal tabs in the current pane, name them, restart to pick up on the name, and execute a command in there to start an agent, prompt them to read the process and report liveliness so that we can dispatch tasks. We should be able to specifiy e.g. claude, codex, genini, and their flags, e.g. model, fast, pro;
To tie this all in, I suspect we will need to run these agents with full permission and let the @@Architect and myself handle permissions via the survey method instead. This means adding the flags necessary to make them run. We should also have some kind of pre-flight to set them up and an initial troubleshooting in case they come up requesting auth, login, need the user to restart etc. This should be easy to repro, and you can use my gemini if you need to scratch the ~/.{settings} and later move back
Once this is all set, we should be able to echo/broadcast to the agents via this notification system which i will likely to approve/deny/survey from the rich prompt bubbles
















