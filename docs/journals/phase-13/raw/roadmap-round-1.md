# Phase 13 round 1
## Bugs
- When we create a new document, the Editor should come up with the cursor ready to type - in today's editor the focus and cursor aren't set
- We still keep seeing "Unsaved changes from a previous session were found." on a brand new document, draft or not; need to be clicking 'restore' or 'discard'
  - This is very disruptive to users who are new to the editor and will be wondering which changes is this thing referring to? nothing other than the editor made changes here
- The bug on the way we create lists is still present in this version: we want lists to preserve what the user actually wanted: hyphen, bullet, number
  - In our current version, even if I start a list with a dash, it changes to bullet; i know this is just rendering, but i want the rendering to reflect what's in the source code
- The highlight of an empty pane is thicker inside the pane than it is at the top bar; we want the thickness of the topbar all across, not the one from the body: _[screenshot removed: empty-pane highlight thicker inside the pane than at the top bar]_
- The panes themselves should be using the css hover wobble, for when the mouse goes in and out of a pane; this is the same effect we already use for the tab pills, for the right-click menus as well; they should be in the panes
  - We also want this effect when we switch panes via the keyboard shortcut, since the pane is receiving / losing focus
  - Shift-enter used to work in our previous version of the Hybrid Terminal. Now when I press shift-enter inside an agent's prompt it does not do what I expect (new line) and submits the prompt instead; I usually test with claude and codex, and neither are working as expected atm

## Enhancements
Changes expected for the next patch version.
### Hybrid Inspector
This section covers enhancements of the Inspector widget which we currently display on various Hybrid components:
- File Browser (except docked): when users click on a file or directory, the Inspector displays information related to that path; we use (and will continue using) the same / slightly variant Inspector across Editor (the DETAILS right-click menu), Graph (when users click on a node), and so on; we also have the same inspector in the SEARCH OverlayShell, which we haven't checked in a while but should be using the same inspector of all other Hybrid components
  - We currently have a "Show path" link there, which currently just displays the path of the file relative to the workspace root; we will change to show the absolute full path and include a [COPY] icon so that if the user clicks it copies the path - we already have this functionality in the right-click menu: Copy path to selection (or something like that) and we could reuse the code if possible
  - The FILE KINDS and LANGUAGES shown in the inspector should all become links to the Hybrid Graph once we land the enhancements required to support these other KINDS of graph
    - This means we keep "Graph from here" for KIND=path, but then clicking the hashtag or a contact or a language is effectively "Graph from here" for that kind - see the Graph details below
  - The inspector for the WORKSPACE root today is different than the others, and it should become like the inspector of any other folder except for the icon being different because it's the workspace root (different icon is already the case so no change)

### Hybrid Graph
We are going to re-introduce the idea of KINDS for the Hybrid Graph:
- Path (file (binary, special (symlink? hardlink? block devices, socket, fifo, etc?)), directory, document, media (pdf, jpg, png, etc...))
- Path
  - File:
    * Text (.txt, .py, source code, etc)
    * Document (.md)
    * Media (pdf, jpg, png, avi, mpg, mp4, divx, etc)
    - Binary
    - Special (symlink, hardlink, block devices, socket, fifo, etc?)
  - Directory / Folder
- Contact (@@mention or .md with frontmatter kind: contact)
- Hashtag
- Language

We keep / update / reuse the exising colour code for nodes and edges.

We always plot the whole filter set above turned on, meaning we show all the "layers" which are:

