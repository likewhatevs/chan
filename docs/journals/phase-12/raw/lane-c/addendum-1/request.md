# Bugs
## Rich Prompt
- Launching the Rich Prompt's pre-flight works fine except that the agent that spawns in the Rich Prompt's terminal is only executed *after* the bootstrap... see the example here: _[screenshot removed: Rich Prompt bootstrap; the spawned agent runs only after the bootstrap]_
- When we stage the "Launch Agents" with the fswatcher for events, they live in the Drafts folder and our prompts refer to them via relative paths like from the beginning of the drive... this does not work because the Drafts folder lives outside the drive's root, but the agents should be able to find it via the MCP tool.

## Editor
- Contant "external edits to the file" while editing a draft, and after pasting an image.
- Our lists starting with '-' are still becoming '\*' lists; we do NOT want this behaviour.. if the user picks dash for list we use that, if the user picks asterisk we use that, if the user picks numbers we enumerate with 'N. ' and so on; no switching from '-' to asterisk automatically pls
## Terminal
* We still have terminal rendering bugs in v0.15.5 and we need to nail down the cause and fix it once and for all.



