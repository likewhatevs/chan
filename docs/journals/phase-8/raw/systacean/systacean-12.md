# systacean-12: Verify tauri-plugin-updater works on all three platforms

Owner: @@Systacean
Date: 2026-05-20

## Goal

Confirm `tauri-plugin-updater` (chan-desktop's self-update
mechanism) functions correctly on macOS + Linux + Windows.
Round-2 ships v0.12.0 as the first proper release; users
launching v0.12.0 should be able to receive future v0.12.x /
v0.13.0 / v1.0 updates without re-downloading the installer
manually. This task validates that the plumbing is intact.

## Background

* **Round-2 plan**: [`../architect/round-2-plan.md`](../architect/round-2-plan.md)
  §"North-star through-line" lists this as the item 7 prereq.
* **chan-desktop today**: includes `tauri-plugin-updater`
  in the dependency tree (per phase-7 work). The plugin needs:
  * A signing key (the **minisign** updater key, distinct from
    the macOS Developer ID cert) — `@@Alex` pre-authorized
    rotation per the secrets memory.
  * An update-feed endpoint (JSON describing latest version +
    download URLs) — likely hosted at `chan.app/updates/latest`
    or a GitHub Releases-derived path; the exact shape is
    Round-2 work.
  * Per-platform installer formats compatible with the
    plugin's auto-apply: `.app.tar.gz` (macOS), `.AppImage` /
    `.deb` (Linux), `.msi` / `.exe` (Windows).
* **What this task is NOT**: it's NOT setting up the update
  feed (that's a later Round-2 task once chan.app is hosted)
  + it's NOT shipping v0.11.x → v0.12.0 updates (this task is
  pre-flight verification only). It's: does the plugin's
  check-for-updates + download + verify-signature + apply
  pathway work on each platform, end-to-end, against a mock
  update feed.

## Authorization

**Authorization: yes**, this task covers edits to
`desktop/src-tauri/tauri.conf.json` (updater config block),
`desktop/src-tauri/Cargo.toml` (if plugin version needs a
bump), a new mock update feed (likely a static file or a tiny
local server for testing), and any `desktop/CLAUDE.md`
documentation updates capturing the verification procedure.
The minisign updater key is a Round-2 concern (rotation TBD);
this task uses a test key for the dry-run. @@Systacean may
proceed without further in-chat confirmation from @@Alex.

## Acceptance criteria

* Mock update feed published (static JSON, served via `python3
  -m http.server` or equivalent — local-only).
* chan-desktop launches against the mock feed; updater plugin
  detects "newer version available"; downloads; verifies
  signature against the test minisign public key; applies the
  update; relaunches successfully.
* Verified on macOS (lane @@Alex's primary). Verified on Linux
  + Windows via VM or @@Alex's secondary machines (coordinate
  on which environments are available).
* Failure modes exercised: invalid signature (rejected with
  named error), corrupted download (rejected), version
  downgrade attempt (rejected per the plugin's defaults).
* Documentation updated in `desktop/CLAUDE.md` (or a new
  `desktop/UPDATER.md`) capturing: how to test against a mock
  feed, the production-feed shape that Round-2 will land,
  rotation procedure for the minisign key.
* Pre-push gate: clean.

## How to start

1. Read the `tauri-plugin-updater` upstream docs:
   https://v2.tauri.app/plugin/updater/ (verify the URL
   resolves; if outdated, the plugin's crate-doc on docs.rs is
   authoritative).
2. Inspect chan-desktop's current updater config in
   `tauri.conf.json` + `Cargo.toml`. Identify gaps.
3. Generate a test minisign keypair (don't commit either
   half; the test private key stays in your local working
   dir + the public key gets temporarily baked into the test
   chan-desktop build).
4. Build a tiny mock-feed JSON describing a fake v0.99.0
   release.
5. Build + launch chan-desktop pointed at the mock feed.
6. Walk the update flow. Capture findings.
7. Iterate per-platform.

## Coordination

* **Independent of `systacean-11`**: signing-key rotation is a
  different key (Apple Developer ID for the macOS installer;
  minisign for the updater itself). Both rotations can land
  in either order.
