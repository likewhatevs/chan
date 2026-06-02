# Round-1 part-c: P2 + DT1/DT2 design (@@LaneC)

Status: DESIGN. Build lands round-2 (lane-c.md). P1 (cs pre-flight) landed
separately this round; see `event-lane-c.md`.

Scope: the desktop launcher and its onboarding modal. Three coupled pieces:

- **P2** moves the desktop onboarding modal into the SPA, OPEN-THEN-CONFIGURE.
- **DT1** redesigns the launcher list (header, columns, row, gear removal,
  inbound/outbound indicator).
- **DT2** replaces the two header buttons with one `New` that opens a window
  with three creation choices, laid out like Team Work's real-estate selector.

These are sequenced P2 -> DT1 -> DT2 because DT1 removes the per-row settings
gear, and that is only safe once P2 has moved the per-workspace settings into
the SPA (otherwise enabling Semantic / Reports has no home).

Source: `desktop-redesign-draft/draft.md` + `image.png` / `-1` / `-2`.

--------------------------------------------------------------------------------

## Current state (anchors)

Local-workspace add flow (`desktop/src/main.js`):

```
pickAndAdd()                  402  native folder picker
  -> showPreflightDialog()    440  modal: summary + Semantic/Reports toggles
                              586    invoke compute_workspace_preflight(path)
                              600    renderPreflightReport(): file/size/scm rows
  -> invoke add_workspace     418  {path, features:{bge,reports}}
```

Per-row settings gear (`desktop/src/main.js`):

```
gear toggles a panel    ~980  loadFeaturesInto() -> get_workspace_features  1023
                              on change -> set_workspace_features           1007
```

Desktop backend (`desktop/src-tauri/src/main.rs`):

```
compute_workspace_preflight  708  walk + SCM + already_registered -> report
set_workspace_features       743  write {bge,reports} in-process
get_workspace_features            read the persisted pair
```

Server side already shared (phase-14): readiness overlay
`web/src/components/PreflightOverlay.svelte` + `GET /api/preflight`
(+ `/api/preflight/decision`). SPA Settings can already toggle Semantic /
Reports via the existing `/api/semantic/*` and `/api/reports/*` routes (desktop
CLAUDE.md: "Both layers can be enabled later from the workspace row or
Settings.").

Attach flows (`image-2.png`): `Open by URL` (outbound,
`attachOutboundUrl()` 1367) and `Receive a remote workspace` (inbound listen,
`Start listening`). Both live behind the `Attach` header button today.

--------------------------------------------------------------------------------

## P2: move onboarding into the SPA (open-then-configure)

Principle shift. Today configuration happens BEFORE the workspace is added:
the modal collects `{bge,reports}` and `add_workspace` boots with them. P2
inverts this: the server OPENS the workspace first (BM25-only default), the SPA
shows the workspace summary + the optional-layer toggles on first load, and the
SPA reconfigures Semantic / Reports through routes that already exist. One flow
for `chan serve` and desktop, matching the readiness overlay and the new cs
card that are already server/SPA-driven.

### What moves where

```
piece                     today (desktop)          P2 (shared SPA)
------------------------- ------------------------ --------------------------
workspace summary rows    compute_workspace_pre-   server-derived after open
(files, md, size, media,  flight (Tauri walk)      (indexer/report already
 scm, already_registered)                          walked it); SPA reads it
Semantic/Reports toggles  showPreflightDialog +    SPA onboarding surface ->
                          per-row gear panel       /api/semantic|reports/*
add_workspace features    {path, features}         {path} only; boots default
gear panel on each row    main.js ~980             removed (DT1)
```

### Server work

The summary needs a home on chan-server so the SPA can render it post-open
without a desktop-only Tauri walk. Two options:

1. Extend the pre-flight snapshot with an optional `summary` block once
   `phase == ready` (file counts from `index_stats`, language/SLOC roll-up
   from report state when Reports is on, SCM from a cheap `.git`/`.hg`/`.svn`
   probe). Reuses the existing poll; no new endpoint.
2. A dedicated `GET /api/workspace/summary`. Cleaner separation, one more
   route. Lower coupling to the locked readiness gate.

Recommendation: option 1 for the counts that are free from `index_stats`, so
the onboarding card needs no extra round-trip. SCM is a one-line probe added
server-side. This keeps the onboarding surface a pure consumer of `/api/
preflight` plus the existing `/api/semantic/*` + `/api/reports/*` writers.

### SPA onboarding surface

A first-load card (sibling of the readiness overlay; @@LaneC owns
`PreflightOverlay.svelte`). It is NON-LOCKING, like the cs card: the workspace
is already open and usable, this just surfaces "here is what we found + enable
optional layers." Shows once per workspace (persisted dismissal, same pattern
as the cs card's localStorage key, but keyed per workspace metadata-key so a
second new workspace still onboards).

