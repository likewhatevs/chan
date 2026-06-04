# TODO list for chan v0.26.0
We are currently on v0.25.0 and here is a list of fixes and changes we will need to complete to land v0.26.0.
## Items
### Repo
We are going to consolidate all of `docs/journals` into phase-N.md docs with the phase's roadmap, rounds, waves, and retrospective. We are going to consolidate the `docs/agents` into a minimum set of agents which are mentioned in the journals and phases. We want to capture the essense of what's needed for our new agents to learn from the execution and success and mistakes of previous phases.
We are going to clean up the repo, deleting:
- `.claude`
- `.codex`
- `docs/archive`
- `docs/agents`
- `docs/journals`
### Editor
Markdown editor bugs:
* We need to clean up the bullet lists and the extra behaviour around them; the way ENUMERATED lists work is perfect (the way the cursor moves, where we place the cursor on clicks); I want the bullet lists and hyphenated lists to have the same behaviour of the cursor and indent and clicks of the enumerated list
  * for context: There's some annoying bugs with unordered lists now, which is when you have indented items, you are on row 1, you press arrow down to go to the next item, it places the cursor *before* the glyph, which makes absolutely no sense
  * Where are my hyphenated lists? we seem to have regressed and they are now bullet again... I did ask for a change in [phase 17 (see retrospective)](../../phase-17/retrospective.md) to match the google docs style, but that was only for bullet lists, not hyphenated lists.
* The scroll with trackpad does not scroll freely over the document... even over this very document the scroll hangs a bit while going down or up; to repro, the cursor must be on one part of the doc (e.g. top) and you scroll to the bottom and back up; if the cursor is at the bottom and you scroll up and down, you should be able to repro: scroll hangs a bit, moves opposite and back, then scrolls to where you wanted; this is bad, we need to fix it
* The `[[` should be able to complete paths of the local workspace, it doesn't! I know the path and can type it in and it will work but the editor won't help with autocomplete when it should

### File Browser
* Bug / enhancement: the right-click context menu have been merged with the less-complete right-click menu of the docked file browser. This is a regression from phase-17 as well, when I asked to clean up the context menus and missed this regression specifically with the file browser tab right click.
  * What I want:
    * Remove the "Reload" button from the right-click menu on file browser
    * Below "Expand all directories" we are going to add:
      * "New file or Directory" which creates a new file or directory from the workspace root
      * "New Terminal" which opens a new terminal on the workspace root
      * "New Graph" which opens a graph tab from the workspace root
* The right-click context menu (when there's a file or directory selected) is not showing the keyboard shortcuts (new terminal cmd+t, new graph cmd+shift+m, copy delete (backspace), settings (cmd+,) and we need to adjust for the supported platforms and ensure these are recorded in our central shortcut store so that they can be ported to linux and macos and web: ![](./image-6.png#w=250)
* File browser hanging on "Loading" while trying to expand a directory, and only reload of the window makes it work again... after the reload I opened the console and I keep seeing this: ![](./image-9.png#w=250)  but I don't know if this comes from file browser or editor


### Graph
Bugs:
* When we click "Graph from here" on the inspector, in all cases, the new graph should load with the node from "Graph from here" selected; today we redraw the graph but do not select the node used to "graph from here" which makes it hard for the user to find that node
* I am still finding cases where we have a directory plotted but no visible edge, here is an example where the Drafts folder (the one outside the workspace) is showing but has no edge: ![](./image-5.png#w=250)
* Binary file rendering as contact node: ![](./image-10.png#w=250)  and this graph keeps reloading on its own every few seconds; the graph reloads every time I edit/update my document in the other tab... and my document is NOT at the workspace root so it shouldn't trigger a reload of the graph.. if this is a real bug, any change to any file in the workspace would trigger a graph reload, this is BAD and shouldn't be the case at all
* Found another case where we plot directories without edges to the workspace root: ![](./image-12.png#w=250)

Enhancement:
* Make it possible for graph tabs to have links to reproduce the tab/graph: add a "Copy link to graph" button to the right-click menu of the Graph tab, where the "Reload" button is - we will remove the Reload button.
* We should be able to copy these links into a markdown file and open the graph tab on click

### Terminal
Bugs:
- When we hide the rich prompt on a terminal with the menu click or cmd+shift+p, the focus must go back to the terminal - today the focus does not go to the terminal
- Similar to File Browser's context right-click menu not showing shortcuts: ![](./image-7.png#w=250)  and we can cmd+c/cmd+v for copy/paste
- Our terminal is not printing certain characters correctly, neither on `less` or `vim` I can see these from [](../docs/config-reference.md) : ![](./image-14.png#w=250)  on less and ![](./image-15.png#w=250)  on vim
### Inspector
These are enhancements for how the inspector is presented for each item on each hybrid component. We will use a single pill-type button with a main action and a dropdown with the other actions.

* File Browser
  * Directory
    * We currently show Upload, Download, and Graph from here
    * We want: main action "Open" which opens the directory in a new file browser tab
    * In the drop down, we will have:
      * Upload file here
      * Download tarball
      * New terminal here
      * Graph from here
  * File (editable files)
    * We currently show Open, Upload, Download, Graph from here
    * We want: main action "Open" which opens the file in the Hybrid Editor
    * In the drop down:
      * Download file
      * New terminal here (opens a new terminal tab with seeded input: "{cursor}{space}{relative-path-to-file}"
      * Graph from here
  * Media
    * We currently have View / Zoom, Upload, Download, Graph from here
    * We want: main action "View / Zoom" which opens the same view/zoom of today
    * In the dropdown:
      * Download file
      * New terminal here (same as for editable files, seeded input)
      * Graph from here
  * Binary (including symlinks, all BINARY category)
    * We currently have Upload, Download, Graph from here
    * We want: main action "Download file" and in the dropdown "Graph from here"
* Editor
  * When we open the inspector for the file being edited (Show Details menu option) the inspector has Upload, Download, Show File, Graph from here
  * What we want: main action "Show file" which opens a new file browser tab with the file selected
  * In the dropdown we want:
    * Download file
    * New terminal here (same behaviour of File Browser, seeding with {cursor}{space}{relative-path})
    * Graph from here



### chan-desktop
When we go through the [New] workspace flow and go for a local disk, the old pre-flight dialog still shows and mess up with the new menu... we should NOT have any pre-flight in the chan-desktop app anymove since this have moved over to chan's SPA during boot.
References: ![](./image-1.png#w=250) and ![](./image-2.png#w=250)

