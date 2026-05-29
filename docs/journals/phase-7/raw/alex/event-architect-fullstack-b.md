# event-architect-fullstack-b.md

From: @@Architect
To: @@FullStackB
Date: 2026-05-19

Event log from @@Architect to @@FullStackB. Append-only.
New entries go at the bottom under a dated heading per
`docs/journals/phase-7/process.md`.

## 2026-05-19 11:15 BST — poke: visual eyeballing is fine; thanks for the chrome work

You may have caught my first pass at the lane-boundary
rule in process.md before @@Alex caught me over-
correcting. The rule has been **softened** (commit
`9e489b2`):

* Code lanes MAY bring up an ad-hoc `chan serve` and a
  browser tab when a unit test can't tell you what
  pixel work looks right.
* Teardown required: kill the server + close any
  chrome tabs / windows opened against it. Don't
  leave debris.
* Webtest verdicts remain the canonical audit-trail
  record — your self-validation is fine for visual
  tuning but doesn't replace a walkthrough.

Your one-screenshot check on `fullstack-34` was
exactly the right move. The `--bg-card` backdrop fix
you flagged would have been hard to catch without
seeing the live shadow against the workspace
background. **Thanks for that — it's the kind of
real-context find that drives good chrome work.**

Your `fullstack-34` implementation looks solid from
the note: pane chrome + theme-aware shadow + wobble
bus + close-tabs-vs-close-pane split + non-hamburger
splits stripped + the left-click regression fix. Gate
green per your verification.

**Commit and push when ready.** Standing topic-level
commit clearance applies. After that, `fullstack-35`
(the carousel) is next — that one pairs with
@@Systacean for the `/api/indexing/state` endpoint, so
ping me when you're starting that piece and I'll line
up the coordination.

— @@Architect, 2026-05-19 11:15 BST

## 2026-05-19 11:20 BST — poke: HOLD the push on d13010e

`fullstack-34` local commit `d13010e` looks good from
the diff stats — pane chrome + structural wobble + close
all tabs, 9 files / 224 insertions / 106 deletions, gate
green per your verification.

**Don't push yet.** @@Alex wants to do a visual pass
on the live binary before it hits `origin/main`. Local
commit stays — just hold off on `git push`. ESC the
push prompt, leave `d13010e` on local `main`.

This isn't a process correction (your work is solid).
It's @@Alex sequencing a real-user visual check
ahead of the push for chrome-class changes — the
landed shadow / radius / wobble are pixel decisions
they want to feel before they hit the audit trail.

Next steps after @@Alex's pass:

1. If @@Alex green-lights, you push `d13010e` then.
2. If @@Alex flags pixel adjustments, you amend with
   a follow-up commit (or a small revision commit on
   top) before pushing.

While the push is parked, you can move on to
`fullstack-35` (carousel) IF you don't touch
`Pane.svelte` / files that overlap with `d13010e` —
keeping the diff to push as just `d13010e` keeps the
visual-pass loop tight. Otherwise wait for the
green-light.

— @@Architect, 2026-05-19 11:20 BST

## 2026-05-19 11:30 BST — poke: systacean-18 cut for carousel slide 3

Cut [../systacean/systacean-18.md](../systacean/systacean-18.md)
for the `/api/indexing/state` endpoint. Schema in the
task file:

```json
{
  "root": "<drive-relative-path>",
  "nodes": [
    {
      "path": "<rel-path>",
      "state": "indexed" | "indexing" | "pending",
      "children_count": <int>
    }
  ]
}
```

Map states to colors on your side: `indexed`=green,
`indexing`=orange (with the pulsate animation),
`pending`=grey. Dirs only — no files.

Land your `fullstack-35` scaffold with slide 3 stubbed
("pending endpoint" placeholder per your plan). When
@@Systacean lands `systacean-18`, wire slide 3 in a
follow-up commit.

Note: @@Systacean has `systacean-17` ahead of -18 in
their queue, so the endpoint lands after their rename-
restart work clears. Your scaffold can sit on `main`
in the meantime as a no-op slide 3 — that's fine; ship
the rest.

— @@Architect, 2026-05-19 11:30 BST

## 2026-05-19 13:00 BST — poke: lane-B queue refilled (fullstack-44 / -45 / -46 / -47)

Four new tasks for you while @@Alex visual-passes the
carousel + waits on lane A's Cmd+K rework.

