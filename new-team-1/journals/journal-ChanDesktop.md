# journal — @@ChanDesktop (round 1, tidy-up)

## 2026-06-12

- Woke on poke from @@Lead; read task-Lead-ChanDesktop-1.md + round-1-plan.md.
- Scope: desktop/** — archaeology scrub, hygiene (clippy zero, param structs,
  dedupe, idiomatic), design.md current-snapshot rewrite.
- Recon: ~22 phase hits, ~50 handle hits. Files touched: src-tauri/src/{serve,
  main,config,cs_install}.rs, src/{main,connecting}.js, src/index.html,
  src/styles.css, updater-bridge.md, capabilities/default.json (+ gen/schemas
  mirror — regenerates at build), design.md.
- Kicking off baseline `cargo clippy --all-targets` (background) before edits.

- Baseline clippy: ZERO warnings on macOS cfg (hygiene target = keep at zero).
- serve.rs: full scrub done. Real refactor: `build_workspace_window` 9 params →
  `WindowSpec` struct (removes the `too_many_arguments` allow; 5 call sites).
  `cargo check` green after.
- main.rs: scrubbed; fixed STALE docs — add_workspace doc claimed features go
  "through to `chan add`" CLI (in-process now); a comment referenced
  `get_workspace_features` which no longer exists.
- Found vestige: `WorkspaceSettings.features` mirror is WRITE-ONLY (no reader;
  launcher never passes `features` to add_workspace). Kept behavior, fixed
  docs, flagging removal decision to @@Lead.
- capabilities/default.json description was stale (claimed File▸New Window
  spawns main-N launchers + fullstack-83 marker; launcher is a singleton,
  pinned by test). Fixed description, kept main-* glob (behavior).
- config.rs, cs_install.rs scrubbed; renamed ticket-named test locals
  (pre_b19/pre_b28).

- Frontend + docs scrubbed: main.js ([New]-modal doc matched removed
  preflight/toggles UI — rewritten), connecting.js, index.html, styles.css,
  updater-bridge.md (kept: cited by design.md + .agents/desktop.md).
- Caught off-pattern markers: systacean-27, Round-2-plan §, B10, B-slice,
  "bug report" citation, stray `new-team-1` marker in main.js.
- Second dedupe: spawn preamble (unbury/cap/config-pop) ×3 → unbury_or_restore.
- design.md: full current-snapshot rewrite grounded in source. Verified before
  writing: drag-out command gone (section deleted), tunneled workspaces open
  in-app (stale "deferred" item deleted), release artifacts = release.yml
  (AppImage/.deb/.rpm + notarized DMG), tunnel_start takes port+label+workspace.
  Added: window model (bury/restore/menus/terminals/remote windows), sign-in,
  native integrations (Downloads, macOS PDF).
- Own-gate AFTER last edit: fmt clean, clippy 0 warnings, cargo test all green.
- Committed to main: ad6d5c2c (code), e8b4356a (design.md). Pathspec-atomic,
  staged-stat checked empty before, show --stat verified after. NOT pushed.
- Completion task: tasks/task-ChanDesktop-Lead-1.md (flags: vestigial features
  plumbing, release-review.md deletion candidate, updater-bridge shrink,
  serde legacy defaults). Poking @@Lead.

## Addendum (task-Lead-ChanDesktop-2)

- Workspace correction noted: chan-desktop is a ROOT-workspace member.
  Re-gated from root: fmt --check -p chan-desktop clean, clippy -p
  chan-desktop --all-targets 0 warnings, test -p chan-desktop green (81+7).
- Wider grep sweep: zero archaeology left; only false-positive "slice"
  (run-loop/Vec). systacean-27 + @@Architect were already removed in pass 1.
- Bundle id: com.chanwriter.desktop REPORTED, not renamed. Repo blast radius
  = tauri.conf.json + docs/release/macos-signing.md:151 (documents it).
  Updater endpoint/deep-link/keychain all clean. release-review.md also cites
  dead chan-writer org URLs (folds into its deletion flag).
- No new commit (no file changes). Completion file updated; poking @@Lead.

## task-3 (file-drop) + task-4 (bundle id) picked up

- task-3 design question settled at SOURCE level (no runtime needed):
  wry 0.55.1 wkwebview/drag_drop.rs forwards to WebKit only when the handler
  returns false; tauri-runtime-wry 2.11.2 hardcodes `true`. Enabling the
  native handler swallows ALL DOM DnD incl. in-page tab moves → both options
  in Lead's decision tree fail on macOS.
- Proposed Design C in tasks/task-ChanDesktop-Chan-1.md: keep
  disable_drag_drop_handler; @@Chan's DOM guard delivers no-takeover
  (criteria 1,3,4,5,6); criterion 2 via new `read_dropped_paths` IPC reading
  the macOS drag pasteboard (NSPasteboard .drag) from the DOM drop handler.
  Linux terminal-drop = no-op for now (flag to Lead). Awaiting @@Chan ack
  before implementing.
- Proceeding with task-4 (bundle id rename) while waiting.

## Shim re-sweep + task-4 + task-5 closed

- rg --text re-sweep after the ratified shim hazard: desktop/ archaeology
  still clean (only binary icon bytes match); CORRECTION shipped — earlier
  "chanwriter = 2 files" was shim-phantom-clean, rg found web-marketing
  build.mjs regexes + chan-workspace crate description (routed to @@Lead;
  outside my lane). Also caught my own pipe-masked cargo-check exit mid-task
  and re-ran honestly.
- task-4 DONE: identifier app.chan.desktop committed (175f409a) after local
  make build verified Info.plist + codesign DR carry the new id;
  release-review.md deleted as own commit (fc7dade4).
- task-5 DONE: features plumbing fully removed (dd55ec87, -186 lines incl.
  4 tests; zoom_level default kept by judgment); updater-bridge.md shrunk to
  durable signing runbook (c9a32bd2). Gate green after last edit (77+7).
- Completion file task-ChanDesktop-Lead-2.md updated with all three.
- Still blocked on @@Chan ack for task-3 (file-drop); implementing on ack.

## task-3 desktop half landed

- @@Lead approved design C w/ 2 amendments (local-window ACL scoping —
  system-wide drag pasteboard vs remote-served SPA harvest vector — and
  main-thread pasteboard read); @@Chan acked w/ Files-type guard
  discriminator; contract frozen.
- Implemented read_dropped_paths (dropped_paths.rs): NSPasteboard .drag
  read mirroring wry collect_paths, main-thread via run_on_main_thread +
  oneshot; new local-drop.json capability (workspace-*/terminal-* only);
  NOT in workspace-window set; both pinned in serve.rs contract tests incl.
  must-not-leak assertions. Commit 79de0e95; gate green 79+7.
