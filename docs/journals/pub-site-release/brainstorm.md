# Public site release updates
Author: @@Alex
2026-05-31 

**Agents**: Do not use, this is a draft.

Ideas for the launching of the website. We currently have the website published but it's the auto-generated one with content and screenshots from the early versions of Chan, totally innacurate.
What we probably want to highlight:
- Mission: make the world move faster with artificial intelligence
- What is Chan: chan is the first IDE for the sigma generation, born with AI
- What does it provide: seamless local and cloud development with a powerful integrated tool
- What do I get?
  - 100x your productivity with continuous and automated development
  - A familiar environment: markdown, terminal, your well known agents and your setup
  - A unique one-of-a-kind experience using the Hybrid UI:
    - A multi-tab and multi-pane tiling user interface optimised for the keyboard and mouse
    - Tabs can be one of Editor, Terminal, Team Work, File Browser, Graph, or Dashboard
    - Code reports, semantic search, integration with common agents (Claude, Codex, Gemini)
    - Chan does not store any control files in your project's workspace, it's all outside
  - About the Hybrid UI
    - Heavily inspired by a mix of hypr.land and iTerm2's panes
    - Unique experience in using a Hybrid UI integrating editor, terminal and other components
    - Very fluid workflow: from draft to prototype and team in straightforward steps
    - All UI components ship with command-line friendly tooling for ad-hoc use and automation
    - Editor
      - A powerful Markdown-first editor with familiar features and mode of operation
      - Can also edit source code, in case you still need to do that in the sigma generation
      - The Editor is heavily inspired by a mix of Github Markdown, Google Docs, and Obsidian
    - Terminal
      - A fully functional and familiar feeling terminal with broadcast groups, inspired by iTerm2
      - Sessions are persisted server-side resembling `tmux -CC` with a fully integrated terminal
    - Team Work
      - Automated multi-terminal session integrated with agents such as Claude, Codex, Gemini
      - Works out-of-the-box on any project, no pre-requisites other than being on Chan
    - File Browser
      - The embedded file browser for the workspace, on tabs or docked on the left and right
      - Supports importing and exporting workspace metadata across machines
      - Supports importing Google Contacts from CSV into Markdown contacts
    - Graph
      - A multi-layer graph rooted on the workspace's filesystem tree
      - Layered by Markdown links, contacts and mentions, and hashtags
      - Layered by per-file and per-directory code and programming language reports
    - Dashboard
      - Carrousel-like widget showing information about Chan itself and the workspace
      - Configuration of add-ons like screen lock, semantic search, global dark/light setting

## Screenshots
### Chan.app on macOS

Main screen:
![](./image.png#w=250)

![](./image-9.png#w=250)

Editor:
![](./image-1.png#w=250)

![](./image-5.png#w=250)

Terminal:
![](./image-2.png#w=250)
Graph:
![](./image-4.png#w=250)
![](./image-7.png#w=250)

Dashboard / Search (indexing of the workspace):
![](./image-3.png#w=250)
Search:
![](./image-6.png#w=250)
Team Work:
![](./image-13.png#w=250)
![](./image-12.png#w=250)

### Online service https://id.chan.app
The source code of the gateway (identity service and workspace proxy) are part of the chan monorepo and ship with their own admin tools for the command line - enroll users over oauth, enable/disable remote drives, auditing. You can run your own!
TODO: We will need to change my name, username picture.
![](./image-10.png#w=250)
![](./image-11.png#w=250)
