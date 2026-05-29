# Phase 14 round 3

Round 3 has two themes: harden the hot paths end to end so the frontend
never blocks on large data, and move the new-workspace pre-flight from
chan-desktop into chan-server so the experience is identical across
local and remote workspaces. Both preserve today's outcomes; this is
hardening and relocation, not new surfaces.

## Theme 1: paced, streamed hot paths (server <-> indices <-> frontend)

Today the graph payloads are sent whole. On a large workspace (e.g.
`/tmp/linux`, the Linux kernel source) the dashboard's indexing graph
and the graph tab pull enough data at once to hog the API bus and
freeze the UI while it plots.

Target: **the frontend stays responsive at all times.** The backend
transmits small amounts of data that the frontend uses to gradually
construct the graph, never one large blocking payload. Even when the
user cranks the depth slider on a large repo, the system relentlessly
paces and gradually loads the data without disrupting the editor, file
browser, terminal, or other open graphs.

Surfaces and seams in play (harden each leg):

- On-disk indices: `chan-workspace` indexer / graph / search.
- chan-server graph endpoints: `/api/graph` (`api_graph`),
  `/api/fs-graph` (`api_fs_graph`, via `build_fs_graph` /
  `FsGraphScope`), `/api/graph/languages`, and the indexing-state
  surfaces (`/api/index/status`, `/api/indexing/state`).
- The `/ws` event bus (`bus.rs`), which already carries index /
  graph-rebuild / embed-batch progress.
- The SPA graph rendering (the graph tab and the dashboard indexing
  graph).

Direction (intent, not a prescription):

- Deliver graph data incrementally - chunked / cursor-paged / streamed
  (over the `/ws` bus or paged API), small frames the SPA appends as
  they arrive, with backpressure so the producer never outruns the UI.
- The depth slider drives incremental fetches (expand by batches, as
  the round-1 graph design intends), not a single large refetch; raising
  it on a huge repo keeps paging gradually rather than stalling.
- Cap per-frame work on both ends so a large workspace degrades into
  "slowly fills in" rather than "freezes". The other surfaces (editor,
  file browser, terminal, additional graphs) must remain interactive
  throughout.

Graph interaction model (revised this round):

- Single click = select + open the inspector (unchanged). The "graph
  from here" rescope lives in the inspector (and the existing
  right-click / chord).
- Double click on a DIRECTORY node = expand / collapse that directory
  in place, WITHOUT reloading the graph. Expanding reveals the
  directory's next degree (fetched incrementally per theme 1 if not
  already loaded); collapsing hides its subtree. This is the per-node
  1-by-1 control.
- The old double-click "graph from here" is dropped: rescope is already
  reachable from the inspector, so the gesture is freed for
  expand/collapse.
- The depth slider stays authoritative. Its `find -d N` scope defines
  what is expanded vs collapsed; moving the slider re-establishes the
  expanded set to depth N and overrides individual expand/collapse
  toggles.
- The expanded / collapsed directory set persists across a window
  reload, the same way the File Browser does (reuse the
  `treeExpanded` + sessionStorage persistence pattern).
- Background tap clears the selection.

Correctness bar: graph *contents* are unchanged and delivery is paced;
the interaction changes above (double-click expand/collapse, dropped
rescope-on-double-click, persistence) are intentional and additive.

## Theme 2: new-workspace pre-flight moves to chan-server

The new-workspace pre-flight currently lives in chan-desktop
(`default_workspace.rs`, `serve.rs`). Move it onto chan-server's first
boot so the checks run in the UI, where the user makes their decisions
on the spot - the same flow for local and remote workspaces (inbound
and outbound).

In practice:

- From chan-desktop, "add a workspace" (the desktop's current Open
  action) immediately starts chan-server, which runs the pre-flight on
  the spot before the UI is usable.
- Present the pre-flight on the OverlayShell, **locked until
  completion**: hide/remove the close button, do not accept ESC to
  dismiss, and guide the user toward booting up the workspace.
- One consistent pre-flight experience regardless of where the
  workspace runs; chan-desktop stops owning the flow and just launches
  chan serve.

## Non-goals

- No new product surfaces; the graphs and the workspace flow do the
  same things, just responsively and from chan-server.
- Not a rewrite of the indexer or the graph model; reuse the existing
  `build_fs_graph` / scope model and the `/ws` bus.

## Definition of done

- Opening the graph tab or the dashboard indexing graph on a large
  workspace (`/tmp/linux`) never freezes the UI; the editor, file
  browser, and terminal stay interactive while the graph fills in.
- Cranking the depth slider on a large repo paces the load gradually
  with no stall and no disruption to other surfaces.
- API/WS payloads on the hot paths are bounded and incremental, not
  single large blobs.
- Double-click on a directory node expands/collapses it in place with
  no graph reload; the depth slider overrides individual toggles; the
  expanded/collapsed set persists across a window reload (File Browser
  parity). Double-click no longer rescopes; "graph from here" remains
  in the inspector.
- New-workspace pre-flight runs in chan-server on first boot, presented
  on a locked OverlayShell (no close button, ESC ignored), identical
  for local and remote workspaces; chan-desktop only launches the
  server.
- Graph results are unchanged from today; gates stay green.