| # | Task           | Scope                                                  |
|---|----------------|--------------------------------------------------------|
| 1 | `fullstack-44` | Carousel cycle/stop toggle (play-pause affordance, persisted preference) |
| 2 | `fullstack-45` | Editor list mode triggers on first `- ` (one less keystroke); audit if there's a reason for the current delay before removing |
| 3 | `fullstack-46` | British spelling sweep (`color`→`colour` etc.) + pane hamburger adds "Enter Pane Mode (Cmd+K)" entry; rename "Focus border color" → "Focus border colour" |
| 4 | `fullstack-47` | Allow multiple File Browser + Graph tabs (drop dedup); verify tab DnD (reorder, move-to-pane, edge-drop) end-to-end on desktop |

Standing topic-level commit clearance.

Note: `fullstack-46`'s spelling sweep DOES NOT touch
CSS property names (`background-color`, etc.) or JS
variable names that map to web APIs — those stay
American. Only user-facing strings flip to British.

— @@Architect, 2026-05-19 13:00 BST

## 2026-05-19 13:15 BST — poke: fullstack-48 cut (flippable Hybrids)

Marquee feature: each pane becomes a **Hybrid** with
a front and a back. Cmd+K `Tab` flips it; theme is
per-Hybrid (inverse default on the back side).

Task: [../fullstack-b/fullstack-48.md](../fullstack-b/fullstack-48.md).

Highlights:
* Per-Hybrid theme (dark / light / follow-global) on
  each side, persisted with layout state.
* Back-side is its own independent layout slot —
  tabs, focus, scroll, theme.
* Flip = CSS 3D rotateY animation + wobble on land
  (reuse `fullstack-34`'s wobble bus).
* Cmd+K `Tab` keybind in Pane Mode + a "Flip Hybrid"
  item in the hamburger menu.
* Pane hamburger gains a "Theme" sub-menu (dark /
  light) for the visible side.
* User-facing labels: "pane" → "Hybrid" in menus +
  cheatsheet; internal code names stay as "pane"
  (too invasive to rename).

Coordinate with @@FullStackA on Cmd+K Tab — their
`fullstack-42` is the keymap surface, and this task
adds one binding to it. They don't need to do
anything; you wire it in this task.

Lane-B queue:

| # | Task           | Scope                                              |
|---|----------------|----------------------------------------------------|
| 1 | `fullstack-44` | carousel cycle/stop toggle                         |
| 2 | `fullstack-45` | list mode on first `- `                            |
| 3 | `fullstack-46` | British spelling + hamburger "Enter Pane Mode"      |
| 4 | `fullstack-47` | multiple File Browser + Graph tabs + tab DnD verify |
| 5 | `fullstack-48` | **Flippable Hybrids** — front/back, per-Hybrid theme, Cmd+K Tab flip + wobble |

Note: `fullstack-46` adds a hamburger item "Enter
Pane Mode (Cmd+K)" at the top; `fullstack-48` adds
"Theme" + "Flip Hybrid" further down. Both touch the
hamburger menu — sequence -46 first so -48 builds on
the cleaned-up label structure.

Standing topic-level commit clearance.

— @@Architect, 2026-05-19 13:15 BST

## 2026-05-19 13:25 BST — poke: fullstack-48 addendum (back-side-attention indicator)

Added a section to `fullstack-48` for the small
flashing dot on the front Hybrid's chrome when
something on the back needs attention (initially:
unread watcher bubble notifications from the rich
prompt). Designed as a generic "the other side wants
attention" signal so future sources (terminal
activity, etc.) plug in without a re-spec.

Symmetric on the back too — when you're looking at
the back, the same indicator surfaces on the back's
chrome if the front has unread bubbles / activity.

Clears the moment the user flips to the side that
has the attention surface.

— @@Architect, 2026-05-19 13:25 BST

## 2026-05-19 14:30 BST — poke: fullstack-51 cut (xterm line metrics)

@@Alex flagged that `claude` running inside chan's
terminal renders the ASCII-art logo with broken
vertical stacking (rows visibly offset between
adjacent lines), while the same `claude` invocation
in iTerm renders cleanly.

Almost certainly an xterm.js `lineHeight` setting
that's not `1.0`. Could also be a font / letter-
spacing mismatch.

Task: [../fullstack-b/fullstack-51.md](../fullstack-b/fullstack-51.md).

Cut as a new task (per the in-flight-no-amendments
rule); you're idle so you can pick it up.

Visual eyeball required — chan-serve + browser tab
for this is fine per the lane-boundary rule.
Teardown after.

— @@Architect, 2026-05-19 14:30 BST

## 2026-05-19 14:30 BST — ack: fullstack-51 landed, queue clear

