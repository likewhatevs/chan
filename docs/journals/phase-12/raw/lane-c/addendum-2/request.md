# Addendum 2
Status: Review
Agents: Please review, do not edit

More changes for Lane C or a new Lane D.

## Bugs
- When opening a File Browser tab and expanding directories, the same directories are expanded in the docked File Browser; they must be independent at the UI layer!
- Editor still reloading my file while I write and place the cursor on the first line, first column; this is disrupting to my writing!
- The terminal font completely broke after the screensaver and screen sleep: _[screenshot removed: terminal font broken after screensaver and sleep (1 of 3)]_  and now _[screenshot removed: terminal font broken after screensaver and sleep (2 of 3)]_
but then auto-recovered after a few seconds. Subsequent screensaver in/out did not affect the terminals anymore. They remain slightly glitchy: _[screenshot removed: terminals remain slightly glitchy after auto-recovery]_

## Enhancements
- Drag & Dropping images: we recently added the capability of drag & drop for images which are in a single line of the backing markdown source code. With this, we can now drag & drop the images across different rows of the source code and render them appropriately, plus offer the left/center/right alignment. The next evolution here is to support images which are *within* a line.. this means we'd need to understand the beginning and end of the *paragraph* where the image is, and move that entire paragraph up/down the backing source code, so we can move the image plus the surrounding text; the easy case is 'text \!\[...](...) text' and just move this entire row; the more complex case is when the row belongs to a paragraph but there's no clarity on what is the definition of a paragraph: is the doc has e.g. 80 column wrapped text, a paragraph may mean period follow by empty line; or without the period. Bullet points may be more straightforward, and we may decide to support just this use case (of bullet points) for now
- Shortcuts
  - We will implement a new policy for shortcuts across the 3 supported platforms:
  - Use cmd for macOS, ctrl for Linux
    - web (a few shortcuts, mostly incentived to use Hybrid Nav):
      - alt + shift + [ / ] for prev / next tab in the current pane
      - alt + [ / ] for prev / next pane
      - cmd + , for settings
      - cmd + s for search
      - cmd + .  for Hybrid Nav
    - desktop-native tauri, macOS and Linux
      - cmd + 1..9 for tabs of the current pane
      - cmd + shift + [ / ] for prev / next tab in the current pane
      - cmd + [ / ] for prev / next pane
      - cmd + / for split right
      - cmd + \ for split bottom
      - cmd +w or ctrl+d (linux gets ctrl + [w or d])
      - cmd + w on an empty pane closes the *window*
      - cmd + s for search
      - cmd + . for Hybrid Nav
      - cmd + + / - / 0 for zoom in, out, reset (zoom of the whole window like Chrome's)
      - Hybrid elements mantain their "context aware" functionality of "start from here" and the shortcuts remain, just documenting here for assessment and confirmation:
        - cmd + t for terminal
        - cmd + o for file browser
        - cmd + n for new draft (editor)
        - cmd + shift + m for graph
        - cmd + p for rich prompt
        - cmd + i for info (new? create a new infographics tab in the current pane)
          - We may need to add this one to theHybrid Hamburger only!
    - In both cases, for web and for desktop-native, we should support the baseline expected shortcuts for a text editor. We need to disambiguiate which ones already exist in the browser vs what we should implement for native-desktop app:
    - cmd + c for copy
    - cmd + v for paste
    - cmd + x for cut
    - cmd + a for select all
      - on macOS ctrl + a is the readline "beginning of line" and should remain that way
      - on Linux ctrl +a loses the readline binding and becomes "select all"
    - cmd + f for find in document
    - cmd + g for find next
    - cmd + shift + g for find prev