Copy is the load-bearing explanatory text currently in `showPreflightDialog`
(baseline graph+BM25 is mandatory; Semantic and Reports are optional layers
that default off and drop their per-workspace data when disabled). Reuse it
verbatim; it was written for exactly this decision.

### Desktop work

- `pickAndAdd()` drops `showPreflightDialog`; calls `add_workspace {path}` and
  opens the window. The native folder picker (`image-1`) stays.
- Remove `compute_workspace_preflight`, `set_workspace_features`,
  `get_workspace_features` Tauri commands and the modal/gear DOM. `add_workspace`
  loses its `features` arg (pre-release, no back-compat shim).

### Open question (for @@Lead -> @@Host)

The SPA Settings panel ALSO toggles Semantic/Reports. Is the onboarding card a
thin "first-run nudge" that points at Settings, or a full inline toggle pair?
Leaning thin nudge (one less duplicate surface), but this is a product call.

--------------------------------------------------------------------------------

## DT1: launcher list redesign

```
header today : [icon] Workspaces ...  [Open workspace] [Attach] [theme]
header DT1   : [icon] Workspaces ...  [New]            [theme]

columns today: ON | PATH
columns DT1  : ON | WHERE          (WHERE = local path OR remote URL)

row today    : [on/off] [computer|home icon] /path ... [gear] [Open v]
row DT1      : [on/off] [computer|home|network icon] [path|URL] ... [Open v]
               (no gear; + INBOUND/OUTBOUND indicator on remote rows)
```

