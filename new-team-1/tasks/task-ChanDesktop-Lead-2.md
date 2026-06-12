# task-ChanDesktop-Lead-2 — bundle-id rename (task-4): DONE pending bundle-verify

From: @@ChanDesktop. To: @@Lead. Re: task-Lead-ChanDesktop-4.
(task-3 file-drop runs in parallel: spec is with @@Chan for ack, see
task-ChanDesktop-Chan-1.md; its completion will be a separate file.)

## The change

- `desktop/src-tauri/tauri.conf.json` identifier:
  `com.chanwriter.desktop` → `app.chan.desktop`. Checked against
  Tauri's identifier constraints (alphanumeric/`-`/`.` reverse-DNS) —
  no conflict; no macOS CFBundleIdentifier constraint violated; TLD-
  first `app.` style matches common practice (e.g. md.obsidian). No
  flag needed.
- `docs/release/macos-signing.md:151` Bundle ID cell updated in the
  same commit (authorized cross-boundary edit).
- `desktop/release-review.md` DELETED. Your "current-snapshot that
  doc" + my standing deletion flag: every finding in it reviews
  machinery that no longer exists (BinStatus preflight, `chan serve`
  subprocess supervisor) and it had zero inbound references, so the
  faithful current-snapshot of a point-in-time review doc is removal.
  It also carried the last `github.com/chan-writer/chan-core` dead-org
  URLs. Trivially revertable if you wanted a rewrite instead.
- Post-change sweep: `grep -rni 'chanwriter|chan-writer'` over the
  whole repo → zero hits.

## Gate

- `cargo clippy -p chan-desktop --all-targets` 0 warnings,
  `cargo test -p chan-desktop` green (81 + 7) after the conf change
  (tauri.conf.json is compiled in via generate_context!, so this
  rebuilds against the new identifier).
- Local `make build` app bundle: PENDING at time of writing — result
  + Info.plist CFBundleIdentifier check appended below when done.

## What the rename orphans on an installed 0.31.1 (REPORT, no migration)

Everything below is source-verified, not guessed.

SURVIVES (identifier-independent):
- The desktop config — outbound attachments, window-restore stack,
  tunnel preferences, recorded features. `config.rs::config_path()`
  hand-builds `<config>/Chan Desktop/config.json` (macOS) /
  `<config>/chan-desktop/` (Linux) from `dirs::config_dir()`; no
  Tauri path-resolver (`app_config_dir` etc.) is used anywhere in
  desktop code.
- The chan registry + workspaces (`~/.chan/**`) — never
  identifier-keyed.
- Updater continuity: self-upgrade is data-driven
  (`chan.app/dl/desktop/latest.json` + minisign pubkey); neither the
  updater config nor the manifest generator pins the identifier, and
  tauri-plugin-updater verifies signature + version only. The
  0.31.1 → next upgrade installs normally across the rename.
- Keychain ITEM name: `keyring` service `chan-desktop`, account
  `id.chan.app` — service string is identifier-independent.

ORPHANED / one-time friction for @@Alex after upgrading:
- WKWebView website data `~/Library/WebKit/com.chanwriter.desktop/`
  (+ `~/Library/Caches/com.chanwriter.desktop/`): launcher
  localStorage (theme choice, tunnel-mode toggle) and any SPA
  localStorage reset once; old dirs left behind, deletable by hand.
- Keychain ACL: the macOS keychain authorizes the CREATING app via
  its code-signing designated requirement, which embeds the bundle
  id. Expect a one-time "wants to access" prompt for the id.chan.app
  PAT (or a re-sign-in if denied). Same Developer ID team, so it is
  prompt-level, not silent loss.
- TCC permissions (folder-access grants etc.) are bundle-id-keyed:
  re-prompt on first need.
- `~/Library/Saved Application State/com.chanwriter.desktop.savedState`
  and any `~/Library/Preferences/com.chanwriter.desktop.plist`:
  orphaned, harmless (the desktop owns its own window restore).
- LaunchServices: the `chan://` deep-link scheme re-registers under
  the new id on first launch (Info.plist is rebuilt); the stale
  registration ages out. Sign-in redirects keep working.

Net expectation for Alex post-upgrade: launcher theme resets, one
keychain prompt, possibly a folder-permission re-prompt. Registry,
workspaces, window restore, attachments, tunnel prefs all intact.
This rides the next release; he validates the installed-app arc there.

---

# Update 1: bundle VERIFIED + commits landed (task-4 DONE)

- Local `make build` green; the bundled app carries the new identity:
  Info.plist `CFBundleIdentifier = app.chan.desktop` AND the codesign
  designated requirement embeds `identifier "app.chan.desktop"` —
  which directly confirms the keychain-ACL prediction above (the DR
  is what keychain ACL matching keys on).
- Commits: `fc7dade4` (release-review.md deletion, own commit per
  flag-2 authorization) + `175f409a` (identifier + signing-doc cell).

# Update 2: rg --text re-sweep (shim hazard) — one CORRECTION

Re-ran every sweep I had reported clean, per the round-plan addendum
(main.rs is 111KB — squarely in shim-skip territory):

- Archaeology patterns (original + extended): desktop/ still CLEAN.
  Only non-source matches are binary icon bytes (PNG/icns) that
  happen to contain `@@` sequences.
- CORRECTION to my earlier "chanwriter blast radius = exactly two
  files": that was shim-phantom-clean. rg finds two MORE surfaces,
  both OUTSIDE desktop/ (not mine to edit, routing to you):
  - `web-marketing/scripts/build.mjs:534,595` — regexes matching
    `github.com/chan-writer/chan` (link rewriting/guard logic).
  - `crates/chan-workspace/Cargo.toml:8` + `src/lib.rs:1` —
    "chan-writer workspaces" in the crate description (@@Chan's
    surface; also user-visible on a crates.io publish).
  After my commits, desktop/ + docs/release/ are rg-verified zero.

# Update 3: task-5 flags executed (DONE)

- Flags 1+4 (`dd55ec87`): WorkspaceSettings/WorkspaceFeatures + the
  Config.workspaces map + the dead add_workspace `features` param +
  the remove-side mirror cleanup all deleted; register_and_boot →
  register_workspace_path (the set_semantic/set_reports/boot arm was
  reachable only via non-default features). Four features-serde tests
  deleted. zoom_level missing-field default KEPT (judgment: it is the
  natural default for fresh/hand-edited configs, not just legacy
  parsing). config-reference.md: three workspaces.* rows gone + the
  Open Findings section removed (its only entry was the mirror-drift
  finding this resolves). serve.rs contract test renamed + trimmed.
  Existing config.json files: the orphaned `workspaces` key is
  ignored on load and dropped on next save — no migration needed.
- Flag 2 (`fc7dade4`): release-review.md deleted as its own commit.
- Flag 3 (`c9a32bd2`): updater-bridge.md shrunk 209→~100 lines to the
  durable halves (key identity, secret hygiene, production signing,
  payload forensics, never-bridged failure mode); bridge procedure
  dropped — every live install is past it. Filename kept (cited).
  design.md citation line updated to match.
- Gate after last edit: fmt --check clean, clippy -p chan-desktop
  --all-targets 0 warnings, test -p chan-desktop 77+7 green (81→77 =
  exactly the four deleted tests).

Queue state: task-3 spec still awaiting @@Chan's ack
(task-ChanDesktop-Chan-1.md); I implement the desktop half on ack.
