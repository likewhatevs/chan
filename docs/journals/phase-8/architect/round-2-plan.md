# Phase 8 Round-2 plan (draft, pre-discussion with @@Alex)

Author: @@Architect
Date: 2026-05-20

Status: **draft, not dispatched**. Restructured 2026-05-20
after @@Alex split the original Round 2 into Round 2 + Round
3. Round 2 = features + the full signed+notarized DMG
pipeline tested with real Apple Developer ID keys, with the
repo still private. Open-source flip + multi-model picker
+ whole-codebase polish all moved to Round 3 (see
[`round-3-plan.md`](round-3-plan.md)).

**Numbering note (2026-05-20)**: every Round-2 task slot
listed below carries a provisional number. Round-1 detour
work keeps pulling tasks into Round 1, which shifts the
real next-available slot per lane each time. Treat the
specific numbers as illustrative; the actual numbering
gets assigned at fan-out based on what's free per lane
(after Round-1 close, count the highest committed
fullstack-a-N / fullstack-b-N / systacean-N / ci-N and
start Round 2 from there +1). The per-task SHAPE is the
load-bearing part of this plan; the NUMBERS are not.

Source material: [`../../phase-7/next-phase-backlog.md`](../../phase-7/next-phase-backlog.md)
items 1-7 (item 8 → Round 3; item 9 already done as
`fullstack-b-6` in Round 1).

## Decisions (all locked 2026-05-20)

Status: all six gates cleared. Round-2 fan-out unblocked.

1. **Sequencing — LOCKED**: items 7 + ci-7 stay coupled at
   the top (DMG north star). Then item 6 (website + manual +
   first-launch UX + CI). Then items 1 + 4 (carousel +
   Infographics, coupled). Then 2 (BOOT). Then 3
   (screensaver). Then 5 (config audit).
2. **Item-6 hosting — LOCKED**: GitHub Pages with custom
   domain. Static markdown source lives in the chan repo
   (`docs/manual/` per decision 5); apex-domain TLS via
   Pages' built-in cert provisioning. No external infra
   dependency beyond GitHub.
3. **Item-7 bundled-chan storage layout — LOCKED**:
   PATH-first with bundled fallback + version match. At
   launch chan-desktop probes `which chan`, checks
   `chan --version` against the bundled version; matches use
   PATH, anything else falls through to bundled. Power users
   who run their own chan build override naturally; broken /
   stale PATH installs don't brick the app.
4. **Item-3 PIN hash — LOCKED**: SHA-256 with per-install
   salt. Over @@Alex's earlier "md5 or something" framing
   — SHA-256 is the modern minimum and Argon2's slowness
   isn't needed for a local-only screensaver PIN.
5. **Manual home — LOCKED**: `docs/manual/` (markdown source
   in main repo). Symmetric with `docs/journals/` and
   `docs/release/`; rendered by the website pipeline (item
   6) at `chan.app/manual/`.
6. **First-release version — LOCKED**: **v0.12.0** at
   Round-2 close. @@Alex's framing: "needs way more testing"
   — v0.12.0 positions this as a working release for friend-
   feedback, not the polished v1.0 launch. Round 3's
   open-source flip + polish wave still targets v1.0 as the
   public-stable version.

## North-star through-line

Notarized macOS `.dmg` shipped via tag-triggered CI **with
real keys provisioned**. Repo stays private through Round 2
so we can exercise the pipeline end-to-end before opening it.

Critical path:

| Step                                                | Owner        | Round-2 task  |
|-----------------------------------------------------|--------------|---------------|
| @@Alex completes cert checklist from `ci-3` brief   | @@Alex       | out-of-band   |
| Six secrets populated into GitHub Actions Secrets   | @@Alex       | out-of-band   |
| Workflow YAML consuming the six secrets             | @@CI         | ci-8          |
| chan-desktop signing-key rotation per CLAUDE.md     | @@Systacean  | systacean-8   |
| Bundled chan binary in chan-desktop resources       | @@FullStackB | fullstack-b-11|
| Launch-time version probe + binary selection        | @@FullStackB | fullstack-b-12|
| First-launch chan-desktop verify across 3 platforms | @@Systacean  | systacean-9   |
| End-to-end DMG-on-tag dry-run with real keys        | @@CI         | ci-8          |
| First proper binary release: tag v0.12.0 (or v1.0)  | @@Systacean  | (Round-2 close task) |

## Round-2 dispatch (preliminary numbering)

Numbers shift as Round 1 closes; tracking the next-available
slot for each agent at the time this draft was written.

### Wave 1 — north-star track (concurrent)

| Task           | Owner        | Source                                                                                  |
|----------------|--------------|-----------------------------------------------------------------------------------------|
| ci-6           | @@CI         | Workflow YAML for tag-triggered signed + notarized chan-desktop build (consumes secrets) |
| systacean-8    | @@Systacean  | chan-desktop signing-key rotation per `desktop/CLAUDE.md` (DEV key → release key)       |
| fullstack-b-11 | @@FullStackB | Bundled chan binary inside chan-desktop app resources (item 7 piece 1)                  |
| fullstack-b-12 | @@FullStackB | Launch-time version probe + binary selection (`which chan` vs bundled) (item 7 piece 2) |
| ci-7           | @@CI         | DMG-on-tag dry-run with real keys; smoke-test first signed DMG opens cleanly on a second Mac |
| systacean-9    | @@Systacean  | Verify tauri-plugin-updater works on all three platforms (item 7 prereq)                |