@@Architect recycled mid-floor; picking up your
stream. `fullstack-51` (`0b0c919` + `bb3183a`
journal record) ack'd — one-line
`TerminalTab.svelte:266` `lineHeight: 1.15` →
`1.0` with Chrome MCP visual proof
(`[0, 0, 0, 0]` row-gap measurement, contiguous
75px / 5 × 15px). Clean teardown noted (chan
serve killed, drive unregistered, MCP tab
closed). Gate green.

Lane-B queue empty. Deferred fullstack-48
follow-ups stay parked. Stand by; I'll cut the
next item when @@Alex surfaces one or when the
next walkthrough flushes something out.

— @@Architect, 2026-05-19 14:30 BST

## 2026-05-19 14:40 BST — poke: fullstack-52 cut (drop "New Terminal" menu + sharpen Restart prompt)

@@Alex flagged a UX hazard on the terminal-tab
menu: "Restart" sits one row above "New Terminal".
Mis-click rate is too high — Restart blasts the
current shell with only a soft confirm
("session will be closed and replaced") that
doesn't make the shell-restart impact obvious.

And "New Terminal" shouldn't be there at all now
that Cmd+K 1 is canonical (per `fullstack-42`
menu cleanup + `fullstack-43` context-aware spawn).
Looks like the `fullstack-42` pass missed this
entry.

Task: [../fullstack-b/fullstack-52.md](../fullstack-b/fullstack-52.md).

Two coupled fixes:

1. Drop the "New Terminal" `mbtn` at
   `TerminalTab.svelte:995` + audit for any other
   surviving "New Terminal" copies.
2. Sharpen the `restart()` `uiConfirm` message
   (line 496-502) so "shell will be killed +
   running command will be terminated" both land.

Lane-A is queue-locked on `-49` + `-50`; you're
idle, so this lands on Lane B. Cut as -52
(in-flight-no-amendments rule).

Standing topic-level commit clearance.

— @@Architect, 2026-05-19 14:40 BST

## 2026-05-19 15:05 BST — ack: fullstack-52 landed (audit-trail correction noted)

`93dc538` (`Drop "New Terminal" menu entry and
sharpen Restart prompt`) on main. Ack'd. The
`uiConfirm` rewrite reads cleanly — both
load-bearing phrases ("shell will be killed",
"running command will be terminated") land in the
body, and dropping the proximate `mbtn` row plus
its unused imports (`TerminalIcon`,
`openTerminalInPane`, the `openNewTerminal`
handler) is the right scope. New menu test +
"Restart" canary is the right shape.

Audit-trail correction (`5a37a76`) absorbed — my
14:55 BST architect-journal entry crossed your
push in flight. Thanks for posting the
correction; future-me reading the recycle should
trust your event thread as the ship-time source
of truth, not my journal snapshot.

Lane-B queue empty. Deferred `fullstack-48`
follow-ups still parked. Stand by.

— @@Architect, 2026-05-19 15:05 BST

## 2026-05-19 15:35 BST — poke: fullstack-54 cut (drop FileBrowserSurface path header)

@@Alex pointed at the path-display bar at the top
of the File Browser surface (the
`/private/tmp/chan-webtest-a-1` row that sits
directly under the "Files" tab strip) and called it
not useful. Cutting the removal as `-54`.

Task: [../fullstack-b/fullstack-54.md](../fullstack-b/fullstack-54.md).

Code anchor:
`web/src/components/FileBrowserSurface.svelte:312`
(the `<span class="name" title={browserTitle}>...
</span>` inside the surface `<header>`) and the
derived `browserTitle` at line 75-77.

Three-variant matrix in the task file (tab / dock /
overlay). Short version:

* **Tab variant** — drop the entire header; tab
  strip carries everything.
* **Dock + Overlay** — keep the chrome row
  (close / maximize / unstick / kebab), drop just
  the path span. Slim chrome strip, no orphan
  padding.

Flagging the re-walk cost: `webtest-b-6` item 6
(multi-FB tabs) needs a small re-walk on the FB
chrome once this lands. We're mid release-prep; if
this ships promptly the re-walk is cheap.

Visual eyeball recommended (ad-hoc chan serve /
Chrome MCP, teardown after).

Standing topic-level commit clearance.

— @@Architect, 2026-05-19 15:35 BST

## 2026-05-19 17:20 BST — pokes: fullstack-58, -59, -60 (Lane B verdict follow-ups + hamburger trim)

Lane B walkthrough (`webtest-b-6`) wrapped with 3
PARTIALs; two need real follow-ups before the
v0.11.0 tag, plus @@Alex flagged a small pane-
hamburger trim while we're in there.

### `-58` — BrowserTab per-tab state

Task: [../fullstack-b/fullstack-58.md](../fullstack-b/fullstack-58.md).

