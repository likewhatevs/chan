# Phase 17 - round 1
## Bugs
- The rich prompt shortcut cmd+shift+p must bring up the rich prompt only on the selected terminal in the selected pane. If there's no terminal selected, the shortcut should not do anything. In today's implementation the shortcut cmd+shift+p toggles the rich prompt bubble on all terminals, which is terrible. And eve worse, it places the cursor and focus on the last terminal, while the pane's focus is on the first in my case. The cmd+shift+p toggle is a per-terminal shortcut, isolated to the current tab in the current pane with the focus. When we toggle to show the rich prompt, we place the focus and cursor in the prompt area. We should be able to resize the top of the rich prompt all the way to the top of the terminal, using the same margin we have at the bottom. And the survey bubbles must always show ON TOP of the rich prompt, not under it - wouldn't be visible.
- The unordered list still needs more refinement, and we are going to copy the bullet glyps from Google Docs: ![](./image.png#w=250)
- Loading an existing team is not straightforward, you have to type `/` to complete the path so that it loads the config. This is super confusing UX: ![](./image-1.png#w=250) 
- The `cs pane split` command must resemble the options from the hybrid pane's hamburger, meaning the options for split must be **right** and **bottom**.  Also the cs pane close command on a window with 2 panes, from the top pane with a single terminal sending the command to split to bottom did the split but then moved the focus there, took it away from the terminal... and after i put the focus back on the terminal and sent a cs pane close to close the bottom pane, it did close, but my terminal remained sort of stuck perhaps in transaction mode? these one-shot cs commands shouldn't enter hybrid nav transaction mode, at all; and ideally the terminal which sent the command (whatever command it is), does not lose the focus, except if the command is to change the pane / tab focus.
- The MCP server keeps failing when we start codex. I suspect it's because codex requires configuration in files beyond just setting environment variables. I want us to 1) never touch the user's config files for MCP settings, and 2) start up terminals with the MCP env vars option DISABLED by default. Agents will still be able to use `cs search` and friends and when the MCP server is disabled.
- When we save an editor draft which is a directory, because it has images, the dialog to save to a path does not contain the auto-complete, and it should: ![](./image-10.png#w=250)
- The public website does not offer links to various of the packages we build in the release, and does not mention the chan gateway and its components
- The `cs terminal write --submit codex` does not submit on codex. I tried on this machine and it writes the command followed by what looks like a new line, and does not submit; fix it, you can test locally with naked write and the submit sequence if any - def not the current \r
- The graph: when I press cmd+shift+m to open a new graph, from a fresh new window, the graph opens as expected, but if I try to double-click the directories they don't expand.. that is until I click on the workspace root, Graph from here, and only then I can double click the directories to expand them
  - Once I can expand and collapse them, I can no longer use the depth slider from the context menu...
  - What I want, regarding the slider, is that it will expand all of the directories from the currently selected directory node onwards; if the selected node is the workspace root, the slider to the maximum right will show the entire workspace.. if there's like 2 sub-directories deep and I crank up the slider, it will only expand from the currently selected node/leaf onwards, if there are sub-directories
  - Also it seems that when I click "graph from here" on a directory, we lose the initial layers that we had when I pressed cmd+shift+m.. in that case, we were showing all files and languages and tags and contacts, but once we hit the "graph from here" on a directory (in this csse the workspace root) we only see directories.. what I want is to see all the layers (spine / directory tree) and files with edges to directories, in case of markdown they can have edges to their linked and backlinked files plus hashtags and contacts/mentions; and languages can have edges to pretty much any files (im re-describing what we have and also what i expect when we hit graph from here on directories)
- When I run `chan serve .` on a very large workspace like a shallow clone of the linux kernel (e.g. git clone --depth=1 https://github.com/torvalds/linux /tmp/linux-shallow) the command runs "in silence" for a long time, even with --verbose, and it's hard for the users to understand what is happening unless they open another terminal and run htop.. we should print something to the terminal, not too excessive, about what chan is doing there, before it spits out the url for the web view
- 


## Enhancements
The Spawn agents dialog must have an auto-assign button after the user chooses a layout: ![](./image-2.png#w=250)
After selecting the layout, e.g. 2x2, 1x4, 4x1, a button on the same row on the right hand side with an icon for auto-assigning the robots to the selected layout. ![](./image-3.png#w=250)


## Documentation
Both the readme and the website should open with an example of using chan:
1. Download chan for the command line: curl | bash
2. Open the IDE on any existing git repo, or clone chan's for example:
  3. git clone https://github.com/fiorix/chan
  4. chan serve ./chan
5. The IDE will open in the local browser, use it like you use your machine

The in-browser experience provides the full featured IDE, but the keyboard shortcuts are suboptimal - still powerful though, check out the hybrid nav.

The chan-desktop provides a native version of the IDE for macOS and Linux, and it supports attaching sessions from remote workspaces, inbound and outbound:

1. Open the chan-desktop native app
  2. Chan.app (add llink) on macOS and Chan.AppImage (add link) on Linux
3. Connect to a remote machine, e.g. a lima-vm on your mac, install chan, and serve from there:
  4. limactl shell default ... cat < EOF ... (insert the curl | bash to install chan, the git clone for chan's source code, then chan serve on it)
5. Click New -> Remote -> Outbound
  6. Paste the URL from `chan serve` from the terminal, and have local experience on the remove env
  7. You can repeat this process using an ssh tunnel, e.g.
    8. ssh user@host -L 8787:localhost:8787 cat < EOF | bash ... curl | bash to install chan, git clone for chan's source code, then chan serve on it)
    9. New -> Remote -> Outbound, paste the URL...

If you cannot listen on the remote machine, chan can make a reverse tunnel, e.g.:

1. Open the chan-desktop app
2. New -> Remote -> Inbound
  3. Pick the port to listen on
  4. Copy paste the chan command to initiate the tunnel back to chan-desktop


We will need to audit and test these commands before publishing.

### Chan gateway
The gateway/ directory in the source code provides the SPA for online services: profile and identity services, database management and command line administration tools, and a workspace proxy to serve `chan serve --tunnel-url=<workspace-proxy>` so that you can share your local drives remotely and access anywhere from the browser, behind OAuth. The codebase supports common proviers and examples for setting up the services locally on a macOS or Linux machine for development and testing, and also available for you to deploy your own.

## Screenshots for the website

### Writing specification while running a session
This very document: ![](./image-4.png#w=250) ![](./image-9.png#w=250)

About running a session with multi-agents working on the chan-desktop roadmap.


### Navigating the UI with the keyboard
The hybrid nav, for optimal navigation even in the browser: ![](./image-5.png#w=250)

### Onboarding of a new workspace
The onboarding of a new workspace, e.g. a git repo: ![](./image-6.png#w=250)

### Operating the UI from terminal

Sending commands to coordinate agents:
![](./image-11.png#w=250)
Agents can coordinate with each other. Any agents can coordinate with each other.

Agents can set up and tear down other agents:
![](./image-13.png#w=250)


### Diagrams and agents
Through the command line tools, through the MCP server where they can get extra information:
![](./image-12.png#w=250)