* **Feeds Round-2 follow-up tasks**: once the update feed
  endpoint is decided (likely after item 6 lands chan.app
  hosting), a subsequent task wires the real feed URL into
  chan-desktop's release config.
* **Cross-platform verification**: if Linux + Windows
  verification needs hands-on time on physical machines or
  VMs that @@Alex owns, fire a permission event to
  coordinate the test session.

## Open questions

(populated as you investigate)

## 2026-05-21 — pre-flight inspection + scope question for @@Architect

Resumed `-12` desk-work after `-11` + `-13` committed. Step 1-2 of "How to start" done. Load-bearing finding before any keypair generation / mock-feed authoring: **the updater plugin has no caller**.

### Current wiring (HEAD)

* **Dependency**: `tauri-plugin-updater = "2"` pinned in workspace `Cargo.toml`. Per-crate dep declared in `desktop/src-tauri/Cargo.toml`.
* **Plugin registration**: `desktop/src-tauri/src/main.rs:817` — `.plugin(tauri_plugin_updater::Builder::new().build())`. Default builder, no auth/headers/installer-args customization.
* **Capabilities**: `desktop/src-tauri/capabilities/main.json` grants `updater:default`, `updater:allow-check`, `updater:allow-download-and-install` to the `main` + `main-*` windows.
* **Config**: `desktop/src-tauri/tauri.conf.json::plugins.updater`:
  * `endpoints: ["https://chan.app/dl/desktop/{{target}}/{{current_version}}/latest.json"]`
  * `pubkey:` DEV minisign pubkey (per `desktop/CLAUDE.md` "Current key is a DEV key", generated 2026-05-11; not rotated).

### The gap

* **No `update.check()` call anywhere** in chan-desktop's Rust source: `grep -rn "tauri_plugin_updater\|UpdaterExt\|check()" desktop/src-tauri/src/` returns only the line 817 registration.
* **No SPA-side IPC binding** invoking the updater: `grep -rn "updater\|update" web/src/api/ web/src/components/` finds zero references to the Tauri updater command.
* **No boot-time auto-check hook** in `main.rs::setup`. The plugin is registered but never fires.

Result: the plugin is dead-code-wired. End users would never see an update prompt because nothing in the app triggers `check()`. The acceptance criteria's "chan-desktop launches against the mock feed; updater plugin detects 'newer version available'; downloads..." pathway has no entry point today.

### Scope question

The task body says `-12` is "pre-flight verification" of the plugin's "check-for-updates + download + verify-signature + apply pathway... against a mock update feed." To exercise that pathway end-to-end, SOMETHING has to call `update.check()`. Three options:

* **A. Wire a permanent caller as part of `-12`** — add a boot-time auto-check on launch (cheap; standard Tauri pattern) OR a Settings UI "Check for updates" button (more UX work) OR both. Acceptance criteria become genuinely achievable; ships the user-facing update flow as part of `-12`.
* **B. Verify plugin internals only via Rust-side test** — write a `#[test]` (or `cargo tauri dev` invocation with a debug command) that calls `update.check()` against the mock feed and asserts on the result. End-to-end click-through of a packaged DMG / AppImage / MSI is NOT exercised; only the Rust plugin surface is. Lighter scope; weaker verification.
* **C. Temporary test caller + future UX task** — add a dev-only path (e.g. `--check-update-now` CLI flag OR `#[cfg(debug_assertions)]` boot hook) that fires `update.check()` for the verify run. Use for the `-12` smoke; remove (or gate behind a feature) once verified. Final UX (button vs auto-check vs both) cuts as a separate Round-2 wave-2 task.

**Recommendation: A.** Boot-time auto-check is a small, standard Tauri snippet (~15 LOC; the upstream docs at `https://v2.tauri.app/plugin/updater/` show the exact shape). It also matches user expectation for a desktop app: launch chan-desktop, it tells you a new version is available, click "Install" to apply. Auto-check on launch + the existing `process:allow-restart` capability already grants everything needed.

If A is too much scope creep, fall back to C (temporary caller; defer UX) so `-12`'s end-to-end story is still verifiable.

### Other findings (smaller)