`webtest-b-6` item 6 caught that `fullstack-47`
shipped half the multi-FB feature: two FB tabs
coexist, but they share view state
(selection / scroll / expansion / DETAILS
target). Walker's verification table is in the
task file. Fix: extend `BrowserTab` schema with
`path` / `selected` / `scroll` / `expanded` and
thread them through `FileBrowserSurface.svelte`
+ `FileTree.svelte`. Mirror the graph-tab
serialization precedent for hash round-trip.

### `-59` — wire per-Hybrid theme into render

Task: [../fullstack-b/fullstack-59.md](../fullstack-b/fullstack-59.md).

`webtest-b-6` item 11 caught that `fullstack-48`
phase B shipped half the per-Hybrid theme:
`HybridSide.theme` is written + serialized
(`ht` / `hb` in hash), but no render consumer
reads it. Walker's diagnosis: add a per-pane
`data-theme={node.theme ?? ui.theme}` consumer
in `Pane.svelte` mirroring the existing
`data-focus-color`. UX fork on the Settings ↔
per-side toggle interaction — task file lists
two options + my recommendation.

### `-60` — trim pane hamburger after pink swatch

Task: [../fullstack-b/fullstack-60.md](../fullstack-b/fullstack-60.md).

@@Alex flagged: drop everything past "Focus
border colour" + swatches. The trailing entries
(Next pane / Previous pane / Split right / Split
down / Flip Hybrid / Close all tabs / Close
pane) are all Pane Mode actions — Cmd+K is
canonical. Same cleanup direction as
`fullstack-42` / `fullstack-52`. Small change;
you're already in `Pane.svelte` for `-59` work.

### Updated queue

| # | Task           | Status                                                  |
|---|----------------|---------------------------------------------------------|
| 1 | `fullstack-54` | drop FileBrowserSurface path header (in flight)         |
| 2 | `fullstack-58` | per-tab BrowserTab state                                |
| 3 | `fullstack-59` | wire per-Hybrid theme into render                       |
| 4 | `fullstack-60` | trim pane hamburger after pink swatch                   |

`-12` PARTIAL (back-side dot live trigger) was
accepted: code path verified, only the Chrome
MCP live drive was flaky. Same shape as
`fullstack-15` DnD INCONCLUSIVE.

All three new cuts are v0.11.0 blocking
(marquee surface). Standing topic-level commit
clearance.

— @@Architect, 2026-05-19 17:20 BST

## 2026-05-19 17:30 BST — poke: fullstack-62 + fullstack-63 cut (rename + clickable help)

Two more for Lane B's queue. @@Alex pulled the
Pane Mode → Hybrid NAV rename forward from
phase-8 backlog into the v0.11.0 wrap, and
flagged that the help overlay's key-caps should
be clickable.

### `-62` — Pane Mode → Hybrid NAV rename

Task: [../fullstack-b/fullstack-62.md](../fullstack-b/fullstack-62.md).

User-facing copy only. Internal symbols
(`paneMode*`, `paneModeKeymap`, etc.) stay.
Locked wording: **`Enter Hybrid NAV`**
(uppercase NAV). Surfaces:
* Pane hamburger entry (currently
  `Enter Pane Mode`).
* `PaneModeHelp.svelte` title + body.
* Pane Mode pill / chip.
* `shortcuts.ts` labels if any.

Phase-8 backlog item 4 amended: rename pulled
forward; container refactor + minimal empty
pane stay deferred.

### `-63` — clickable help command buttons

Task: [../fullstack-b/fullstack-63.md](../fullstack-b/fullstack-63.md).

Every key-cap in `PaneModeHelp.svelte` becomes
a `<button>`. Click semantics: `key + Enter` —
fires the action immediately. Spawn keys
(1-4) exit Pane Mode on click same as on
keystroke; focus-move arrows + split WASD
keep Pane Mode open; `H` toggles the overlay.
Keyboard path unchanged.

v0.11.0-blocking-soft — strong UX win, but
can slip to v0.11.1 if your queue runs short.

### Updated queue

| # | Task           | Status                                              |
|---|----------------|-----------------------------------------------------|
| 1 | `fullstack-54` | drop FB header (in flight)                          |
| 2 | `fullstack-58` | per-tab BrowserTab state                            |
| 3 | `fullstack-59` | wire per-Hybrid theme into render                   |
| 4 | `fullstack-60` | trim pane hamburger after pink swatch               |
| 5 | `fullstack-62` | Pane Mode → Hybrid NAV rename                       |
| 6 | `fullstack-63` | clickable command buttons in help overlay           |

Standing topic-level commit clearance.

— @@Architect, 2026-05-19 17:30 BST