Path: this is the spine of the graph and starts with the workspace root at the bottom, edges going upwards to nodes representing the filesystem. Always starts with depth=1 which is the equivalent of "collapse all directories from here" where "from here" is the starting point of the graph: either the root of the workspace or a sub-directory. In all cases for subdirectories, we will show the backwards edges to directory tree all the way to the workspace root. Broken links (e.g. symlinks) will point to ghost nodes which we use dotted lines to represent, with a ghost icon inside (already the case today).
When users click on a node that is directory, it is the equivalent of "expand" in the File Browser, loading the next immediate 1st degree of files and folders inside that directory. It also loads the file or directory inspector like it does today. If the user clicks on a node that is node in the current path's tree, we collapse it back so that we can expand the other path to the node they clicked. The depth slider should do the same thing but it does in batches to all first-degree directories from the selected directory. Increasing the slider is effectively the "expand all immediate subdirectories and stop at depth=N" in File Browser's lingo. Clicking individual nodes is effectively the same as navigating the file browser and expanding individual directories, with the exception that if you start expanding them from back in the tree, in the graph we start hiding the old path and showing the new one.

All files have edges to their parent dir, of course.

File: special files need different icon and colour code; symlinks within the workspace should resolve naturally as edges between the symlink and its target file or dir. I want the edges of the same colour of the symlink node. Similar for hardlink. 

Documents (.md) are presented as a "layer" over the graph, because they link to each other and they also have link to hashtags and contacts (kind: contact and @@mention). 

Their edges should follow the colour code of the target link: e.g. if it's a link between 2 documents the edge is orange (in today's colour set), if the link is between a document and a hashtag (because the doc has the hashtag and we indexed it) the edge should be green (because the hashtag is green in our current colourset), contacts yellow, so on and so forth.

The hashtags and contacts should have edges back to all nodes which are documents - the backlinks.

Language: these are now going to be bubles no longer having edges to a directory (they currently only have edge to workspace root, that's wrong) and from now on the language node will have edges to all files of that language.

The concept of the DEPTH does not apply for hashtag and language because they have direct edges to their targets.

The Graph's tab title should reflect the kind: path=x, tag=y, lang=n, contact=z.

### Hybrid Infographics
* Renaming from Infographics -> Dashboard

Thesis:
I want to make enhancements to our overall UI which are heavily leaning towards moving functionality away from the current Settings OverlayShell over to specific components inside the Infographics, which from now on I will call Hybrid Dashboard until we settle on the name.
How I envision the Dashboard working:
- Today it is already a carrousel widget, living inside a Hybrid Tab; it shall remain like that
- The carrousel widget inside the tab must be adjusted to be of the size (width/height) of the tab, auto resizeable with the tab itself (if users resize the tab, the stuff inside the widget should become aware of the new size, that's what i mean), so the items inside can be either centralised or maximised to the size of the tab

Which wigets we will have there:
* About
  * The about information from the Settings Overlayshell will move here, and we will also include the qr-donate.png embedded here, with the same text we have in the web-marketing: fund this etc
    * Include link to website and source code repo with icons and link
- Workspace-wide information
  - This is effectively the new inspector for the workspace root directory
- Search: the index graph we have today, but updated to use the same graph we use in the graph tab in read-only mode, only showing the spine of directories (no files) starting from the workspace root at the bottom and all subdirs going up; the colour codes remain here: grey node and edges = pending, green node and edge = indexed, pulsing orange and orange edge = indexing. This graph will be always displayed with depth=max showing the entire workspace's directories - still, follow the same standard as our main graph described above.

Settings:
* The current Settings OverlayShell is going away since the "About" part becomes the first item in the carrousel
* The remaining items, Appearence and Screen Lock are going to move to become Settings of the Hybrid Dashboard, meaning when we flip the Dashboard, we will see a window similar to the configuration of the Editor, Terminal, Graph with a title, dark/ligh switch but in this case we have a default for System as well, and this setting changes the overall system/dark/light for the entire chan, not just this tab;
* Then below this we will have the screensaveer config
* Then the OK button like the other settings back-of-the-hybrid cases

With this change we can:
- Retire the Settings OverlayShell
- Remap the Cmd+, shortcut to each individual component of the Hybrid:
  - Cmd+, on Terminal, Editor, Graph, File Browser, Dashboard will then toggle flip these elements instead (it flips the focused element)



