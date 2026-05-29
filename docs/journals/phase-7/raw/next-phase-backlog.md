# Phase 8 backlog (scoped during phase 7)

Items @@Alex flagged during phase 7 but explicitly
deferred to phase 8. Captured here so the next
architect inherits the scope instead of re-doing
the analysis.

Append new items below as @@Alex surfaces them.

---

## Phase 8 headline deliverable

**Ship a notarized macOS `.dmg` (and the
equivalent signed installers on Windows + Linux)
that other users can download and install
without Gatekeeper / SmartScreen friction.**

This is the load-bearing phase-8 exit criterion
— @@Alex called it out 2026-05-19 23:55 BST.
Tag-triggered CI produces the signed installer
artifact, hosted via the release pipeline.

Cross-references across the items below:
* **Item 6** (Website migration + manual + CI):
  CI pipeline that turns a git tag into
  published installers, plus the website
  hosting the download.
* **Item 7** (chan-desktop upgrade model +
  bundled chan binary): the binary the user
  downloads ships chan as a bundled sidecar +
  the launch-time `--version` selection.
* **Item 8** (Open-source the repo + CI test
  lane): the CI infrastructure that builds +
  signs + notarizes against GitHub Actions
  secrets.
* **Prerequisite (chan-desktop key rotation)**:
  `desktop/CLAUDE.md` documents the current
  updater key as a dev key. The notarization
  + signing identity used at build time is
  separate (Apple Developer ID), but both
  need to be production-grade before the
  first public DMG ships. Rotate / provision
  before tag-triggered CI publishes.

Coordinated cut shape (proposed for phase-8
architect):

* `systacean-N` — chan-desktop key audit +
  rotation plan; Apple Developer ID
  provisioning; CI secrets handling.
* `systacean-N+1` — GitHub Actions workflow:
  on `chan-v*` tag → `make build` (per
  `desktop/Makefile`) → notarize → upload to
  GitHub Release.
* `systacean-N+2` — equivalent Windows
  signing (Authenticode) + Linux packaging
  (deb / rpm / AppImage — pick the right
  long tail). Cross-platform polish; defer
  if scope creeps.
* `architect-led` — release-process
  documentation: how to cut a release, what
  the CI does, where artifacts land, manual
  verification steps.

Implication for the existing items: 6, 7, 8
fuse into the headline deliverable. They're
still useful as separable scopes (the website
is content + DNS work that doesn't NEED to
land with the CI cutover, etc.) but the
"phase 8 ships a notarized DMG" framing is
the through-line.

---

## 1. Drive metadata carousel — redesign

Date scoped: 2026-05-19 (phase 7).
Owner-at-scope: @@Architect.
Scope doc: [`architect/architect-2.md`](architect/architect-2.md).

**One-line**: replace the underwhelming single-bar
slide 2 with a multi-slide drive-dashboard story —
drive overview, code language breakdown, markdown
breakdown, and a slide for chan's metadata
footprint outside the drive root (`~/.chan`, BM25
size, graph index size, etc.).

**Status**: scope captured, implementation deferred
to phase 8. Existing endpoints cover most of it;
one new endpoint needed (`GET /api/chan_meta`).
See `architect-2.md` for the full proposed slide
layout + data mapping + release-tradeoff analysis.

---

## 2. Drive pre-flight checks + BOOT process

Date scoped: 2026-05-19 (phase 7).
Owner-at-scope: @@Architect.

**One-line**: stop cold-starting drives. Add a
pre-flight inspection pass when a user proposes a
new drive path, surface what we find, and then run
a deliberate "boot" stage that fills in the
indexes asynchronously while the user sees
progress. Determines a real "boot complete" signal
even when the underlying filesystem keeps churning.

### Pre-flight checks

Run against the proposed drive path before
registering:

* **Permissions** — can we read it? can we write?
  Surface read-only as a hard fail unless the
  user explicitly opts into "view-only drive"
  (open question: do we support that?).
* **Size class** — quick size sniff (file count
  + bytes). Classify as small / medium / large
  with tuned thresholds. Drives the index-strategy
  decision (eager full-walk for small, lazy
  on-demand for large, somewhere in between for
  medium).
* **Media class** — local SSD vs spinning disk
  vs network mount (NFS / SMB / Tauri sandbox
  alias). Affects expected first-pass speed and
  what "boot complete" means in practice. macOS
  `statfs` exposes mount type; Linux likewise.
  Network mounts get a slower index profile.