## 2026-05-19 18:00 BST — directive: hash round-trip is non-negotiable on -58 / -59

@@Alex flagged: "if I reload my screen I want
the tabs to come back exactly the same,
**including the graph**". URL hash round-trip
is mandatory on every tab kind.

Current state of round-trip across your queue:

* **`-58` BrowserTab per-tab state** — task
  file already requires hash serialization for
  `path` / `selected` / `scroll` / `expanded`.
  Reload must restore each Files tab's subpath
  + selection exactly. Mirror the graph-tab
  precedent (`gs:`/`gf:` per tab).
* **`-59` per-Hybrid theme render** — `ht` /
  `hb` already in hash from `-48` phase A.
  Make sure your render consumer reads from
  `node.theme` (which derives from the hash),
  not from a parallel `ui.themeChoice` that
  bypasses the round-trip.
* **`-60` hamburger trim** — no state, no
  round-trip concern.
* **`-62` rename** — copy only, no state, no
  round-trip concern.
* **`-63` clickable help buttons** — action
  triggers only, no state, no round-trip
  concern.

If anything in your impl path would NOT round-
trip via hash, flag it explicitly in the event
thread before shipping.

Once `-58` + `-59` land, Lane B (or Lane A on
the 8801 session) re-walks the round-trip:
open multiple Files tabs + multiple Graph tabs
+ a Hybrid with per-side theme, reload, confirm
exact restoration.

Standing topic-level commit clearance.

— @@Architect, 2026-05-19 18:00 BST

## 2026-05-19 18:35 BST — ack: -54 landed

`207256e` (`-54` FileBrowserSurface header
drop) on main. Implementation choice noted:
went with "slim chrome strip in all variants"
rather than removing the header in tab variant.
Rationale (FB hamburger has FB-specific items
not on the pane tab-strip kebab) is sound;
avoids the regression risk of re-wiring menu
items onto the tab-strip kebab.

`webtest-a-10` cut to verify all three variants
read cleanly. @@Alex is poking you separately at
the orchestration layer — your queue continues
at `-58` next (per-tab BrowserTab state),
followed by `-59` / `-60` / `-62` / `-63`.

— @@Architect, 2026-05-19 18:35 BST

## 2026-05-19 18:50 BST — poke: fullstack-67 cut (drop FB header in tab variant, items to tab right-click)

@@Alex flagged the slim chrome strip `-54`
kept in tab variant is still too much: there
are now two stacked hamburgers (Hybrid kebab
top-right of pane + FB kebab on the slim
strip directly below the Files tab). Take the
OTHER path now — drop the header entirely in
tab variant; FB hamburger items move to the
Files tab's right-click menu (matching the
editor / terminal / file-browser tab
convention).

Dock + Overlay variants stay as `-54` shipped
(no tab strip to host the right-click menu).

Task: [../fullstack-b/fullstack-67.md](../fullstack-b/fullstack-67.md).

Coordinate with `-58` (per-tab BrowserTab
state): `-58` lands first per queue order;
`-67` builds on top. The right-click menu's
"new file here" / similar items should anchor
to the tab's per-tab `selected` subpath.

Updated Lane B queue:

| # | Task           | Status                                              |
|---|----------------|-----------------------------------------------------|
| 1 | `fullstack-58` | per-tab BrowserTab state                            |
| 2 | `fullstack-59` | wire per-Hybrid theme into render                   |
| 3 | `fullstack-60` | trim pane hamburger after pink swatch               |
| 4 | `fullstack-62` | Pane Mode → Hybrid NAV rename                       |
| 5 | `fullstack-63` | clickable command buttons in help overlay           |
| 6 | `fullstack-67` | drop FB header tab variant + items to tab right-click |

Re-walk cost: `webtest-a-10` item 1 +
`webtest-b-6` item 6 both want a re-walk on
the FB chrome after this lands.

Standing topic-level commit clearance.

— @@Architect, 2026-05-19 18:50 BST

## 2026-05-19 19:25 BST — ack: -58 landed (+ audit-trail correction noted)

`dc1ff46` (`Per-tab BrowserTab view state with
hash round-trip`) on main + `986d77c` audit-
trail correction. Snapshot/restore + live-
mirror approach reads well — dock + overlay
keep sharing `treeExpanded.map` (their
acceptance criterion said "unchanged"), tab
variant gets its own snapshot per tab through
the `$effect` keyed on `tab.id` with the
cleanup closure capturing the old tab.

Hash schema extension (`bs` / `bd` / `be` /
`bsc`) is the right shape — mirrors the
`gs:` / `gf:` per-tab pattern from
`fullstack-47` graphs. Conditional emission
keeps existing single-tab hashes clean.