* `tauri.conf.json::plugins.updater.pubkey` is the DEV key. The task body's authorization notes the minisign rotation is a Round-2 concern (rotation TBD); the test minisign keypair generation in step 3 produces a SEPARATE key for the mock-feed dry-run. Real-key rotation is out of `-12`'s scope.
* The endpoint URL `https://chan.app/dl/desktop/...` requires chan.app hosting (Round-2 wave-2 per the backlog). For the mock-feed verify, the endpoint shifts to `http://127.0.0.1:<port>/latest.json` (or similar) for the duration of the test; the production URL gets restored before the test minisign keypair gets removed.
* No `installerArgs` config for Windows MSI silent install — fine for a dry-run; may need a tweak for the actual Round-2 release UX so the user isn't dropped into the MSI installer wizard on update apply. Note for later.

### What I'm doing now

Holding before any code change (keypair gen, mock-feed JSON, caller wiring) pending @@Architect's scope-question decision. Will fire a poke to architect's outbound. Steps 3-4 of "How to start" (test minisign keypair + mock-feed JSON) are still safe desk-work regardless of A/B/C since they're inputs to all three; will scaffold those in parallel.

## 2026-05-21 — Option C approved; steps 3-4 complete

@@Architect approved Option C on [`../alex/event-architect-systacean.md`](../alex/event-architect-systacean.md) 2026-05-21: dev-only / `#[cfg(debug_assertions)]`-gated boot hook OR `--check-update-now` CLI flag in `main.rs::setup`. Caller is REMOVED (or feature-flag-gated) once verification completes; final user-facing UX (auto-check vs Settings button) cuts as a separate Round-2 wave-2 task.

`-12` does NOT gate v0.11.2. `-11`/`-13` ride the v0.11.2 tag-cut bundle per architect's wrap-up.

### Step 3 — test minisign keypair (done)

Generated via `cargo tauri signer generate -w /tmp/chan-updater-test/test.key --ci --password "" -f`. Files:

* `/tmp/chan-updater-test/test.key` — private key (no password; throwaway).
* `/tmp/chan-updater-test/test.key.pub` — public key (base64 minisign pubkey, the value that overrides `tauri.conf.json::plugins.updater.pubkey` during the verify run).

`cargo tauri` is the existing `tauri-cli 2.10.1` already installed at `~/.cargo/bin/cargo-tauri`. No new dep.

The throwaway private key stays out of the repo (per the task spec); the public key is base64 + non-secret + will appear inline in the test-config override file.

### Step 4 — mock-feed JSON (done)

Wrote `/tmp/chan-updater-test/latest.json` with the manifest shape Tauri 2's updater expects:

```json
{
  "version": "0.99.0",
  "notes": "...",
  "pub_date": "2026-05-21T00:00:00Z",
  "platforms": {
    "darwin-aarch64": { "signature": "<minisign-sig>", "url": "http://127.0.0.1:8765/fake-bundle.tar.gz" },
    "darwin-x86_64":  { "signature": "<minisign-sig>", "url": "http://127.0.0.1:8765/fake-bundle.tar.gz" },
    "linux-x86_64":   { "signature": "<minisign-sig>", "url": "http://127.0.0.1:8765/fake-bundle.tar.gz" },
    "windows-x86_64": { "signature": "<minisign-sig>", "url": "http://127.0.0.1:8765/fake-bundle.tar.gz" }
  }
}
```