* **SCM detection** — if the proposed path is
  inside a `.git` or `.hg` working copy, offer to
  use the SCM root instead. (Explicit user note:
  git + hg matter; svn / fossil / etc. don't.)
  This is a UI prompt, not a forced redirect —
  the user may want a sub-directory drive.
* **Can-we-create-a-drive-here check** — does
  this path conflict with another registered
  drive (overlap, nested, identical)? Anything
  that would corrupt the per-drive state.

Output: a structured "pre-flight report" the UI
renders before commit. User confirms or backs out.

### BOOT process

After pre-flight passes and the user commits:

1. **Sync seed** — register the drive in the
   registry, create per-drive state dirs, write
   `chan/drives/<id>/` skeleton. Fast.
2. **Async index roll** — kick off graph +
   indexing + report scans in the background.
   These can run for minutes on a large drive;
   the UI shouldn't block.
3. **Progress surface** — UI shows "booting"
   with sub-stages (graph at N%, BM25 at M%,
   report at K%). Carousel / status bar /
   dedicated boot overlay — TBD by phase 8 UI
   design.
4. **Boot-complete signal** — a deterministic
   event when all initial passes settle, even
   while the watcher is already streaming deltas
   in. Watcher deltas applied incrementally from
   the moment the indexers come online; "boot
   complete" means "we've caught up to the
   filesystem snapshot we started with, and
   we're now in steady-state delta mode".

### Architecture notes for phase 8

* The pre-flight inspection lives in chan-drive
  (filesystem concern), exposed via a new
  chan-server endpoint (`POST /api/drive/preflight`
  with the candidate path; returns the report
  without registering anything).
* Boot progress lives in chan-server, fed by
  chan-drive index callbacks. The existing
  `/api/indexing/state` endpoint already exposes
  per-dir state; phase 8 may want to extend it
  with a top-level "boot phase" enum
  (`preflight` | `booting` | `steady`).
* The carousel's "indexing graph" slide
  (`fullstack-35` slide 3) is the natural surface
  for boot progress — coordinate the carousel
  redesign (item 1 above) with this work; they
  share UI real estate.
* Tauri shell: the drive-launcher's
  "Open drive" / "Attach" buttons are the UX
  entry points. The pre-flight prompt lives
  here before the drive lands in the registry.

### Async `chan serve` + boot API

(Added 2026-05-19 16:35 BST — @@Alex
extension.)

The boot process must run async inside
`chan serve`. The UI should never block on
"drive is still indexing" — it loads, paints
chrome, and responds to interaction
immediately. A boot API exposes progress so
the UI can render the right state:

* `chan serve` starts → registers the drive,
  begins the boot sequence in background
  tasks (graph build, BM25, report scan,
  whatever else).
* HTTP server starts accepting connections
  immediately, NOT after boot completes.
* New endpoint (`GET /api/boot` or extend
  `/api/indexing/state` with a top-level
  `boot_phase` enum):
  * Returns current boot phase
    (`preflight` | `booting` | `steady`).
  * Returns per-subsystem progress (BM25 N%,
    graph N%, report N%).
  * Returns expected completion ETAs if cheap
    to derive (file count remaining vs files
    processed).
* SPA polls (or subscribes via WS) and renders
  the boot progress in the indexing-graph
  slide + a discreet status pill /
  notification surface.

Correctness + reliability discipline (the
trade-off @@Alex explicitly called out):

* Smooth UX is the goal, but only if we keep
  delivering correct results.
* While boot is in flight, search / graph /
  inspector responses should clearly indicate
  "partial result, indexer still booting"
  rather than silently returning
  ranked-against-empty results.
* Watcher deltas during boot apply to the
  in-flight state (no waiting for boot to
  end before observing the watcher).
* Read-the-room: do we need more iron in
  `chan-drive` to handle this gracefully?
  Phase-8 audit pass: are there places
  where a half-built index produces wrong
  answers vs partial answers? Identify and
  fix before exposing the boot API to the
  UI.

### Benchmark / stress test

@@Alex's chosen benchmark for phase 8:

```bash
git clone --depth 1 https://github.com/torvalds/linux /tmp/chan-bench-linux
chan add /tmp/chan-bench-linux
chan open
```

Watch from the test-server browser:

* **Indexing graph slide** populates
  gradually as `chan-drive` walks the tree.
  Watch the orange→green transition propagate
  through `drivers/`, `arch/`, `fs/`, `mm/`,
  etc. Expect minutes, not seconds, on cold
  disk.
* **Search index** populates incrementally —
  searching for `static inline` at t=10s
  returns a partial result; at t=60s, a much
  fuller one; at t=300s, the full corpus.
  Each response self-describes whether it's
  partial.
* **Drive metadata carousel** (item 1
  redesign) shows the language breakdown
  growing over time: at t=5s shows just
  Markdown + a few C files; at t=60s shows
  the full C dominance plus assembly + Kconfig
  + Devicetree; at t=300s shows the final
  steady-state distribution including Rust.

Numbers to log for the post-bench report:

* Time to first paint (`chan serve` → SPA
  visible).
* Time to first non-empty search result.
* Time to indexing graph showing > 10%
  green.
* Time to BOOT_COMPLETE signal.
* Memory footprint under load (resident +
  index sizes for BM25 / graph / report).
* Watcher event throughput during boot
  (events/sec the indexer can keep up with).

The Linux kernel as a benchmark stresses:

* ~70k files, many languages (C, asm,
  Makefile, Kconfig, Devicetree, Rust now).
* Deep directory hierarchies.
* Subsystem-distinct concentrations
  (`drivers/` dominates, `fs/` is dense,
  `arch/` is multi-shard).
* Heavy use of headers + cross-includes
  (graph link density).

Useful both as "does this scale" and as a
"does this feel right" check. If the carousel
+ indexing graph + search updates feel
choppy / wrong during the boot window, that's
the signal to revisit chan-drive's
incrementality.

### Out of scope (deliberately)

* Re-walking on every chan start. The boot-
  complete signal is about a fresh drive
  registration, not every cold app launch.
* Cross-drive coordination. Single-drive boot
  is enough; multi-drive concurrency is a
  separate phase.

---

## 3. Screensaver with PIN unlock

Date scoped: 2026-05-19 (phase 7).
Owner-at-scope: @@Architect.

**One-line**: add an inactivity-triggered
screensaver overlay with an optional PIN unlock,
plus two pickable visual themes (Matrix rain by
default, Castaway as an alternate).

### Settings surface

* **Inactivity timeout** — user-configurable
  duration (minutes). 0 / "off" disables.
* **PIN** — optional 4-8 alphanumeric. When set,
  the screensaver requires the PIN to dismiss.
  When unset, any keystroke / mouse activity
  dismisses.
* **Theme picker** — Matrix (default) | Castaway.

### PIN storage

Store a hashed PIN in the per-drive preferences
file (or app-level config, depending on whether
the screensaver is per-drive or app-wide; default
to app-wide). @@Alex's note: "md5 or something
like that". Threat-model note:

* The threat model is "casual passerby looking
  at the user's screen", not "remote attacker
  brute-forcing the hash". MD5 is cryptographically
  broken but adequate for this surface; SHA-256
  is a one-line upgrade if we want defense in
  depth. A 4-8 alphanum PIN has trivial entropy
  either way — the hash mainly stops a
  shoulder-surf of the config file's plaintext,
  not a serious attack.
* Recommendation: SHA-256 over MD5 (no real cost,
  avoids the "we still ship MD5" smell), with a
  one-line bcrypt/argon2 upgrade noted as a
  follow-up if anyone cares.
* Salt isn't really useful for a single-user
  PIN, but a random salt per-install costs
  nothing and prevents rainbow-table lookup of
  a known weak PIN like `1234`.

### Visual themes

Both linked repos need feasibility / port audit
during phase 8:

1. **Matrix (default)** —
   <https://github.com/dcragusa/MatrixScreensaver>.
   Quick check: that repo is **Python + curses**
   (terminal-rendered). NOT directly portable to a
   browser surface. We'd rewrite the algorithm as
   a Svelte component over HTML5 canvas. The
   Matrix-rain algorithm is well-known — character
   columns drop at varying speeds, head-glyph
   bright + trailing glyphs fade. ~200 LOC of TS
   if we do it ourselves; many MIT-licensed JS
   reference implementations exist. Probably the
   right path: write our own. Cite the dcragusa
   repo in the source as inspiration.
2. **Castaway** —
   <https://github.com/xesf/castaway>. Repo
   audit pending. Whatever it renders, the same
   "is it browser-portable, or do we rewrite?"
   question applies. Phase-8 architect to audit
   and decide.

### Activation surface

* **Auto-trigger on inactivity** — mouse-move +
  keydown listeners reset the inactivity timer.
  Standard DOM event plumbing; no platform
  integration needed for the basic case.
* **Manual trigger: `Cmd+K L`** — Pane Mode
  binding to lock immediately, regardless of the
  inactivity timer. Same activation path as the
  inactivity trigger; user holding the keys to
  pre-empt a step-away. Add `L` to the Pane
  Mode help cheatsheet
  (`PaneModeHelp.svelte`).
  Note: independent of whether a PIN is set —
  Cmd+K L just shows the screensaver; if no PIN,
  any keystroke dismisses; if PIN, PIN dismisses.
* On Tauri shells, optionally hook the system-
  level idle signal (macOS `IOHIDIdleTime`,
  Linux `xprintidle`-equivalent). Not required
  for v1; the DOM listeners are good enough.
* Tab-focus-loss: auto-trigger should NOT kick
  in when the window is unfocused (that's just
  the user looking at another app); only when
  the window IS focused but idle. Manual Cmd+K L
  works regardless.

### Layering

Render the screensaver as a full-window overlay
above the workspace, beneath any modal dialog
that's already open. Esc / typing the PIN
dismisses; the overlay blocks all keyboard +
mouse routing to the underlying surfaces until
unlocked.

### Out of scope

* OS-level lock integration (forcing the system
  lock screen). chan stays in its own UI bubble.
* Multi-monitor handling. Single-window scope.

### Phase-8 cuts (proposed)

* `fullstack-N` — screensaver overlay component +
  settings panel surface + Matrix theme.
* `systacean-N` — PIN hashing helper + per-drive
  / app-level config schema additions.
* `fullstack-N+1` — Castaway theme (after the
  repo audit).

---

## 4. Infographics tabs + minimal empty pane + Hybrid Nav rename

Date scoped: 2026-05-19 (phase 7).
Owner-at-scope: @@Architect.

**One-line**: refactor the empty-pane carousel
into a first-class "Infographics" tab type
spawned via `Cmd+K 9` (multi-instance like FB /
Graph), strip the empty pane back to just the
chan logo + a hint pointing at Hybrid Nav, and
rename "Pane Mode" → "Hybrid Nav" in user-facing
copy.

Coordinates tightly with item 1 (carousel
redesign) — the slide content from item 1 lives
inside the new Infographics tabs. Cut item 4's
container refactor first, then item 1's slide
content into the container.

### Carousel → Infographics tab

* New tab kind: `infographics` (alongside
  existing `terminal`, `file-editor`,
  `file-browser`, `graph`, `search`, etc.).
* Spawn binding: `Cmd+K 9` in Hybrid Nav, opens
  an Infographics tab in the focused pane.
  Mirror the spawn semantics of `Cmd+K 2` (FB) /
  `Cmd+K 3` (Graph) — context-aware where it
  makes sense (the drive-overview slide is
  always whole-drive, but if we have per-dir
  infographics later, a doc/dir context could
  scope them).
* **Multi-instance**: more than one Infographics
  tab can live at once (e.g. one with the
  drive-overview slides, another pinned to the
  chan-meta slide). Pattern from `fullstack-47`
  (multi-FB / multi-Graph) applies — independent
  per-tab state for current slide, cycle
  toggle, etc.
* The current `EmptyPaneCarousel.svelte` is the
  starting point; lifts into a standalone
  `InfographicsTab.svelte` (or similar).
  Carousel cycle / stop / DnD logic ports over;
  empty-pane mount surface drops it.

### Empty pane minimal

After the carousel becomes a tab, the empty pane
goes back to a simple landing:

* Chan logo centred.
* Single hint line below: something like
  `Press Cmd+K to enter Hybrid Nav` (or whatever
  wording lands in the rename — see below).
* That's it. No shortcut table, no drive
  summary, no carousel.

The drive summary / shortcut table that used to
live on slide 1 of the carousel can either move
into the Infographics tabs (probably their own
slide) or get dropped entirely. Phase-8
architect's call.

### "Pane Mode" → "Hybrid NAV" rename

**Pulled forward to phase 7** as `fullstack-62`
(2026-05-19). @@Alex confirmed "Hybrid NAV"
wording and pulled the rename into the v0.11.0
wrap. The container refactor + minimal empty
pane stay in phase 8; only the rename moves.

### Phase-8 cut decomposition (proposed)

1. **fullstack-N — Infographics tab container**.
   Lift `EmptyPaneCarousel.svelte` → tab type.
   `Cmd+K 9` binding. Multi-instance state.
   Lands with placeholder slide content (same
   slides as today, just inside a tab).
2. **fullstack-N+1 — empty pane simplification +
   rename**. Drop the carousel from empty-pane.
   Add chan-logo + Hybrid-Nav hint. Sweep
   user-facing "Pane Mode" → "Hybrid NAV" copy.
3. **(coordinated with item 1)** Carousel
   redesign content lands inside the new
   Infographics tab type.

If the empty-pane simplification + rename are
small enough they can fold into the container
cut. Phase-8 architect's call.

### Coupling with item 1

Cut item 4's container refactor BEFORE item 1's
slide-content redesign. Otherwise the slide
redesign happens against a moving target. Phase
8 sequencing: 4 → 1 → 2 → 3 (container →
content → BOOT → screensaver), or whichever
phase-8 architect prefers based on lane
availability.

---

## 5. Chan config currency audit + screensaver settings

Date scoped: 2026-05-19 (phase 7).
Owner-at-scope: @@Architect.

**One-line**: audit the chan config schema for
drift against what the app currently consumes,
fold in the new screensaver settings (item 3),
and document the shape so future settings work
doesn't accumulate drift again.

### Currency audit

Phase 7 added a non-trivial amount of state
that may or may not flow through the config
files cleanly. Examples surfaced during this
phase:

* Pane Mode / Hybrid NAV keymap (post `-40` /
  `-42` rework).
* Per-Hybrid theme override (`ht` / `hb` on
  the URL hash — is there an app-level
  default we should respect from config?).
* Hash schema extensions from `-58`
  (per-tab BrowserTab `bs` / `bd` / `be` /
  `bsc`) — pure session state, not config,
  but the boundary is worth confirming.
* `-46` British spelling sweep — any locale
  preference in config?
* `-56` Cmd+S drop — autosave debounce
  threshold lives in tabs.svelte.ts as a
  hardcoded constant; should it be in
  config?

Walk:
1. Enumerate every config-driven setting the
   app actually reads.
2. Cross-reference with the config schema
   (per-drive prefs + app-level config).
3. Identify drift (settings the app uses but
   doesn't expose; settings the schema
   declares but no consumer reads).
4. Add missing exposers; drop dead schema
   keys; document each surviving setting's
   purpose.

### Screensaver settings (from item 3)

Once phase 8 cuts the screensaver feature
(backlog item 3), the config needs:

* `screensaver.enabled: bool` — master
  toggle (could be inferred from non-zero
  timeout; keep explicit for clarity).
* `screensaver.inactivity_minutes: u32` —
  triggers timer. `0` disables.
* `screensaver.theme: "matrix" | "castaway"`
  — locked enum at phase-8 cut time, picker
  in Settings UI.
* `screensaver.pin_hash: Option<String>` —
  hashed PIN; SHA-256 recommended per
  backlog item 3 threat-model analysis.
  Optional; absent = no PIN required to
  dismiss.
* `screensaver.pin_salt: Option<String>` —
  per-install random salt (cheap defense).

App-level vs per-drive: probably app-level —
the screensaver protects the chan window
regardless of which drive's open. Per-drive
override might be nice for "this drive is
extra-sensitive" but adds complexity; skip
for v1.

### Migration

If schema changes break old config files,
include a one-time migration on chan startup:
read the old shape, write the new shape, log
the migration. Standard practice; codify in
the phase-8 cut.

### Documentation

The phase-8 cut should produce a
`docs/config.md` (or update an existing
one) listing every config key + default +
which subsystem consumes it. Avoids future
drift.

### Phase-8 cuts (proposed)

* `systacean-N` — config schema currency
  audit + diff report.
* `systacean-N+1` — schema additions for
  screensaver (coordinated with item 3's
  fullstack cut for the Settings UI).
* `fullstack-N` — Settings UI exposes the
  new screensaver controls.

---

## 6. Website migration + manual + first-launch UX + CI

Date scoped: 2026-05-19 (phase 7).
Owner-at-scope: @@Architect.

**One-line**: migrate `chan.app` from its
current VPS to GitHub-hosted static site, write
a shipping `docs/manual`, wire chan-desktop to
offer the manual on first install, and
automate the whole pipeline via CI.

This is genuinely four coupled sub-projects;
phase-8 architect should consider whether to
ship them in one block or stagger across
phases.

### 6.1 Website migration (chan.app)

* @@Alex will point at the current chan.app
  source when phase 8 opens; copy the source
  into this repo (under `web-marketing/` or
  similar — pick a name that doesn't
  collide with the existing embedded SPA at
  `web/`).
* Build target: static site, GitHub Pages
  (or equivalent) hostable.
* Move chan.app DNS off the VPS to point at
  the GitHub hosting. Requires:
  * DNS records (A / CNAME / TXT — depends
    on the host).
  * TLS cert provisioning (GitHub Pages
    handles auto-cert for `*.github.io`;
    for `chan.app` apex domain we may need
    Cloudflare or a similar fronting layer
    to terminate TLS, OR rely on the host's
    apex-domain support).
* Decommission the VPS once DNS is verified +
  the new site is serving for a soak period.
* **Donation QR placement** (added 2026-05-21):
  the same donation QR shipped in the
  in-app About section
  (`web/public/qr-donate.png`, see
  [`../phase-8/fullstack-a/fullstack-a-42.md`](../phase-8/fullstack-a/fullstack-a-42.md))
  also lands somewhere on chan.app —
  candidate placements: footer "support the
  project" block; a small Support page
  linked from the nav; inline on the
  download page next to the install
  instructions. Implementer picks at task-
  cut. Same asset on both surfaces (in-app
  + website); if the QR ever rotates, both
  surfaces need refresh — flag at fan-out.
  Copy tone matches @@Alex's voice (not
  marketing): "If Chan is a daily driver
  for you, scan to send a tip. Optional;
  the project is free either way."