**Defensive `--features embed-model` gate already in
both release workflows** (from `ci-6`, 2026-05-20): the
BGE-bundle cache step + the `fetch-models` invocation in
`release.yml` + `release-desktop.yml` are parked behind
`if: false`. Today no matrix entry builds
`--features embed-model`, so the gate is dead code; when
a feature-on lane is added (the offline-install /
power-user variant per `systacean-6` Q2 — recommended
NOT to bundle on the default chan-desktop sidecar),
flip `if: false` → `if: matrix.embed_model` (or whatever
gating mechanism fits the new lane). `ci-6`'s inline
comments document the flip recipe.

### Chord migration + surface unification (added 2026-05-20)

@@Alex requested top-level chords replacing the `Cmd+K
<key>` spawn family, plus surface unification across the
empty-pane carousel slide 1, the pane hamburger menu, and
the empty-pane right-click menu. Coupled with item 1 + 4
(carousel redesign + Infographics tab container) since the
carousel's shortcut table will move into the new
Infographics tab type.

#### New chord set

| Action          | Native (Chan.app) | Web fallback   | Universal (Hybrid NAV) |
|-----------------|-------------------|----------------|------------------------|
| New terminal    | Cmd+T (done in `fullstack-b-9`) | Cmd+Alt+T | Mod+. t |
| File browser    | Cmd+O             | Cmd+Alt+O      | Mod+. o |
| Rich prompt     | Cmd+P             | Cmd+Alt+P      | Mod+. p |
| Graph           | Cmd+Shift+M       | (Hybrid NAV only) | Mod+. v |

Cmd+Shift+M is a placeholder per @@Alex's "pick anything
else, cmd+shift+m for ex for now" — confirm or swap when
the task lands. Chrome uses Cmd+Shift+M for the people
menu; chan-desktop overrides Tauri-side. No web fallback
needed for the graph chord since the action is reachable
via the carousel + hamburger + right-click surfaces and
through `Mod+. v`.

#### Removal

Drop these:
* `Cmd+K 1` (was: spawn terminal — Cmd+T now)
* `Cmd+K 2` (was: spawn file browser — Cmd+O now)
* `Cmd+K 3` (was: spawn graph — Cmd+Shift+M now)
* `Cmd+K 4` (was: new file — no top-level chord; available
  via FB context-menu / FB plus button)
* `Cmd+K p` (was: rich prompt — Cmd+P now)

Keep: `Cmd+K t/T` (aliases for terminal, established in
`fullstack-b-9`), `Cmd+K f/F` (Search overlay focus),
`Cmd+K h/H` (help toggle), `Cmd+K <`/`>` (dock toggles),
`Cmd+K Backspace` (kill pane, from phase-7
`fullstack-77`).

#### Rich-prompt context-sensitive behaviour

Cmd+P semantics per @@Alex: "if on terminal, toggle; if
not on terminal, open one with rich prompt". This is
already what `showOrSpawnRichPromptInFocusedPane` does
(the function `Cmd+K p` currently routes to). No new
logic needed — just wire Cmd+P to the same function.

#### Surface unification

Three menus must show the same first-class items:

1. **Empty-pane carousel slide 1**
   (`EmptyPaneCarousel.svelte`) — replace the current
   shortcut table with the four spawn actions.
2. **Pane hamburger menu** (`Pane.svelte::paneMenu`) —
   first-class items at the top.
3. **Empty-pane right-click menu**
   (`Pane.svelte::emptyPaneMenu`) — first-class items at
   the top.

Item ordering: Terminal, File Browser, Rich Prompt,
Graph. Separator. Then existing items (highlight colour
picker, any other current entries).

#### Task (preliminary numbering — slot in Round-2
Wave 1 alongside item 1 + 4)

| Task           | Owner        | Scope                                                                                                |
|----------------|--------------|------------------------------------------------------------------------------------------------------|
| fullstack-a-NN | @@FullStackA | Chord migration (drop Cmd+K 1/2/3/4/p, add Cmd+O/P/Shift+M with triple-binding) + surface unification across the three menus + PaneModeHelp + SERVE_LONG_ABOUT resync |

Single task / single commit since the menus reference the
chord set; splitting would produce a half-state where hints
mismatch the runtime.

### Pre-flight feature toggles (added 2026-05-20)

@@Alex extended item 2's pre-flight spec: the pre-flight
UI exposes per-drive enable/disable toggles for the two
optional indexing layers — BGE-small semantic search +
chan-reports. Both off by default. Both reachable from
both the pre-flight UI AND the CLI.

#### Semantics

* **Default**: both OFF. Lean drive; BM25-only search; no
  reports.
* **Enable at pre-flight**: feature flags get persisted to
  the drive's config; BOOT process kicks off the relevant
  indexing pass alongside BM25.
* **Enable later (via Settings or CLI)**: trigger an
  incremental indexing pass for whichever feature got
  enabled; from that point the feature is active.
* **Disable later (via Settings or CLI)**: **destructive**.
  Drops the per-drive artifacts for that feature:
  * BGE-small disable → drop the per-drive dense vectors
    in chan-drive's index store. **Does NOT delete the
    shared user-config model file** (other drives may
    still use it).
  * chan-reports disable → drop the per-drive report data.
  Requires an explicit confirmation (UI: modal; CLI: `-y`
  flag or interactive prompt). Once dropped, re-enabling
  triggers a fresh indexing/report-generation pass.

#### UI surface (item 2 pre-flight report + Settings)

