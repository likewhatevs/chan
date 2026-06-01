# Pub-site release: round retrospective

Date: 2026-06-01
Architect: @@LaneA (Claude)
Round: branding/positioning re-steer + marketing site refresh

## What shipped

Committed to main:
- 7e1af154  reposition chan as the AI-native IDE for the modern engineer
- b2f8e94d  docs(pub-site-release): revise brand story, plan, screenshots

Positioning re-centered on the AI engine, the multi-agent fleet, and
Markdown-driven project work:
- "AI-native IDE for the modern engineer" across README, design.md,
  manual, CLAUDE.md, AGENTS.md, the in-app About slide, and the site.
- "keyboard-first" dropped; local-first and plain-files demoted to a
  light trust fact; editor framed Markdown-first (source editing not
  sold as a strength).
- "first/unique" handled as show-don't-claim; "sigma" stays rationale
  only, out of copy.
- Marketing site: new hero, four revised pillars, a Team Work image
  split, and a /story founder page; title and description updated.
- Stale editor screenshots replaced with Team Work screenshots.
- Multi-user collaboration is no longer framed as a non-goal (it is a
  future goal); the chan-workspace crate boundary disclaimer is kept.

## Lanes

- @@LaneA (architect + positioning docs): docs re-steer, multi-user
  fixes, screenshot staging, doc/plan updates, coordination, commits.
- @@LaneB (in-app About slide): redone on-spec; npm build + cargo build
  + browser smoke green.
- @@LaneC (marketing site + /story): home, build.mjs, story; npm run
  check PASS (build + 4 smokes); forbidden-term greps zero.

## Highlights

- Both worker lanes turned the mid-round re-steer around quickly and
  on-spec, self-verified green, and reported via status files.
- @@LaneC handled the local/tunnel demotion and removed the dangling
  editor-shot references without being handed the exact markup.
- Clean 2-commit split (product vs round docs) with verified staging in
  a shared worktree.

## Lowlights and honest feedback

- Architect (me): the @@LaneC gate was nominally satisfied but the
  screenshots were only in the journal dir, not web-marketing/assets/;
  I caught it only at staging time. I also re-wrapped the LaneA copy
  several times as the positioning evolved; locking the line before
  applying would have saved passes. Shell grep flakiness (output
  truncation on a no-match exit) cost a couple of retries; wrapping
  greps to force a zero exit fixed it.
- Process (Alex): the re-steer arrived in waves (keyboard-first ->
  modern-engineer -> multi-user -> screenshots -> delete old shots).
  Each was clear and the iteration sharpened the copy, but front-loading
  the full direction would have cut some LaneA rework.
- Workers: none material. Strong execution.

## Pending / carryover

- /story ships from the founder draft; a voice pass by @@Alex is open.
- Hero uses team-work-fleet.png; there is no current editor screenshot
  (the stale ones were deleted). A fresh editor capture is needed if an
  editor shot is wanted anywhere.
- Local team scaffolding (.claude/, new-team-1/, web-team/) is untracked;
  consider gitignoring or cleaning it up.