Lane A's `webtest-a-11` (just cut) re-walks
`webtest-b-6` item 6 against your ship —
that closes the PARTIAL without Lane B
needing to re-engage.

@@Alex is poking you at the orchestration
layer to pick up `-59`. Queue continues:
`-59` (per-Hybrid theme render) → `-60`
(pane hamburger trim) → `-62` (Hybrid NAV
rename) → `-63` (clickable help buttons) →
`-67` (FB header drop tab variant + items
to tab right-click).

— @@Architect, 2026-05-19 19:25 BST

## 2026-05-19 20:25 BST — ack: -59 + -60 landed

* `ec26939` (`-59` per-Hybrid theme render):
  UX fork option (2) per recommendation —
  global Settings stays as default, per-side
  override sits on the Hybrid chrome via an
  icon button at `.actions`. Two-state cycle
  (undefined → opposite-of-global →
  undefined). Sun/Moon icon shows the theme
  the click WILL apply; `--link` paint
  telegraphs override-active. CSS extends
  `:root` selector groups to also match
  `.pane[data-theme="..."]` — clean cascade
  reuse, no token duplication.
* `01fe97c` (`-60` pane hamburger trim):
  51 lines dropped from JSX (Next/Prev,
  Split right/down, Flip Hybrid, Close all
  tabs, Close pane + their separators).
  Hygiene sweep took down 7 dropped handlers
  + the no-longer-needed imports
  (`canSplit`, `closePane`, `flipHybrid`,
  `selectNextPane`, etc., + the lucide-svelte
  icons). All keystroke equivalents stay
  reachable via Pane Mode. Negative-assertion
  sentinel test added.

`webtest-a-12` cut to verify both on Lane A.
Item 1 closes `webtest-b-6` item 11 PARTIAL.

Queue remaining: `-62` → `-63` → `-67`. The
first and third are v0.11.0 blocking; `-63`
is blocking-soft. @@Alex is poking you at the
orchestration layer to continue.

— @@Architect, 2026-05-19 20:25 BST

## 2026-05-19 20:45 BST — ack: -62 landed

`3b270d0` (`-62` Pane Mode → Hybrid NAV
rename): visible-text only, internal symbols
untouched per spec. Hamburger entry,
PaneModeHelp title + aria-label, shortcuts.ts
labels swept. New `hybridNavRename.test.ts`
sentinel with comment-stripping helper guards
against regression in visible text. Audit
clean (remaining `/Pane Mode/i` matches are
all comments / class names / variable refs).

Queue remaining: `-63` (in flight per @@Alex's
status) → `-67`.

Lane A's `webtest-a-12` already caught the
rename mid-walk and flipped its assertions —
your audit + their re-walk agree.

— @@Architect, 2026-05-19 20:45 BST

## 2026-05-19 21:00 BST — amend: fullstack-67 extended to dock variant (BOTH sides)

@@Alex eyeballed `webtest-a-12` running and
flagged the docked FBs still show a chrome bar
from the old pane shape — both left and right
docks. They want "free space like in between
panes" — no top bar at all in dock variant.

Amended `-67` (you haven't started it yet —
queued behind `-63` which just shipped):

* **Tab variant**: same as before — drop the
  header entirely, items to tab right-click.
* **Dock variant (BOTH left + right)**: drop
  the header bar entirely. Tree starts at the
  top of the dock area. Hamburger items
  relocate to right-click on the dock body.
  Unstick is covered by Cmd+K `<` / `>`
  bindings from `-69` (already shipped).
* **Overlay variant**: unchanged — keeps its
  header (close + maximize + kebab) since the
  close affordance is load-bearing on a
  floating panel.
* Find-bar (Cmd+F) can appear transiently
  where the header used to be; it's
  context-driven chrome, not always-on bar.

Task file updated:
[../fullstack-b/fullstack-67.md](../fullstack-b/fullstack-67.md).

Acceptance criteria + test list updated to
explicitly cover left + right dock. The `-54`
slim-chrome-strip from dock variant is being
superseded by this amendment (overlay keeps
its slim strip).

Standing topic-level commit clearance.

— @@Architect, 2026-05-19 21:00 BST

## 2026-05-19 21:10 BST — correction + poke: fullstack-71 cut (dock variant follow-up)

Correction on my prior poke: I should NOT
have amended `-67` to include dock-variant
work after you'd already picked it up. The
right move is a new task. Doing that now —
`-67` stays as the tab-variant work you
shipped against; `-71` is the dock-variant
follow-up.

Task: [../fullstack-b/fullstack-71.md](../fullstack-b/fullstack-71.md).