* **Pre-flight screen**: alongside the existing checks
  (permissions, size class, media class, SCM, conflict
  check) the report includes two toggles:
  * "Enable semantic search (BGE-small, downloads ~63 MB
    on first enable; shared across drives)"
  * "Enable reports (chan-report)"
  Default both off. User confirms the drive registration
  with their chosen state.

  **Explanatory copy above the toggles** (load-bearing —
  @@Alex wants users to understand the baseline before
  they choose what to layer on):

  > Chan will walk this drive, read every markdown file,
  > and build a documentation graph from the wiki-links
  > between them. This graph plus BM25 keyword search is
  > the minimum needed to operate — it can't be disabled.
  >
  > Two optional layers can be enabled on top:
  >
  > * **Semantic search** adds dense-vector embeddings
  >   for find-by-meaning queries. Needs the BGE-small
  >   model (~63 MB, downloaded once + shared across
  >   drives) and produces per-drive vector data.
  > * **Reports** runs code analysis on every file —
  >   language detection (tokei), source-lines-of-code
  >   counts per file + per-language roll-ups, and a
  >   Basic COCOMO estimate on top. Maintained
  >   incrementally from filesystem events. Per-drive.
  >
  > Both layers can be enabled later from Settings, and
  > both drop their per-drive data when disabled (the
  > shared model file stays).

  Final wording is for the implementer (likely
  `fullstack-b-13` SPA copy + `systacean-10` server-side
  pre-flight report schema) to refine; the load-bearing
  beats are: baseline is mandatory + minimum needed to
  operate, two optional layers, both can be flipped
  later, disable drops per-drive data.
* **Settings page**: per-drive section adds toggles for
  both features. Surface the same explanatory note (or a
  shorter version pointing at the pre-flight description).
  Disable triggers a confirmation modal:
  "Disabling will drop the existing <semantic vectors |
  reports> for this drive. Re-enabling later will
  re-index. Continue?" Yes / Cancel.

#### CLI surface

Drive registration:

```
chan add <path> [--semantic-search] [--reports]
```

Default: both off (matching the UI default). Flags are
opt-in; `--semantic-search` triggers the model-download
flow from `systacean-7` if needed (or fails fast with
"download first via `chan index download-model`" if the
flag is set but the model isn't present — same shape as
`enable-semantic`).

`chan add --help` text reflects the same explanatory beats
as the UI pre-flight screen — the baseline filesystem walk
+ markdown read + documentation graph + BM25 always runs;
the two flags add optional layers on top. Each flag's
description names its per-drive footprint so the user knows
what they're committing to.

Per-drive runtime toggles (extending `systacean-7`):

```
chan index enable-semantic  [--path <drive>]        # already in -7
chan index disable-semantic [--path <drive>] [-y]   # adds confirmation; -y to skip
chan reports enable         [--path <drive>]
chan reports disable        [--path <drive>] [-y]   # destructive; -y to skip
```

The `chan reports enable/disable` subcommands are new in
Round 2; `chan index disable-semantic`'s destructive
behaviour (dropping per-drive vectors) is the new piece
on top of `systacean-7`'s landed shape.

`chan index status --json` (from systacean-7) extends to
include report state:

```json
{
  "semantic": { "mode": "bm25" | "hybrid", "model_present": ..., ... },
  "reports": { "enabled": true | false, "data_size_bytes": ... }
}
```

#### Coupling

* `fullstack-b-13` (chan-desktop launcher pre-flight UX)
  surfaces the toggles in the pre-flight screen.
* `systacean-10` (chan-drive pre-flight + boot phase +
  `/api/boot`) wires the toggle preferences into the
  drive config + the BOOT process branches.
* `systacean-7` (already landed in Round 1) is the
  baseline for the semantic-side CLI; reports CLI lands
  as a new task in Round 2 (likely `systacean-N+M` —
  numbering TBD at fan-out).
* `fullstack-a-NN` (Settings page extension) adds the
  per-drive toggles alongside the global Settings from
  `fullstack-a-21`.

### Rich-prompt + bubbles visual redesign + collapse/expand — MOVED TO ROUND 1 (2026-05-20)

@@Alex pulled this into Round 1. Cut as
[`../fullstack-a/fullstack-a-24.md`](../fullstack-a/fullstack-a-24.md).
The original section below is preserved as the scope
sketch; the task file in fullstack-a/ is the operative
spec.

@@Alex wants the rich prompt + every chat / survey bubble
visually re-shaped: rounded corners, floating-pill style
over the terminal, with a default placeholder copy
"Write a multi-line command and Cmd+Enter". Reference image
in the conversation captures the target: a softly-rounded
pill that floats off the bottom edge, NOT a rectangle
coming off the bottom of the screen as today.

#### Visual deltas

* **Rich prompt container**: rounded corners (suggest
  `border-radius: 12px-16px`; final pick per implementer
  visual sanity). Currently floats but is rectangular and
  abuts the bottom edge with no inset.
* **All chat / survey bubbles**: same rounded-corner
  treatment. Composes with `fullstack-b-5` per-Hybrid
  theme overrides cleanly.
* **Margin / inset**: the rich prompt floats off the
  bottom edge with visible terminal underneath, not
  attached. The reference image shows clear breathing
  room on all four sides.
* **Default placeholder copy**: "Write a multi-line
  command and Cmd+Enter" (currently shows a different
  string — implementer updates).

#### Style toolbar relocation

* Style toolbar (the formatting controls — bold, italic,
  etc.) currently appears OUTSIDE the bubble. Move INSIDE.
* **Default**: toolbar OFF (toggle-to-show).
* When ON: toolbar lives at the TOP of the bubble, with
  margin between the toolbar and the prompt body so the
  cursor at line 1 doesn't disappear under the toolbar.

