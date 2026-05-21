# webtest-a-2 — v0.11.2 cut walkthrough lane A

Owner: @@WebtestA
Cut: 2026-05-21 by @@Architect
Status: dispatched

## Goal

Walk the v0.11.2 cut binary on lane A. Confirm the
Round-1 + v0.11.1 + v0.11.2 mini-wave fixes hold on the
shipped binary; surface any regressions for v0.11.3 /
Round-2 wave-2.

## Background

`chan-v0.11.2` tag shipped 2026-05-21:

* Version-bump commit: `60901c1`.
* `release-desktop.yml` run 26221281508 completed green
  in 19m45s.
* Signed + notarized DMG live at
  https://github.com/fiorix/chan/releases/tag/chan-v0.11.2
  as `Chan_0.11.2_x64.dmg` (16.4 MB).

v0.11.2 mini-wave commits (13 from the v0.11.1 close-out
+ 4 from the v0.11.2 patch wave for fb-20/fb-21/fa-42/
ci-9). Full list in the architect journal entry
"2026-05-20 — v0.11.1 cut + pushed" + the post-v0.11.2
close-out commit `e7468db`.

## Coverage slice (lane A)

Per the lane-A/B split established in `-1`:

* File-browser tab name + tooltips.
* Status bar + notification surface.
* Cmd+K cluster (+ Hybrid NAV migration to Cmd+.).
* Rich-prompt cluster (incl. session-evolution previews).
* Editor cluster (Wysiwyg paste, image-insert, file
  rename band, source-mode list keymap).
* Graph (ancestor breadcrumb + from-here default).
* New: file-browser docked dock + dock-shrink overflow.

## Acceptance criteria

* Build / install: download the signed DMG from the
  GitHub Release page; install via Finder drag-and-drop
  to `/Applications/`. Confirm the launch UX (Gatekeeper
  prompt or no — note the signal).
* Walk each lane-A surface; confirm v0.11.1 + v0.11.2
  fixes hold (per the per-task verdicts from `-1`).
* Surface any regressions: shape the repro tightly + file
  in `phase-8-bugs.md` (Round-2 wave-2 candidates) or
  flag as a v0.11.3 hotfix candidate if regression-class.
* Append per-surface verdict + screenshots to
  [`webtest-a-1.md`](webtest-a-1.md) tail with a fresh
  dated heading `## 2026-05-21 — v0.11.2 cut walkthrough
  lane A`.

## How to start

1. Standing perm covers the test-server workflow.
2. Spin a lane-A test server against any throwaway drive
   (chan-source seed remains the right test bed for
   graph + file-browser surfaces).
3. Build local for direct-from-source verification: this
   walks the binary @@Alex would ship if any v0.11.3 cut
   fires.
4. Walk + capture.

## Coordination

* @@WebtestA lane. Standing perm covers all needed
  actions (test-server + Chrome MCP).
* DO NOT install the signed DMG to `/Applications/`
  unless covered by the chan-desktop runtime tightened
  scope from `event-architect-webtest-b.md` "Scope
  clarification" (that scope-tightening applies to lane
  B specifically, but the discipline is good cross-lane).
  If you want to install the DMG for the user-realistic
  walk, fire a permission event to @@Alex per the
  pause-and-warn shape.

## Numbering

Highest committed `webtest-a-N` is `-1` (lane-omnibus).
This is `-2`. The lane uses single omnibus task files
per cut; this task continues that pattern for the
v0.11.2 cut.
