# Phase-16 round-1 WAVE-3 dispatch (@@Host decisions applied)

Source: round-1-host-decisions.md (filled by @@Host). This doc is durable so
it survives a team-server rebuild/respawn (item 5). Lanes: read YOUR section,
build on committed main (git-based; independent of the running server), commit
pathspec, post sha + poke @@Lead. @@Lead isolate-gates as before.

Status at wave-3 open: 16 slices merged green. gate.sh harness warm.

--------------------------------------------------------------------------------

## @@LaneA - window-id fix (A) + C3b + S1 (channel now GREENLIT)

1. **Window-id fix [PRIORITY]** (item 5). @@Host confirmed his REGULAR terminal
   has `CHAN_WINDOW_ID` (workspace-1abd876af1451ce6-0); only AGENT sessions
   (spawn_team -> window_id:None, SPA attach never backfills) lack it. FIX so
   agent sessions carry CHAN_WINDOW_ID bound to the window that displays them,
   exactly like regular terminals already do. This unblocks `cs pane`/`cs open`/
   `cs survey` from agent contexts (incl @@Lead self-hosting). Your draft fix A.
   Keep your `cs pane --tab-name` selector (B) too if cheap - useful for
   scripted/headless use where no window displays the tab.
2. **C3b** - exec pane ops on the C3 channel: focus / split left|bottom /
   resize / close tab|all|pane with `--force` for dirty/live tabs (PARTIAL
   FAILURE in `blocked`). Additive on the merged C3a channel.
3. **S1** - server->SPA attach-window-command so `cs terminal team new` surfaces
   in the running SPA; reuses the channel; pairs with the window-id fix.
Sequence: window-id fix -> C3b -> S1. Coordinate client.ts with @@LaneC (they
add a P2-summary type there; you add exec/windowReply) - poke before committing
client.ts, keep pathspecs tight.

## @@LaneC - P2 onboarding card = NUDGE (item 2)

Build the first-load onboarding card as a thin first-run NUDGE that points at
Settings (NOT inline toggles). Reuse the PreflightOverlay mount (no new
App.svelte mount, per your design). Reads the summary block your P2-server
already merged onto /api/preflight (64e4fc80). Non-locking, persisted dismissal
keyed per-workspace. Reuse the showPreflightDialog copy. Files: PreflightOverlay
.svelte (+ client.ts/types.ts for the summary type - coordinate client.ts with
@@LaneA). After this, DT1/DT2 build is still round-2 unless a slice is ready.

## @@LaneD - F4 steps 4-5 (items 3 + 4)

#3 = **A**: external-link "open" as a context-MENU item only (body-context shows
"Open link" + "Copy link" when externalUrlAtPos resolves) - NO hover bubble.
#4 = your rec: markdown preview = INTERNAL [[wiki]]/relative-md link only,
MENU-ITEM trigger (no hover, no external-URL fetch); editor interactive,
terminal read-only render. Files: external_links.ts, new link_preview.ts,
FileEditorTab/TerminalTab. Browser-smoke the link actions. These complete F4.

## @@LaneE - item 6 (CI, do FIRST, date-bound) then item 1 (the guide)

1. **Item 6 [date-bound 2026-06-16]**: bump the 2 remaining Node-20 actions -
   actions/deploy-pages@v4 -> v5 (pages.yml + release.yml; bump upload-pages-
   artifact@v3 -> v5 IN LOCKSTEP as the Pages-publish pair) and apple-actions/
   import-codesign-certs@v3 -> v7 (release-desktop.yml + release.yml SIGNING
   path, 4-major jump - read its v7 changelog for breaking input changes; this
   is signing-sensitive shared CI infra, task-authorized here, secret VALUES
   never appear). Verify majors via GitHub API like B1.
