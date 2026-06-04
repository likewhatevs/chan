# Pub-site release - branding re-steer and marketing site refresh

Status: closed (a single standalone round, not a numbered phase)
Span: 2026-06-01, one working day
Versions: none (docs and marketing only; no binary cut)
Tags: #docs #branding #marketing #positioning

This was a discrete branding and positioning round run between the
numbered phases, not part of a phase's release arc. It is recorded here
so its decisions survive; the raw working material lives in git history
under `docs/journals/pub-site-release/`.

## Roadmap (the asks)

Re-steer chan's positioning and refresh the public marketing site:

- Reposition chan as "the AI-native IDE for the modern engineer,"
  centered on the AI engine, the multi-agent fleet, and Markdown-driven
  project work.
- Drop "keyboard-first"; demote local-first and plain-files to a light
  trust fact; frame the editor as Markdown-first (source editing is not
  sold as a strength).
- Handle "first / unique" as show-don't-claim; keep "sigma" as rationale
  only, out of the copy.
- Refresh the marketing site: new hero, four revised pillars, a Team Work
  image split, and a /story founder page; update title and description.
- Replace the stale editor screenshots with Team Work screenshots.
- Stop framing multi-user collaboration as a non-goal (it is a future
  goal); keep the chan-workspace crate-boundary disclaimer.

## Rounds and waves

A single round under one architect. Wave 1 (immediate): positioning docs
and the in-app About slide. Wave 2 (gated on screenshots): the marketing
site and the /story founder page.

## Team and coordination

Three lanes, coordinated through the shared phase-style bus. The
`@@handle` references resolve via the roster in
[`../agents/README.md`](../agents/README.md):

- @@LaneA (architect plus positioning docs): the docs re-steer across
  README, design.md, the manual, CLAUDE.md, AGENTS.md, multi-user fixes,
  screenshot staging, coordination, and commits.
- @@LaneB (in-app About slide): redone on-spec; npm build plus cargo
  build plus browser smoke green.
- @@LaneC (marketing site plus /story): home, build.mjs, story; npm run
  check passing (build plus four smokes); forbidden-term greps at zero.

## What shipped, tried, and undone

Shipped to main (commits bd30b27a, 7e1af154, b2f8e94d):

- "AI-native IDE for the modern engineer" applied across README,
  design.md, the manual, CLAUDE.md, AGENTS.md, the in-app About slide,
  and the marketing site.
- Marketing site: new hero, four revised pillars, a Team Work image
  split, a /story founder page, and updated title and description.
- Stale editor screenshots replaced with Team Work screenshots.
- Multi-user collaboration reframed from non-goal to future goal; the
  chan-workspace crate-boundary disclaimer kept.

Pending at close:

- /story ships from the founder draft; a voice pass by @@Alex was left
  open.
- The hero uses the team-work fleet image; there is no current editor
  screenshot (the stale ones were deleted), so a fresh editor capture is
  needed if an editor shot is wanted.

## Retrospective

Highlights:

- Both worker lanes turned the mid-round re-steer around quickly and
  on-spec, self-verified green, and reported through status files.
- @@LaneC handled the local/tunnel demotion and removed dangling
  editor-shot references without being handed the exact markup.
- A clean two-commit split (product copy vs round docs) with verified
  staging in a shared worktree.

Lowlights and lessons:

- The @@LaneC gate was nominally satisfied but the screenshots were only
  in the journal directory, not in web-marketing/assets/; the architect
  caught it only at staging time. Lesson: a "gate green" claim must point
  at the artifact's final location, not a working copy.
- The positioning copy was re-wrapped several times as the direction
  evolved. Lesson: lock the line before applying it to save passes.
- The re-steer arrived in waves (keyboard-first, then modern-engineer,
  then multi-user, then screenshots, then delete old shots). Each was
  clear and sharpened the copy, but front-loading the full direction
  would have cut some rework.
- Shell grep flakiness (output truncation on a no-match exit) cost a
  couple of retries; wrapping greps to force a zero exit fixed it.

## Notes

This round predates none of the phase terminology shifts; it uses the
current names (workspace, Team Work). The raw working material (the
brainstorm, branding story, execution plan, founder note, and per-lane
status files) is preserved in git history under
`docs/journals/pub-site-release/`.
