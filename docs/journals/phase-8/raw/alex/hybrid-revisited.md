# Revisited Hybrid
These are my current thoughts on Hybrid and the changes I want to make in this phase.
Current state of things (braindump, also will use later for documentation):
- Hybrid currently supports 4 surfaces, and we are adding a 5th
- These are:
  - Hybrid Terminal - our server-backed terminal, inspired by iTerm2 and tmux -CC
    - Supports tab names integrated with environment variables and broacast
    - Provides seamless integration with Chan's MCP server
  - Hybrid Files - our file browser
    - Fully integrated with the rest of Hybrid: Terminal from here, Graph from here
    - Can live in tabs but also docked on left and right
  - Hybrid Editor - document and source code editor
    - Full-featured Markdown editor with familiar interface and features
    - Fully integrated with the rest of Hybrid: Graph from here
  - Hybrid Graph - second brain visualiser 
    - Aids visualising your drive's content: filesystem, documentation, code reports
    - Fully integrated with the rest of Hybrid: Open
- Besides those, the search overlay:
  - Provides semantic and hybrid search (meaning) using bge-small.
  - Index of your drive with code analysis

What I want to spec out, tho, is a change related to the flip.
Instead of having more space for tabs and what not, the flip will now operate like this:
- The tab currently selected will dictate what is shown after the flip
- We will use the back of the pane for configuration of the Hybrid surface, not for more terminals and FBs, etc
- If you are on a terminal tab and flip, the terminal settings will be there
- Equally, if you are on a editor tab and flip, the editor settings will be there
- We will no longer do the automatic dark/light switch on flip, both sides will be the same colour from now on, and they can still switch not individually but both sides at once, individually from other panes yes, from sides no

What goes where?
These are the configuration elements that we'll move from A to B:
- Hybrid Terminal
  - All terminal settings that are today in settings and spec'd out for this phase
  - These should no longer exist in the Settings overlay, and move entirely to behind a terminal tab
  - We should warn that these settings impact all terminals, not just the current terminal
- Hybrid Editor
  - Move from the Settings overlay: Theme, Layout, Date Pills, On Save
- Hybrid Graph
  - We should create a grid of [Node] [Colour] for the nodes we support: Dir, File (Regular, Code, Document, Contact), Hashtag, Mention, Language(Code)
- File Browser
  - Let's leave empty for now

When we flip to the back, each of these surfaces should have their name. My inspiration for the flip is the Propellerheads Reason software, which I used back in the end of the 90s to record music.
