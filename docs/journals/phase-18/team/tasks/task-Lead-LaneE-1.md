# task Lead -> LaneE (1): Terminal + chan-desktop

You are @@LaneE - Terminal + chan-desktop lane. Round-1, Wave 1. START NOW.

## Read first (context lives here, not in this poke)
- Process: docs/journals/phase-18/team/bootstrap.md
- Plan + your lane section + gate/quality bar + shared-file table:
  docs/journals/phase-18/team/round-1-plan.md  ("@@LaneE - Terminal + chan-desktop")
- Verbatim spec: docs/journals/phase-18/round-1/draft.md  ("### Terminal" + "### chan-desktop")
- Images: round-1/image-7.png (terminal menu chords), image-14.png (less garble),
  image-15.png (vim garble), image-1.png + image-2.png (desktop double-dialog).
- Re-verify line anchors against HEAD; they drift.

## Wave 1 scope (4 items)
1. Hide rich prompt -> focus the terminal. Hiding (menu or cmd+shift+p) leaves
   focus off the terminal; return focus + cursor to the xterm instance on hide.
2. Terminal context-menu chords + copy/paste (image-7). Copy / Paste / Copy
   Scrollback have empty chord spans. Wire cmd+c / cmd+v and show the hints;
   record chords in shortcuts.ts.
3. UTF-8 garble in less AND vim (image-14 less, image-15 vim): multibyte UTF-8
   renders as raw bytes. PTY spawn env (terminal_sessions.rs) sets TERM/COLORTERM
   but no LANG/LC_ALL/LC_CTYPE. Set a UTF-8 locale on spawn. Verify in BOTH less
   and vim on docs/config-reference.md style content.
4. chan-desktop: local-disk New-workspace flow still shows the OLD pre-flight
   dialog (image-1, image-2), conflicting with the SPA boot menu. Pre-flight
   moved to the SPA (PreflightOverlay.svelte) in phase-17; REMOVE the
   desktop-side dialog for the local path (desktop/src/main.js renderLocal +
   compute_workspace_preflight scan UI).

## Owned files (edit ONLY these)
web/src/components/{TerminalTab.svelte,RichPrompt.svelte},
web/src/state/richPrompt.svelte.ts, crates/chan-server/src/terminal_sessions.rs,
desktop/src/main.js, web/src/components/PreflightOverlay.svelte,
web/src/state/shortcuts.ts (terminal copy/paste chord additions).

## Shared-file rules (plan "Shared-file contention")
- shortcuts.ts: you AND @@LaneC both append to SHORTCUTS. @@Lead sequences
  C THEN E -> I will poke you to APPEND once C has landed its FB chords, so we
  keep one clean array literal. Until then, do everything else; stage your
  terminal-chord additions but WAIT for my go before appending to shortcuts.ts.
  Do NOT run web/scripts/shortcuts-table.mjs - @@Lead resyncs once after both.
- crates/chan-server: terminal_sessions.rs is yours; graph route is @@LaneB's;
  same crate -> re-`cargo check -p chan-server` green before pausing.
- App.svelte: you = rich-prompt handler (~659); @@LaneC = layout effects (far apart).

## Gate before any "done" report
Frontend: make web-check + svelte-check + npm run build (browser-smoke focus +
copy/paste). Rust: cargo fmt --check + clippy --all-targets -D warnings + cargo
test (-p chan-server). Desktop: cd desktop && make build.
NOTE: terminal render (focus, copy/paste) + the desktop dialog are
WKWebView-specific - agents cannot drive WKWebView. Report them as
gated-green + needs @@Alex hand-smoke; I'll batch that into a survey.

## On completion
Cut task-LaneE-Lead-1.md (own-gate-green + pathspec sha + per-item status +
exactly which SHORTCUTS entries you want appended), poke me. Journal:
journal-LaneE.md. Flag ANY shared-file touch BEFORE landing.
