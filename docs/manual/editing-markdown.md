# Editing Markdown

Chan edits markdown files as plain text with a WYSIWYG surface. The saved
file remains normal markdown.

## Tabs and panes

Open several files in tabs, split the workspace into panes, and move between
files without changing the workspace layout on disk.

## Drafts

Drafts are temporary markdown files managed by Chan. Save a draft when you
want it to become workspace content. Discard it when it should not be kept.

## External edits

If another tool changes a file in the workspace, Chan picks up the change through
the workspace watcher and refreshes its index.

## Dates

Chan recognizes the same date shortcuts as Google Docs. Type `@today` to drop
today's date inline and keep typing. Type `@date` to insert today's date and
open a small calendar, so you can pick a different day or switch the date
format without selecting anything first. The date is written in your workspace
date format (set in preferences) and renders as a pill, but the saved file
holds plain text, so the date stays portable and reads the same in other
editors.

## Diagrams

A fenced code block tagged `mermaid` renders as a diagram. With the cursor
inside the block you edit the Mermaid source as an ordinary code block; move
the cursor out of a complete (closed) block and the diagram renders in its
place, and move back in to edit it again. A block whose source has an error
shows Mermaid's own error message instead of the chart. The diagram renderer
loads only the first time a diagram is shown, so documents without diagrams
carry no extra cost.
