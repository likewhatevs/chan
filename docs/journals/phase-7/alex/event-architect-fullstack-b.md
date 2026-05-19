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