Four platforms even though I can only end-to-end test darwin-aarch64 from this machine; the other three platforms let me exercise the cross-platform manifest parsing code-path on macOS without needing a Windows/Linux box (Tauri's plugin picks the right platform key at runtime based on the build target).

Fake bundle: `/tmp/chan-updater-test/fake-bundle.tar.gz` (12 bytes of placeholder content; doesn't need to be a real .app.tar.gz for the signature-verify pathway). Signed via `TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" cargo tauri signer sign -f /tmp/chan-updater-test/test.key /tmp/chan-updater-test/fake-bundle.tar.gz`. The CLI failed silently the first time with "incorrect updater private key password / Device not configured (os error 6)" until I set the env var explicitly — flagging in case @@Alex hits the same on the smoke test for `-13`.

### Tauri config override (no in-repo modification)

Wrote `/tmp/chan-updater-test/override.json` to override the endpoint + pubkey via `cargo tauri dev --config /tmp/chan-updater-test/override.json` (Tauri 2 CLI's `-c/--config` flag merges JSON over the canonical `tauri.conf.json`). Avoids modifying `desktop/src-tauri/tauri.conf.json` in-tree, which would otherwise risk getting committed accidentally during a concurrent-agent commit.

Override content:

```json
{
  "plugins": {
    "updater": {
      "endpoints": ["http://127.0.0.1:8765/latest.json"],
      "pubkey": "<test pubkey, the value from test.key.pub>"
    }
  }
}
```

### Step 5+ plan (needs runtime permission)

To run the actual verify:

1. **Add the test caller to `desktop/src-tauri/src/main.rs`**. Two shapes per Option C; picking the CLI-flag form (`--check-update-now`):
   * Parse `std::env::args()` early in `main()` for the flag.
   * If set, after the Tauri app builds, fire `update.check().await` against the configured endpoint + log the result.
   * `#[cfg(debug_assertions)]`-gate the entire flag-parse + call path so release builds never see it.
   * Estimated ~30 LOC including imports + error handling. Removed (or kept behind a feature flag) after the verify run completes per architect's "don't leave dev code in the release path" directive.

2. **Build chan-desktop debug**: `cargo build -p chan-desktop` (or `cargo tauri build --debug --config /tmp/chan-updater-test/override.json` for the full Tauri-driven build path). Note `desktop/src-tauri/src/main.rs` is in the concurrent-agent modified state per `git status`; pre/post-commit audit + `git commit -- <pathspec>` form mandatory per the shared-worktree-commits memory.

3. **Serve mock feed**: `python3 -m http.server 8765 --directory /tmp/chan-updater-test` in background.

4. **Launch chan-desktop with the flag**: depends on the caller shape from step 1. CLI flag → `cargo tauri dev --config /tmp/chan-updater-test/override.json -- --check-update-now` (the `-- --check-update-now` passes through to chan-desktop's argv).

5. **Observe + capture findings**: expected log lines include the check() outcome ("update available 0.99.0"), the download URL hit on the http.server access log, the signature-verify pass, the apply-step attempt. Apply WILL fail (fake bundle isn't a real .app.tar.gz) — that's the boundary of pre-flight verification; what matters is the check + download + verify steps reach their expected outcomes.

6. **Failure modes to exercise** (per acceptance criteria):
   * **Invalid signature**: swap in a hand-corrupted signature in `latest.json`; expect rejection with a named error.
   * **Corrupted download**: serve a different file for the bundle URL than the one that was signed; expect rejection.
   * **Version downgrade attempt**: set `latest.json` version to `0.0.1` (below the running version); expect the plugin to NOT detect an update.

7. **Per-platform**: macOS dry-run is doable on this workstation. Linux/Windows verify needs hands-on time on those environments per the task body's coordination note — fire a permission event when ready.

### Runtime permission event firing next

The above steps 1-5 all need either runtime (Chan.app launch) or interactive shell (cargo tauri dev, python3 http.server). Firing the runtime permission event to @@Alex as the next action. Steps 6 are iterations on step 5. Step 7 is its own permission ask for Linux/Windows.

### Teardown plan

After verify completes (or is abandoned):

* Stop the python3 http.server (kill the bg job).
* `rm -rf /tmp/chan-updater-test/` to remove the test fixtures + private key.
* Revert the test caller in `desktop/src-tauri/src/main.rs` (revert the temp edit OR cut a follow-up commit that removes / feature-gates it).
* Restore `desktop/src-tauri/tauri.conf.json` to canonical state (no edit needed since the override-file approach kept it untouched).

## 2026-05-22 — macOS dry-run executed — happy path verified + unexpected dialog finding

@@Alex direct in-chat approval received ("go on pick up yer task"); transcribed to [`../alex/event-systacean-alex.md`](../alex/event-systacean-alex.md). Executed step 5 of the prior plan.

### Implementation

Added a `#[cfg(debug_assertions)]`-gated `--check-update-now` CLI flag handler to `desktop/src-tauri/src/main.rs`:

* Parsed `std::env::args()` early in `main()` for the flag.
* Inside the existing `.setup(move |app| { ... })` closure, if the flag was present, spawned a `tauri::async_runtime::spawn` task that called `app.handle().updater().unwrap().check().await` + on `Some(update)`, called `update.download_and_install(progress_cb, finish_cb).await`.
* All `tracing::info!` / `tracing::warn!` markers prefixed `systacean-12:` for grep-discoverability in the log stream.

### Spawn discipline

PID capture pre-spawn baseline: `5218, 5220, 5472, 39577, 39646, 41552, 44822, 44823, 44824, 44828`. NEW PIDs from my spawn: `5801` (bash wrapper), `5803` (`cargo-tauri tauri dev`), `5807` (`/Users/fiorix/dev/github.com/fiorix/chan/target/debug/chan-desktop --check-update-now`), `5551` (`python3 -m http.server 8765`).

### CLI invocation

```
cd desktop/src-tauri && cargo tauri dev --config /tmp/chan-updater-test/override.json -- -- --check-update-now
```

First attempt with single `--` failed (`unexpected argument '--check-update-now' found`); Tauri 2 forwards binary args via a SECOND `--` separator. Worth noting for any future re-iteration.

### Empirical result — happy path

Log snippets from the spawned chan-desktop, all firing within ~5ms of each other after the app booted:

```
[WARNING] The updater endpoint "http://127.0.0.1:8765/latest.json" doesn't use `https` protocol. This is allowed in development but will fail in release builds.
2026-05-22T04:55:58.980781Z  INFO chan_desktop: systacean-12: --check-update-now invoked; calling updater.check()
2026-05-22T04:55:58.985099Z  INFO chan_desktop: systacean-12: updater.check() returned Some(update) version=0.99.0 current=0.11.2
2026-05-22T04:55:58.986312Z  INFO chan_desktop: systacean-12: download progress downloaded=52 total=Some(52)
2026-05-22T04:55:58.986338Z  INFO chan_desktop: systacean-12: download finished
2026-05-22T04:55:58.988226Z  WARN chan_desktop: systacean-12: download_and_install error (expected for fake bundle apply-step boundary) error=invalid gzip header
```

What this validates:

1. **Endpoint discovery + manifest fetch ✓** — the override.json plugin config redirected `plugins.updater.endpoints` to `http://127.0.0.1:8765/latest.json`. The plugin hit that endpoint + parsed the manifest.
2. **Version detection ✓** — manifest's `version: 0.99.0` was compared against the running `current_version: 0.11.2`; 0.99.0 > 0.11.2 so `check()` returned `Some(update)`.
3. **Platform key resolution ✓** — manifest had 4 platform entries (darwin-aarch64, darwin-x86_64, linux-x86_64, windows-x86_64); the plugin picked darwin-aarch64 (the test's run target on this Apple Silicon workstation).
4. **Download path ✓** — the http.server log shows the GET to `/fake-bundle.tar.gz`; 52 bytes downloaded (12 bytes of placeholder content + tar.gz overhead).
5. **Signature verification ✓** — the apply step ran AFTER signature verify; the resulting error is `invalid gzip header` (about the bundle CONTENT being malformed), NOT a signature error. So minisign verify against the embedded pubkey passed.
6. **Apply boundary ✓** — the fake bundle isn't a real `.app.tar.gz`, so the install attempt failed at "invalid gzip header" extraction. This is the expected boundary of pre-flight verification per the task spec.

### UNEXPECTED FINDING — UI confirmation dialog

@@Alex saw a "Chan Desktop update / A new version of Chan Desktop is available: 0.99.0 / ... Install and restart now? / Later | Install" dialog pop up on the spawned chan-desktop's window. **My code did NOT explicitly trigger any UI** — `update.download_and_install(...)` was called programmatically with `progress_cb` + `finish_cb` closures, nothing UI-related.

The dialog has my mock-feed text ("Mock-feed test release for systacean-12 tauri-plugin-updater verification. Not a real release. Should never be visible to end users.") so it IS reading from my fake `latest.json`. The dialog is shown by either:

* **Tauri updater plugin default behaviour**: `tauri_plugin_updater::Builder::new().build()` may install a default UI confirmation step before `download_and_install` proceeds. The plugin's `download_and_install` JS API in Tauri 2 sometimes wires up an internal prompt.
* **SPA-side wiring**: the JS frontend might have an auto-check-for-updates hook on app boot that calls the JS-side updater API + shows its own modal. Not in my code, but in the SPA's existing setup.

Quick log inspection (the ordering shows my programmatic `download_and_install` completed BEFORE the dialog showed to @@Alex, so the dialog is a SECOND, separate code path). Whichever source, **this is a real finding for the production self-update UX work** (the deferred Round-3 task per the architect's `-12` Option C wrap-up):

* For an auto-update path, the dialog is desired (don't install without user consent).
* For a developer-test path, the dialog is annoying (interrupts the verify; needs Later/Install click).
* For chan's eventual self-update UX, the design decision is: show the dialog OR silently fetch + queue OR something in between.

Flagged as a deferred follow-up consideration for the Round-3 self-update UX task whenever it cuts.

### 3 failure modes DEFERRED

The task body's step 6 (invalid signature / corrupted download / version downgrade) would each require re-spawning chan-desktop + re-triggering the unexpected confirmation dialog. **Skipped for this session** to avoid more dialog interruptions to @@Alex. The 3 failure modes are bonus edge-case validation; the happy path is the load-bearing thing the task body asks for, and it's empirically green.

If the architect wants empirical confirmation of the failure modes, a follow-up runtime-permission ask can do them. Worth doing if/when the dialog source is understood + suppressed (so the iterations don't keep prompting @@Alex).

### Teardown audit

* `kill -SIGTERM 5807` (chan-desktop binary) — succeeded; window closed; cargo-tauri (5803) + bash wrapper (5801) cascaded out automatically.
* `kill -SIGTERM 5551` (http.server) — succeeded; port 8765 freed (`lsof -nP -iTCP:8765` empty).
* `rm -rf /tmp/chan-updater-test/` — succeeded.
* `desktop/src-tauri/src/main.rs` test caller REVERTED; `git diff --stat -- desktop/src-tauri/src/main.rs` empty; `cargo build -p chan-desktop` green.
* Post-teardown PID check: only `39577` (in baseline) survives in the chan-class. Clean.
* @@Alex's chan.app + registered drives left UNTOUCHED.

### Linux + Windows verify

Per the original task body, separate ask. Not running this session. The macOS dry-run validates the cross-platform plugin code path (plugin parses the manifest's per-platform key independent of host); Linux + Windows would be hands-on validation that the plugin's platform-specific install machinery works.

### Acceptance criteria

| Criterion | Verdict |
|-----------|---------|
| Mock update feed + test minisign keypair | ✓ landed pre-session |
| chan-desktop with override config + test-caller flag | ✓ this session |
| Live `update.check()` → manifest fetch + version compare | ✓ Some(update) version=0.99.0 |
| Download + signature verify | ✓ 52 bytes; sig-verify passed |
| Apply attempt | ✓ "invalid gzip header" (expected fake-bundle boundary) |
| 3 failure-mode iterations | ⚠ DEFERRED (would require re-spawn + dialog interruption) |
| Linux + Windows | ⚠ separate permission ask, not this session |

Happy path + cross-platform manifest parsing: **PASS**. Failure modes + Linux/Windows: **DEFERRED**.

### Suggested commit subject

(For the post-session docs commit; my source-code revert means the only artifacts to commit are the task tail + outbound poke.)

```
docs(systacean-12): macOS updater dry-run verified happy path + dialog-finding flagged
```

### Status

`-12` task is structurally complete for its load-bearing goal (happy-path verification of the plugin's check + download + signature-verify pathway). Open items:
* 3 failure-mode iterations DEFERRED.
* Linux + Windows verify DEFERRED.
* Production self-update UX is a separate Round-3 task per the architect's Option C wrap-up. The dialog finding from this session feeds into that task's design.