### 6.2 Documentation manual (`docs/manual`)

* Write a real user-facing manual covering:
  * Drive concept + how to set one up.
  * Editor walkthrough (WYSIWYG, list mode,
    hashtags, contacts).
  * Hybrid Nav (Cmd+K) + the keybinds that
    landed in phase 7.
  * File Browser + Graph + Search.
  * Terminal + watcher + rich prompt.
  * Settings + per-Hybrid theme + screensaver
    (once phase-8 item 3 lands).
  * MCP discovery + external-agent
    integration.
  * Tunnel mode + the
    `{user}.drive.chan.app/{drive}` flow.
* Format: Markdown. Phase-8 architect picks
  the renderer (Hugo / mdBook / a simple
  generated index from the repo). Aim:
  shippable as static HTML and readable as
  raw `.md` in the repo.
* The manual lives in the **main chan repo**
  under `docs/manual/`, NOT in the marketing
  site repo. The marketing site builds its
  manual section from this source.

### 6.3 First-launch UX in chan-desktop

* On first launch (no drives registered yet
  OR an explicit "show me the manual" CTA),
  offer to download the manual + open it in
  a webview.
* Download mechanism: HTTPS fetch from
  GitHub raw / GitHub Pages / chan.app — TBD
  by the hosting decision in 6.1. Cache
  locally (`<config>/chan/manual-<ver>/`) so
  subsequent opens don't re-fetch.