2. **Item 1 - prod-LIKE LOCAL gateway guide** (the reframed D1 deep-dive).
   @@Host's intent: a guide so users can stand up the FULL prod-like gateway
   stack LOCALLY - Mac via a lima-vm + sdme (the way @@Host runs prod), Linux
   via a local setup. EXPLAIN the choices (DNS provider, cert method dns-01 vs
   http-01) without prescribing; mirror @@Host's nginx vhost layout where it
   makes sense; mirror LITTLE of chan-prod-setup. NOT ngrok, NOT real-cloud
   production. Consolidate existing gateway/docs/{dev-setup,testing-on-linux-
   and-macos}.md + README ##Dev (you found these already cover the loopback
   dev case). SCOPE-FIRST: post a short outline + the sdme/lima specifics you
   need from @@Host to @@Lead BEFORE writing the full guide (it's his prod
   workflow; don't guess the sdme commands).

## @@LaneB - stand by

G1 merged; G2 is round-2. No wave-3 assignment. Stand by for a pickup.

--------------------------------------------------------------------------------

## Rebuild/restart (item 5 main) - @@Host-driven, see round-1-status.md

@@Host said "rebuild now + keep rebuilding each wave." The team server hosts
EVERY tab (incl @@Host's + @@Lead's), so restarting it respawns the whole team
(work is safe in git; agent context resets -> rebuild from this doc + round-1-
status.md + event-lane-*.md). @@Lead prepares build+reinstall+restart steps;
@@Host triggers the restart from a NATIVE terminal (outside the chan window) at
a chosen point. Lanes recover via this doc if respawned mid-task.

--------------------------------------------------------------------------------

## WAVE-3 FOLLOW-UP (@@Host approved): gateway default-port change

@@Host (02:4x) approved changing the gateway SERVICE default ports so out-of-box
macOS doesn't collide with Apple AirPlay Receiver (:7000, and :7001). Owner:
@@LaneE (gateway area). This is a gateway-CODE config change - @@Host-authorized,
state that inline in the commit for the classifier; secret VALUES never appear.

SCOPE (grounded):
- Bump the default bind ports to the 17000+ scheme the dev guide already uses:
  identity 7000->17000 (gateway/crates/identity/src/config.rs:53 env-fallback +
  :273 test default + any BASE_URL/IDENTITY_URL default), profile 7001->17001
  (profile/src/config.rs:33 + test default), workspace-proxy 7002->17002
  (workspace-proxy/src/config.rs:76 + :284) and tunnel 7100->17100 (:81 + :285).
- Fix cross-service URL defaults that hardcode the old ports (IDENTITY_URL,
  PROFILE_SERVICE_URL, CHAN_ADMIN_PROFILE_URL/WORKSPACE_URL, BASE_URL, OAuth
  homepage/callback) in the gateway crates + gateway/README.md.
- Update gateway/docs/dev-setup.md: defaults NOW avoid AirPlay, so the "dev
  runner offsets to 17000+ because defaults are 7000-range" explanation becomes
  "defaults already dodge the macOS AirPlay :7000 clash" - simplify the note;
  if scripts/dev/run.sh hardcoded the 17000 offset, it can drop to the defaults.
- AFTER: grep gateway/ for any remaining 7000/7001/7002/7100; none should be a
  live default.
VERIFY: `make gateway-build` (gateway is a SEPARATE workspace, linux amd64/arm64
only - if it won't build on macOS, verify via CI gateway-ci.yml or lima+sdme;
@@Lead's gate.sh does NOT include gateway-build, so call this out explicitly).
INDEPENDENT of the respawn (gateway != the chan team-server binary) - proceed
regardless of respawn timing. If respawned mid-task, the new @@LaneE resumes
this section. Scope-first if the cross-ref surface is bigger than it looks.

### TOPOLOGY DECISION (@@Host, supersedes the host-binaries default)

@@Host: "all services in their sdme containers, like we do in prod.. nothing on
the host." So the prod-like-local guide must run EVERY gateway service
(identity, profile, workspace-proxy) as its OWN sdme container inside Lima
(mirroring the chan-psql.sdme Postgres-container pattern the docs already show)
- NOTHING as a host cargo binary. This is a REWORK of the committed guide
(dfbf3c57 currently uses host-binaries) and is COUPLED with the port change
above (do both as one coherent gateway-guide-v2 pass).

SCOPE-FIRST (this needs @@Host's prod approach, since it mirrors prod): post to
@@Lead what you need to write the services-as-sdme-containers steps accurately -
e.g. how each service is containerized in prod (sdme container build per
service like chan-psql.sdme? systemd inside the container or a bare run? how
nginx fits - its own container or the VM host?). "Mirror LITTLE of chan-prod-
setup" still holds: show the PATTERN, don't copy every prod config. @@Lead
relays your questions to @@Host. Do NOT guess the prod container shape.

--------------------------------------------------------------------------------

## WAVE-3 FOLLOW-UP (@@Host): image-drag source-row indicator (@@LaneD)

@@Host: when editing a doc with an image, dragging the image UP/DOWN repositions
it to a different ROW of the markdown SOURCE; he wants to SEE the target source
row LIVE during the drag, so he knows exactly which line it lands on.
GROUNDING: the editor ALREADY has the image drag-to-move affordance - Wysiwyg
.svelte:1221 (writable image atom draggable="true" + data-dragging styling),
widgets/image.ts (image atom + drag handle), bubbles/image_drop.ts (drop). The
editor is CM6, so the target source line is computable from the drag Y:
posAtCoords -> doc.lineAt -> line number/content.
ASK: during the vertical image drag, render a LIVE indicator of the target
SOURCE ROW the image will land on (e.g. a drop-cursor at the target line + a
small "line N" / source-row badge, or highlight the target source line). Make
it track the pointer as it moves so @@Host sees the landing row before release.
DESIGN-FIRST (light): post a short proposal (the indicator UX - drop-cursor +
line badge? target-line highlight? show the line's text?) for @@Lead/@@Host
review, THEN implement. Browser-smoke the drag (Svelte/CM6 reactivity - static
gates miss it). Files: Wysiwyg.svelte, widgets/image.ts, bubbles/image_drop.ts
(all @@LaneD). Frontend/git-based, independent of the respawn.

--------------------------------------------------------------------------------

## WAVE-3 FOLLOW-UP (@@Host): mermaid code-block flip-to-render (@@LaneD)

@@Host: detect fenced code blocks with language `mermaid`, add a button to RENDER
them into the diagram. Button placement: SAME COLUMN as the code-block 'copy'
button but BOTTOM-RIGHT. Clicking applies a HORIZONTAL CSS FLIP to show the
rendered chart; same button flips back to source. Ref: https://mermaid-cjv.pages
.dev/ (a Svelte mermaid renderer; chan's frontend is Svelte). @@Host wants this
IMPLEMENTED + a test server to validate (he gave a pie-chart example).
DOABLE NOW. Sequence: AFTER the image-drag indicator (also @@LaneD).
GROUNDING + key choices (state them, light design, then implement - @@Host wants
impl not heavy design):
- Detect lang==`mermaid` on the fenced-code widget (the same info-string the
  copy button sits on); add the flip button bottom-right of that button column.
- The flip is a NEW in-content CSS flip (rotateY/backface on the code-block
  element) - NOT the pane-level flipHybrid (Cmd+,). Reuse its visual feel.
- Render via mermaid. DEP AUTHORIZED (@@Lead): add mermaid to web/, but DYNAMIC-
  IMPORT it (lazy-load only when a mermaid block is first flipped) so the initial
  bundle + binary stay lean - mermaid is large; this holds the single-binary line
  (build-time dep, no runtime daemon) while not bloating first paint. mermaid-cjv
  is an option but mermaid is the core dep; pick the lighter integration.
- Theme-match: feed chan's light/dark into the mermaid theme so diagrams match.
- Handle render errors (bad mermaid source) gracefully on the back face.
- WYSIWYG render path (where code blocks render); confirm source-mode behavior.
Files: @@LaneD editor (the code-block widget + a new mermaid render module +
web/package.json). Browser-smoke the flip + render (Chrome-smokeable). Post a
sha when gated; @@Lead then spins a test server seeded with @@Host's mermaid
example for him to validate.

--------------------------------------------------------------------------------

## WAVE-3 FOLLOW-UP (@@Host): image browser - prev/next in the viewer (@@LaneD)

@@Host: enhance the fullscreen image VIEW (the zoom button top-right of images).
Add PREV/NEXT buttons (left/right sides) to navigate a SET of images, where the
set is CONTEXT-DEPENDENT:
- opened from EDITING TEXT (editor zoom button on a doc image) -> the set = the
  OTHER images IN THE SAME DOCUMENT (navigate the doc's images in order).
- opened from the FILE BROWSER (View on an image) -> the set = the OTHER images
  in the SAME DIRECTORY (browse the dir's images).
GROUNDING: web/src/state/imageZoom.ts is the fullscreen viewer (openImageZoom
(src, fromPath); pdfViewer.ts is a mirror). Editor VIEW/zoom button: editor/
bubbles/image.ts + widgets/image.ts -> openImageZoom. File-browser View:
FileInfoBody.svelte "View/Zoom" for images (~:684). isImage() is in state/
fileTypes.ts. File browser: FileBrowserSurface.svelte / FileTree.svelte.
PLAN: extend openImageZoom to take an ORDERED image list + the current index
(or a navigator) instead of a lone src; add prev/next UI (left/right buttons +
arrow-key nav) to the modal; provide the list from each caller - editor =
the doc's image srcs in document order; file browser = the current directory's
images. DESIGN-FIRST (brief - a few lines): post the set-definition decisions
(dir = flat current dir only vs recursive; sort order; wrap-around at the ends
yes/no) + the openImageZoom API shape, for @@Lead review, THEN implement.
Browser-smoke both entry points. AFTER the mermaid rework. Files: imageZoom.ts
+ the editor image side + the file-browser side (all @@LaneD frontend).

--------------------------------------------------------------------------------

## WAVE-3 FOLLOW-UP (@@Host): P2 card -> actionable + reports-on-by-default (@@LaneC)

@@Host, two coupled changes to the P2 onboarding nudge (your 35aa69e8):
1. REPORTS ON BY DEFAULT: make chan-reports enabled by default for a workspace
   (today it defaults OFF, like Semantic). Find where the reports default lives
   (your preflight/reports-config area; /api/reports/* + the workspace-open
   default) and flip it on. Pre-release: new workspaces get reports on; don't
   add a migration for existing - note the behavior.
2. CARD = ACTUAL CLICKABLE OPTIONS, not a dashboard pointer: replace the
   "show dashboard" action with actual clickable OPTIONS in the card (the
   actionable layer enable/configure controls - e.g. enable Semantic search,
   the Reports control now that it's on, whatever optional layers apply), each
   wired to actually toggle/enable via the existing /api/semantic|reports/*.
   KEEP the "dismiss" action (per-workspace dismissal stays).
NOTE: this PIVOTS the card from the thin-nudge we shipped toward the actionable/
inline-options direction - that's @@Host's call, fine. DESIGN-FIRST (brief):
post what the clickable options ARE + how each wires (which route) + where the
reports-default change is, for @@Lead/@@Host review BEFORE implementing. Don't
restyle beyond what the options need. Files: PreflightOverlay.svelte + the
reports-default location (yours). Browser-smoke the card actions.

--------------------------------------------------------------------------------

## WAVE-3 FOLLOW-UP (@@Host): dashboard-config polish (@@LaneB - your phase-15 area)

@@LaneB: two dashboard-config changes (you own the Dashboard from phase-15;
@@LaneD is loaded, this balances). Both in the dashboard CONFIG (back side).
1. CAROUSEL NAVIGATOR replaces the segmented widget-selector. Today the config
   back uses a segmented "About | Workspace | Search" button selector (@@Host
   image #9, "weird widget selector"). @@Host wants a CAROUSEL NAVIGATOR
   instead (image #8): a ‹ prev + next › chevron pair + a DOT pager (• • •,
   one dot per slot, active dot filled) + a pause/play (||) toggle. It
   navigates the SAME slots (About/Workspace/Search) and the pause/play
   controls the carousel autoRotate. Grounding: DashboardTab.svelte +
   EmptyPaneCarousel.svelte (the front carousel already has rotation+autoRotate
   + carouselSlide; reuse its slot model + the existing front carousel's
   control affordances if there's a shared nav). Brief design note (how it
   wires to the slots + autoRotate), then implement.
2. SCREENSAVER PREVIEW FIDELITY. The matrix-screensaver PREVIEW shown in the
   dashboard config is "a ridiculously inaccurate representation of the matrix
   screensaver" (@@Host). Make the preview ACCURATELY reflect the REAL matrix
   screensaver (find the actual screensaver impl - routes/screensaver.rs +
   wherever the matrix-rain renders - and make the config preview match its
   look: characters, columns, fall, colour/trail). Browser-smoke vs the real
   screensaver side by side.
Files: the dashboard components (DashboardTab.svelte, EmptyPaneCarousel.svelte,
the dashboard/ subdir, the screensaver preview) - all dashboard, collision-free
with @@LaneD's editor work. Browser-smoke both. Post a sha per item when gated.

--------------------------------------------------------------------------------

## WAVE-3 FOLLOW-UP (@@Host): manual - markdown-editing section additions (@@LaneE)

@@Host: the manual's markdown-editing coverage already mentions basic markdown,
the `[[` wiki picker, and the `@{contact}` search; ADD mentions of:
1. `@today` / `@date` date macros (Google-Docs-compatible). Ground in
   web/src/editor/commands/date_macros.ts: `@today` bakes today's date (no
   popover); `@date` inserts + opens the calendar/format picker. Frame the
   Google Docs compatibility @@Host noted.
2. The new MERMAID renderer for charts. Ground in web/src/editor/widgets/
   mermaid.ts (merged feature): a fenced ```mermaid block renders as a chart;
   cursor INSIDE the block = editable source (byte-for-byte a normal code
   block), cursor LEAVES = auto-renders the diagram (flip), errors render
   mermaid's message; lazy-loaded. (Some mermaid polish is still in flight -
   flip symmetry, error-line highlight - but the user-facing behavior above is
   stable; document that, don't wait.)
LOCATE the right section: docs/manual/editing-markdown.md covers Tabs/Drafts/
External-edits; the `[[`/`@{contact}` coverage may live in docs/manual/wiki-
links.md or editing-markdown.md body - put the new mentions where the existing
`[[`/`@{contact}` ones are. GROUND descriptions in source (don't invent the
behavior). Docs = review-gated, no isolate-gate. Post a sha; I relay to @@Host.
