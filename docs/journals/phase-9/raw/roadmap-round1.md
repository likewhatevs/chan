# Roadmap for Phase 9
Author: @@Alex
2026-05-23 
#roadmap #pre-release
**Agents:** do not touch this file.
---
## Deliverables
- [ ] Complete outstanding work from previous phases
- [ ] Sort out the entire list of bugs provided below under "Bugs"
- [ ] Implement the enhancements described below under "Enhancements"
## Outstanding work
Agents to discover the carry over list of items from previous phase.
## Bugs
- Terminal fonts not rendering after switching tabs: _[screenshot removed: terminal glyphs corrupted after switching tabs]_
- Our MCP server seems to be broken for codex in chan v0.13.0:


main* mbp ~/dev/github.com/fiorix/chan $ codex
⚠ MCP client for `chan` failed to start: MCP startup failed: handshaking with MCP server failed: connection closed:
  initialize response

  
- Search (or something else?) making changes to the file I'm editing, without me asking: _[screenshot removed: external-edit-detected modal; file mutated while editing]_
- Bullet lists now indenting like '-' lists do: _[screenshot removed: bullet list with dot markers and indent guides]_ vs _[screenshot removed: dash list shown as the contrasting case]_  
- Using `[[` to search yield no results, yet the search does show results.
- Receiving error of "too many open files" while editing this very file in this repo
- The markdown '---' is not working as it used to, rendering as <hr>
- When we flip the Hybrid and the hamburger goes to the left hand side, we click the menu and it renders outside of the screen towards the left - we need it to always render inside the screen towards the right
- Our terminal cannot render certain characteres, e.g. em dash: _[screenshot removed: terminal showing raw bytes; em dash not rendering]_
- In this very document, if my cursor is sitting at the bottom and I try to scoll up, the editor fights me and scrolls down
## Enhancements
### Backend
chan-drive:
- We are going to entirely separate and isolate the data from each drive
  - Where: ~/.chan/drives/{name}/
  - Inside each drive (sessions and tokens are LRU with 50 slots each):
    - sessions/
    - thrash/  (tombstone delete gradually... this is so we can drop large metadata quickly by mv)
    - report/
    - locks/
    - graph/
    - drafts/
    - tokens/
  - Global: the metadata already in ~/.chan today
- The `chan` binary itself will always use ~/.chan for its metadata, on macOS and on Linux
  - No longer using ~/Library/Application Support/chan
- We need an extra hardening session to validate our hot code paths work as expected
- We need to try to break it editing files and changing the FS / index, and we need to prove a consistent experience editing

chan-server:
- Each chan-server must be able to open multiple isolated drives
- The end-to-end code path from a user editing a markdown file all the way to this being managed on disk, must not interfere with other operations - we want to prioritise editing text and using the terminal above all else
- We are going to embed the chan-server in chan-desktop, instead of forking the chan binary
- We must ensure that there are no synchronous calls that can block our tokio runtime

### Frontend
- Hybrid's Hamburger
  - Changing from this: _[screenshot removed: current Hybrid hamburger menu]_
  - To this:
    - New Draft  Cmd+N
    - Terminal  Cmd+T
    - File Browser  Cmd+O
    - Rich Prompt  Cmd+P
    - Graph  Cmd+shift+M
    - Separator
    - Enter Hybrid Nav  Cmd+.
    - Split right     Cmd+. /
    - Split bottom      Cmd+. \
    - Next  pane   Cmd+]   (goes to next hybrid pane, similar to cmd+. -> enter)
    - Previous  pane    Cmd+[ 
  - Separator
    - Focus border colour (as-is)
      - blue (default; we record when the user changes)
      - orange
      - green
      - pink

Settings page:

  - We no longer need Semantic search and chan-reports in the Settings OverlayShell because we already have it as part of the File Browser setting (on the back of file browser)
    - This setting in any file browser must apply to file browsers of the entire drive
  - Source Code Pro font -> moving to Terminal settings (on the back of terminal, see below)
  - Screen lock needs a Test button, so that we can trigger the screen lock to see what it looks like
  - We also need the default option to be a dark/light screen with nothing (like we had before the matrix and castaway) just the unlock dialog if it's set, or just unlock if no pin is set and the mouse moves... btw I set the timer to 10s and left the machine alone, and it didn't trigger for me at all

Terminal settings (back of the Terminal):

* Changes to the values of these settings apply to ALL terminals in the drive - old and new.

  - Terminal appearance: Light / Dark
    - Changes the *body* of ALL terminal TABS of this drive, not the tab's pill, not the entire Hybrid
  - Scrollback
  - Default TERM (this dropdown didnt pick up the style of the other dropdowns, fix)
  - Terminal Font (ditto)
    - The Source Code Pro font setting moves here; no longer in the Settings OverlayShell
  - Add OK button, to flip the tab back

Editor settings (back of the Editor):

* Changes to the value of these settings apply to ALL editors in the drive - old and new.

  - Editor appearance: Light / Dark
    - Changes the *body* of ALL editor TABS of this drive, not the tab's pill, not the entire Hybrid
  - Other configs remain the same, except for the Date Pills dropdown which needs to pick up on our dropdown style
-   Add OK button, to flip the tab back


Graph settings (back of the Graph):

* Changes to the value of the settings (colours) apply to all Graphs of this drive - old and new.

- Settings as-is
- Add OK button, to flip the tab back

File Browser settings (back of the File Browser):

* Changes to the value of the settings apply to all File Browsers of this drive - old and new.

- Settings as-is except that the dropdowns should pick up on our css style
- Add OK button, to flip the tab back

Infographics:
- Add Settings right-click menu
- Add the appearence settings for light/dark
- Add the OK button like the others
