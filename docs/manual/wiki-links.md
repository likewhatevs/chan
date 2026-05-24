# Wiki-links

Wiki-links are file and path links inside the drive.

## Picker contract

Typing `[[` opens a file/path picker. The picker is for drive paths, not
global content search. Use search when you want to search document contents.

## Relative paths

Links resolve against paths in the drive. Moving the drive folder does not
change the link targets because the links are not tied to an external
database.

## Backlinks

Chan derives backlinks from markdown content. When one markdown file links to
another, the graph and backlink views can show that connection.