- `New` replaces `Open workspace` + `Attach` (both fold into DT2's picker).
- `WHERE` column renamed because a row is now a path OR a URL uniformly.
- Per-row gear removed (settings live in the SPA after P2). This is the half of
  P2 <-> DT1 that must not ship before P2.
- Remote/URL rows gain an inbound-vs-outbound indicator: OUTBOUND = this machine
  dials a remote `chan serve` (http2 or `--tunnel-url`); INBOUND = this machine
  listens for an incoming tunnel reverse-forward (today's "Receive a remote
  workspace"). Icon set: `computer` (local path), `home` (local under $HOME),
  `network` + direction glyph (remote).

Open question: the draft shows a `URL` badge on remote rows (`image.png`,
`image-2.png`). Keep the badge AND add a direction glyph, or replace the badge
with a directional network icon? Leaning directional icon (one signal, not
two). For @@Host.

--------------------------------------------------------------------------------

## DT2: `New` window, three choices, Team-Work-style layout swap

`New` opens a new window (its own `w=<label>` per the desktop window model)
presenting three creation modes whose selection swaps the form below, the way
Team Work's real-estate selector swaps layout/options on tab-vs-split:

```
[ Local directory ] [ Outbound remote ] [ Inbound remote ]
-----------------------------------------------------------
Local directory : folder picker (image-1) + "Open"
Outbound remote : URL + Name [+ tunnel-token?]  -> attachOutboundUrl()
Inbound remote  : Port + Label + Workspace       -> Start listening
                  (today's "Receive a remote workspace", image-2)
```

The three modes already exist as code paths (`pickAndAdd`,
`attachOutboundUrl`, the inbound listen). DT2 is mostly a re-host: one window,
a three-way selector, each mode rendering its existing form. Reference the Team
Work selector component for the swap mechanics so the interaction matches.

Open question: the draft says "resemble the Team Work one." Confirm whether DT2
should literally reuse the Team Work selector component/styles, or just match
its interaction. Leaning shared styles, separate instance. For @@Host.

--------------------------------------------------------------------------------

## Cross-lane items (route via @@Lead)

- **Cmd+P ESC bug** (draft.md, "one unrelated bugfix"): the Team Work dialog
  ignores ESC. That dialog is the SPA `TeamDialog.svelte`, owned by @@LaneD
  (Frontend/UX + Team Work GUI), not the desktop launcher. Hand to @@LaneD.
- **F6 theme icon** overlaps @@LaneD's `web-marketing` theme-toggle work; the
  DT1 header theme icon is the desktop launcher's own, separate file. No
  collision expected, but confirm the icon component source if shared.
- **New SPA mount**: P2's onboarding card mounts in the SPA. Per lane-c.md,
  poke @@Lead before adding an `App.svelte` mount so it does not collide with
  @@LaneD's frontend. (The cs card reused `PreflightOverlay.svelte`'s existing
  mount; the onboarding card can do the same to avoid a new mount point.)

## Sequencing

1. P2 server summary + SPA onboarding card (additive; no desktop removal yet).
2. P2 desktop: switch `pickAndAdd` to add-then-open; keep gear temporarily.
3. DT1: remove gear + features IPC, header/columns/row redesign, indicator.
4. DT2: `New` window with the three-way selector.

Each step gates green on its own; DT1's gear removal merges only after P2's
SPA settings path is proven.

================================================================================

# DT1/DT2 refinement: anchored round-2 implementation plan (@@LaneC, r1)

Added after P2 merged (sha 64e4fc80), while the SPA onboarding card waits on
the @@Host card-shape call. Grounds the DT1/DT2 sections above in the real
launcher code so round-2 is a re-host, not a redesign-from-scratch. The
launcher window is PLAIN JS (`desktop/src/main.js` + `index.html`), not the
Svelte SPA; that is loaded only inside a workspace window.

## Anchors (current launcher)

```
concern                  location                              role
------------------------ ------------------------------------- ----------------
header buttons           index.html:20-21                      Open workspace +
                                                                Attach
theme toggle (sun/moon)  index.html:24-34                      already present
list render + row model  main.js render() :673                 builds the table
path classifier          main.js renderPath() :125             home vs computer
inbound row              main.js :692  d.kind==='tunneled'     incoming tunnel
outbound row             main.js :720  d.kind==='outbound'     dialed URL
  outbound "url" badge   main.js :726  span.tag-outbound       to be replaced
open dropdown            main.js renderOpenSplit() :845        [Open v]
per-row gear (REMOVE)    main.js renderFeaturesToggle :780     +renderFeatures-
                                       renderFeaturesPanel :802  Panel + wiring
open commands            main.js :881/897/935                  tunneled/outbound/
                                                                local open
attach panel (outbound)  main.js renderOutboundAttachForm:1279 URL + Label form
inbound listen           main.js tunnel_start :1316            port/label/ws form
new launcher window      src-tauri/main.rs open_new_launcher_  WebviewWindow +
                         window() :1947                        next free label
```

## Key finding: no new data model for the indicator

The inbound/outbound/local distinction DT1 needs ALREADY exists as `d.kind`:
`tunneled` = INBOUND (this machine listens for an incoming `chan serve
--tunnel`), `outbound` = OUTBOUND (this machine dials a remote `chan serve`),
absent/local = a registered local path. So DT1's indicator is a pure RENDER
change in `render()`; no backend or model work. Resolves the "how do we know
inbound vs outbound" question outright.

## DT1 (launcher list) - concrete edits

1. Header (index.html:19-35): replace `#open-workspace` + `#tunnel-btn` with a
   single `#new-workspace` "New" button that calls DT2's window opener. Keep
   `#theme-toggle` (the sun/moon SVGs at 24-34 are the launcher's OWN, not the
   F6 web-marketing toggle). `#auth-btn` stays hidden. `#tunnel-panel-slot`
   (the in-header Attach panel) is retired; attach moves into the New window.
2. Columns: in `render()`, the `PATH` header becomes `WHERE` (a row is now a
   local path OR a remote URL uniformly).
3. Row icon, driven by `d.kind` + `renderPath()`:
   - local under $HOME -> home icon; local elsewhere -> computer icon.
   - `tunneled` -> network icon + INBOUND glyph (arrow into a box).
   - `outbound` -> network icon + OUTBOUND glyph (arrow out of a box).
   Replace the `span.tag-outbound` "url" badge (:726) with the directional
   icon (resolves the badge-vs-icon question: one directional signal, not a
   badge plus a glyph).
4. Gear removal: delete `renderFeaturesToggle` (:780), `renderFeaturesPanel`
   (:802), their wiring in `render()`, and the `get_workspace_features` /
   `set_workspace_features` Tauri commands they drive. Pre-release, no shim.
   This is the DT1<->P2 coupling: it merges only after the SPA settings path is
   proven. That path already exists (the SPA Settings panel toggles Semantic /
   Reports today), so the dependency is satisfied as soon as P2's onboarding
   card ships, or arguably now via Settings. Confirm with @@Lead before pulling
   the gear so onboarding never has a gap.

## DT2 (New window, 3 choices) - concrete edits

- The window: reuse the existing `open_new_launcher_window` path
  (main.rs:1947) which already spawns a fresh `index.html` window with its own
  `main-N` label. Add a mode signal (recommend a `?new=1` query param) so
  `main.js boot()` renders the 3-choice picker instead of the workspace list.
  No new window machinery; one branch in boot.
- The selector (plain-JS, Team-Work-interaction match - the Svelte component
  cannot be literally reused in the plain-JS launcher, so match interaction +
  visual, not code):

```
  [ Local directory ] [ Outbound remote ] [ Inbound remote ]
  ----------------------------------------------------------------
  Local directory : native folder picker (pickAndAdd :402, minus the
                    showPreflightDialog modal P2 removed) -> add_workspace{path}
  Outbound remote : URL + Name  (renderOutboundAttachForm :1279)
                                -> attachOutboundUrl()
  Inbound remote  : Port + Label + Workspace
                                -> tunnel_start{preferredPort,label,workspace}
```

  Each pane is an EXISTING handler relocated. DT2 carries almost no new logic;
  the risk is layout/wiring, not behavior.

## Header + list mockup (DT1 target)

```
  (O) Workspaces  what are we working on today?          [ New ]  [ (moon) ]
  -----------------------------------------------------------------------------
   ON   WHERE
  [ () ] [comp]  /private/tmp/chan-test-lanea                          [Open v]
  [ (O)] [home]  ~/dev/github.com/fiorix/chan                          [Open v]
  [ () ] [home]  ~/Documents/Chan                                      [Open v]
  [ (O)] [net <-] prod-setup  (inbound)                                [Open v]
  [ () ] [net ->] team.example.com/notes  (outbound)                   [Open v]
```

## Open questions - now resolved (were for @@Host)

- Badge vs directional icon -> DIRECTIONAL ICON (net + in/out glyph), drop the
  `url` badge. One signal.
- Literal Team Work component reuse vs interaction match -> INTERACTION MATCH in
  plain JS. The launcher is not Svelte, so a component port does not apply;
  share the visual language only.

These were the two DT open questions; both now have a recommended answer, so DT
does not need a @@Host survey unless @@Host wants to overrule. The only
remaining @@Host call is P2's CARD SHAPE (thin nudge vs inline toggles), which
gates the SPA card slice, NOT DT.

## Verify (round-2)

All desktop changes are WKWebView-only: gate green + flag empirically-unverified
for @@Host to confirm on a real chan-desktop build (per lane-c.md).
