# Wiki-links

Wiki-links are file and path links inside the workspace.

## Picker contract

Typing `[[` opens a file/path picker. The picker is for workspace paths, not
global content search. Use search when you want to search document contents.

## Relative paths

Links resolve against paths in the workspace. Moving the workspace folder does not
change the link targets because the links are not tied to an external
database.

## Backlinks

Chan derives backlinks from markdown content. When one markdown file links to
another, the graph and backlink views can show that connection.