#### New collapse/expand affordance

In addition to the close button, the rich prompt gets a
collapse/expand control:

* **Expanded** (default): current full-height behaviour.
* **Collapsed**: minimal-height bar; chat / survey
  bubbles above get more visible area. Rich prompt stays
  attached + ready to expand.
* **Close** stays as the dismiss path. **Collapse** is
  the "stay-but-out-of-the-way" path.
* Affordance: small chevron / minimize glyph next to the
  close button. Click toggles. Possibly a chord too —
  recommend leaving that decision to the implementer
  unless an obvious one fits the existing chord taxonomy.

#### Task (preliminary numbering)

| Task          | Owner       | Scope                                                                                   |
|---------------|-------------|-----------------------------------------------------------------------------------------|
| fullstack-a-N | @@FullStackA | Rich-prompt + bubble visual redesign (rounded corners, float-pill style, placeholder copy) + style toolbar inside bubble + collapse/expand control. Single commit; all three pieces are visually-linked. |

### Terminal scrollback buffer setting + default TERM — MOVED TO ROUND 1 (2026-05-20)

@@Alex pulled this into Round 1. Cut as
[`../fullstack-b/fullstack-b-11.md`](../fullstack-b/fullstack-b-11.md).
The original section below is preserved as the scope
sketch; the task file in fullstack-b/ is the operative
spec. Round-2 fullstack-b numbering shifts: bundled chan
binary → `fullstack-b-12`, launch-time probe →
`fullstack-b-13`, BOOT desktop → `fullstack-b-14`,
web-marketing port → `fullstack-b-15`.

