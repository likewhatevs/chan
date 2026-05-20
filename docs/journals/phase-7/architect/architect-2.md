# architect-2: drive metadata carousel — redesign scoping

**Status: deferred to next phase.** @@Alex scoped
this for phase 8, not for v0.11.0 or any current-
phase follow-up. This doc captures the design
shape so phase 8's architect (or future-me)
inherits the analysis instead of re-doing the
recon. Do NOT cut `systacean-20` or `fullstack-57`
from this doc during phase 7.

Owner: @@Architect (scoping only; implementation
       waits for phase 8).
Date: 2026-05-19

## Why

@@Alex flagged the current carousel slide 2
("Drive metadata") as poor: a single horizontal
stacked bar of file kinds and a tiny legend.
Real value lives elsewhere (the file inspector's
language dashboard, the report endpoint's totals)
but the carousel doesn't pull it together.

Goal: turn the carousel into the drive's
**dashboard moment** — the surface a user lands
on when they open an empty pane and want to see
"what is this drive". Pull from the data we
already surface in inspectors plus add the
metadata-on-our-side dimension chan keeps
outside the drive root.

## Existing data sources (no new server work)

| Source                            | Provides                                                                 |
|-----------------------------------|--------------------------------------------------------------------------|
| `GET /api/drive`                  | `name`, `root` (absolute path)                                           |
| `GET /api/inspector?path=`        | whole-drive: `subtree.files`, `subtree.directories`, `subtree.bytes`, `subtree.file_kinds`, optional `report_summary` |
| `GET /api/report/prefix?path=`    | whole-drive code roll-up: `totals`, `by_language[]`, `cocomo`            |
| `GET /api/contacts`               | contacts list (count)                                                    |
| `GET /api/indexing/state`         | per-dir indexing state (already consumed by slide 3)                     |

Hashtags: chan-drive has a hashtag indexer used
by search; need to confirm whether there's an
existing whole-drive hashtag count or whether
that needs a one-line addition to the search
state endpoint. Audit during implementation.

## New backend surface (one task)

`systacean-20`: `GET /api/chan_meta` — the
"metadata outside the drive root" view. Returns
the on-disk footprint of the chan binary's own
state directories. Schema (proposed):

```json
{
  "config_dir": "<absolute path>",
  "config_size_bytes": <int>,
  "drive_meta_dir": "<absolute path>",
  "drive_meta_size_bytes": <int>,
  "indexes": {
    "bm25": {
      "path": "<absolute path>",
      "size_bytes": <int>,
      "ready": true|false
    },
    "graph": {
      "path": "<absolute path>",
      "size_bytes": <int>,
      "ready": true|false
    }
  },
  "report": {
    "path": "<absolute path>",
    "size_bytes": <int>,
    "ready": true|false
  }
}
```

Field choices:
* `config_dir` / `drive_meta_dir` separated because
  chan keeps app-level config at `~/.chan` (or the
  XDG dir) and per-drive metadata under
  `<config>/chan/drives/<drive-id>/`. The carousel
  cares about both; the user pointed at `~/.chan`
  as the umbrella.
* `indexes.bm25` / `indexes.graph` / `report` —
  separated so the SPA can render per-index size
  + ready status. If chan-server can probe these
  without expensive walks (recursive size of a
  directory is cheap for small dirs), great; if
  not, return `null` for unready and let the SPA
  render "not yet built".
* Sizes in bytes; SPA formats. Match the existing
  `format_bytes` style if there's one (audit).

@@Systacean owns. Cut as `systacean-20` after this
design is ack'd.

## Frontend redesign (multi-slide story)

Current carousel has 3 slides:
1. Welcome (logo + drive summary + shortcuts)
2. Drive metadata (single stacked bar of file kinds)
3. Indexing-state graph (slide 3 from `systacean-18`)

Proposed shape (5 slides):

| # | Slide              | Content                                                                                  |
|---|--------------------|------------------------------------------------------------------------------------------|
| 1 | Welcome            | Logo + shortcut table. (`-55` already drops the stats row from here.)                    |
| 2 | Drive overview     | Drive name, root path, size, file count, directory count, file-kind breakdown (the existing stacked bar, expanded). |
| 3 | Markdown breakdown | Document count, hashtag count, contacts count, link/backlink stats if cheap.             |
| 4 | Code stats         | Whole-drive `by_language[]` rendered as a horizontal bar + legend (mirror `FileInfoBody.svelte:501-525`). Top N languages, "+M more" affordance. COCOMO block optional. |
| 5 | Chan metadata      | `~/.chan` paths + sizes. BM25 / graph / report index sizes + ready status. Where our state lives, how big it is.  |
| 6 | Indexing graph     | The current slide-3 indexing-state graph; moved to the end so the value slides land first. |

Open design questions (let me know what to lock):

1. **Number of slides** — 6 is more than today's 3. Carousel cycle interval may need tuning. Alternative: pack overview + markdown breakdown into one slide.
2. **Indexing graph position** — moving it from slide 3 to slide 6 is a notable shuffle. Keep at 3 for muscle-memory continuity, or move it to make room for the value slides?
3. **COCOMO** — include on the code-stats slide (matches `FileInfoBody` shape) or drop (cute but not strictly informational)?
4. **Markdown breakdown** — do we have hashtag counts already, or does that need a small backend add?

Recommendation on the open questions:

1. Go with 5 slides; pack drive overview + indexing graph into one (with the size/kind summary up top and the indexing-state visualization below). Net: Welcome / Overview+Indexing / Markdown / Code / Chan meta.
2. ↑ resolved by recommendation 1.
3. Drop COCOMO on the carousel; the file inspector still has it for users who want the deep dive.
4. Audit during implementation; if hashtag count isn't cheap to surface, drop hashtags from slide 3 — fold in only documents + contacts.

## Implementation cuts (after design ack)

1. **`systacean-20`** — `GET /api/chan_meta` endpoint per the schema above.
2. **`fullstack-57`** — carousel redesign. Consumes existing endpoints + `systacean-20`. Single big frontend task; if it gets too thick, can split into per-slide cuts.

`fullstack-57` should NOT start until `systacean-20`
ships (slide 5 depends on the endpoint). Slides
2-4 only depend on existing endpoints, so the
frontend can scaffold those first if Lane B is
unblocked while Systacean lands -20.

## Release-tag impact

v0.11.0 was locked pending walkthrough verdicts +
`-53`/`-54`/`-55`/`-56` ship. This redesign adds
non-trivial work (one backend endpoint + a
multi-slide carousel rewrite). Two paths:

* **Path A — fold into v0.11.0**: tag waits for
  `systacean-20` + `fullstack-57` to land. Likely
  another half-day of work; walkthroughs may need
  partial re-walks on the carousel surface.
* **Path B — ship v0.11.0 now, redesign as
  v0.11.1**: walkthrough cluster lands → tag
  v0.11.0 with current carousel → cut the
  redesign as the v0.11.1 follow-up.

Recommendation: **Path B**. The current carousel
isn't broken; it's underwhelming. Shipping
v0.11.0 with today's confirmed surface (Hybrid
flip, Pane Mode, etc.) and following up with the
carousel redesign as a focused v0.11.1 keeps the
release rhythm tight and gives the redesign its
own air. v0.11.0's changelog headline is already
loud enough.

## Phase 8 handoff

When phase 8 opens, the architect there should:

1. Re-confirm the slide layout (5 vs 6, indexing
   graph position, COCOMO inclusion).
2. Re-audit data sources — endpoints may have
   shifted between phases.
3. Cut `systacean-N` for the new
   `GET /api/chan_meta` endpoint per the schema
   in this doc.
4. Cut `fullstack-N` (or split) for the carousel
   redesign once the backend lands.

This file is the design memory; the index pointer
lives in
[`../next-phase-backlog.md`](../next-phase-backlog.md).

— @@Architect, 2026-05-19 16:05 BST (scoping
captured); deferred to phase 8 per @@Alex's
2026-05-19 16:10 BST call.
