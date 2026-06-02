# Requirements
- Carry over work from previous phase (bullet list glyph size, pulsing spine dashboard search, etc)
-   Review all context menus
  -  @@Architect: Please provide a comprehensive list of all hamburger and context menus of our widgets
    -  I want to review them to adjust when to show what, e.g. if there is text selected in the text editor or the terminal, the right-click (context) menu shoudl show options like Copy/Cut/Paste/ and contextualised options, not the whole menu like right-click on the tab's name
    -  These more specific and contextualised options should / could be removed from the tab's right-click menu, if they don't belong
    -  Terminal and editor could provide more seamless experience to users regarding rendering links: e.g. markdown previews (although terminal would be read-only), external link previews (new feature, small bubble with "open" (in external browser, or new tab if on web frontend))

## Feature
### Workspace pre-flight and boot check
Add check for the `cs` symlink existing in $PATH during workspace boot and pre-flight in chan-server (landing on the SPA for both chan and desktop native), then offer to create/fix if missing and acept/continue if we cannot do it - do not block the user.

### The `cs pane` command
We are going to add a new `cs pane` command so that we can get information about windows and panes, the layout, which pane is selected. We should be able to set the pane focus and run split left and bottom commands, as well as close tabs and close all tabs, and close the pane. Tabs with drafts and terminals will immadiately return partial failure if these tabs exist because they block the completion of closing all tabs or the pane; we should provide a --force option to force their killing.
### The `cs terminal scrollback` command
Add ‘cs terminal scrollback’  to read the scrollback of a given tab; no support for group here.

## Fixes

### Terminal
I want to move the whole "Broadcast input on/off" menu section to be at the top, after the Group row.

### Tunnel
The new website and readme sell the idea of the tunnel as if the online service was the feature itself. It's totally not that... the software that I currently run in https://id.chan.app and https://workspace.chan.app is part of the codebase here, of chan's monorepo. It's in the `gateway/` directory and is the server-side counterpart of `chan` for using the `--tunnel-url` and `--tunnel-token` settings.
At this point, the online service experimental and is not meant to be a feature of chan, it is meant to give users their own portable "Google Drive" equivalent service for them to run in their own infra.
The tunnel functionality remains a CORE capability of chan, though: chan-desktop can attach incoming tunnels from remote runs of `chan serve --tunnel=url` as well as connect to a remote machine running `chan serve` through regular http2.
So the point is that tunnel is core to chan, the online service is not the big selling point here. And by default it is disabled for everyone (users are not enrolled). We should mention the command admin tools for managing the online service, configuring oauth, enrolling users to be able to authenticate with oauth, enabling users to share their workspaces. Explain how to setup the online service with DNS configuration with wildcard and Let's Encrypt - we can share the way we setup based in my private chan-prod-setup repo.

### Graph
I noticed when we plot the graph from the lang=x, we show the backlinks to files, but then we are not plotting the directory spine all the way back to the workspace's root.. we must do that; there should be no files in the graph that have no edges to a directory.
If we are doing the same thing for hashtags, and mentions, we should fix those cases too. A great example of the graph's spine is the dashboard / search graph - what i want for the graph tab is that kind of spine, with the files around the directories, and at the top the languages and hashtags and contacts; this is because the spine starts from the bottom up, so the languages would be somewhere around them on the sides or up but im not too strong about the position so long we can get the spine.
One other very important aspect of building the graph, especially on large workspaces (e.g. a shallow clone of the linux kernel soure code) should gradually load the nodes and their edges, instead of loading a large amount of data and plot at once - which is what seems to be the case today. We are fine to wait 10..20..30 seconds(maybe?) to plot a large workspace like the linux kernel; and we have to be smart about how to load it: start from the spine, load the specific directories we need on demand, and keep going from there... it's ok to enqueue the load and gradually receive the nodes so long we are capable of adding/removing nodes to/from the graph without having to reload it.

### Inspector
We need separator between sections: buttons, file size etc, code, contacts, each section with a separator.