Scope:
* Left dock: no header bar, tree at top.
* Right dock: same.
* FB hamburger items → right-click on the
  dock body (parallels `-67`'s tab right-
  click pattern; no `tab.id` to key off, so
  the dock body's `oncontextmenu` invokes
  the menu directly).
* Unstick also reachable via existing
  `Cmd+K <` / `Cmd+K >` bindings from `-69`.
* Overlay variant unchanged.

Queue position: after `-67`. Updated lane B
queue:

| # | Task           | Status                                              |
|---|----------------|-----------------------------------------------------|
| 1 | `fullstack-67` | drop FB header tab variant (in flight / shipping)   |
| 2 | `fullstack-71` | drop FB header dock variant (both sides)            |

Standing topic-level commit clearance.

— @@Architect, 2026-05-19 21:10 BST

## 2026-05-19 22:15 BST — acks (-67/-71) + pokes (-78/-79)

### Acks

* `74c7d01` `-67` FB tab variant header dropped, items to tab right-click. Triggerless HamburgerMenu pattern with `tabMenu.openForTabId` listener — clean.
* `33c93c9` `-71` FB dock variant header dropped, both sides. Narrowed gate from `!isTab` to `isOverlay`; right-click on dock body flows through existing `onBrowserContextMenu` → no new handler needed. Plus the `unstick()` hygiene sweep is nice.

### `-78` — per-pane theme propagation to xterm.js

@@Alex caught a real bug from `-59`: pane
theme toggle flips chrome + DOM but not the
xterm.js terminal body (xterm renders to its
own canvas with theme set at construction;
CSS cascade can't reach it).

Task: [../fullstack-b/fullstack-78.md](../fullstack-b/fullstack-78.md).

Add a reactive `$effect` that re-applies the
effective theme to every xterm Terminal in
the pane's subtree on `pane.theme` /
`ui.themeChoice` change. Plus audit
GraphCanvas + CodeMirror for the same class
of bug (they may already follow the CSS
cascade, but worth confirming).

v0.11.0-blocking — half-flipped pane is
conspicuously broken.

### `-79` — auto-focus rich prompt on entry

@@Alex flagged: when entering rich prompt
mode (`Cmd+K + p` from `-50`, or Alt+Space
global), the cursor should auto-focus the
input. Currently the user has to click —
extra friction on a keyboard-driven feature.

Task: [../fullstack-b/fullstack-79.md](../fullstack-b/fullstack-79.md).

Pattern reference: the find-bar in
FileBrowserSurface (`findInputEl?.focus()`
after `tick()`).

v0.11.0-blocking-soft.

### Updated Lane B queue

| # | Task           | Status                                              |
|---|----------------|-----------------------------------------------------|
| 1 | `fullstack-78` | per-pane theme propagation to xterm.js              |
| 2 | `fullstack-79` | auto-focus rich prompt on entry                     |

Lane B queue from before (`-67`/`-71`) cleared.
Standing topic-level commit clearance.

— @@Architect, 2026-05-19 22:15 BST

## 2026-05-19 22:25 BST — poke: fullstack-80 cut (right-click trims + FB click-to-inspector)

@@Alex flagged four coupled changes — all
right-click cleanup + an FB behaviour tweak.
Bundled into one task since the surfaces are
adjacent.

### Menu trims

* **Terminal right-click**: drop `Search`,
  `Settings`.
* **File Browser right-click** (post-`-67` /
  `-71`): drop `Search this`, `Settings`,
  `Show/Hide Details`.
* **Graph right-click** (post-`-68` / `-75`):
  drop `Settings`, `Show/Hide Details`.
  (Coordinate: `-75` lands first on Lane A,
  giving the bubble standard row shape; you
  trim against that.)

### FB click-to-inspector

* Tab + Overlay variants: clicking a
  file/dir row auto-opens the inspector
  (sets `inspectorOpen = true` per-tab).
* Dock variants (left + right): click only
  selects; inspector state unchanged.
* `Show/Hide Details` row drop is fine
  because the inspector now auto-opens on
  click in the variants where it matters
  (tab/overlay), and dock variants don't
  need the affordance.

Task: [../fullstack-b/fullstack-80.md](../fullstack-b/fullstack-80.md).

Rationale for the trims: Search → `Cmd+K f`
post-`-74`, Settings → `Cmd+,`. Both are
global keystroke surfaces; per-tab right-
click menus shouldn't duplicate.

v0.11.0-blocking-soft. Lane B queue:

| # | Task           | Status                                              |
|---|----------------|-----------------------------------------------------|
| 1 | `fullstack-78` | per-pane theme propagation to xterm.js              |
| 2 | `fullstack-79` | auto-focus rich prompt on entry                     |
| 3 | `fullstack-80` | right-click trims + FB click-to-inspector           |

Standing topic-level commit clearance.

— @@Architect, 2026-05-19 22:25 BST

## 2026-05-19 22:40 BST — poke: fullstack-82 cut (drop "Open overlay" from FB dock menu)

Per `-71`'s impl note, tab + dock variants
share the same HamburgerMenu instance — so
`-80`'s trims (Search this / Settings /
Show/Hide Details) automatically reach the
dock menu when they ship.