* Open in:
  * Default browser (simplest), OR
  * An in-app Tauri webview pointed at the
    cached / live URL.
  Phase-8 architect picks; webview is more
  integrated but adds Tauri config burden.
* The first-launch CTA should be dismissible
  + retrievable from the launcher's existing
  menus.

### 6.4 CI wiring

Three pipelines to wire:

1. **Main chan repo CI**: on every push to
   `main`, builds the website + manual,
   publishes the built artifacts to the
   marketing-site repo / GitHub Pages branch.
2. **Marketing-site repo CI** (if separate
   from the main chan repo — decide during
   phase 8): on push, build + deploy to
   GitHub Pages.
3. **Release CI**: when a chan release tag
   lands, bundle the matching manual version
   into the chan-desktop installer (or arrange
   the chan-desktop binary to fetch the
   matching version from GitHub).
* Existing chan-desktop release pipeline
  (Tauri build + signing per `desktop/CLAUDE.md`)
  stays as-is; this adds a manual-bundle step.
* Watch for: certificate / signing-key
  exposure; CI secrets handling.

### Dependencies + sequencing

* DNS changes have a TTL tail; do the DNS cut
  early in phase 8 with the VPS still
  serving as fallback, soak for a few days,
  then cut over.
* Manual content can drift; tag versions of
  the manual to chan releases so v0.11.0
  users see v0.11.0 docs even after main
  ships v0.12.0.
