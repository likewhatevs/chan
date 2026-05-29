# Phase 14 addendum-1: follow-ups from phase-13 round 2

Filed 2026-05-29 by @@LaneB at @@Alex's direction. These are carryovers
from the phase-13 round-2 close (v0.18.0); they are an ADDENDUM to the
main phase-14 scope, not a reordering of it. Detail + provenance live in
`docs/journals/phase-13/retrospective-round-2.md` and the lane journals.

## 1. Release/Pages deploy hardening (partially fixed)

Context: cutting v0.18.0 surfaced a `/dl` outage. Pushing main for the
release touched `Cargo.toml`, which matched `pages.yml`'s old
`push.paths` filter and auto-fired a MARKETING Pages deploy concurrent
with the release deploy. They raced the one `github-pages` environment
(no shared concurrency group), and the marketing deploy wiped `/dl`.

- DONE in r2: `pages.yml` is now manual-only (`workflow_dispatch`); the
  `push: branches:[main]` trigger was removed. `release.yml` owns the
  automatic site deploy (it rebuilds the marketing site + `/dl` together
  on every tag). A main push can no longer race/clobber `/dl`.
- STILL OPEN (phase-14): `web-marketing/scripts/preserve-release-metadata.mjs`
  is a circular, self-perpetuating guard. It fetches the LIVE
  `https://chan.app/dl/releases.json`; on 404 it preserves NOTHING. So
  once `/dl` is 404 (clobbered once), every later marketing deploy keeps
  it 404 until a release regenerates it. Fix: source `/dl` from the
  GitHub release assets (the way `release.yml`'s
  `generate-release-metadata.mjs` does) instead of self-fetching the
  live site. Optional: put `pages.yml` + `release.yml` in a shared
  concurrency group so they can never deploy `github-pages` at once.

## 2. Flaky test gates releases

`chan-workspace::tests::write_text_does_not_wait_for_indexer_serial_lock`
is timing/concurrency-sensitive (part of the known chan-workspace
indexer-flake family, cf. `writes_to_drafts_subtree_get_indexed_...`).
It failed ONCE on the v0.18.0 re-publish (522 passed, 1 failed),
blocking `publish GitHub Release` + the Pages deploy; it passed on
re-run. De-flake it (or mark it so a timing failure cannot gate a
release). A flaky test in the release `validation` job is a release-
availability risk.

## 3. chan-desktop WKWebView walk (empirical verify still pending)

The round-2 desktop-shape changes are gated + Chrome-smoked but need a
human WKWebView walk (Chrome/Blink cannot reproduce desktop rendering,
cf. `feedback_terminal_webgl_wkwebview`):
- Cmd+Shift+N opens a new window of the focused workspace (B-slice 3).
- Cmd+I no longer opens Dashboard (removed from the native KEY_BRIDGE).
- Cmd+P fires the Team Work flow via the renamed `app.terminal.teamWork`.
- Self-upgrade 0.17.0 -> 0.18.0 (data-driven from `/dl`, now serving).
Build the combined dmg from main: `make macos-chan-dmg`.

## 4. Team Work notification bubbles return

Round 2 reduced the bubble overlay to a frontend-only static stub and
deleted the fsnotify-watcher agent-event coordination backend. Equivalent
notification functionality is planned to return in a later phase. The
orchestration skill (`docs/agents/orchestration/README.md` +
`spawn-protocol.md` + `atomic-writes.md`) still describes the removed
watcher / event-file / bubble-reply system as the BLUEPRINT for the
returning implementation; when the replacement lands, those docs need a
full rewrite (r2 only added "removed, returning later" notes).

## 5. Vestigial draft convention

The `rich-prompt-N` draft-dir convention was renamed to `team-work-N`
during the rich-prompt scrub, but nothing creates `team-work-N` dirs
anymore (the Team Work lead editor uses the standard `untitled-N` Cmd+N
draft path; team config goes to a user path like `/tmp/new-team-1`).
Consider removing the `team-work-N` convention from chan-workspace
(`drafts.rs` / `workspace.rs` / `paths.rs` comments + test examples) if
it is not part of the returning Team Work notification work.
