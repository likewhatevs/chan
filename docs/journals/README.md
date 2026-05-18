# Journals

Per-phase development logs for chan. Each subdirectory is a
self-contained record of a phase: the source request, the
process, the per-author journals + task files, and (when the
phase closes) a summary.

## Phases

| Dir       | Status        | Notes                                       |
|-----------|---------------|---------------------------------------------|
| phase-1   | closed        | Initial scaffold, drive + server seam.      |
| phase-2   | closed        | First broad feature push.                   |
| phase-3   | closed        | Stabilization + design snapshot.            |
| phase-4   | missing       | Bug-bounty notes only (import pending).     |
| phase-5   | closed        | Tunnel + MCP-only refactor.                 |
| phase-6   | closed        | Graph-as-filesystem, terminology codemod.   |
| phase-7   | in progress   | Project hygiene + UX bugfix wave.           |

Phase dirs renamed from the legacy `chan-pre-release-phase-N`
form on 2026-05-18. The shorter name is fine inside the
`docs/journals/` namespace; the prefix was a top-level
disambiguator we don't need anymore. Phase-4 will be
backfilled from `~/Documents/ChanRoadmap/chan-pre-release-phase-4/`.

## Conventions (phase 7 onward)

* One directory per author under each phase
  (`{phase}/{agent}/`).
* `{agent}/journal.md` is append-only with a dated header.
* Task files at `{agent}/{agent}-{task}.md`, also append-only.
* Agent references in prose use the `@@{name}` form. Contact
  cards live at [`../agents/`](../agents/).

Earlier phases (1, 2, 3, 5, 6) predate this layout and use
flat task files at the phase root. They remain in their
historical shape; the `@@{name}` references are anchored via
contact-card predecessors (e.g., `@@Backend` resolves to
@@FullStack's contact card).

## Why phase 4 is missing

Phase 4 was skipped in the original numbering and never had a
directory. The next phase after 3 jumped to 5 by convention.