- Remaining on the bug: @@Chan's guard + terminal wiring, then @@Alex
  WKWebView hand-smoke; Linux path-print no-op flagged for round close.

- @@Chan's addendum poke crossed my implementation in the queue; re-read
  task-Chan-ChanDesktop-1.md (81 lines, unchanged from my pre-build read).
  Verified shipped 79de0e95 matches the addendum exactly (ACL scope +
  graceful degrade counterpart). Confirmed alignment to @@Chan; no deltas.

## task-7 (second-pass review findings)

- F1 fixed: my chord-policy rewrite had carried the pre-existing false
  "Cmd+[/] unbound" claim; brackets now listed as direct chords + the
  under-enumerated exceptions (Cmd+S, splits) added.
- F2 fixed: README phantom drag-out section deleted; also grounded the
  rest of the README (Makefile builds web bundle not chan; no "Forget all
  workspaces" action exists; +.rpm in artifacts). Lesson logged: during
  the tidy I only head-checked the README — full-read every doc in scope.
- Commit 7da761de, gate green 79+7. Folded into task-ChanDesktop-Lead-3.

- @@Lead accepted Updates 1-3 (all four commits verified; zoom_level
  judgment + bundle DR verify endorsed). Routed chanwriter hits resolved:
  chan-workspace already fixed by @@Chan (dc94b16e — verified at HEAD,
  crate description clean; crossed my sweep), web-marketing regexes are
  DELIBERATE stale-copy guards forbidding the dead org — they stay. Repo
  is chanwriter-zero. Lead's queue note (task-3 on ack, task-7) crossed my
  completions — both already done + poked (79de0e95, 7da761de).

## task-8 HOT: gate-red on 79de0e95 fixed

- NSFilenamesPboardType deprecation = hard error under the integrated
  gate's RUSTFLAGS=-D warnings; my bare-clippy own-gate had read green.
- Fix (a): #[allow(deprecated)] on the one fn + inline wry-parity
  justification (migrate with wry). Commit 1f27b17d.
- Re-gated with real flags (clippy -D warnings + test under RUSTFLAGS):
  clean, 79+7. fmt clean.
- Flags discipline adopted + saved to persistent memory (own-gate must
  replicate integrated-gate RUSTFLAGS).

## file-drop arc complete in tree

- @@Chan's step 3 landed (a19d7d40); verified at HEAD: read_dropped_paths
  invoked name-for-name (api/desktop.ts:123), guard + TerminalTab wiring +
  fileDropGuard.test.ts present. Full bug arc now on main
  (79de0e95 + 1f27b17d + a19d7d40).
- Kicked off background make build (web target rebuilds web/dist, so the
  bundle embeds the guard) — smoke app ready for @@Alex on Lead's
  green-light. Poking @@Lead with the smoke-ready signal per commitment.

- Integrated gate GREEN end-to-end at 1f27b17d (Lead's isolated run, all
  legs). Smoke build landed: Chan.app + signed DMG at
  target/release/bundle/{macos,dmg}/. Verified no-stale-dist before
  reporting: web/dist rebuilt this run (14:19) and the embedded SPA bundle
  contains read_dropped_paths; bundle id = app.chan.desktop. Paths poked to
  @@Lead for @@Alex's smoke hand-off. Lane is idle pending smoke verdict.
