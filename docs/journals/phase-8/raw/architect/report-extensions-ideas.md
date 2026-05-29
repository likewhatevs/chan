# Report extensions — candidate ideas for Round 3 (or later)

Author: @@Architect
Date: 2026-05-20

Status: **idea parking lot, not scoped**. @@Alex will pick
which of these (if any) get cut as Round-3 tasks. Out of
scope until then; no implementation work on any of these
without an explicit task cut.

## Current state of chan-report

What `crates/chan-report` produces today (per `README.md`
+ `design.md`):

* Per-file language detection (tokei).
* Per-file source-lines-of-code counts (code / comment /
  blank).
* Per-language roll-ups across the drive.
* Basic COCOMO estimate (effort, schedule, staffing) on
  top of the SLOC totals.
* Incremental updates from filesystem events (single-file
  re-count instead of full re-walk).
* Per-drive state persisted at `.chan/report.jsonl`.
* Scopes: All / Prefix / Files within the drive.

In the chan UI this surfaces as the indexing-graph slide
of the carousel + the language breakdown bar on slide 2 of
the drive-overview carousel.

## Candidate extensions

Each row: one-line scope description + dependency notes +
rough effort framing. Order is alphabetical, not priority.

### Churn metrics

Per-file change frequency over time. "This file has been
modified N times in the last 90 days." Useful for
identifying hotspots + decaying surfaces.

* **Depends on**: a git history walk OR chan's own
  watcher event log (if we persist enough to reconstruct
  per-file change frequencies).
* **Source preference**: git, since chan likely doesn't
  log every watcher event with the granularity needed.
  Requires the drive to be inside a git repo (chan
  already does SCM detection in the pre-flight per
  backlog item 2).
* **Effort**: small-to-medium. `git log --name-only`
  parse + bucket into time windows.
* **Surface**: per-file column in the carousel infographic
  + a "churn map" view (top-N most-changed files).

### Complexity metrics

Cyclomatic complexity, cognitive complexity, or similar
per-function / per-file. Highlights code that's hard to
read or test.

* **Depends on**: language-specific parsers. Tokei covers
  detection only; complexity needs parsers per language
  (tree-sitter is the obvious common substrate).
* **Effort**: large per language. Realistically supports
  Rust + TypeScript + Python + Markdown initially; other
  languages fall back to "no complexity reported."
* **Surface**: per-file complexity score in the report;
  a "complexity hot list" view; per-function drill-down
  is probably too granular for the v1 report shape.

### Contributor stats

Per-file lines-changed by contributor. Counts who has
touched what. Pairs with churn.

* **Depends on**: git blame / git log. Requires git repo.
* **Effort**: small. `git log --pretty=format` + parse.
* **Surface**: per-file contributor breakdown; a "top
  contributors" panel scoped to the drive.
* **Privacy note**: if chan opens to public + the drive
  is a private project, surfacing contributor names
  needs to respect the drive's privacy posture. Probably
  off by default; flag if scoped.

### Dependency graphs (per-language)

For each supported language, parse imports / requires /
use statements and build the cross-file dependency graph.
"This Rust file imports from these other files."

* **Depends on**: language-specific parsers (tree-sitter
  again). One implementation per language.
* **Effort**: medium per language. Probably Rust + TS +
  Python first; Markdown's wiki-link graph is already
  covered by chan-drive's existing graph indexer.
* **Surface**: a "code graph" view alongside the existing
  Graph tab (which today is the markdown wiki-link
  graph). Cross-pollination opportunity: the same Graph
  canvas could render either layer, or both layered.
* **Interaction with existing graph**: markdown wiki-link
  graph stays the always-on baseline (per pre-flight
  invariant). Code dependency graph would be an opt-in
  per-language layer on top of the report.

### Other ideas worth considering

These are sketches without scope; @@Alex picks if any
go into the candidate-list above.

* **Test coverage import**: read coverage.json from
  common formats (lcov, cobertura) if present in the
  drive; surface coverage % per file alongside SLOC.
  Depends on the user running their own coverage tool;
  chan reads + displays.
* **Build-time / artifact-size tracking**: if the drive
  has a `target/` or `dist/` dir, track artifact sizes
  over time. Probably out of scope for chan (touches
  build infrastructure; not a notes-app concern).
* **Markdown-specific report dimensions**: word count
  per file + per directory, reading-time estimates,
  tag frequency, internal-link density. Markdown is
  chan's primary content type; the report today is
  code-skewed (SLOC + COCOMO). A markdown report could
  surface as a sibling.
* **Cross-drive aggregation** (the thing I incorrectly
  attributed to current chan-report 2026-05-20): roll up
  language / SLOC totals across all registered drives.
  Optional + opt-in per drive; lets a user see "how
  much code lives across all my drives." Architecturally
  different from per-drive reports (needs a registry-
  level aggregator). Listed here for completeness; can
  drop if @@Alex doesn't want this surface.

## Coupling with the pre-flight toggle work

If any of these land in Round 3, the pre-flight reports
toggle becomes layered:

```
[x] Enable reports
    [ ] Code analysis (language / SLOC / COCOMO)        ← today's chan-report
    [ ] Churn metrics                                   ← needs git
    [ ] Complexity (Rust + TS + Python)                 ← needs parsers
    [ ] Contributor stats                               ← needs git
    [ ] Code dependency graphs                          ← needs parsers
```

OR a single "Reports" toggle that turns on whatever sub-
features are available for this drive (git availability +
language coverage). The layered shape is more honest about
the per-feature dependencies (e.g. "your drive isn't in a
git repo, so churn + contributors are unavailable"); the
single toggle is simpler UX.

Decision deferred until @@Alex scopes which of the
candidates make Round 3.

## How to use this file

* @@Alex reads, picks N candidates for Round 3, writes a
  task per candidate against the right lane (mostly
  @@Systacean for the chan-report extensions; @@FullStackA
  for the SPA surfaces).
* If a candidate gets cut as a Round-3 task, mark it
  here as `dispatched as <task-id>`.
* If a candidate gets dropped, mark it `wontfix` with the
  reason.
* Anything still in this file when phase 8 closes rolls
  forward into phase 9's backlog source material.

## What this file is NOT

* A task list. Each candidate above is a scope sketch,
  not a ready-to-dispatch task. The actual task files
  get cut when @@Alex confirms.
* A feature commitment. Listing here doesn't mean
  shipping; @@Alex picks.
* An exhaustive ideas dump. New report ideas land here
  via append; this is the working set.