* First-launch UX (6.3) depends on the
  hosting URL being stable (6.1) but doesn't
  depend on the manual being complete (6.2)
  — partial manual is better than no manual
  on first install.

### Phase-8 cuts (proposed decomposition)

* `systacean-N` — DNS cutover plan + TLS
  story + VPS decommission timeline.
* `systacean-N+1` — CI pipelines (chan repo
  → marketing site repo; marketing → GH
  Pages; release → manual bundle).
* `fullstack-N` (web-marketing) — port the
  chan.app source into this repo.
* `fullstack-N+1` — chan-desktop first-launch
  manual UX.
* `docs-N` (or just architect-led) — write
  `docs/manual/` content. Probably staggered
  across multiple cuts as content matures.

### Out of scope

* Translating the manual into multiple
  languages. v1 is English; i18n is post-v1.
* Search inside the manual. The static site
  generator's built-in search is fine; no
  custom indexer.

---

## 7. Upgrade model: chan-desktop self-update + bundled vs system chan binary

Date scoped: 2026-05-19 (phase 7).
Owner-at-scope: @@Architect.

**One-line**: chan-desktop ships with its own
chan binary AND its own auto-updater (mirroring
the existing chan self-upgrade pattern). On
launch, chan-desktop runs whichever of
{bundled, on-PATH} chan binary has the higher
`--version` — no settings UI, no user picker.

