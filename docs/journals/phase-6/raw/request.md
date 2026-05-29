# Phase 6
More refinements, new features.
This time we are going to run parallel tracks because there is a diverse set of items.
The process remains in  [](./process.md) and we will follow as usual.

The team will be:
- @@Architect
- @@WebtestA, @@WebtestB
- @@Frontend
- @@Backsystacean which is our mix of @@Backend + @@Syseng + @@Rustacean

## Architectural cleanups
We are going to fix the way we index and plot the graph. From now own, the primary layer of the graph is the filesystem, starting from the drive. When we click "Graph this" from the file browser, the graph starts from the drive - we already have this, it's just not the default. Now it will be the default.
For this most fundamental layer, we have:
- 1. The drive itself, which has its own inspector widget already
- 3.  Files
  - Capable of recognising all kinds of files (fs specific, devices, syn/hard links, etc); also permissions: locked (ro)
    - Locked / read-only directories mean dead-end on graph, for example
  - 3.1. Directories
    - We really need to settle between directory and folder; I am picking directory and dir for short); lets codemod folder out
  -  4.  Markdown, special treatment
    - 4.1.  Regular markdown
    - 4.2.  Frontmatter markdown
      -  4.2.1.  Today we support kind: chan.contact
      -  4.2.2.  We are later going to support kind: chan.{other}
    -  4.3. Other: #{tags} and @@{mention} parented and indexed only from markdown files, can be the scope of graph
  -  5.  Text files
    - 5.1.  We allow editing them in the plain text / source code editor (has syntax highlight, etc)
    - 5.2. We accumulate chan-report data and provide info in the file inspector across surfaces (file browser, graph, search)
  - 6. Binary files
    - 6.2. We show minimal information
The layers above are:
* Markdown and cross links, tags, and mentions; in our case contacts are rendered different but they're just markdown links
* Language, which binds to directory in the graph, is present in the inspector for all files and directories, including the drive itself (whcih has a full breakdown of the drive); the color code for language today collapses with tags (green); we need new color - code is pink, but not very shiny pink, like royal pink! 

## Bugs / nits
- [ ] The new file dialog should start from this: the current dir + selected "untitled".md, without the first step where we request the user to press tab:
_[screenshot removed: new-file dialog prompting the user to press Tab first]_
  - The "New File" from the file editor menu should fire up with a file in the same parent directory of the current file being edited
* [ ]    When we switch from dark to light mode and vice-versa, the terminals do not refresh. Only when we click to Reload.
* [ ]    The left-click in the PANE has a single button "Reload"; we should include a button to turn on the inspector
* [ ]    When we open the overlays, the area outside the overlay has no binding for right-click - it shows the browser's; let's show the same 2-button menu from the PANE, where we have Reload and toggle Inspector
* [ ]    In the file browser, and in the file editor's menu, we should include a button 'copy file path'
* [ ]    the information in the terminal's top bar today [size] ... [search] [copy] [restart] moves to the terminal's bubble menu
* [ ]    the terminal's right click needs basic copy/paste functionality to interact with the terminal, also Copy path to dir (to CWD), Show Dir (to open in file browser), Graph dir, New Terminal, the split pane buttons, the search buttons, and the settings button
* [ ]    Each new terminal starts with an enumerated name, Terminal-N
* [ ]    When 2 files in the pane have the same name (e.g. foo.md or foo.c), show their most common ancestor / [... ] / name to disambiguate; we want the shortest form possible to save space in the tab's name; hovering could show the full path and we already have the button to show in the file browser
* [ ]    The file browser should have a Copy Path in all files and directories
* [ ]    When we ^d on the shell today, it stays stuck in this stuck state.. which is finel; we should be able to detect and print 'press ^d to close the tab' and wire that up
* [ ]    When we change the tab name, the ENV does not change... how can we fix that?
* [ ]    In our embedded terminal, shift+enter does not work on programs like claude or codex; it just does enter instead
