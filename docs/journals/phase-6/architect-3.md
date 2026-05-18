# @@Architect task 3: commit groupings for phase 6

Owner: @@Architect
Status: DRAFT (locked at phase close)

## Goal

Stage the commit groupings for phase 6 so the REVIEW pile can land
in coherent units at phase close, matching the style of phase 5's
six-commit shape.

## Relevant links

* Phase 5 commit groupings for reference:
  `phase-5/architect-2.md` and the resulting
  `git log` lines `c748484`, `58fe80a`, `9e121d5`, `790fd02`,
  `9ecb27d`, `7da49f6`.
* Phase 6 design memo: [architect-2.md](./architect-2.md).
* Phase 6 dispatch: [journal.md](./journal.md).

## Proposed commit shape

Six commits, area-grouped. Order keeps each commit independently
buildable: chan-drive first (foundational types), chan-report
next (rolls up classifier-aware data), chan-server (consumes both),
web (consumes the routes), docs, release.

Refreshed 2026-05-18 to cover the additional REVIEW lanes from
backsystacean-6 through -9 + frontend-3, -4, -6, -7, -8, -10, -12
dir-half. Six-commit shape unchanged.

| Order | Subject (draft)                                          | Lanes covered |
|-------|----------------------------------------------------------|---------------|
| 1     | chan-drive: file classifier + frontmatter kind registry  | [backsystacean-2](./backsystacean-2.md) classifier, [backsystacean-4](./backsystacean-4.md) chan_kind registry + tag/mention markdown-only rule + design.md doc, [backsystacean-5](./backsystacean-5.md) terminology codemod (chan-drive portion), [backsystacean-8](./backsystacean-8.md) `Drive::list` symlink visibility. |
| 2     | chan-report: byte counts + language rollups              | [backsystacean-3](./backsystacean-3.md) chan-report portion (byte counts, sorted summaries). |
| 3     | chan-server: inspector + merged graph + indexer state    | [backsystacean-2](./backsystacean-2.md) `/api/files` path_class + fs_graph path_class + PTY CHAN_MCP_* clearing, [backsystacean-3](./backsystacean-3.md) `/api/inspector` route, [backsystacean-5](./backsystacean-5.md) terminology codemod (chan-server portion), [backsystacean-7](./backsystacean-7.md) indexer state on `/api/health`, [backsystacean-8](./backsystacean-8.md) inspector inode dedupe + `frontmatter_kind` + fs-graph special-file `path_class`, [backsystacean-9](./backsystacean-9.md) merged `/api/graph` (fs + language + semantic). |
| 4     | web: phase-6 frontend                                    | [frontend-1](./frontend-1.md), [frontend-2](./frontend-2.md), [frontend-3](./frontend-3.md), [frontend-4](./frontend-4.md), [frontend-5](./frontend-5.md) (partial: user-visible copy + wire vocab), [frontend-6](./frontend-6.md), [frontend-7](./frontend-7.md), [frontend-8](./frontend-8.md), [frontend-10](./frontend-10.md), [frontend-12](./frontend-12.md) dir-half, [backsystacean-1](./backsystacean-1.md) (web-only changes). |
| 5     | docs: refresh phase-6 boundary                           | chan-drive design.md updates from [backsystacean-4](./backsystacean-4.md) (nested frontmatter shape) + [backsystacean-8](./backsystacean-8.md), CLAUDE.md / design.md residue from the codemod, [backsystacean-6](./backsystacean-6.md) memo outcome (option a recorded). |
| 6     | release: close phase 6 tasks                             | This directory. |

## Risks and notes

* **Working tree overlap**: today's 33-file diff includes work from
  multiple REVIEW lanes (frontend-1 + backsystacean-1 share files;
  backsystacean-2/3/4 share chan-server + chan-drive files). The
  commit boundaries above respect the lane ownership and the
  rebase order keeps each unit independently buildable. The
  agents themselves do not need to rebase before phase close;
  @@Architect stages the commits at the end.
* **Codemod twins**: [frontend-5](./frontend-5.md) +
  [backsystacean-5](./backsystacean-5.md) ship across the same
  rename. They land in commits 1, 3, and 4 above (not their own
  commit) so each surface stays self-contained.
* **Per-commit gate**: pre-push gate green on the final HEAD
  only. Intermediate commits do not need to be individually
  buildable beyond the area's own crate tests.
* **Push timing**: at phase close on Alex's explicit go.

## Open

* @@Backsystacean's [backsystacean-1](./backsystacean-1.md) is
  web-only despite its profile label; it folds into commit 4 (web)
  rather than commit 3 (chan-server). Recorded for clarity.
* Whether to bump version + ship a 0.10.0 release commit on top of
  the six (matching phase 5's `chore: bump version` +
  `release: phase wrap`). Alex's call at close.

## Progress

* 2026-05-18: Draft groupings recorded. Updated as wave-2 work
  lands.

## Completion notes

(populated at phase close once the actual commits are written)