### Current state

* `chan` binary has self-upgrade
  (`crates/chan/src/update.rs`). Battle-tested
  against local tunnel on chan-desktop —
  works fine. **Don't change.**
* `chan-desktop` is the Tauri shell. Per
  `desktop/CLAUDE.md`, it has tauri-plugin-
  updater wired with minisign signature
  verification. Pubkey embedded in
  `desktop/src-tauri/tauri.conf.json`. Current
  signing key is a DEV key; rotate before
  public release (already noted in
  `desktop/CLAUDE.md`).

### Target model

Three pieces:

1. **chan-desktop self-updates** via its
   existing tauri-plugin-updater. Works on
   macOS / Windows / Linux per Tauri's
   docs; phase-8 architect verifies all
   three platforms before signing off.
   Bridge release + key rotation per
   `desktop/CLAUDE.md` must precede first
   public release.
2. **chan-desktop ships with a bundled
   `chan` binary** inside the app
   resources. Single chan version per
   chan-desktop release.
3. **Binary selection on launch**:
   * Probe `which chan` (Unix) / `where
     chan` (Windows) for a system-installed
     chan binary.
   * Run `<bundled> --version` and
     `<system> --version` (if found).
   * Parse semver, pick the **higher**
     version. Tie → bundled wins (predictable
     fallback).
   * Spawn `chan serve` etc. through the
     selected binary.
   * Log the choice + version on startup
     so a tail of the launcher logs
     surfaces "running chan v0.X.Y from
     {bundled,system}".

No settings UI, no user toggle. Rule:
"always latest" via `--version` decides.

### Edge cases

* **No system chan**: use bundled. Easy.
* **System chan older**: use bundled.
* **System chan newer** (user ran
  `chan upgrade` separately, or installed
  via Homebrew / a Linux package manager
  that fronts a newer release): use system.
* **System chan unreadable / corrupt**:
  fall back to bundled, log the issue.
* **Different bitness / arch**: irrelevant
  for the version probe but worth a sanity
  check; running a foreign-arch binary
  is a hard fail.
* **Bundled chan can self-upgrade**:
  what happens if the bundled chan
  self-upgrades to a newer version? It
  modifies the binary inside chan-desktop's
  resources directory. On next launch,
  the bundled-version probe picks up the
  new version. **Tauri sandbox / app-
  bundle permissions** may forbid writing
  into the app bundle on macOS / Windows;
  if so, the bundled chan effectively
  can't self-upgrade, and only
  chan-desktop's own updater (which
  brings a new bundled chan with each
  release) updates it. That's likely fine
  and aligns with the "always latest"
  rule — chan-desktop pulls in chan
  updates as part of its release.

### Open questions for phase 8

* **Bundled chan storage layout**: inside
  the Tauri app bundle's `resources/` dir
  on macOS, somewhere appropriate on
  Windows / Linux. Verify Tauri's
  `tauri.conf.json` resource declaration
  supports this.
