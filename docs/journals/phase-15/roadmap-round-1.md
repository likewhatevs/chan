# Phase 15 round 1
Multiple enhancement requests, and some fixes.

Author: @@Alex
2026-05-30 

## Bugs
- The Settings / Flip of the Hybrid tabs is still problematic. The effect we are getting now is that pressing Cmd+, immediately flips the tab without the css flip effect. And if you press Cmd+, again, it correctly flips it back to the front of the tab, however still without the css flip effect. Only when the focus goes away from the pane the css effect kicks in. And from that point on the effect and focus and flip are all entangled in a bad way.
- The directory inspector in the dashboard Search (shown when clicking on nodes) is missing the buttons: "Show Directory" (to open a new tab with the selection in file browser), also "New Terminal" to open a terminal in $cwd (the selected directory); it should NOT show the [Upload] button in that particular inspector in the dashboard Search

## Search
In the Search Overlayshell we still have SCOPE and the SEARCH STATUS button + OverlayShell, which we no longer need. There's an item below to move the "index" bit (where the info/rebuild index button is) over to the carrousel search settings page. With that, we are ready to completely remove the scope selector, the search status button, and the search status overlay page entirely.

## Dashboard
1. We are going to add a right-click menu to the Dashboard like all other tabs, which is activated by right-clicking the tab's title.
  2. In this menu we will have a list of the carrousel item displayed vertically, with checkboxes on/off
    3. At least one of them must be selected; by default, from boot, all of them are checked
    4. Checked items will participate in the carrousel's auto-rotation; unchecked items are skipped
    5. These properties are **per tab** meaning if the user opens another tab, they start with the default all-checked state
    4. Each checkbox represents a slot in the carrousel, and each slot has front and back panels
  5. Separator
  6. Settings Cmd+,
    5. When the user press Cmd+, or click the right-click menu "Settings", the tab will flip and show its back
    6. It will remain showing the carrousel slot picker < ... > [play/pause] (always paused when showing the back, user-defined state on the front, default play)
    7. Since each slot has front and back, when the back is shown, each slot can have its own configuration

About the new carrousel and requirements:
1. Each carrousel slot has a front and back surface
2. The carrousel tab will always show the slot picker (the widget at the bottom of the carrousel tab < .. > [play/pause]), on the front and the back of the tab, per slot
3. When showing the back of the tab, the slot picker will always pause (user cannot click to play, they can click to < prev and ... (specific slot) and next >)

### Carrousel slots
The carrousel slots and their configuration:

**About**

Front:
- chan version {version} {license}
- [global-icon] [link: chan.app] ... </> githit repo (same as today)
- [separator]
- Fund the work (same as today)

Back:
- Appearance (same as today)
- Screen lock
  - [info text: Auto-lock the workspace ... [x] On/Off] (same as today)
  - Preview (new widget, a screen-like widget of size relative to the width of the tab 25% margin left and right, with a preview of the screensaver)
  - [Info: Theme] [Dropdown: Default, Matrix] (same as today but s/Plain/Default, and changing the value changes the preview)
  - [label: Inactivity timeout (seconds):] [numeric input from 10-3600] (same as today)
  - [Test] [Set Pin (button)] [info: No PIN set; lockout informational only.] (same as today but info s/yet/set)

 ### Workspace
 Front:
 - Same as today, the Workspace root's Inspector

Back:
- chan-reports
  - Move the entire "chan-reports" section from the current File Browser's Settings from there over to here

### Search
Front:
- Same as today, the graph with the index view
- One change here: at the bottom we show the legend (indexed, indexing, pending) and I'd like to change it:
  - Indexed (will always show)
  - Indexing (will only show when there are nodes in indexing state)
  - Pending (similar to Indexing, will only show when there are nodes in Pending state)

Back:
- Index (we are moving this from a widget that is today in the Search OverlayShell with info text "show search index status"); we will remove it from there entirely and move it here
  - state: {index state} (idle / indexing)
  - chunks: {n}
  - vectors: {n}
  - model: {model}
  - file: {path-to-index}
  - [button: Rebuild index]
