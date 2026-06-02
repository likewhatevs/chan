# chan-desktop redesign
This is today's chan-desktop:
![](./image.png#w=250)
And when you click the Open workspace:

![](./image-1.png#w=250)

And when you click Attach:
![](./image-2.png#w=250)
We are going to redesign the workflow and these UIs, as follows (sections / rows):
- [CHAN-ICON] Workspaces  .... [ New ] [ Sun/Moon ICON ]
- ON  ... WHERE 
- [ON/OFF button] [COMPUTER or HOME or NETWORK icon] [PATH or URL] ... [ OPEN + DRODOWN]

The main change is the removal of the SETTINGS button from here because from now on these will only exist inside chan's SPA not configurable from here at all. This will make the UI more uniform with URLs.

For URL use cases, we also need an indication of whether this is a INBOUND or OUTBOUND attached workspace (if we listen or if we connect to).

The [New] button will open a new window, where the user will be presented with 3 choices:
1. Local directory (a git repository, any directory)
2. A remote attached workspace outbound (icon/image of outbound connection) and inbound (icon/image of inbound connection)

I want this widget to resemble the Team Work one when you select the real estate and changing the tabs vs split panes changes the layout and options. In this case here, each of the 3 choices would show different layout.

One unrelated bugfix I want to report and found while doing this: if I hit cmd+p to bring up the team work dialog, I want to be able to press ESC to cancel; today it ignores ESC