@@Alex flagged: agents refresh the terminal a lot, so we
need generous scrollback buffers; but unbounded growth is
also wrong. Make it configurable from Settings (NOT inside
the terminal itself — preferences belong in Settings).
Same setting page also exposes the default TERM value
(today's runtime appears to use `xterm-256color`).

#### Sizing decision sketch

Today the scrollback is 20k lines (per `fullstack-b-2`
Round-1 commit). At ~80 cols × ~1.5 bytes/char UTF-8 avg
that's ~2.4 MB per terminal. xterm.js measures
scrollback in LINES, not bytes; we convert MB → lines at
terminal creation time using the current column width as
the per-line byte estimator.

**Default proposal**: **50 MB per terminal**. Generous
enough for agent activity (translates to ~400k lines at
typical width), bounded. Range exposed: 10 MB - 500 MB.

@@Alex confirms the default + range when the task lands.
Defensible alternatives if 50 MB feels too generous /
stingy:

* 25 MB default (200k lines @ typical width).
* 100 MB default (800k lines @ typical width).

The MB unit is what the user sees; xterm.js sees the
derived line count.

#### Default TERM value

Today (per @@Alex's recall) the runtime uses
`xterm-256color`. Setting exposes this as a configurable
value; default stays `xterm-256color`. Alternatives in a
dropdown:

* `xterm-256color` (default, broadest compat with
  256-colour applications).
* `xterm` (basic; for compat with older systems).
* `tmux-256color` (if user runs tmux inside chan's
  terminals).
* `screen-256color` (similar use case).

Or a free-text input for power users (most likely shape
since exotic TERM values are a thing).

#### Settings tab placement

Lives in the Settings page under a "Terminal" section
(create the section if it doesn't exist; pairs with the
`fullstack-b-2` line-height work already there or
adjacent).

#### Task (preliminary numbering)

| Task           | Owner        | Scope                                                                                       |
|----------------|--------------|---------------------------------------------------------------------------------------------|
| fullstack-b-N  | @@FullStackB | New Settings entries: "Terminal scrollback buffer size (MB)" + "Default TERM value". Plumb the MB→lines computation. Persist via existing settings infrastructure. |

Setting applies to NEWLY-spawned terminals (existing
terminals keep their current scrollback until session
restart — simpler than retroactive resize). Document this
in the setting's hint text.

### Editor: trailing-space removal moves from menu to Settings — MOVED TO ROUND 1 (2026-05-20)

@@Alex pulled this into Round 1. Cut as
[`../fullstack-a/fullstack-a-25.md`](../fullstack-a/fullstack-a-25.md).
The original section below is preserved as the scope
sketch; the task file in fullstack-a/ is the operative
spec.

@@Alex flagged: the "auto-remove trailing whitespace"
option currently sits as a checkbox in the editor menu.
That's the wrong surface — preferences belong in
Settings.

#### Scope

* Find the trailing-space checkbox in the editor's
  right-click / hamburger menu (search the editor
  components for the binding).
* Remove from the menu.
* Add an equivalent entry to the Settings page (Editor
  section if it exists; create if not).
* Preserve current behaviour: when on, trailing
  whitespace gets stripped on save; when off, it
  doesn't.
* Default value preserved (whatever it is today).
* Migration: existing user preferences flip cleanly
  from the old storage shape to the Settings persisted
  storage (likely the same key, just exposed through a
  different surface).

#### Task (preliminary numbering)

| Task           | Owner        | Scope                                                                                            |
|----------------|--------------|--------------------------------------------------------------------------------------------------|
| fullstack-a-N  | @@FullStackA | Move trailing-whitespace removal toggle from editor menu to Settings. Single commit; small.    |

Same family as `fullstack-a-21` (Settings panel
extensions) and the chord-migration carousel/hamburger
unification task already in this plan — group these into
a "Settings + menu cleanup wave" within Round-2 Wave-1 if
the lane has bandwidth.

### Chan metadata import/export + drive-state remediation — MOVED TO ROUND 3 (2026-05-20)

@@Alex 2026-05-20 (after the initial Round-2 spec):

> btw the import/export should be in round 3, not
> round 2, ok? we will def need to recycle the session
> before doing all that...

Full spec preserved below as the scope sketch; the
operative spec lives in
[`round-3-plan.md`](round-3-plan.md) under its new
section heading. Round 2 still surfaces the
broken/missing pre-flight states detected by item 2's
BOOT process; the remediation card surfaces a
"Rebuild" + "Skip read-only" pair in Round 2, with
"Import from backup" landing in Round 3 alongside the
import/export feature itself.

(Original Round-2 spec scope sketch retained below for
reference; canonical task spec moves to Round 3.)


@@Alex 2026-05-20:

> Our boot process should always catch [missing /
> broken chan-drive metadata] when chan comes up, and
> offer remediation path; there's also a feature i was
> planning which i tried to spec out previously and it
> didn't land well, which is actually a simple idea of
> 'chan metadata {import|export}' so that we can do a
> 'checkout' of all the metadata for chan-drive that
> would make it possible to export from 1 host to
> another, if they have the same drive layout (e.g.
> same git repo); we should detect via scm and accept
> that the fs will be slightly different, and make our
> import process adapt and rescan if needed.

#### Pairs with item 2 (pre-flight + BOOT)

The import/export feature composes with the BOOT process
from backlog item 2:

* **BOOT detection**: when chan opens a registered drive,
  the pre-flight pass runs; it now includes a chan-drive
  metadata integrity check. Three states surfaced:
  * **Healthy**: metadata present + consistent → BOOT
    completes normally.
  * **Broken**: metadata present but inconsistent
    (corrupt index, partial write, schema drift) →
    surface a "Repair" remediation card with options:
    "Rebuild from scratch", "Import from backup",
    "Skip (open read-only)".
  * **Missing**: no metadata at all (fresh chan on an
    existing drive content tree, or first-time re-open
    after a manual `.chan/` deletion) → surface "Build
    fresh", "Import from backup", "Skip (open
    read-only)".
* **Import as remediation path**: "Import from backup"
  in either broken/missing state consumes a `chan
  metadata export` artifact from elsewhere on disk;
  validates SCM identity (same git repo if both sides
  have one); copies the metadata in; adapts to the
  local FS layout via a rescan if paths differ.

#### Export shape

```
chan metadata export <drive-path> <output-path>
```

* Captures `.chan/` subtree contents: search index, graph
  index, report data, watcher state, per-drive prefs,
  session state — whatever lives under the drive's
  `.chan/` today + whatever Round-2 boot-state schema
  adds.
* Output: a single archive (`.tar.zst` or similar; pick
  what fits the existing model-bundle pattern from
  `crates/chan-server/resources/`).
* Includes a manifest header with:
  * SCM identity (git remote URL + HEAD commit hash, if
    in a git repo).
  * chan version that produced the export.
  * Schema version of the chan-drive metadata.
  * Timestamp + host identifier (informational; not
    load-bearing for identity).

#### Import shape

```
chan metadata import <drive-path> <archive-path> [--rescan]
```

* Validates the manifest:
  * SCM identity matches (same git remote; HEAD commit
    can differ — the rescan picks up the delta).
  * Schema version supported (current or older with a
    migration path; future schema versions refuse
    cleanly).
* Unpacks into the drive's `.chan/` (atomically: temp
  dir + rename, mirroring the existing atomic-write
  pattern for chan-drive writes).
* On `--rescan` (or auto-detect when the working tree
  differs from the manifest's HEAD): triggers an
  incremental indexing pass that reconciles the
  imported metadata against the local FS state.
* Refuses cleanly if SCM identity mismatches ("this
  archive is from a different drive — repository
  remote URL differs").

#### Use cases @@Alex named

1. **Local metadata backup**: power user runs
   `chan metadata export` periodically to a separate
   location; if the local `.chan/` ever corrupts, they
   restore from the backup instead of re-indexing from
   scratch (which on a large drive like the Linux
   kernel can take minutes).
2. **Cross-host session transfer**: desktop-native user
   working across multiple machines (laptop + desktop)
   with the same git repo clone on each. Export
   metadata from machine A, import on machine B; they
   pick up where they left off — search index, graph,
   session state all replicated.
3. **Recovery during pre-flight**: when chan's BOOT
   pass detects broken / missing metadata, "Import from
   backup" is one of the remediation options in the UI
   card.

#### Why @@Alex's "didn't land well" earlier attempt
  
The previous spec attempt (not in this phase's audit
trail; predates phase-8) tried to do too much at once —
likely conflating "export metadata" with "export the
whole drive" or trying to handle cross-drive layout
adaptation generically. The shape here is intentionally
narrow:

* Same logical drive (SCM-identity gate).
* Slightly different FS layouts allowed (different
  absolute paths to the same files, picked up by the
  rescan).
* No attempt to merge concurrent edits across hosts —
  this is "snapshot + replay", not CRDT-style
  reconciliation.

#### UI surfaces — two access paths

@@Alex 2026-05-20 (clarification): the import/export
buttons live in the **Infographics tab for the drive**.
That's the ongoing-access surface — users do backups +
restores from the normal drive-overview flow, not just
during a recovery scenario.

So two access paths:

1. **Infographics tab for the drive**: "Export metadata"
   + "Import metadata" buttons sit alongside the
   drive-overview content (drive name + path + size class
   + language breakdown). This is the canonical
   user-facing surface — normal-flow backup + transfer.
2. **Pre-flight remediation card** (broken / missing
   state): when BOOT detects broken or missing metadata,
   the card surfaces "Import from backup" as one of the
   three remediation options (alongside Rebuild + Skip).
   Same underlying import action; different entry point
   for the "something's wrong" path.

Both surfaces call the same chan-server endpoint + the
same import/export logic. The Infographics tab is the
default user-discoverable affordance; the pre-flight
card is the recovery affordance.

#### Benchmark — Linux kernel round-trip (@@Alex 2026-05-20)

The acceptance benchmark for the import/export feature
is the Linux kernel source tree, exercising both the
clean-clone path AND the post-change re-export path:

```bash
# Cold-index baseline (already named in backlog item 2 BOOT bench notes)
git clone --depth 1 https://github.com/torvalds/linux /tmp/chan-bench-linux
chan add /tmp/chan-bench-linux
chan open  # let BOOT complete; capture full-index wall-clock

# Round-trip #1 — clean
chan metadata export /tmp/chan-bench-linux /tmp/linux-meta-v1.tar.zst
chan metadata import /tmp/chan-bench-linux-mirror /tmp/linux-meta-v1.tar.zst
# (where /tmp/chan-bench-linux-mirror is a fresh clone of the same commit on a different path)
# assert: post-import state == pre-export state across search index, graph, report

# Round-trip #2 — branch + code delta
git -C /tmp/chan-bench-linux checkout <some-active-branch>   # different HEAD
# OR
$EDITOR /tmp/chan-bench-linux/drivers/usb/core/hub.c          # small / medium edit
chan metadata export /tmp/chan-bench-linux /tmp/linux-meta-v2.tar.zst
chan metadata import /tmp/chan-bench-linux-mirror /tmp/linux-meta-v2.tar.zst --rescan
# assert: rescan picks up the FS delta cleanly; post-import state matches the
# new content tree's expected graph + search results
```

Numbers to capture for the benchmark report:

| Metric                                       | Captured by  |
|----------------------------------------------|--------------|
| Cold-index wall-clock                        | BOOT log     |
| Cold-index resident memory peak              | OS sampler   |
| Export wall-clock                            | CLI timing   |
| Export archive size (compressed `.tar.zst`)  | `ls -la`     |
| Import wall-clock (clean)                    | CLI timing   |
| Import wall-clock (with `--rescan` post-checkout) | CLI timing |
| Post-import correctness: search "static inline" → result count matches pre-export | smoke test |
| Post-import correctness: graph node count + edge count match pre-export | smoke test |
| Post-import correctness: report SLOC totals match pre-export | smoke test |

The Linux kernel stress test surfaces:

* **~70k files** — exercises the archive's per-file
  manifest scale.
* **Deep + dense graph** (header cross-includes) — the
  graph data is non-trivial to serialise / restore.
* **chan-report COCOMO output** — non-trivial archive
  payload from the per-language roll-ups.
* **Branch-checkout delta** — `--rescan` adaptation to FS
  changes is the load-bearing reliability check.

Acceptance bar (rough targets, @@Alex confirms when the
benchmark runs):

* Export wall-clock < 30 s on a warm SSD.
* Import wall-clock (clean) < 60 s.
* Import wall-clock with `--rescan` (small / medium
  delta) < 90 s.
* Compressed archive size: order-of-magnitude estimate
  100-500 MB for the Linux kernel; tunable via zstd
  level. If meaningfully over 500 MB, profile + tune.

If the numbers come in worse, the audit pass (Round-3
Track-3 efficiency) re-visits.

#### Coupling with item 4 (Infographics tab container)

Backlog item 4 (`fullstack-a-N` Infographics tab
container + carousel content redesign) is the
**prerequisite** for the Infographics surface. Sequencing
inside Round 2:

1. Item 4 lands first — Infographics tab type +
   `Cmd+. 9` spawn (or whichever chord lands post chord
   migration) + initial slide content.
2. Then the metadata import/export buttons get added to
   the drive-overview slide of the Infographics tab.
3. In parallel, BOOT integration + pre-flight remediation
   card from item 2 lands the recovery surface.

#### Task (preliminary numbering)

| Task          | Owner        | Scope                                                                                |
|---------------|--------------|---------------------------------------------------------------------------------------|
| systacean-N   | @@Systacean  | `chan metadata export` + `chan metadata import` CLI + the `.tar.zst` manifest shape + chan-server endpoints |
| systacean-N+1 | @@Systacean  | BOOT integration: detect broken / missing metadata states; expose remediation API   |
| fullstack-a-N | @@FullStackA | Infographics tab drive-overview slide: add Export / Import buttons consuming the chan-server endpoints |
| fullstack-b-N | @@FullStackB | Pre-flight UI: surface the three states + remediation card with the three options (Rebuild / Import / Skip read-only) |

Two SPA-side surfaces (Infographics tab + pre-flight
card) live in different lanes since they're different
component contexts; both consume the same underlying API.

@@Alex's framing: "very easy to implement and reproduce
with our local tools today." Sized as a small wave inside
Round 2; pairs with items 2 (pre-flight + BOOT) + 4
(Infographics tab) so it composes naturally with the
two structural changes in this round.

### Hybrid back-side revisited — flip becomes per-surface configuration (added 2026-05-21)

Source: [`../alex/hybrid-revisited.md`](../alex/hybrid-revisited.md).
@@Alex's spec for the Hybrid back-side semantics change. The
back of a Hybrid pane stops being "more tabs" and becomes a
**per-surface configuration surface** scoped to the type of
the currently-selected front tab. Inspiration: Propellerheads
Reason (90s music software) — flip the rack to see the wiring
behind the front panel.

#### Design summary

* **Front side**: still the content tabs (terminals, files,
  FB, graph). Unchanged from today.
* **Back side**: a configuration surface specific to the type
  of the currently-active front tab. NOT another collection
  of content tabs.
* **Flip semantic**: flip reveals settings for the surface
  family the user is in. Switching front tab while flipped
  swaps the back's content to the new tab's settings.
* **Theme**: drop front/back independent theme (the override
  landed in `-b-5`). Both sides of a Hybrid share the same
  per-Hybrid theme. The hamburger theme toggle from `-a-27`
  flips both sides at once; per-pane theme still differs
  from other panes.
* **Flip animation**: keep `-a-22`'s half-flip animation.
  Only the WHAT-IS-BEHIND changes; the HOW-IT-LOOKS-FLIPPING
  stays.

#### Per-surface back-side scope

| Front-tab type | Back-side content                                                                                        |
|----------------|----------------------------------------------------------------------------------------------------------|
| Hybrid Terminal | All terminal settings currently in the Settings overlay (scrollback MB, default TERM, font, etc.). Carries an explicit warning: "these settings apply to ALL terminals, not just this one". |
| Hybrid Editor   | Editor settings currently in Settings: Theme, Layout, Date Pills, On Save (per `-a-25`).                |
| Hybrid Graph    | Legend grid: `[Node] [Colour]` rows for each node type chan supports — `Dir`, `File (Regular, Code, Document, Contact)`, `Hashtag`, `Mention`, `Language (Code)`. |
| Hybrid File Browser | Placeholder for now ("FB has no per-surface configuration; reserved for future use" or similar). |

Each back-side surface carries the family name as its title
band (e.g., "Hybrid Terminal" / "Hybrid Editor" / "Hybrid
Graph" / "Hybrid File Browser"). The title is the visual
anchor that confirms which surface's settings the user is
looking at after the flip.

#### Settings-overlay residue

The Settings overlay is NOT going away entirely — drive-level
+ app-level settings stay there. What MOVES:

* **Out of Settings, into Hybrid Terminal back**:
  scrollback MB (from `-b-11`), default TERM (from `-b-11`),
  any future font controls (from the deferred bundled-font
  work).
* **Out of Settings, into Hybrid Editor back**:
  Theme, Layout, Date Pills, On Save (from `-a-25`).
* **Stays in Settings**: drive-level toggles (semantic search
  per `-a-21`, future per-drive Reports), app-level config
  (window state per `-b-1`), About / attribution (Source Code
  Pro OFL from `-b-12`, future markmap MIT).

Settings overlay still spawns via `Cmd+,` per macOS convention
(established by `-a-7`).

#### Churn note — v0.11.1 work partially relocates

Several v0.11.1-landed Settings entries get migrated out of
the Settings overlay into their respective Hybrid back-sides:

* `-b-11` Terminal section (scrollback MB + default TERM)
  → Hybrid Terminal back.
* `-a-25` On Save toggle → Hybrid Editor back.
* Future editor settings (Theme, Layout, Date Pills if any
  of those entries exist or land later) → Hybrid Editor back.

This is acceptable churn — the Settings entries themselves
keep their persistence shape (same `Preferences` fields,
same autosave wire); only the UI mounting point changes.
Worth flagging in the migration task that the
storage-vs-presentation split makes this a UI-only relocation
in code, not a data migration.

#### Implementation breakdown (preliminary)

Substantial. Likely 3-5 tasks across `-a-N` slots:

* **Task A — Hybrid back-side architecture refactor**
  (@@FullStackA): introduce the back-side configuration-
  surface concept. New per-surface component type
  (`HybridTerminalConfig.svelte` / `HybridEditorConfig.svelte` /
  `HybridGraphConfig.svelte` / placeholder). Pane.svelte's
  flip behaviour reads the active front-tab type + mounts the
  matching back-side component. Drop front/back independent
  theme + tabs collections; back side becomes single config
  view, not a tab strip.
* **Task B — Terminal Settings migration** (@@FullStackA):
  move `-b-11`'s Terminal section out of `SettingsPanel.svelte`
  into `HybridTerminalConfig.svelte`. Settings storage shape
  unchanged. Warning copy about "applies to all terminals"
  added.
* **Task C — Editor Settings migration** (@@FullStackA):
  move the Editor section (Theme / Layout / Date Pills / On
  Save) out of `SettingsPanel.svelte` into
  `HybridEditorConfig.svelte`. Settings storage shape
  unchanged.
* **Task D — Hybrid Graph legend grid** (@@FullStackA): build
  the `[Node] [Colour]` legend grid for the 6 node-type
  families in `HybridGraphConfig.svelte`. Read colour tokens
  from the graph's existing colour-scheme map. Composes with
  `-a-33`'s graph-from-here default + ancestor breadcrumb
  work.
* **Task E — Drop front/back independent theme**
  (@@FullStackA): simplify `-b-5`'s per-Hybrid theme override
  to be SINGLE per-Hybrid value (no front/back split). Update
  `-a-27` hamburger theme toggle to flip the single value.
  Documentation / test updates.

#### Open questions for @@Alex (survey at scoping time)

1. **Per-tab vs per-surface scope**: confirmed per the bug
   bullet "settings impact all terminals, not just the
   current terminal" — settings are PER-TYPE (one terminal
   config back applies to every terminal in this Hybrid),
   not PER-TAB. Confirmed.
2. **Where does Hybrid File Browser back land in v1**: empty
   placeholder ("reserved for future use" message), or a
   minimal first config (e.g., default watcher-scope mode
   per `-b-6`)? Recommend placeholder for v1; add config
   later if affordances emerge.
3. **Hybrid Search back**: search overlay isn't a Hybrid
   surface today (it's a global overlay, spawn via `Cmd+K F`).
   Does it ever become one? @@Alex's spec lists 4 (5 with
   markmap if/when it lands as a tab type) Hybrid surfaces +
   the search overlay separately. Recommend: search overlay
   stays out-of-Hybrid for now; revisit only if it earns the
   per-Hybrid scope.
4. **Sequencing within Round-2**: Wave 2 or Wave 3? This is
   a major SPA architecture change; pairing with the
   rich-prompt session-evolution wave 2 might be too much
   surface area in one wave. Recommend: split — Task A
   (architecture refactor) rides Wave 2 as a hard-prereq;
   Tasks B/C/D/E land in Wave 3. Or all 5 in Wave 3 if Wave 2
   feels full.

#### Cross-impact with this session's Round-2 Wave-1 work

* **No conflict** with Wave-1's signed-DMG pipeline (CI +
  Systacean + chan-desktop bundling) — Wave-1 is build-
  pipeline work, Hybrid-revisited is SPA UI architecture.
* **Composes with `-a-32`'s chord migration** — chord set
  unchanged; only the back-side behaviour shifts.
* **Composes with `-a-22`'s flip animation** — animation
  unchanged; only the destination changes.
* **Simplifies `-b-5`** — front/back theme split was load-
  bearing for the prior "back is another tab collection"
  shape; under the new shape, single per-Hybrid theme is
  sufficient.
* **Markmap feature** (filed earlier today) — third StyleToolbar
  mode (wysiwyg / source / markmap) is orthogonal to the
  Hybrid back-side refactor. Markmap is read-only content
  within the Hybrid Editor front; Hybrid Editor back keeps the
  same config surface regardless of which front-side mode is
  active.

### Wave 2 — feature track (sequenced after wave 1 stabilises)

| Task           | Owner        | Source                                                                              |
|----------------|--------------|-------------------------------------------------------------------------------------|
| fullstack-a-23 | @@FullStackA | Item 4 — Infographics tab container (lift carousel out of empty pane)               |
| fullstack-a-24 | @@FullStackA | Item 4 — empty pane minimal landing (chan logo + Hybrid NAV hint)                   |
| fullstack-a-25 | @@FullStackA | Item 1 — drive metadata carousel content redesign (lives inside Infographics tabs)  |
| fullstack-b-13 | @@FullStackB | Item 2 — drive pre-flight checks + BOOT process (chan-desktop launcher side) — see "Pre-flight feature toggles" below |
| systacean-10   | @@Systacean  | Item 2 — chan-drive pre-flight inspection + boot phase enum + `/api/boot` — see "Pre-flight feature toggles" below |
| fullstack-a-26 | @@FullStackA | Item 3 — screensaver overlay component + Matrix theme + Settings panel surface      |
| systacean-11   | @@Systacean  | Item 3 — PIN hashing helper (SHA-256 + per-install salt) + config schema add        |
| fullstack-a-27 | @@FullStackA | Item 3 — Castaway theme (after the repo audit)                                      |
| systacean-12   | @@Systacean  | Item 5 — chan config currency audit + schema cleanup + migration                    |
| fullstack-a-28 | @@FullStackA | Item 6 — chan-desktop first-launch manual UX                                        |
| fullstack-b-14 | @@FullStackB | Item 6 — port chan.app source into `web-marketing/`                                 |
| systacean-13   | @@Systacean  | Item 6 — DNS cutover plan + TLS story + VPS decommission timeline                   |
| ci-9           | @@CI         | Item 6 — CI pipelines for marketing site + manual + release-tag manual-bundle       |
| architect-2    | @@Architect-led | Item 6 — `docs/manual/` content (probably staggered across multiple cuts)        |

### Round-2 close

Round 2 closes with the first signed+notarized DMG shipping
under `chan-v0.12.0` (or whatever version @@Alex picks). At
that point all six platform installers (macOS DMG, Linux
AppImage + .deb + .rpm, Windows MSI/EXE) ship via GitHub
Release. Repo still private; users get binaries via direct
download from the release page.

After Round 2 closes → agent recycle → Round 3 fan-out per
[`round-3-plan.md`](round-3-plan.md).

## Capacity assumptions for Round 2

Same six slots + @@Architect dispatcher, recycled between
rounds:

* @@FullStackA — frontend feature work; Settings panel +
  carousel + screensaver UI.
* @@FullStackB — chan-desktop / Tauri lane; bundled-binary
  + selection + first-launch UX.
* @@Systacean — config schema, indexer, chan-drive pre-
  flight, signing-key rotation, BOOT API.
* @@CI — signing workflow + DMG dry-run + release pipeline +
  manual-bundle CI from item 6.
* @@WebtestA / @@WebtestB — feature verification per
  walkthrough; first-DMG installs verified on real Macs.

Cross-cutting decisions sit with @@Architect; brief @@Alex
on each before fan-out.

## What this plan is NOT

* A commit-grouping plan. That gets cut as
  `commit-plan-v0.12.0.md` (or whatever) at Round-2 close.
* A push trigger. The first GitHub Release at Round-2 close
  is gated on @@Alex's explicit "cut it" signal after the
  signed-DMG smoke-test passes.
* A scope-creep gate. Bugs that surface during Round-2
  walkthroughs land in Round 2 if they're regressions or
  release-blockers; otherwise roll to Round 3 or later.