- [Separator]
- Semantic search
  - We will move the entire "Semantic search" and "Embedding model" sections from the current File Browser Settings to here
  - For now, the File Browser will have no settings displayed, we can put a placeholder saying "No settings here, cheers." for now. This is because we're moving the search items from File Browser settings over to the back of the Search carrousel slot, and we are also moving the chan-reports section from the File Browser settings over to the Workspace carrousel slot's settings (the back of the tab there)


## Chan Shell
We are going to introduce the `chan shell` subcommand to enable greater integration between the command line and chan's user interface. We will also support a mode in which argv[0]=`cs` calls `chan shell` automatically, so we can make a symlink from chan to cs and get the `cs` `--flags` right away.
We will not automatically create the symlink, but we will create tests to validate that the trigger from argv[0] works as expected.
The subcommands for `chan shell` or `cs` :
- open [path]
  - The path argument is required. If it is a directory, it will open a new File Browser tab with the directory expanded and selected
  - If the path is an editable file (plain text) we open in a new tab of Hybrid Editor, otherwise we open a File Browser with the file selected (all directories expanded up to the file's path)
- graph [path]
  - Path argument is mandatory; if possible, we could allow `#hashtag`  and `lang={x}`  as an argument here - similar to what you get from the UI
  - Same action as "Graph from here" on path={path}, opens a Hybrid Graph tab with the path as the starting point
- term [path] [--tab-name=x] [--tab-group=y]
  - Optional path (must be directory, e.g. "."), if missing we open the new terminal in the workspace's root
  - The `--tag-*` flags automatically initialise tab name and group (group feature is new, see below)
- term-write [cmd] [--stdin] [--tab-name=x] [--tab-group=y]
  - Writes to terminals, either [cmd] is present (e.g. term-write "echo hello\n" --flags) or --stdin, in which case we stream every N bytes (not lines, to prevent GB-long lines) to the terminal 
  - Requires either tab-name (single write) or tab-group (broadcast write)
- dashboard [--carrousel-on=true] [--carrousel-index=1]
  - Opens a dashboard tab with the carrousel configured to play/pause and to start from the certain slot/slide

TODO: check if we can wire up the "chan shell open" to today's "chan open" which has a somawhat different meaning - see below.

### The chan-desktop integration point
Today's chan-desktop on the macOS (and probably Linux desktop) has a `chan open` to support OS integration. We support opening files and directories, but in a way that differs from the way I have just specified `chan shell open`. This is because from the desktop integration standpoint, `chan open` does not have an associated workspace. This means that before acting on the %path% passed to `chan open` we need to assess whether the path belongs to an existing workspace, then offer to turn on / open, or reject because the path does not belong to a known workspace. On rejection, we should inform the user about how to create a workspace from the command line if they want to.. using `chan serve` for example, because it will be integrated with the desktop.

The `chan shell open` or `cs open` on the other hand, is a command that is expected to be run from inside chan's terminal and relies on the env vars (plus some security measure, we need to inject the web server's token or something?) to work correctly. Should inform the user when it cannot operate and why, e.g. users running cs commands via their ssh session instead of chan's terminal.

## Terminal
In addition to Hybrid Terminal's tab name, which we use to set $CHAN_TAB_NAME, we will also introduce tab group, set as $CHAN_TAB_GROUP. In the context right-click menu of the Terminal, we are going to include a new option Group right below the first option, Name. Same style, different label: Group.

Similar to configuring the name, groups will be configured as a string. The default group for new tabs is "default", unless specified otherwise. Just like the tab name, changing the tab group requires restarting the shell for changes to take effect.

### How we use groups
Here is where the main change will happen: from the moment we have groups, we will use them to channel the terminal broadcast function. What this means is that if the user is on a terminal on the "default" group and enables broadcast via the menu or cmd+shift+i, the broadcast input lands on all other terminals of the same group. If the user has other terminals in a different group, the cmd+shift+i does not even impact them.
The way to validate that this works is by creating 4 terminals: 2 in group "default" and 2 in group "foobar". Enabling broadcast from a terminal of a given group only enables it for other terminals in the same group.