* **Permissions for the bundled-chan
  invocation**: launching a child process
  needs the right Tauri capability. Audit
  `desktop/src-tauri/capabilities/`
  for what's already granted vs what's
  needed.
* **`--version` output format**: is it
  stable parseable semver, or does it
  include build metadata that complicates
  parsing? Check the current `chan
  --version` output and lock in a
  parser-friendly shape if not already.
* **First-launch UX**: if the bundled-chan
  vs system-chan selection ever matters to
  the user (e.g. a downgrade scenario),
  surfacing the choice in the launcher
  log is fine; we're explicit "no
  settings UI" here.

### Out of scope

* User-facing binary picker. Hard "no" per
  @@Alex.
* Multi-version coexistence. Single chan
  per chan-desktop launch — selected at
  startup and used for the session.
* Cross-version compatibility (running
  chan-desktop v0.11.0 against chan
  v0.10.x or v0.12.x). Bundled is the
  reference pair; if a user's system
  chan diverges and breaks compatibility,
  that's their call to make.

### Phase-8 cuts (proposed)

* `systacean-N` — verify tauri-plugin-
  updater works on all three platforms;
  signing-key rotation per `desktop/CLAUDE.md`.
* `fullstack-N` (chan-desktop) — bundled
  chan binary in app resources; launch-
  time version probe + selection.
* `systacean-N+1` — CI extension: bundle
  the matching chan release into each
  chan-desktop installer artifact.

---

## 8. Open-source the repo + CI testing in a separate lane

Date scoped: 2026-05-19 (phase 7).
Owner-at-scope: @@Architect.

**One-line**: flip the chan repo to public,
write the open-source plumbing (license,
contributing guides, issue / PR templates),
and stand up CI in a separate test lane
that iterates against the v0.11.0 phase-7
outcome.

### Open-source the repo

* **License pick**: MIT vs Apache-2.0 vs
  dual (Rust convention: MIT OR Apache-2.0).
  Recommend dual-MIT-Apache for Rust
  contributor compatibility, but @@Alex's
  call.
* **`LICENSE` file** at repo root (or
  `LICENSE-MIT` + `LICENSE-APACHE` for the
  dual case).
* **Code audit before flipping public**:
  * Scan for secrets in commit history
    (`git log -p | grep -iE
    "(password|secret|token|api[_-]?key)"`
    — false-positive heavy; use a tool like
    `gitleaks` or `truffleHog` for real
    coverage).
  * Scan for internal references that
    don't make sense publicly (TODO names,
    Slack channel references, embarrassing
    comments).
  * Scan for unintended PII in journals.
    The phase-7 journals reference @@Alex
    by handle; that's fine. But agent-side
    journals may have captured throwaway
    notes. Audit pass.
* **Repo visibility flip**: GitHub repo
  `chan-writer/chan` private → public.
  Coordinate with item 6's DNS work (the
  marketing site references the repo URL).
* **`CONTRIBUTING.md`**: how to set up the
  dev environment, what `scripts/install-
  hooks` does, the pre-push gate
  expectations.
* **`CODE_OF_CONDUCT.md`**: standard
  Contributor Covenant or similar.
* **Issue + PR templates** in `.github/`:
  bug report, feature request, security
  disclosure (SECURITY.md).
* **Maintainer notes**: phase-7 journals
  contain a lot of "AI agent orchestration"
  vocabulary that public contributors won't
  parse without context. Consider a
  `docs/coordination.md` explaining the
  multi-agent dev pattern, OR archive the
  journals under `docs/journals/private/`
  and present only a curated changelog.

### CI testing in a separate lane

* "Separate lane" reading: a CI test setup
  distinct from the main code lanes (Lane A /
  Lane B / Systacean / Webtests). Probably
  a new agent role for phase 8 — e.g.
  `@@CI` — with its own contact card +
  journal directory.
* CI scope:
  * GitHub Actions workflows for the Rust
    build matrix (Linux / macOS / Windows).
  * Lint + test on every PR
    (`cargo fmt --check`, `cargo clippy
    -- -D warnings`, `cargo test`,
    `web/npm run check`, `web/npm run test`,
    `web/npm run build`, `scripts/pre-push`).
  * Release artifact builds tagged on each
    `chan-v*` git tag (chan binary +
    chan-desktop installer per platform).
  * Coordinate with item 6 (CI for the
    marketing site + manual deployment +
    chan-desktop bundle).
* **Start point**: v0.11.0 (phase 7
  outcome) is the first stable artifact.
  Build the CI against this known-good
  state; iterate from there.
* **Secrets handling**: minisign signing
  keys, Apple Developer ID for
  chan-desktop notarization, etc. CI uses
  GitHub Actions secrets; rotate keys
  (`desktop/CLAUDE.md` already calls out
  the dev-key rotation requirement before
  public release — coordinate with this
  item).

### Dependencies + sequencing

* Open-source the repo BEFORE shipping v1
  publicly, but the order between this
  item and item 6 (website + manual)
  matters less than coordinating them.
  Probably both land together as the
  "v0.11.0 → public" cutover moment.
