# Phase 11 round 1
Tracker of items below.

## Bugs

* The lists are being default to bullet, this is a regression; we want '-' when user types '- ' and '\*' and user types '\* ' and numbered lists when the user starts from '{number}. ' and these are all from the beginning of an empty line - regular list workflow like form all other editors; we are not inventing anything new here, we just had a regression and must fix it
* The desktop app for macOS is crashing on drag-out from File Browser; I want to remove the drag in and out, entirely; we will operate via the Upload and Download buttons for now; in the case of Download, we need a download indicator in the native-desktop app to mimic what users on browser would get by clicking download in there; the drag in and out will no longer work for any macOS or Linux
* Do an assessment of our binary sizes across all releases and make sure that we are not embedding any of the bge models in none of the binaries; we want all of our release binaries to be small and embed only what they really need: in case of chan and chan-desktop, this is pretty much the SPA; since now all other add-ons are optional (bge search, source-code pro font, etc) we should ship as little single binaries as we can
* New File or Dir menu not allowing me to enter a directory despite the caption to do so: ![](./attachments/image.png#w=250)
* When pasting an image into this document, it lands in the first row of the document; this is wrong
* Terminals still struggling with refresh, although less than before, and now when I click the terminal or resize they refresh correctly; it's just that while editing a file and running 2 other terminals, the idle one started acting up like this: ![](./attachments/image-1.png#w=250) (btw this image pasted correctly on the cursor)
* Bug while editing this very file while running a couple of terminals, and hitting the Too Many Open Files error, failing autosave, hanging the server until pkill
* native-desktop app auto-reloading during editing, and hanging on loading... hitting cmd+resolves it
* Status pill stuck on reindexing the doc we're in: ![](./image.png#w=250)
* When the user hit Cmd+N and we open a new draft, we do almost everything right except that the cursor is currently not placed in the document, meaning the user cannot start typing right away

## Features
- We must be able to drag & drop images across rows of the document, to change where their respective markdown code will land. We will allow the user to move across rows, and once settled on a row, they use the regular dropdown for images to choose between left/center/right on that row.
- Partial load
  - None of our operations should send large bulks of data, and we must be able to transfer information in smaller and resumable / retriable chunks
  - No synchronous operations within the boundaries of chan-server, our server is async and should spawn threads when needed
  - Drive bootstrap / pre-flight check review:
    - Which files and directories are ignored by us: the usual node_modules, venv, etc; we want consistency across doing this from chan-desktop and `chan serve` cmdline
    - Start by walking the filesystem and discovering the directory tree and number of files, their size
    - Having this as the spine of the drive, and used across File Browser, Graph, and also the background operations that we dispatch async/paced for building up the drive's graph
    - This is pretty much all we need to show the UI and be prepared to start
  - Once we start, we then kick off the chan-report and search index jobs
    - And we pace these especially in number of open files, so that we can prioritise editing files and using the terminal
    - We remain providing the progress of index and graph in the infographics widgets
- File Browser
  - Each instance of the File Browser must have its own associated metadata; expanding/collapsing dirs in one instance must not impact other instances
  - When we open the File Browser, we scan the root of the drive and we put an observer in there from chan-server so that if there are updates there, we can inform all of the UIs which are subscribed to the drive's root - pretty much all FB instances - these could be broadcast from server to all instances
  - However when users expand a directory from one or more instances, we should only then load the contents of that directory and put a watcher in the server; so that if other isntances expand the same, we can reuse the watcher (this is likely a pub/sub between ui/chan-server); when they collapse, they unsubscribe and on the last instance chan-server tears down the watcher; we need end-to-end and hardening tests for this:
    - Sub 1, create watcher; sub 2, reuse watcher; unsub 1 (original creator), keep watcher; unsub 2, tear down the watcher
    - These watchers must be per directory, starting from the drive, and only watch the first degree - the immediate files and directories in them
    - Only when the user expands to see a sub-directory we'd then create another watcher
    - See below how the Graph will reuse this same pub/sub api and mechanism to plot their depth, similarly to expand/collapse of the File Browser
- Graph
  - Similar to File Browser, we will load the graph nodes gradually
  - When the user requests to plot the whole drive, we will show the drive and its first-degree nodes and edges
    - Similarly to File Browser, we must put a watcher on the directory selected so that if there are changes in the filesystem we can redraw the graph
    - We will use the same mechanism as File Browser so that we can reuse the pub/sub APIs between UI and backend
    - Only when the user increases de *depth* slider, we then load the second-degree and so forth
      - When the user decreases the depth, we remove the nodes and stop watching for fs updates
    - What this means is depth = 2 means drive -> first layer of folders -> second layer of folders
    - This is similarly to File Browser's expand and collapse feature
    - All of these folders also show their files
  - We are going to change the colour of the edges to match the colour of the objects
    - All directory->directory (starting from drive root onwards) and directory->file remain grey
    - Other edges will match document type, e.g. markdown has orange edges, hashtag has green edges
      - We will respect the configuration of colours as done in the Grab's settings in the back of the Graph tab

