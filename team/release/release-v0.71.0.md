# Release v0.71.0 - OpenCode, exact-origin desktop trust, and unified workspace search

Delivery round run 2026-07-19 off the v0.70.3 tag. The first release cut through the new `team/roadmap` + `team/release` structure (the process migration that landed as this version's first item). Five technical items shipped from a single sequenced team round: a Claude lead coordinating two Codex and two Kimi implementer subagents on disjoint file lanes, plus the pre-existing OpenCode terminal branch taken in as the round's control-plane base. Validated through a `release.yml publish=false` dry run on the `0.71.0-rc1` branch. Coordination artifacts live in the untracked `dev/v0.71.0/` tree of the round host's checkout.

## What shipped

Five items, each an accepted roadmap proposal, merged into a single control plane and gated together.

- **OpenCode is a first-class terminal agent** ([terminal-gemini-opencode](../roadmap/done/terminal-gemini-opencode.md)). `SubmitAgent::OpenCode` joins Claude, Codex, and Gemini everywhere identity is derived from the spawn command or `CHAN_AGENT`: `cs terminal write --submit=opencode`, `CHAN_SUBMIT_OPENCODE`, `[opencode]` in `submit.toml`, Team Work bootstrap, and the SPA's server-reported identity. OpenCode submits as one bracketed-paste-plus-Return PTY write (multiline and ~20 KiB paste-sized proven); Gemini keeps its deliberate two-write split because Gemini 0.51.0 treats a Return within 30 ms of inserted text as Shift+Return. This branch was merged first and became the base every other lane rebased on, since it and the graph lane both edit the control-plane trio `wire.rs` / `control_socket.rs` / `cli.rs`.
- **Authenticated exact-origin desktop trust replaces the static wildcard grant** ([tauri-permission](../roadmap/done/tauri-permission.md)). The desktop app dropped the blanket `*.chan.app` / `*.devserver.chan.app` capability. Each gateway devserver is now trusted only for its exact authenticated origin, derived from the gateway's entry response and persisted per gateway as a `(gateway id, owner, full 64-char devserver id)` trust tuple. A shared row warns and asks for consent before its first connect, trust survives a restart (with the restart-only additive-ACL purge behaving), revoke tears down the row's windows, roster drift tears down cleanly, and a sibling, apex, wrong-port, or unrelated origin is refused. The gateway wire and API version did not change - only a focused entry-response test was strengthened.
- **Workspace search and graph traversal share one bounded contract** ([chan-workspace-graph-fix](../roadmap/done/chan-workspace-graph-fix.md)). `cs search`, `chan workspace search`/`graph`, the new `POST /api/search/workspace` route, and the MCP surface go through a single `workspace_search`; the four separate read tools collapse into one, with typed query/from/domain/depth/direction/edge-kind/limit selectors, and `--scope`/`--target`/`GraphScope` are gone. Link and mention/contact normalization moved into a shared `graph_normalize` with `/api/graph` output held byte-identical, and an 18-case golden fixture is asserted byte-for-byte in both the Rust core and the SPA's Vitest lens-parity test. This lane owned the control-plane trio for the round.
- **`chan upgrade --version X.Y.Z` resolves older releases** ([chan-upgrade-release-history-fix](../roadmap/done/chan-upgrade-release-history-fix.md)). The `/dl` metadata generator retains the last five GA versions as per-version CLI and desktop manifests plus a multi-entry `releases.json`, so pinning a non-latest version resolves instead of 404ing on `latest` only. This closes the v0.70.2 follow-up that noted the CLI threaded the version correctly but the deploy hosted only `latest`.
- **Two editor cosmetics** ([cosmetics](../roadmap/done/cosmetics.md)). The light-mode fenced-code fill sat within a few RGB steps of the page and read as no fill; it now uses GitHub's Primer gray as a distinct slab (siblings matched). The dark-mode selection was rendering CodeMirror's hard-coded light-grey base-theme default; the investigation corrected the brief's premise (the `--selection-bg` token had no consumer) and wired `.cm-selectionBackground` through the GitHub-blue token, keeping selected text legible.

Round 2 of this version - the terminal write-queue-drain fix - did not run; it is deferred (see Follow-ups).

## Team / process

A five-member team (`v0710r1`): one Claude lead as coordinator and integrator, two Codex subagents (graph-fix, tauri-permission) for the most complex lanes, and two Kimi subagents (upgrade-history, cosmetics) for the lighter ones. The lead owned the OpenCode intake, the shared control plane, the integration gate, all merges, the journals, and the RC cut; it did not implement lane work. Lanes were file-disjoint with a hard rule that only the graph lane touched the control-plane trio, so OpenCode and graph-fix serialized on it while the three independents ran in parallel.

Every UI-visible change committed a headless-Chrome browser-smoke check under `scripts/e2e/browser-smoke/checks/`; CLI/gateway/devserver behavior extended the committed sdme container e2e. Each lane reported a scoped-green commit with its harness; the lead ran the full gate plus an adversarial review (a second model trying to break the change) before accepting - nothing merged on "looks fine". The upgrade lane took two review cycles; the rest one.

## Validation

Each lane own-gated (scoped cargo tests, `clippy -D warnings`, `cargo fmt --check`, and the relevant vitest/browser-smoke). The lead gated each accepted lane from an isolated worktree and ran a full integrated `make pre-push` plus the complete browser-smoke suite over all four lanes on a clean `target/`, green across fmt, warnings-denied clippy, all-target tests, the no-default-features build, the gateway build, web-check with the full Vitest and production build, and the marketing smokes. GA validation was a `release.yml publish=false` dispatch on the `0.71.0-rc1` branch: context, Linux (CLI and desktop, both arches), Windows (signed), gateway (both arches), the macOS sign+notarize path, and Docker all green; publish/deploy jobs correctly skipped. The full cross-platform artifact set was downloaded and version-checked before GA.

The desktop's native-shell ACL enforcement and the real OAuth deep-link are the one surface that cannot be exercised headless; that host-only matrix (native macOS/Windows against the local `localtest.me` gateway stack or the unchanged live `id.chan.app`, plus the Gemini model-backed positive submit) is scripted in `dev/v0.71.0/host-smoke-round.md` and is the owner's to run. Everything the launcher SPA, the entry-response/authorization logic, and the backend guards do is covered by the browser-smoke launcher trust flow and the sdme devserver e2e (authenticated owner/editor entries, owner-409 and editor-403 behind `require_local_mutation`).

## Retrospective

### Highlights

- Adversarial review earned its place. The upgrade lane's first commit passed its full gate but the review caught that a forced `--tag` bypassed the GA filter, so a manual `pages.yml` dispatch of an rc tag would have republished `/dl` with the rc as `latest` and self-upgraded every client onto it; it was sent back and fixed with regression pins before intake.
- The graph lane caught a false gate. Its own-gate on the shared tree failed on a removed test and a stale golden - artifacts left by the round's concurrent Cargo processes in the shared `target/`; a zeroed target passed. The final integrated gate was rerun on a fresh `target/`, which is now the rule for the acceptance gate.
- The exact-origin trust rewrite held under a five-lens security review (wildcard fully removed, both mutation guards, box-fix serialization-safe, no missed construction site) and the entry-response-derived ACL turned out not to be `chan.app`-specific, which is what lets the host smoke run against a purely local gateway.

### Lowlights

- The shared `target/` both went stale (a false red on the first integrated gate) and ballooned to 162 GB under four concurrent lane builds; the fix was a clean-target regate, but the lesson is that a multi-agent round should not point its acceptance gate at the warm shared cache.
- The two Kimi lanes failed to spawn (the `kimi` binary was present but off the terminal-spawn PATH); they were closed and respawned with an absolute path. A pre-existing macOS timing test (`close_tears_down_the_separate_serve_process`) flaked once in the dry run and passed on re-run.

### Honest feedback

This was the first real exercise of the lead-plus-subagents team model end to end, and the disjoint-lane + serialize-the-control-plane discipline held: no cross-lane conflict reached the tree. The two things that bit were both about the shared working tree rather than the code - concurrent Cargo in one `target/` produces false test results, and lanes must own their staging surgically. Both are process rules now, not code changes. The host-only native smoke remaining as an owner task is the honest boundary of what can be automated for a native desktop security change.

## Follow-ups

- **Terminal write-queue-drain (round 2)** deferred out of v0.71.0; it rebases on the merged OpenCode + graph-fix control plane and adds only a `TermWrite.submit` field. Moved to the next active version's roadmap.
- The macOS `close_tears_down_the_separate_serve_process` test is timing-flaky on the CI runner (empty `chan close` stdout under load); it is pre-existing and in a code path this round did not touch, but it wants a budget widen or a deterministic-output hardening.
- The editor selection rule routes both focused and unfocused editors through the app-blue `--selection-bg`, dropping CodeMirror's unfocused dimming - a deliberate call that meets the "color + readability" ask; scope it to `.cm-focused` if the focus cue is wanted back.
- The `/dl` retention has one uncovered path (the just-tagged release also present in the fetched list) that is logic-verified and fail-safe via the generator's tested duplicate rejection, but has no fixture; coverable offline by extending the test-only `--release-json` envelope.

## Roadmap closure

Closed to `team/roadmap/done/`, each linking back here: [terminal-gemini-opencode](../roadmap/done/terminal-gemini-opencode.md), [tauri-permission](../roadmap/done/tauri-permission.md), [chan-workspace-graph-fix](../roadmap/done/chan-workspace-graph-fix.md), [chan-upgrade-release-history-fix](../roadmap/done/chan-upgrade-release-history-fix.md), [cosmetics](../roadmap/done/cosmetics.md), and the process migration [release-flow](../roadmap/done/release-flow.md). Deferred and carried forward: terminal-write-queue-drain.