What `-80` doesn't cover: the **`Open
overlay`** entry. It's dock-variant-gated
(doesn't render in tab variant where you're
already in the FB context). That's what
`-82` removes.

Side context: `webtest-a-10` flagged that
`Open overlay` actually calls `openBrowser()`
which opens a tab, not the overlay. The
label/behavior mismatch has been there a
while. Dropping the entry resolves it
without needing to choose between "rename to
Open as tab" or "rewire to truly open
overlay".

Task: [../fullstack-b/fullstack-82.md](../fullstack-b/fullstack-82.md).

Depends on `-80` having shipped first
(broader trims land via the shared menu
instance). Queue order makes that natural.

Updated Lane B queue:

| # | Task           | Status                                              |
|---|----------------|-----------------------------------------------------|
| 1 | `fullstack-78` | per-pane theme propagation to xterm.js              |
| 2 | `fullstack-79` | auto-focus rich prompt on entry                     |
| 3 | `fullstack-80` | right-click trims + FB click-to-inspector           |
| 4 | `fullstack-82` | drop "Open overlay" from FB dock menu               |

Standing topic-level commit clearance.

— @@Architect, 2026-05-19 22:40 BST

## 2026-05-19 23:00 BST — ack: -78 landing + poke: fullstack-84 cut (per-tab inspector width)

### `-78` ack

Per-pane theme propagation to xterm.js
shipping per your impl note. Clean
implementation:
* `effectivePaneTheme()` helper reads
  `layout.nodes[paneId]?.theme` with
  `ui.theme` fallback.
* `terminalTheme()` reads CSS vars from
  `host` (terminal container inside the
  pane) so the `.pane[data-theme]`
  cascade reaches.
* `$effect` tracks both `ui.theme` AND
  per-pane override.
* **GraphCanvas**: MutationObserver
  extended to also watch nearest `.pane`
  ancestor's `data-theme`.
* **CodeMirror**: noted partial coverage —
  CSS-token side already cascades; syntax
  palette branch (`themeExtensions(theme)`)
  reads `ui.theme` directly and isn't
  per-pane. Deferred as a small follow-up
  if visible enough to matter. Worth a note
  for @@Alex's eyeball.

### `-84` — new bug: per-tab inspector width

@@Alex caught: drag-resize inspector in one
FB tab → all FB tabs (and across panes)
resize to same width. Same class as `-58`'s
schema gap, but for inspector width state
this time. `paneWidths.browser` /
`paneWidths.graph` / `paneWidths.inspector`
are module-level singletons; need per-tab
`inspectorWidth` field on BrowserTab /
GraphTab / FileTab.

Task: [../fullstack-b/fullstack-84.md](../fullstack-b/fullstack-84.md).

v0.11.0-blocking — marquee multi-tab
surface half-shipped without it.

Approach mirrors `-58`'s pattern:
* Add per-tab `inspectorWidth?: number`
  field.
* Bind Inspector consumers to per-tab
  field with fallback to the module
  singleton.
* Hash round-trip with conditional
  emission.

Updated Lane B queue:

| # | Task           | Status                                              |
|---|----------------|-----------------------------------------------------|
| 1 | `fullstack-79` | auto-focus rich prompt on entry                     |
| 2 | `fullstack-80` | right-click trims + FB click-to-inspector           |
| 3 | `fullstack-82` | drop "Open overlay" from FB dock menu               |
| 4 | `fullstack-84` | per-tab inspector width                             |

Standing topic-level commit clearance.

— @@Architect, 2026-05-19 23:00 BST

## 2026-05-19 23:10 BST — ack: -79 shipping

`-79` rich prompt auto-focus shipping per
your impl note. Focus-nonce pattern
(mirroring the find-bar's nonce at
`tabs.svelte.ts:95`) is the right shape —
catches both fresh-mount and re-show.
`{#key mode()}` remount handled via tick().
Source mode parallel binding. Clean.

Lane B queue: `-80` → `-82` → `-84`.

— @@Architect, 2026-05-19 23:10 BST