* CI testing lane can start work
  immediately once phase 8 opens — v0.11.0
  is the artifact baseline.
* DNS cutover (item 6) and repo
  public-flip should be aligned: don't
  publicize a repo URL until the docs
  site it points to is live.

### Phase-8 cuts (proposed)

* `@@CI-N` (new lane) — set up GitHub
  Actions workflows for build matrix +
  lint + test + release artifacts.
* `systacean-N` — license audit, secrets
  scan, code-history audit pass.
* `architect-led` — `CONTRIBUTING.md` /
  `CODE_OF_CONDUCT.md` / `SECURITY.md` +
  the GitHub templates. Architect can
  draft; @@Alex reviews + ships.
* Coordinate with item 6's CI pipelines
  + the upgrade-model CI extension from
  item 7.

### Out of scope

* Soliciting contributors. v1 is "make it
  possible"; community-building is a
  separate phase concern.
* Translating contributor docs. English
  only for the first public release.
* Patent / trademark filing for "chan".
  Legal — phase-8-architect flags to
  @@Alex if it becomes relevant.

---

## 9. Scope FB watcher to current dir (or parent of selected file)

Date scoped: 2026-05-19 (phase 7).
Owner-at-scope: @@Architect.

**One-line**: the file browser currently
refreshes its tree on **any** change anywhere
in the drive. Narrow the watcher / refresh
scope to the currently-selected directory
(or the parent dir if a file is selected) so
high-churn elsewhere doesn't disrupt the
user's navigation context.

### Why

@@Alex hit this live during phase 7: open
the FB on `docs/journals/`, lots of code
landing in `src/` simultaneously, and the FB
tree was reloading constantly. The
in-progress browse / scroll / expansion got
disrupted by changes the user wasn't looking
at.

The FB's role is "what's at this path"; it
doesn't need to observe drive-wide deltas to
do that job. Watching the *visible scope*
keeps the UI calm.

### Spec direction (for phase-8 architect)

* When the FB tab has a `selected` path that
  resolves to a directory → watcher scope is
  that directory.
* When the `selected` is a file → watcher
  scope is the **parent** directory of that
  file.
* No selection → watcher scope is the drive
  root (current behaviour, no change).
* Watcher attach is per-tab (multi-FB-tabs
  per `-58` already gives us per-tab
  selection). Each tab attaches its own
  scoped watcher.
* Switching tabs / changing selection →
  detach + re-attach to the new scope.
* Closing the tab → detach the watcher
  (don't leak chan-server watcher state).

### Edge cases for phase 8

* **Expanded subtrees that span outside the
  scope**: if the user has `src/` expanded
  in the tree but selection is on
  `docs/journals/`, the watcher misses
  changes inside `src/`. Two options:
  - **Strict scope** (the simpler path):
    only watch the selected dir. Expanded
    siblings stay rendered at their last-
    known state until next refresh.
  - **Scope + expansion**: watch the
    selected dir AND any user-expanded
    directories. More plumbing; closer to
    "what's visible".
  Phase-8 architect picks; lean toward
  strict initially.
* **Watcher API limits**: chan-server's
  per-terminal watcher (`systacean-19`'s
  drive-root-constrained surface) was
  built for terminal use cases. The FB
  needs its own watcher channel or an
  extension to the existing one. Audit
  the chan-drive watcher API during the
  scoping pass — what does the SPA
  currently subscribe to?
* **Search index churn vs FB tree
  churn**: search index can keep watching
  the whole drive (it's a background
  process); this scoping is about the
  FB tree render only.
* **Carousel slide 3 (indexing graph)**
  still wants drive-wide signal — that's
  by design. This item only affects the
  FB tab/dock/overlay tree refresh path.

### Coordinates with

* Item 2 (drive pre-flight + BOOT process)
  — the indexing-state machinery there
  produces dir-level updates the FB
  could subscribe to. May factor in.
* `-58` (per-tab BrowserTab state) —
  selection is already per-tab, hash
  serializable. Watcher attach follows
  that key.
* `systacean-19` (watcher drive-root
  constraint) — same principle (boundaries
  matter); this extends the discipline
  one level inward.

### Phase-8 cuts (proposed)

* `systacean-N` — chan-drive watcher API
  extension: subscribe-by-prefix (or
  similar) so the SPA can ask "watch only
  this subdirectory".
* `fullstack-N` — FB attaches per-tab
  watchers scoped to the selection (or
  parent of selected file); detaches on
  tab close / selection change.

### Out of scope

* Re-architecting the drive-level watcher.
  chan-drive's watcher stays drive-wide;
  this item is purely about the FB
  *subscription* shape.

---

## How to add to this backlog

@@Alex flags items mid-phase that are clearly
"next phase, not this phase". Append a new
numbered section below the existing ones with:

* **One-line** summary.
* **Status** (scope captured / scope deferred /
  needs design).
* Optional scope-doc link (e.g.
  `architect/architect-N.md`) if the item is big
  enough to warrant a full doc.

When phase 8 opens, the new architect uses this
file as the queue. Each item gets a phase-8 task
or design doc; the entry here gets stricken
through once dispatched.
