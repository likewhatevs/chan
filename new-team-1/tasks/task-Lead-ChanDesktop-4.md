# task-Lead-ChanDesktop-4 — kill the chanwriter bundle identifier

From: @@Lead. To: @@ChanDesktop. QUEUED behind task-3. @@Alex's
direct ask: "I want the chanwriter thing gone."

## The change

- `desktop/src-tauri/tauri.conf.json` `"identifier"`:
  `com.chanwriter.desktop` → `app.chan.desktop` (my call:
  reverse-DNS of chan.app; flag before landing if some
  Tauri/macOS constraint makes it wrong).
- Sweep desktop/ for the rest: `grep -rni 'chanwriter\|chan-writer'
  desktop/` — known hits: desktop/release-review.md:642-643 cite
  `github.com/chan-writer/chan-core` git deps in an example;
  current-snapshot that doc while you're in it.
- Update the Bundle ID cell in docs/release/macos-signing.md:151 in
  the SAME commit (mechanical cross-boundary edit into my file —
  authorized here).

## Investigate and REPORT (no migration code — pre-release)

The identifier feeds macOS app identity and possibly
identifier-derived paths. Report in your completion file what a
rename orphans on @@Alex's installed 0.31.1, so he knows what to
expect after upgrading:

- Tauri path-resolver dirs (app config/data/cache): identifier- or
  productName-derived? The desktop `Config` lives at
  `<config>/Chan Desktop/config.json` (macOS) per
  docs/config-reference.md — verify which name that derives from.
- Updater: self-upgrade metadata is data-driven (latest.json), but
  confirm nothing pins the old identifier (updater config, signing,
  Info.plist-adjacent values).
- LaunchServices / window-restore state: note anything else keyed by
  the old id. No graceful-degrade paths — just the list.

## Gate

Desktop build green (`cargo clippy/test -p chan-desktop` + a local
app build so the new identifier actually bundles). This rides the
next release; @@Alex validates the installed-app arc there.
