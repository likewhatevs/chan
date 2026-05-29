# Round 1 closing tests
Follow up from @@LaneB questionnaire:

⏺ Desktop relinking (Rust deps are warm). Smoke checklist for you when the window opens:

  KIND chips → lens-centered graphs
  - Click on a file → Inspector → click the file's parent-dir chip → graph tab opens with path=<dir>/ lens.

@@Alex: Clicked on a file, inspector -> parent-dir chip -> graph from here.
1. Does not show path=<dir> in the tab's name, just <dir> without path=
2. The workspace root dir is missing the 'Graph from here' and Download buttons. Plus, the "NOTES DIRECTORIES" should now only exist in the dashboard, not in this inspector anymore. I know the dashboard uses the inspector, but I want this to only show in the dashboard and here we need consistency across directories and the way they are presented. The only special thing about the workspace root is that it's the root of the tree and FB/Graph/Search are sandboxed here but not the Terminal

However, when I click on the parent dir, the inspector for the directory is missing.

  - Click on a contact chip (e.g., @@somename in inspector) → tab title contact=<basename>; graph centers on the contact file
  with backlinks bidirectional.

@@Alex: despite having multiple mentions across various .md files, I do not see any of them in the graph, at all. Perhaps our graph is only considering contacts the .md files with frontmatter kind: contact, because I also realised the @@{mentions} aren't available for searching when I type `@{name}` in the editor: _[screenshot removed: mentions not appearing in the graph; no @-mention search]_
 
  
  - Click on a #tag chip → tab title tag=#name; graph centers on the tag, edges to docs that reference it.

@@Alex: tags do show in the tab name, but nothing comes up in the graph; I'd expect to see the backlinks to the documents, which are in turn showing their parent dir on the spine as well

  - Click on a language chip → tab title lang=<name>; bubble + 1-hop edges to every file of that language.

@@Alex: clicked on Markdown, it is only showing 1 directory (out of MANY that we have markdown on), and the language itself has no Inspector: _[screenshot removed: Markdown shows only one directory; language has no inspector]_


  Dashboard carousel (open the Dashboard via Cmd+I or hamburger menu → Infographics)

@@Alex: missing the "Dashboard" option which should be after Graph here: _[screenshot removed: missing the Dashboard option after Graph]_

Also the main button still says Infographics, it should say Dashboard: _[screenshot removed: main button still says Infographics, should say Dashboard]_

  
  - Slide 1: About — version, embeddings flag, font/screensaver attributions, QR donate image, "Fund the work" copy, chan.app +
   github.com icon-links.

@@Alex: the qr-donate is not showing; we must embed this image in our server: _[screenshot removed: qr-donate image not showing]_

Questions / concerns:
1. How to enable / disable Source Code Pro? imho this should be a setting in the back of the TERMINAL because it's a terminal setting
2. Where is the Screen Lock configuration now? I explicitly asked this to be configured in the settings of the Hybrid Dashboard, which is currently empty: _[screenshot removed: Screen Lock config missing from the Dashboard settings]_ 
   
  - Slide 2: Workspace info — WorkspaceInfoBody content (file counts, languages, top-N).

@@Alex: This one is almost great; we are going to add the buttons for here and for the Graph's inspector, making it more like any other directory: Show in File Browser, Graph from here, Upload, Download (i described some of these above, this is a more complete list)

I also want a separator between COCOMO and NOTES DIRECTORIES

  - Slide 3: Indexing graph — directory spine with grey/green/orange colors; orange directories should pulse unless you have
  prefers-reduced-motion: reduce on.

@@Alex: looking almost good; nits:
1. When we click a node, we should show the label of that node and all immediate siblings / 1st degree connections
2. Ideally the graph should maximise the use of the viewport by default, like a decent zoom showing the entire spine.. the default here is too small

In all of these, we have a right-click menu with only a "Settings" option now showing the Cmd+, shortcut. We must add the Reload Cmd+R especially for widgets like the workspace and the graph in case users want to refresh.

I actually realised that if I right-click any of these dashboard panes and click Settings, it shows the settings.. but if I hit Cmd+, it shows an empty settings for the same widget; this is a bug!


 -  Cmd+, flip behavior (the big rebind)

Buggy as described above

  - Cmd+, on a focused Terminal → flips to HybridTerminalConfig.

Correct

  - Cmd+, on Editor → HybridEditorConfig.

Correct

  - Cmd+, on Graph → HybridGraphConfig.

Correct

  - Cmd+, on FB → HybridFileBrowserConfig.

Correct

  - Cmd+, on Dashboard → back-of-card shows Appearance / Screen Lock / Screensaver / Metadata archive + OK.

Not working; Cmd+, on Dashboard back-of-card shows empty window; right-click and Settings works, but clicking again does not flip it back, only OK does

  - Cmd+, again on each → flips back to front.

Not working

  - Cmd+, on an empty pane → no-op (no surface to flip).

Cmd+, on empty pane flips the whole pane: _[screenshot removed: graph scoping too aggressive; whole graph lost on a node click at depth 5]_


  - Lane A Shift+Enter in the terminal rich prompt (claude/codex agent) → inserts a newline instead of submitting.

Works! Cheers