# Chan pre-release phase 3 request

Source: Alex's "Bug fixes / feature requests" screenshots supplied on
2026-05-16, with text version supplied afterward.

## Bug fixes / feature requests

- [ ] Big change: Assistant -> Agent everywhere, code and surfaces.
- [ ] Clicking on the status bar when an event is shown should pop up the
  overlay related to that event.
  - [ ] Examples: one of the assistant/agent chats, an index event which would land in
    the index status page, etc.
- [ ] Assistant CLI does not seem to be resuming properly.
  - [ ] Current symptom: CODEx banner appears on CLAUDE.
  - [ ] Screenshot showed the CLI banner area rendering "CODEX CLI" while the
    right-side assistant selector was set to "Claude CLI".
- [ ] We should have the exact state of each screen reflected in the URL so
  that we can reload the page.
- [ ] The banners we have today are a copy of CLAUDE's banner; we need actual
  banners representing the assistants we support.
- [ ] Cmd+F on documents should move the cursor to the beginning of the word
  match when the user presses Enter.
- [ ] The File Browser overlay should support Cmd+F for finding filenames and
  folders from what is expanded and visible in the file browser.
- [ ] File Browser right-click context menu should open next to the clicked
  file/folder label, not far away toward the lower-right of the click point.
- [ ] The Agent overlay should support Cmd+F to search the chat history of the
  current session.
- [ ] When selected editor text is sent to Agent with Cmd+I, the quote should
  be inserted and the prompt caret should land on the first editable line after
  the quote, not at the beginning of the quote.
- [ ] Settings / Layout should change from `[tight] [standard]` to
  `[standard] [compact]`.
  - [ ] Standard should be the default.
  - [ ] Compact should be adjusted to land between the old tight and standard
    layouts.
- [ ] When creating a new file, support tab-complete.
- [ ] Path inputs for new file, new folder, rename/move, and similar flows
  should support normal Tab completion.
  - [ ] Tab completes directory paths and preserves/adds the trailing `/`.
  - [ ] In new-file flows, completing a directory should suggest a `.md`
    filename that can be Tab-completed, then Enter confirms.
  - [ ] Enter confirms; it should not be the only way to complete suggestions.
- [ ] Consistent color coding across different resource types that have special
  treatment in the inspector, file browser, search, agent, and graph.
  - [ ] Markdown documents: orange.
  - [ ] Contacts:
    - [ ] Markdown with frontmatter `chan.kind: contact`: yellow.
    - [ ] `@@contact`: yellow.
  - [ ] Media: all images, videos, and audio files: purple.
  - [ ] Binary: zip, tarballs, executables, and the rest: blue, matching the
    FILE blue from the inspector.
  - [ ] Tag: `#hashtags` only in Markdown documents: green.
  - [ ] Folder: grey.
- [ ] Consistency across graph modes.
  - [ ] When graphing Markdown and links, include a filter to show/hide the
    parent directory of the Markdown files as well as their path to the drive
    root as depth increases.
  - [ ] When graphing Folders, include a filter to show/hide links across
    Markdown documents and their paths.
  - [ ] Consider whether this means basically having all filters for all graph
    modes: language, folder, symlink, hardlink, link, tag, contact, media.
  - [ ] Clarify if there is a fundamental issue here. Hiding folders as nodes
    may resolve the whole-drive view much better.
  - [ ] When the graph SCOPE is a `.md` file, include its parent folder in the
    SCOPE options as well.
  - [ ] If multiple `.md` files are in SCOPE because of pane structure, include
    the first common ancestor folder in the SCOPE options as well.
- [ ] Editor bug on multi-level indent causing de-indent of the next line of a
  long sentence.
  - [ ] Screenshot showed a nested list where a long wrapped line/image context
    caused the following line to lose its expected indentation level.
- [ ] Switch the file and folder icons to GitHub's style.
  - [ ] Also use the folder icon in the file browser.
  - [ ] Screenshot showed GitHub-style file browser icons: chevrons, folder
    glyphs, and document glyphs in a compact dark file tree.
- [ ] On lines like "Let's switch", the cursor becomes as high as the image
  from the previous line.
  - [ ] Screenshots showed a text-line cursor/selection guide stretching to the
    height of an image embedded on the preceding list line.
- [ ] Fix image behavior in document lists:
  - [ ] The horizontal lines are very helpful, but they also break on images.
  - [ ] Screenshot showed list guide lines crossing or misaligning around
    embedded images instead of staying visually tied to text/list structure.
  - [ ] Auto-hide horizontal guide lines after 1.5s when the cursor is not
    within the list.
- [ ] Text editor selection can get stuck around image/list blocks and cannot
  be cleared normally.
  - [ ] Screenshot showed large blue selection rectangles spanning image/list
    rows while the caret had moved elsewhere.
  - [ ] After moving the cursor to another line, the old selection partially
    remained and split into stale blue blocks around the embedded images.
- [ ] The window behind all tabs, where we currently print keyboard shortcuts
  and the Chan logo, will become our primary dashboard from now on.
