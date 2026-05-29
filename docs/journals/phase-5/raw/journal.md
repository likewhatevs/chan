# Chan Pre-Release Phase 5 Journal

Owner: @@Architect. Host: Alex.

Source request: [request.md](./request.md). Process: [process.md](./process.md).

## Plan summary

Phase 5 is a structural cleanup followed by hardening and bug fixes.

1. **Cleanup**. Bring the `../chan-term` terminal work cleanly onto `main`,
   then strip the in-app Agent overlay and Agent history overlay end to end:
   frontend components, store state, backend `/api/llm/*` and assistant-blob
   routes, and the chan-llm pieces that exclusively back the in-app agent.
   Preserve the chan MCP server and drive access for external agents
   (claude, codex, gemini) that connect through the MCP bridge.
2. **Enhancements**. Wire the embedded terminal's PTY environment so common
   external agents pick up chan's MCP server automatically. Prioritise graph +
   chan-report indexing ahead of full-text search; expose a search-indexer
   aggression knob; detect sudden filesystem changes (git/hg checkouts) and
   harden indexer resumption around them with end-to-end and correctness
   coverage.
3. **Bug fixes**. Confirm-on-close for tabs with unsaved files or live
   terminals (reload is fine), per-window pane/tab state for chan-desktop,
   editor scroll behavior when the cursor is near the top of a screen-sized
   page.
4. **Hardening close-out**. End-to-end run-throughs of every workflow above
   plus the final summary. Alex decides when the session closes.

## Repo state at phase start

* `chan` (this repo) is on `main`, **3 commits ahead of `origin/main`**.
  HEAD is `963bade web: add terminal tab controls`.
* The three local commits are the `../chan-term` terminal work, already
  rebased on top of phase-4 (`06017f4`). The merge for checklist item one
  is therefore already a local fast-forward; what remains is verification,
  any cleanup, and pushing.
* `../chan-term` is detached at the same `963bade`, clean working tree.
  Reflog shows the rebase that produced the current local main.

This invalidates the original merge plan drafted in
[backend-1.md](./backend-1.md), which assumed the commits still had to be
brought across. That task will be reset to a verification + push-readiness
brief once the capacity plan is agreed.

## Request checklist

### Cleanup
- [x] Plan and finalise the `../chan-term` merge onto `main` (already on
  local main; push owed at phase close per Alex's decision).
- [x] Remove the Agent overlay and Agent history overlay from the frontend.
  ([frontend-1](./frontend-1.md), [frontend-2](./frontend-2.md))
- [x] Remove the backend that backed those overlays.
  ([backend-1](./backend-1.md))
- [x] Strip Agent-only pieces from chan-llm and global config.
  ([systacean-1](./systacean-1.md): chan-llm is MCP-only; chan-drive's
  `*_assistant` blob API is gone.) MCP server + drive access preserved.
- [x] Documentation refresh so design.md, CLAUDE.md, README.md, and any
  references reflect the new boundary. ([architect-2](./architect-2.md):
  CLAUDE.md + design.md rewritten, README.md spot audit clean. Commit
  groupings still owed in architect-2 once wave-2 closes.)

### Enhancements
- [x] Embedded terminal sets ENV variables for claude / codex / gemini so
  they pick up chan's MCP server by default. ([backend-1](./backend-1.md);
  validated live by [webtest-1](./webtest-1.md) and
  [webtest-2](./webtest-2.md). Real CLI end-to-end validation owed on a
  machine that has claude/codex/gemini installed; tracked in
  [architect-1](./architect-1.md) follow-ups.)
- [x] Prioritise graph and chan-report indexing ahead of search indexing.
  ([systacean-2](./systacean-2.md): watcher gate + indexer scheduling
  in REVIEW.)
- [x] Configurable knob controlling how aggressive the search indexer is.
  ([systacean-3](./systacean-3.md): `[search].aggression` enum +
  `chan serve --search-aggression`.)
- [x] Detect sudden filesystem changes (git/hg checkouts) and index
  appropriately when the drive is a git or hg repo.
  ([systacean-4](./systacean-4.md): drive-root VCS detection +
  coalesced rebuild path.)
- [x] End-to-end tests and benchmarks for indexing under sudden fs changes,
  including correctness tests. ([systacean-4](./systacean-4.md):
  convergence + resume tests + ignored manual profile.)
- [x] Harden indexing interruption and resume to survive sudden fs changes.
  ([systacean-4](./systacean-4.md): staged-row mtime/size check.)
- [x] Persistent terminal sessions so terminal tabs survive window
  reloads. Direction set by [architect-tmux-1](./architect-tmux-1.md)
  (Option 4: chan-native PTY session registry). Implementation:
  [systacean-5](./systacean-5.md) (server registry),
  [frontend-4](./frontend-4.md) (client reattach),
  [systacean-8](./systacean-8.md) (bootstrap hydration ordering),
  [systacean-10](./systacean-10.md) (alt-screen sniff + winsize
  wobble for TUI structural redraw). Validated live by
  [webtest-1](./webtest-1.md) round-10: htop / vim / less reload
  screenshot diff matches fresh-launch pixel-for-pixel; alt-screen
  enter/exit debug logs fire on every transition. **Open question
  not blocking the phase**: plain bash scrollback isn't re-rendered
  on reload (current contract slices ring `since=last_seq`,
  reload's xterm.js buffer starts empty). Pre-existing, predates
  systacean-10. Recorded in the "Notes & decisions" section below.

### Bug fixes
- [x] Closing a tab with unsaved files or a live terminal session prompts
  for confirmation. Reload remains uninterrupted.
  ([frontend-3](./frontend-3.md))
- [x] chan-desktop windows keep distinct pane/tab state across reloads.
  ([frontend-3](./frontend-3.md))
- [x] Editor does not auto-scroll when the cursor is near the top of a
  screen-sized page (final scroll policy is open; flagged for decision).
  ([frontend-3](./frontend-3.md))

### Closing
- [ ] End-to-end hardening pass across all of the above before Alex calls
  the phase complete.

Statuses on the table below: TODO, IN_PROGRESS, BLOCKED, REVIEW, DONE.

## Dispatch

Wave 1 was already in flight by @@Backend and @@Frontend by the time
@@Architect finished orientation; cleanup substantially landed before the
formal task table was written. Wave 1 is reconciled below; wave 2 covers
remaining cleanup residue plus enhancements and bug fixes.

| Task | Owner | Status | Notes |
|------|-------|--------|-------|
| [backend-1](./backend-1.md) | @@Backend | REVIEW | Wave 1. Removed `/api/llm/*`, `/api/assistant/*`, `/api/answers`, `llm.toml` state, LLM `/ws` events; preserved MCP bridge; added terminal MCP env (`CLAUDE_MCP_SERVER_JSON` / `CODEX_MCP_SERVER_JSON` / `GEMINI_MCP_SERVER_JSON` + chan-native helpers). Also removed the unused `/api/llm/*` client methods + types from `web/src/api/client.ts`. Verified `cargo check -p chan-server`, `cargo test -p chan-server`, `npm run check`, `npm test`. Needs @@Systacean review and full pre-push gate. |
| [frontend-1](./frontend-1.md) | @@Frontend | REVIEW | Wave 1. Removed Agent / Agent history overlay mounts, keyboard routing, central shortcut entries, file-tab + empty-pane menu entries, Settings section, and orphan component files. Verified `npm run check`, `npm test` (14 files, 168 tests). |
| [frontend-2](./frontend-2.md) | @@Frontend | REVIEW | Wave 1 residue. Removed remaining assistant/scope-history store state, hash/session paths, stale tab/file cleanup hooks, assistant blob client/type exports, assistant store tests, and stale frontend comments/design notes. `web/src/state/store.test.ts` now covers graph hash/watch behavior only. Verified `npm --prefix web run check`, `npm --prefix web test -- --run`, `npm --prefix web run build` (build warnings only). |
| [systacean-1](./systacean-1.md) | @@Systacean | REVIEW | Wave 1 deep prune complete. chan-llm is MCP-only (prompts/tools/MCP server); in-app session, CLI config/detection, backend modules, bench, dead error variants, and stale docs are gone. chan-drive assistant blob helpers/tests/state-dir reservation are gone; reset progress now counts four state subsystems. MCP bridge and `chan __mcp` preserved. Full gate green: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --no-default-features`, `cargo test`, `npm --prefix web run check`, `npm --prefix web test -- --run`, `npm --prefix web run build`. |
| [systacean-2](./systacean-2.md) | @@Systacean | REVIEW | Wave 2 indexer lane. Tightened chan-server watcher scheduling: provider-error/path-less events request coalesced rebuild, incremental gate matches chan-drive `is_indexable_text`, due deletions apply before upserts. Verified `cargo fmt --check`, `cargo test -p chan-server indexer`, `cargo clippy -p chan-server --all-targets -- -D warnings`. |
| [backend-2](./backend-2.md) | @@Backend | REVIEW | Wave 2 desktop session fix. chan-desktop appends `w=<window-label>` to each drive window URL; the web session API uses that key for `/api/session` with browser fallback to `default`; pagehide keepalive uses the same path. Added focused client tests. Verified `cargo fmt --check`, `cargo check -p chan-desktop`, `cargo build -p chan`, `npm --prefix web run check`, focused Vitest, full `npm --prefix web test`, and `npm --prefix web run build`. |
| [webtest-1](./webtest-1.md) | @@Webtest A | IN_PROGRESS | Round-5 smoke against PID 67369 on `/private/tmp/chan-test-phase5`. All wave-1 + wave-2 acceptance PASS except **frontend-4 PARTIAL FAIL**: BUG-WT5-C (terminal reload spawns a fresh PTY when the URL hash is present, even though the server-side PTY persists). Also surfaced OBS-WT5-D (two plain-browser tabs share `w=default`, last-writer-wins on the session blob). [frontend-6](./frontend-6.md) and [frontend-7](./frontend-7.md) are now REVIEW; rebuild/re-smoke can validate both fixes. |
| [webtest-2](./webtest-2.md) | @@Webtest B | REVIEW | All six scenarios PASS on the post-frontend-2 / post-systacean-1 bundle (removed routes 404, /api/config agent-free, /ws frame types, terminal MCP env 8/8, hash-state probe, shortcut chord sanity, settings round-trip, network panel walk, per-window reload baseline captured). Five follow-ups filed; reconciliation in the log below. |
| [architect-1](./architect-1.md) | @@Architect | IN_PROGRESS | Coordination, decisions log, wave-2 dispatch below. |
| [architect-2](./architect-2.md) | @@Architect | TODO | Docs sweep (CLAUDE.md, design.md, README.md, READMEs across crates) so the new MCP-only / no-in-app-agent boundary lands in writing. |
| [architect-tmux-1](./architect-tmux-1.md) | @@Architect | DECIDED | Option 4 (chan-native PTY session registry, no external compatibility) confirmed by Alex. Memo stays as decision record; implementation in [systacean-5](./systacean-5.md) + [frontend-4](./frontend-4.md). Defaults locked: idle timeout 30 min, session cap 32 per drive, drive close kills sessions immediately. |
| [systacean-3](./systacean-3.md) | @@Systacean | REVIEW | Wave 2 search-aggression knob. `[search].aggression` enum (conservative / balanced / aggressive) threaded through chan-drive build budgets, chan-server config, `chan serve --search-aggression` CLI flag, `/api/config`, watcher debounce, and storage-reset indexer respawn. Balanced preserves prior worker/queue/embed-batch/debounce defaults; conservative lowers; aggressive raises. Full gate green: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo check --no-default-features`, `cargo test`, ignored fixture profile, `npm run check`, `npm test -- --run`, `npm run build`. |
| [systacean-4](./systacean-4.md) | @@Systacean | REVIEW | Wave 2 fs-change correctness. Drive-root VCS detection for `.git/HEAD` and `.hg/dirstate`; watcher filter only lets `.git/HEAD`, `.git/index`, `.hg/dirstate` through; server indexer coalesces those plus large VCS-aware bursts into one full rebuild. Graph rebuild resume skips staged rows only when `(mtime, size)` still matches disk; checkout modifications purge and reparse stale rows. Added convergence + resume tests + an ignored manual profile (80 files / 20 touched: initial 11078ms, checkout settle 3138ms, staged resume 235ms). Full gate green. |
| [systacean-5](./systacean-5.md) | @@Systacean | REVIEW | Wave 2 terminal persistence (server side). Added `terminal_sessions::Registry` on AppState: long-lived PTYs keyed by random session id, byte-offset ring replay, monotonic seq, idle prune, soft cap, and server-initiated close reasons. Rewrote `routes/terminal::api_terminal_ws` as a thin attach/detach handler and kept the existing `ready` frame after replay for compatibility. Full gate green. |
| [frontend-4](./frontend-4.md) | @@Frontend | REVIEW (with follow-up) | Wave 2 terminal persistence (client side). Persistence + control-frame handling + missed-scrollback UI all landed. End-to-end smoke surfaced BUG-WT5-C: the bootstrap layout-restore path discards the persisted `tsid`/`tseq` when the URL hash is present, so the headline reload-survives contract is currently not met. Tracked separately in [frontend-6](./frontend-6.md); the original frontend-4 scope stays REVIEW (the bug is in the store bootstrap path, not in the TerminalTab plumbing this lane delivered). |
| [frontend-6](./frontend-6.md) | @@Frontend | REVIEW | **HIGH** — BUG-WT5-C fix. Hash-restored terminal tabs now hydrate `tsid`/`tseq` from the matching per-window session-blob layout before TerminalTab mounts. Added regression test `hydrates terminal session ids onto hash-restored terminal tabs`. Verified `npm --prefix web run check`, `npm --prefix web test -- --run`, `npm --prefix web run build` (build warnings only). Webtest A re-runs the reload smoke after rebuild. |
| [frontend-7](./frontend-7.md) | @@Frontend | REVIEW | MEDIUM — OBS-WT5-D fix. Plain-browser tabs without URL `w=` now generate/reuse a per-tab 8-hex sessionStorage key; chan-desktop URL `w=<window-label>` still wins; storage failures fall back to `default` with one warning. Added client tests. Verified `npm --prefix web run check`, `npm --prefix web test -- --run`, `npm --prefix web run build` (build warnings only). Webtest A round-6 confirmed PASS live. |
| [systacean-8](./systacean-8.md) | @@Systacean | REVIEW | **HIGH** — BUG-WT5-C round-2. Fixed the bootstrap ordering race: `bootstrap()` fetches the session blob before hash restore and calls `restoreLayout(fromHash, sessionLayout)` so terminal `tsid`/`tseq` are present before TerminalTab mounts. Added a `bootstrapHydrated` save gate so auto-save/pagehide cannot clobber the session blob with a tsid-less layout during hydration. Verified `npm --prefix web run check`, `npm --prefix web test -- --run`, `npm --prefix web run build`. Webtest A owns the live reload re-smoke. |
| [frontend-6](./frontend-6.md) | @@Frontend | REVIEW (insufficient) | The graft itself works, but the call order means TerminalTab mounts before hydration. Corrective work in [systacean-8](./systacean-8.md); the original frontend-6 patch stays in tree as a building block for the new fix. |
| [frontend-9](./frontend-9.md) | @@Frontend | REVIEW | MEDIUM — Terminal Alt-key word motions. Enabled xterm.js `macOptionIsMeta` so `Alt+letter` emits readline Meta-prefix input, and added a focused custom key handler mapping `Alt+ArrowLeft/Right`, `Alt+Backspace`, and `Alt+Delete` to `\x1bb`, `\x1bf`, `\x1b\x7f`, and `\x1bd` through the normal user-input/broadcast path. Added `web/src/terminal/keymap.test.ts` coverage for mapped bytes, pass-through chords, and the xterm swallow contract. Verified `npm --prefix web run check`, `npm --prefix web test -- --run`, `npm --prefix web run build` (build warnings only). @@Webtest A owns live shell verification on the rebuilt bundle; @@Systacean can post-review the byte sequences. |
| [backend-3](./backend-3.md) | @@Backend | REVIEW | MEDIUM — Scoped terminal MCP env to the CHAN_ namespace only. Dropped `CLAUDE_MCP_SERVER_JSON` / `CODEX_MCP_SERVER_JSON` / `GEMINI_MCP_SERVER_JSON` from PTY env, added `mcp_env=on\|off` on `/api/terminal/ws`, and honored it for fresh PTY session creation only. Updated live docs to the CHAN-only story. Added `routes::terminal::tests::mcp_env_off_omits_chan_mcp_vars`. Full gate green. |
| [frontend-10](./frontend-10.md) | @@Frontend | REVIEW | MEDIUM — Terminal tab title-menu toggle "Set MCP env vars" is implemented per tab with default ON and per-window-session persistence only. Added `sessionMcpEnv` sidecar state for the currently attached PTY, info popover with new-session-only semantics, and a "Show MCP env in terminal" button that injects `env | sort | grep '^CHAN_MCP_'` through the normal input path. `terminalWsPath()` now appends `mcp_env=off` only for fresh opt-out sessions and omits it on reattach. Added URL, serialization, and command-injection tests. Verified `npm --prefix web run check`, `npm --prefix web test -- --run`, `npm --prefix web run build` (build warnings only). Live opt-out behavior can now be validated with [backend-3](./backend-3.md). |
| [systacean-9](./systacean-9.md) | @@Systacean | REVIEW (insufficient) | MEDIUM — BUG-WT5-E fixed server-side. Reattach now requests a redraw after replay by re-applying the current PTY winsize through the controller thread, and sessions sniff `\x1b[?1049h/l` to skip byte-ring replay while alt-screen is active. Added registry tests for alt-screen replay suppression, sniffer sequences, and redraw resize broadcast. Full gate green; Webtest A owns htop/vim/less live re-smoke. |
| [systacean-10](./systacean-10.md) | @@Systacean | REVIEW | MEDIUM — BUG-WT5-E round-2 fix landed server-side. Alt-screen sniffing is now cross-chunk-safe with a rolling tail buffer and debug logs on enter/exit, alt-screen reattach sends the clean-screen prelude before redraw, and redraw now performs a real winsize wobble (rows-1 -> 50ms -> original) on the per-session controller thread. Added registry tests for split escape sequences and wobble resize order. Full local gate green; Webtest A owns htop/vim/less screenshot-diff re-smoke. |
| [systacean-6](./systacean-6.md) | @@Systacean | REVIEW | BUG-WT5-A confirmed resolved by [systacean-4](./systacean-4.md)'s watcher classifier/filter path: Created and Modified indexable files now share the same per-file apply path. Added `create_event_admits_new_indexable_file_into_bm25` for new `.md` and `.txt` create events. Full gate green. |
| [frontend-5](./frontend-5.md) | @@Frontend | REVIEW | Strip unknown legacy keys from the URL hash on the next hash write so pre-Phase-5 fragments stop surviving reloads. `persistStateToHash()` canonicalizes to known keys; regression test keeps live `settings=1` while dropping stale keys. Verified `npm --prefix web run check`, `npm --prefix web test -- --run`, `npm --prefix web run build` (build warnings only). |
| [architect-3](./architect-3.md) | @@Architect | TODO | Rewrite the three chan-term commit messages (`0f4614e`, `980fc3e`, `963bade`) to match the repo's canonical style (68-col wrap, surface-bulleted body, framed why). Drafts in the task file; non-interactive rebase recipe captured. Runs after wave-2 finishes and before the final push. |
| [systacean-7](./systacean-7.md) | @@Systacean | REVIEW | Built and installed Phase 5 `Chan.app` at `/Applications/Chan.app` from local wrapped HEAD. `desktop/make build` produced `target/release/bundle/macos/Chan.app` and `target/release/bundle/dmg/Chan_0.8.1_aarch64.dmg`; installed app is 0.8.1, quarantine-free, ad-hoc re-signed with `codesign --verify --deep --strict` PASS, and launches as `chan-desktop` with window `Chan Desktop`. Alex owns the remaining manual click-through inside the GUI. |
| [webtest-3](./webtest-3.md) | @@Webtest A | BLOCKED | After the commits land, rebuild `target/debug/chan` against committed HEAD and bring the test service back up on the existing `chan-test-phase5` drive so Alex has a plain-browser surface for click-around bug hunting alongside Chan.app. Posts URL + bearer token in the task file. |
| [frontend-3](./frontend-3.md) | @@Frontend | REVIEW | Wave 2 bug fixes. Added shared in-app close confirmation for dirty file tabs and live terminal tabs, reviewed per-window session path/tests from [backend-2](./backend-2.md), and changed saved-caret restore to nearest scrolling. Verified `npm --prefix web run check`, `npm --prefix web test -- --run`, `npm --prefix web run build` (build warnings only). |

## Cleanup surface (confirmed after lib.rs and routes/mod.rs scan)

* `crates/chan-server/src/routes/llm.rs` is **already orphaned**. It is not
  declared in `routes/mod.rs` and not wired in `lib.rs::router()`. Deletion
  is pure source removal.
* `crates/chan-server/src/routes/sessions.rs` still exports `api_get_session`,
  `api_put_session`, `api_delete_session`, `api_list_sessions` (kept,
  window-session blob), plus dead `api_*_assistant` handlers that are
  **not** in `mod.rs` exports or the router. The `*_assistant` block plus
  its tests come out; the window-session blob stays.
* chan-server consumes chan-llm **only** through `chan_llm::mcp::Server`
  in `crates/chan-server/src/mcp_bridge.rs`. No `LlmSession`, `LlmMessage`,
  or `chan_llm::backends::*` is referenced. That means the chan-llm strip
  (session + claude_cli + codex_cli + gemini_cli backends) is safe to do
  without touching chan-server beyond `Cargo.toml` features.
* Frontend `web/src/api/client.ts` still has `llmStatus`, `llmCliDetection`,
  `llmComplete`, `llmTools`, `getAssistantBlob` etc. These call dead
  endpoints today. The frontend overlay removal also removes those
  bindings.
* chan-drive `*_assistant` blob API (sibling chan-core repo) goes too,
  per Alex.

## Decisions (confirmed with Alex 2026-05-17)

1. **Capacity: six slots.** 1 @@Architect, 1 @@Frontend, 1 @@Backend,
   1 @@Systacean, 2 @@Webtest (A & B). @@Systacean is a new combined
   profile addressed directly by @@Architect.
2. **chan-core change is in scope.** Delete chan-drive's `*_assistant`
   blob API in this phase. @@Systacean owns the chan-core change and
   coordinates the path-dep bump back in chan.
3. **Delete chan-llm session + CLI backends now.** Keep `mcp.rs`,
   `tools.rs`, `prompts.rs`, and whatever `cli.rs` / `config.rs` /
   `error.rs` the MCP server still needs. Revisit if mobile shells
   later need an in-app agent.
4. **Push at phase close.** Don't push the three terminal commits
   today. Verify after the cleanup and ship together.

## Capacity proposal

Profile demand for this phase:

* **@@Architect** (me). Coordination, journal, task files, summary.
* **@@Frontend**. Overlay removal, settings/store cleanup, editor scroll
  fix, tab close confirm UI, per-window state plumbing.
* **@@Backend**. Strip `/api/llm/*` and assistant-blob routes, router/lib
  wiring, terminal PTY env vars for external agents (with @@Systacean
  review for systems and Rust quality).
* **@@Systacean** (mix of @@Syseng + @@Rustacean, introduced this phase).
  Indexer prioritisation, search aggression knob, fs-change detection,
  resumption hardening, end-to-end + correctness tests, benchmarks, and
  Rust quality review for backend changes. Also the natural owner for the
  tmux `-CC` design spike.
* **@@Webtest A** and **@@Webtest B**. Per Alex, two webtest slots are
  planned. They run the live test service, smoke landed slices, and route
  reproductions back to owners.

Proposed initial slot map (six profiles, six slots):

| Slot | Profile | Initial focus | Notes |
|------|---------|---------------|-------|
| 1 | @@Architect | journal, task files, decisions | me |
| 2 | @@Frontend | overlay/store/settings removal | switch to bug-fix lane after cleanup |
| 3 | @@Backend | routes + chan-llm strip + terminal env | coordinates with @@Systacean on chan-llm boundary |
| 4 | @@Systacean | indexer + fs-change correctness + Rust review | reviews @@Backend's chan-llm strip |
| 5 | @@Webtest A | live web test service + cleanup smoke | primary smoke owner |
| 6 | @@Webtest B | parallel manual scenarios on the same service | shares state with Webtest A |

Expected later switches:

* @@Frontend pivots from cleanup to bug fixes once removal lands.
* @@Backend can absorb the terminal env-var task or hand it to @@Systacean
  depending on where Rust integration lands; coordination through task
  files.
* @@Systacean stays busy through the indexer + tmux-CC spike.

Known gaps:

* No dedicated security/hardening slot. @@Systacean covers Syseng review;
  hardening passes happen on each lane before commit.
* tmux `-CC` integration is potentially large. The first task should be a
  design memo with Alex, not implementation.

Open questions for Alex before initial task creation:

1. Confirm six slots: 1 Architect, 1 Frontend, 1 Backend, 1 Systacean,
   2 Webtest. Or different numbers?
2. The Agent overlay removal touches `chan-drive`'s `*_assistant` blob
   API in the sibling `chan-core` repo. Are we authorised to make that
   chan-core change in this phase, or should chan-server keep using the
   dead blob API until a separate chan-core PR?
3. The `chan-llm` crate's `LlmSession` / claude_cli / codex_cli / gemini_cli
   backends become unreferenced once the in-app agent is gone. Native
   mobile shells were intended to consume `LlmSession` via uniffi later.
   Delete now, or keep behind feature-gate / module so mobile can use it?
4. Push timing for the unpushed three terminal commits: push now to align
   `origin/main`, or wait until the phase closes so cleanup + terminal
   land together?

## Notes & decisions

* The `../chan-term` work is already on local `main`. Treat the
  "merge plan" checklist item as verification + push readiness.
* The MCP server (`crates/chan-server/src/mcp_bridge.rs` + chan-llm's
  `mcp.rs`, `tools.rs`, `prompts.rs`) is load-bearing for external agents
  and stays. The in-app `LlmSession`, /api/llm/*, /api/assistant/*, and
  overlay UI are what get removed.
* `chan-drive`'s `put_assistant` / `get_assistant` / `list_assistant` /
  `delete_assistant` / `clear_assistant` exist in the sibling chan-core
  repo. Removing them needs an explicit chan-core change; coordination
  recorded in question 2 above.
* Pre-push gate stays: `cargo fmt --check`, `cargo clippy --all-targets
  -- -D warnings`, `cargo test`, `npm --prefix web run check`, and
  `npm --prefix web run build` before any push. CI re-runs these.
* **Open question (parked, not blocking phase-5)**: on plain bash
  reload, the user sees an empty buffer instead of their recent
  scrollback. Current contract slices the ring `since=last_seq`
  and the new xterm.js instance starts blank; the wobble's
  SIGWINCH only makes bash redraw its current prompt. Alt-screen
  TUIs are fine (the program does its own structural redraw).
  Two possible spec changes if Alex wants this fixed in a later
  phase: (a) replay the entire ring (`from start_seq`) on every
  non-alt-screen attach — up to 1 MiB, simple; (b) replay only
  the last screenful (last ~24 × cols bytes) — bounded, cleaner.
  Either change is small and lives in
  `crates/chan-server/src/terminal_sessions.rs`. Filing as
  follow-up; no task file in phase-5.

## Log

* 2026-05-17 @@Architect: read [request.md](./request.md),
  [process.md](./process.md), `~/.ai/skills/architect/guide.md`, prior
  phase-3 journal/task layout, and the current chan/chan-term/chan-llm
  state. Wrote this journal with checklist, capacity proposal, and the
  four open questions above.
* 2026-05-17 @@Architect: Alex confirmed all four open questions:
  six slots as proposed, chan-drive `*_assistant` blob API deletion is
  in scope this phase, delete chan-llm `LlmSession` + CLI backends now
  (revisit if mobile needs it later), and push the three terminal
  commits at phase close (not today).
* 2026-05-17 @@Architect: discovered wave-1 cleanup was already in
  flight while orientation was in progress. @@Backend updated
  [backend-1](./backend-1.md) with the removed `/api/llm/*` and
  `/api/assistant/*` surfaces and the terminal MCP env-var work;
  @@Frontend wrote [frontend-1](./frontend-1.md) covering overlay UI
  removal. Reconciled the dispatch table to mark both REVIEW and
  added wave-1 residue + deep-prune + Webtest tasks. Workspace layout
  note: `crates/chan-drive`, `crates/chan-llm`, `crates/chan-report`,
  and the tunnel crates are workspace members in this repo (CLAUDE.md
  is stale on "sibling chan-core" framing). Deep prune lands inside
  this checkout.
* 2026-05-17 @@Architect: round-2 housekeeping.
  [systacean-1](./systacean-1.md) IN_PROGRESS (deep prune started,
  MCP bridge + `chan __mcp` explicitly preserved).
  [webtest-1](./webtest-1.md) IN_PROGRESS; unblocked @@Webtest A by
  pointing the lane at the existing `chan-test-phase5` drive
  (`/private/tmp/chan-test-phase5`, registered, last opened 18:09
  today) instead of seeding a new throwaway.
  [backend-1](./backend-1.md) extended to also drop unused `/api/llm/*`
  client methods + types from `web/src/api/client.ts`; reflected in
  the dispatch row. [frontend-2](./frontend-2.md) acceptance criteria
  refined: scope is now the remaining `assistantHash16` /
  `getAssistantBlob*` / `*AssistantBlob*` client methods, the LLM
  type set still in `api/types.ts`, and the `assistantStream`
  machinery in `state/store.svelte.ts`. `request.md` was touched
  with no content change.
* 2026-05-17 @@Architect: round-3 housekeeping. Wave-1 is now
  functionally complete: [frontend-2](./frontend-2.md) REVIEW
  (@@Backend / @@Systacean took the residue), [systacean-1](./systacean-1.md)
  REVIEW with the full pre-push gate green (`cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`,
  `cargo build --no-default-features`, `cargo test`, `npm run check`,
  `npm test`, `npm run build`), [webtest-1](./webtest-1.md) and
  [webtest-2](./webtest-2.md) both IN_PROGRESS with end-to-end PASS
  on backend-1 + frontend-1; @@Webtest A self-triggers a rebuild + re-
  smoke now that frontend-2 + systacean-1 are REVIEW. Reconciled
  the request checklist and dispatch table accordingly. Dispatched
  wave 2: [systacean-2](./systacean-2.md) (indexer prioritisation +
  search aggression knob), [systacean-3](./systacean-3.md) (git/hg
  checkout detection + correctness tests + benchmarks + indexer
  resumption), [frontend-3](./frontend-3.md) (the three bug fixes),
  [architect-2](./architect-2.md) (docs sweep), and
  [architect-tmux-1](./architect-tmux-1.md) (tmux `-CC` design memo
  that needs Alex input before any implementation). Working tree
  is 75 files changed, ~23k lines net deleted; not yet committed
  (commit coordination is part of [architect-2](./architect-2.md)).
* 2026-05-17 @@Backend: checked updates and picked up
  [frontend-2](./frontend-2.md) residue. Removed remaining
  assistant/scope-history store state, hash/session paths, stale
  tab/file cleanup hooks, and assistant/LLM API types. Deleted
  scope-history tests and rewrote store tests around graph-only
  behavior. Verified `npm --prefix web run check`,
  `npm --prefix web test`, and `npm --prefix web run build`.
* 2026-05-17 @@Backend: checked updates again and picked up the
  backend side of the chan-desktop per-window session bug as
  [backend-2](./backend-2.md). Desktop drive windows now append
  `w=<window-label>`; web session reads/writes and pagehide keepalive
  use that key, with browser fallback to `default`. After
  [frontend-3](./frontend-3.md) reached REVIEW, reran the current web
  gate and rebuilt `target/debug/chan` for Webtest.
* 2026-05-17 @@Architect: round-4 housekeeping. Wave 2 moved fast
  in parallel while I was drafting briefs:
  [systacean-2](./systacean-2.md) (watcher gate + indexer
  scheduling) REVIEW; [backend-2](./backend-2.md) (per-window
  desktop session keys) self-dispatched by @@Backend and REVIEW;
  [frontend-3](./frontend-3.md) (close confirmation + per-window
  session + caret/scroll change) REVIEW;
  [systacean-3](./systacean-3.md) (search-aggression knob)
  IN_PROGRESS after @@Systacean self-claimed it. Created
  [systacean-4](./systacean-4.md) for fs-change correctness +
  resumption (split from the combined wave-2 brief so each lane has
  clean acceptance criteria).
  [architect-2](./architect-2.md) IN_PROGRESS: rewrote `CLAUDE.md`
  (dropped the stale "sibling chan-core" framing, added the
  MCP-only / no-in-app-agent principle, refreshed contributor
  patterns + docs paths) and the relevant sections of `design.md`
  (workspace layout, chan-server module + routes table, chan-llm
  crate description, feature ownership table). Final grep across
  README + design.md + crates/*/README.md returns only intentional
  historical references. Pending in architect-2: README.md spot
  audit and the per-lane commit groupings once wave-2 finishes.
  tmux memo at [architect-tmux-1](./architect-tmux-1.md) carries
  four questions for Alex; not pinging via prompt right now per
  the no-clarifying-questions directive — Alex can read the memo
  and react in his own time.
* 2026-05-17 @@Systacean: [systacean-3](./systacean-3.md) REVIEW.
  Added the search aggression enum and threaded it through chan-drive
  build budgets, chan-server `[search].aggression`, CLI
  `chan serve --search-aggression`, `/api/config`, watcher debounce,
  and storage-reset indexer respawn. Balanced preserves prior worker,
  queue, embed-batch, and debounce behavior; conservative lowers the
  resource budget; aggressive raises it. Verification green:
  `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo check --no-default-features`, `cargo test`,
  ignored fixture profile, `npm --prefix web run check`,
  `npm --prefix web test -- --run`, and `npm --prefix web run build`.
* 2026-05-17 @@Systacean: [systacean-4](./systacean-4.md) REVIEW.
  Added drive-root VCS detection for `.git/HEAD` and `.hg/dirstate`,
  allowed only `.git/HEAD`, `.git/index`, and `.hg/dirstate` through
  the watcher filter, and taught the server indexer to coalesce those
  control-file events plus large VCS-aware bursts into one full
  rebuild. Hardened graph rebuild resume so already-staged rows are
  skipped only when their `(mtime, size)` still matches disk; checkout
  modifications now purge and reparse stale staged rows. Added
  checkout convergence and resume tests plus an ignored manual profile
  (80 files, 20 touched: initial 11078ms, checkout settle 3138ms,
  staged resume 235ms). Full gate green: `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`,
  `cargo build --no-default-features`, `cargo test`,
  ignored checkout profile, `npm --prefix web run check`,
  `npm --prefix web test -- --run`, and `npm --prefix web run build`.
* 2026-05-17 @@Architect: Alex read
  [architect-tmux-1](./architect-tmux-1.md) and asked for an
  alternative that lives entirely in chan, drops external
  compatibility, and keeps sessions alive on the web service
  while the UI can freely reload. Drafted **Option 4**
  (chan-native PTY session registry with attach/detach
  semantics, ring-buffer replay, and unguessable session ids)
  inside the memo and recommended it. Alex confirmed.
  Implementation split into two new task files:
  [systacean-5](./systacean-5.md) for the chan-server registry +
  lifecycle + tests, and [frontend-4](./frontend-4.md) for the
  tab descriptor + reattach plumbing. Defaults locked: idle
  timeout 30 min, soft cap 32 sessions per drive, drive close
  kills sessions immediately. End-to-end reload + multi-attach
  smoke owed to @@Webtest A on a build that has both lanes.
  Updated the request checklist row from "tmux design" to
  "Persistent terminal sessions" and linked the two
  implementation tasks. Also reconciled
  [systacean-3](./systacean-3.md) and
  [systacean-4](./systacean-4.md) to REVIEW (both shipped while
  the tmux brief was being drafted) and ticked the four
  enhancement checklist items they close.
* 2026-05-17 @@Architect: round-5 housekeeping.
  [frontend-4](./frontend-4.md) REVIEW (terminal-tab client side
  with the agreed wire contract — `{type:"close"}` for explicit
  close, `last_seq` as a byte offset); systacean-5's brief mirrors
  the same contract. [systacean-5](./systacean-5.md) IN_PROGRESS
  on the server registry. [webtest-2](./webtest-2.md) REVIEW with
  all six scenarios PASS on the post-frontend-2 / post-systacean-1
  bundle.
  [webtest-1](./webtest-1.md) stayed IN_PROGRESS and surfaced
  **BUG-WT5-A**: the incremental indexer misses newly-created
  files (modify path admits within ~5 s; create path never does).
  Routed to [systacean-6](./systacean-6.md) with full repro
  commands, two hypotheses, and a debug-log offer from @@Webtest A.
  [systacean-2](./systacean-2.md) stays REVIEW (the regression is
  on the create path, distinct from what that lane delivered) but
  the request-checklist row points back to it for context.
  Reconciled @@Webtest B's five follow-ups: items 1 + 2
  (`web/index.html` Favicon comment, `web/src/design.md`
  `--assistant-accent` references) are **already clean** in the
  current tree (grep returns zero matches across `web/`); item 3
  becomes [frontend-5](./frontend-5.md) (strip unknown hash keys
  on next write; @@Architect made the call); item 4 (PATCH
  `/api/config` rejects partial body) is a backend-semantics
  decision, parked for Alex (not blocking phase close); item 5
  (real `claude`/`codex`/`gemini` CLI validation) needs a host
  with those CLIs installed and is tracked as a known
  open follow-up in [architect-1](./architect-1.md).
  Alex flagged the three chan-term commits as off-style for the
  repo; created [architect-3](./architect-3.md) with full drafts
  for `0f4614e`, `980fc3e`, `963bade` plus a non-interactive
  rebase recipe that matches by current subject line. Holding for
  Alex's sanity-check before the rewrite fires; sequencing put in
  the task file (rewrite after wave-2 finishes, before the final
  push).
* 2026-05-17 @@Systacean: [systacean-5](./systacean-5.md) REVIEW
  and [systacean-6](./systacean-6.md) REVIEW. Server-side terminal
  persistence now has a chan-native PTY session registry on
  `AppState`, with attach/detach WebSockets, byte-offset replay,
  idle pruning, drive/shutdown/explicit close reasons, session cap,
  and `[terminal]` config exposed through `/api/config`,
  preferences, web API types, and `chan config`. The route sends the
  agreed first `{type:"session"}` frame and preserves the old
  `{type:"ready"}` frame after replay for current clients. Confirmed
  BUG-WT5-A was resolved by the [systacean-4](./systacean-4.md)
  watcher classifier/filtering path and added
  `create_event_admits_new_indexable_file_into_bm25` covering new
  `.md` and `.txt` create events. Full gate green:
  `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo build --no-default-features`, `cargo test`,
  `npm --prefix web run check`, `npm --prefix web test -- --run`,
  and `npm --prefix web run build`.
* 2026-05-17 @@Architect: round-6 housekeeping. Every wave-2
  implementation lane is now REVIEW: [systacean-5](./systacean-5.md)
  (server-side PTY session registry + byte-offset replay + idle
  prune + cap + `[terminal]` config), [frontend-4](./frontend-4.md)
  (client-side reattach), [systacean-6](./systacean-6.md)
  (BUG-WT5-A confirmed fallout from systacean-4 + regression test
  `create_event_admits_new_indexable_file_into_bm25` landed),
  [frontend-5](./frontend-5.md) (hash-key strip via
  `persistStateToHash` canonicalisation + tighter test).
  [architect-2](./architect-2.md) docs portion is **DONE**:
  CLAUDE.md + design.md rewritten earlier; README.md spot audit
  this round returned zero matches for `assistant` /
  `agent overlay` / `LlmSession` / `in-app agent`. Docs checklist
  row ticked. Commit groupings remain the open piece in
  architect-2.
  OBS-WT5-B from Webtest A (`--search-aggression` override is
  applied at runtime but `/api/server/config` and `/api/config`
  return the persisted value): @@Architect's call — defer to the
  phase that introduces the Settings UI for this knob, since
  there's no user-visible miss today. Captured as a known
  follow-up in [architect-1](./architect-1.md); proposed fix is
  an `effective_*` field on the response.
  Outstanding work to close the phase: @@Webtest A's PTY-reattach
  + multi-attach + idle-close smoke on a build that has both
  systacean-5 and frontend-4 baked in;
  [architect-3](./architect-3.md) commit-message rewrite (holding
  for Alex's sanity-check); [architect-2](./architect-2.md)
  commit groupings; final summary.md.
* 2026-05-17 @@Architect: round-7 housekeeping. While
  [webtest-1](./webtest-1.md) runs the PTY-reattach smoke:
  drafted [summary.md](./summary.md) (DRAFT; agent rankings +
  final delivery section to be filled at phase close); re-ran
  `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings`
  against the current HEAD, both green. Alex asked the team to
  cut tasks for (a) building Chan.app from this phase's HEAD and
  installing in `/Applications`, and (b) leaving a test server
  up for click-around while he reads the summary. He confirmed
  both fire after the commit lands. Filed
  [systacean-7](./systacean-7.md) for the Chan.app build +
  install (release build via `make build-release` then
  `cd desktop && make build`, copy to `/Applications/Chan.app`,
  clear quarantine, five-check sanity launch) and
  [webtest-3](./webtest-3.md) for the post-commit test service
  rebuild + relaunch on the same `chan-test-phase5` drive. Both
  BLOCKED on [architect-3](./architect-3.md) +
  [architect-2](./architect-2.md) landing the commits. Next
  unblock step: Alex reads the three commit-message drafts in
  [architect-3](./architect-3.md) and gives the green light to
  fire the rebase, then commits group.
* 2026-05-17 @@Architect: round-8 housekeeping. Webtest A's
  round-5 PTY-reattach smoke surfaced two bugs:
  **BUG-WT5-C (HIGH)** — bootstrap layout-restore discards the
  persisted `terminalSessionId` whenever the URL hash is present.
  Result: every plain-browser reload spawns a new PTY and orphans
  the live one even though the server-side `terminal_sessions::
  Registry` is keeping it alive. Root cause is in
  `web/src/state/store.svelte.ts` around line 316: the hash wins
  on layout (right call for shareable URLs, which intentionally
  strip `tsid`/`tseq`), but `applySessionSidecars` doesn't merge
  the session-blob layout's per-tab `tsid`/`tseq` back onto the
  hash-restored tabs. Filed [frontend-6](./frontend-6.md) with
  the diagnosis and a position-match graft as the suggested fix.
  Headline contract for persistent terminal sessions stays
  un-ticked in the request checklist until BUG-WT5-C lands.
  **OBS-WT5-D (MEDIUM)** — two plain-browser tabs on the same
  origin both fall back to `w=default`, so the second tab's
  layout overwrites the first. chan-desktop is fine
  (`w=<window-label>` per window from [backend-2](./backend-2.md)).
  Filed [frontend-7](./frontend-7.md) with a per-tab
  `sessionStorage` key as the proposed fix; chan-desktop and
  tunneled-drive paths unaffected.
  Sequencing impact on the close-out chain:
  [frontend-6](./frontend-6.md) now blocks the commits — we
  should not ship the headline terminal-persistence feature in a
  state where it does not survive a normal browser reload. The
  re-smoke pass that follows the frontend-6 fix will also pick
  up the two-attach + idle-close cases that BUG-WT5-C was
  blocking. [frontend-7](./frontend-7.md) is independent; can
  land in the same wave or get punted to a follow-up phase per
  Alex's call. [systacean-7](./systacean-7.md) (Chan.app build)
  and [webtest-3](./webtest-3.md) (post-commit click-around
  service) stay BLOCKED on commits, which now wait on
  frontend-6.
* 2026-05-17 @@Architect: round-9 housekeeping. Both bug fixes
  landed to REVIEW: [frontend-6](./frontend-6.md) added
  `hydrateTerminalSessionsFromLayout()` in
  `web/src/state/tabs.svelte.ts` and a bootstrap call from
  `store.svelte.ts` so hash-restored terminal tabs pull `tsid`
  / `tseq` from the session-blob layout before TerminalTab
  mounts; new regression test
  `hydrates terminal session ids onto hash-restored terminal tabs`.
  [frontend-7](./frontend-7.md) reworked the session-key
  selection in `web/src/api/client.ts`: chan-desktop URL
  `w=<window-label>` wins, plain browsers generate/reuse a per-
  tab 8-hex `sessionStorage` key, sessionStorage failure falls
  back to `default` with a one-time warning; client tests cover
  both branches. Verified `npm run check`, `npm test`, `npm run
  build` on the merged diff.
  Now blocking the close-out chain: @@Webtest A rebuilds the
  bundle + binary, restarts the service against
  `/private/tmp/chan-test-phase5`, and re-runs the BUG-WT5-C
  repro plus the two-attach + idle-close cases that
  BUG-WT5-C was blocking. Webtest-1's task notes already say
  the lane self-triggers on REVIEW; no separate ping needed.
  Pre-flight on current HEAD (uncommitted wave-1 + wave-2 diff):
  `cargo fmt --check` green, `cargo clippy --all-targets --
  -D warnings` green, `npm --prefix web run check` green,
  `npm --prefix web test -- --run` green (16 files / 144 tests).
  `cargo test --workspace` finished green (exit 0).
  Pre-staged the three terminal-trio commit-message bodies at
  `/tmp/msg-0f4614e.txt`, `/tmp/msg-980fc3e.txt`,
  `/tmp/msg-963bade.txt` so the
  [architect-3](./architect-3.md) rebase can fire the moment
  Alex signs off. Bodies match the drafts in architect-3.md
  verbatim.
* 2026-05-17 @@Architect: round-10 housekeeping. Added an
  explicit re-smoke trigger note to [webtest-1](./webtest-1.md)
  with the six-step rebuild + verify recipe so @@Webtest A has a
  clear handoff while the architect-2 commit plan is finalised.
  Concretised commit groupings in
  [architect-2](./architect-2.md): three thematic mega-commits
  (cleanup / enhancements / terminal + ux) plus a phase-close
  commit for the coordination dir, matching the
  "release: close phase N tasks" precedent in the repo history.
  Per-lane splits were rejected as impractical:
  `web/src/state/store.svelte.ts` alone was touched by six
  frontend lanes, and the hunk surgery to separate them cleanly
  would consume hours and risk per-commit test regressions. The
  three-commit plan keeps each commit internally consistent and
  buildable, with lane attribution living in the body. Captured
  the six untracked entries (terminal_sessions.rs, client.test.ts,
  confirm.svelte.ts, tabs.test.ts, web/src/terminal/, and the
  phase-5 dir itself) so `git add` does not miss anything.
  Round-9 pre-flight recorded in architect-2 as the green
  baseline.
* 2026-05-17 @@Architect: round-11 housekeeping. @@Webtest A
  finished round-6 against the rebuilt bundle (PID 78898).
  **frontend-7 PASS** — per-tab `w=` key working as intended,
  OBS-WT5-D closed.
  **frontend-6 FAIL** — BUG-WT5-C still reproduces. Four
  successive `location.reload()` calls produced four different
  shell PIDs (79138 → 79861 → 80347 → ...). Root cause is a
  bootstrap ordering race: `restoreLayout(fromHash)` synchronously
  mounts TerminalTab, whose `$effect → connect()` fires before
  the bootstrap reaches `await api.getSession()` and
  `hydrateTerminalSessionsFromLayout()`. Network trace also
  caught a second compounding race — the store's auto-save PUTs
  the layout (still tsid-less) before hydration grafts the old
  id. Webtest A wrote three fix-shape options; option (a) — fetch
  session blob first, then `restoreLayout(fromHash, sessionLayout)`
  and graft `tsid`/`tseq` onto descriptors **before** Svelte
  mount — is the smallest cone that closes both races.
  Filed [systacean-8](./systacean-8.md) with the option (a)
  fix shape plus a belt-and-suspenders auto-save gate (option (c)).
  Alex asked this round to route the work to @@Systacean instead
  of @@Frontend for capacity balance; @@Systacean self-claimed
  IN_PROGRESS. Frontend-6 stays REVIEW (insufficient) — the
  graft itself is correct, just lands too late; the new patch
  builds on its `hydrateTerminalSessionsFromLayout` helper.
  Multi-attach + idle-close remain not-yet-validated end to end
  (blocked by BUG-WT5-C); flagged a future webtest-N follow-up to
  drive multi-attach from a raw WebSocket client (`websocat` or
  Node) so the registry contract gets one live exercise pass
  independent of the browser bootstrap race.
  Close-out chain unchanged: commits gated on
  [systacean-8](./systacean-8.md) → re-smoke → architect-3 rebase
  → architect-2 commits → systacean-7 + webtest-3.
* 2026-05-17 @@Architect: round-12 housekeeping.
  [systacean-8](./systacean-8.md) REVIEW. @@Systacean implemented
  both fix shapes from the round-6 diagnosis: option (a) inverts
  the bootstrap order so `restoreLayout(fromHash, sessionLayout)`
  grafts `tsid`/`tseq` onto terminal tab descriptors before
  Svelte mount; option (c) adds a `bootstrapHydrated` rune gating
  `scheduleSessionSave()` and pagehide flushes during the
  bootstrap window so the auto-save can't clobber the blob.
  Two regression tests added — one for hydration ordering and
  one for the save guard. Web gate green.
  Appended a round-7 re-smoke trigger to
  [webtest-1](./webtest-1.md): four-reload PID stability +
  attach-not-create server logs + two-attach + idle-close +
  no-PUT-without-tsid network trace. Webtest A self-triggers on
  REVIEW; the trigger note is for clarity on the bar.
  Close-out chain unchanged: round-7 PASS → architect-3 rebase
  (Alex's commit-msg sign-off) → architect-2 three-commit
  groupings → systacean-7 Chan.app build + webtest-3
  click-around service.
* 2026-05-17 @@Architect: round-13 housekeeping. Alex requested
  one more task: terminal Alt-key word motions (`Alt+<-` /
  `Alt+->` / `Alt+Backspace` / `Alt+Delete`) currently print raw
  escape sequences instead of moving / killing by word. Walked
  through Alex's iTerm2 reference settings; the fix maps cleanly
  to xterm.js's `macOptionIsMeta: true` option (covers
  `Alt+letter` -> `Esc letter` for readline's M-prefix) plus a
  custom `attachCustomKeyEventHandler` for the arrow + backspace
  + delete cases (xterm.js doesn't auto-translate those). Captured
  the architectural advice and fix sketch in
  [frontend-9](./frontend-9.md) — @@Architect took the
  @@Systacean-advise role inline since the protocol is local to
  the browser-side emulator and well-known. @@Frontend
  implements in `web/src/components/TerminalTab.svelte`;
  @@Systacean post-reviews the byte sequences (zsh emacs-mode vs
  vi-mode, fish, etc.) once the diff exists. Out of scope for
  phase 5: configurability surfaces and Cmd-prefix bindings.
  No reshuffle of the close-out chain — frontend-9 is medium
  priority and can ride the same wave as systacean-8 or punt to
  a follow-up if Alex prefers to land the persistence fix first.
* 2026-05-17 @@Frontend: [frontend-9](./frontend-9.md) is REVIEW.
  `TerminalTab.svelte` now sets `macOptionIsMeta: true` and installs a
  custom xterm key handler before websocket connect. The helper in
  `web/src/terminal/keymap.ts` maps `Alt+ArrowLeft/Right`,
  `Alt+Backspace`, and `Alt+Delete` to readline's Meta byte sequences
  and leaves non-target chords to xterm. Added unit coverage in
  `web/src/terminal/keymap.test.ts`. Verified `npm --prefix web run
  check`, `npm --prefix web test -- --run` (17 files / 155 tests), and
  `npm --prefix web run build` (existing Vite warnings only).
* 2026-05-17 @@Architect: round-14 housekeeping. Three new asks
  from Alex after clicking around the live service:
  1. **Env scope**: drop the `CLAUDE_MCP_SERVER_JSON` /
     `CODEX_MCP_SERVER_JSON` / `GEMINI_MCP_SERVER_JSON` aliases
     from the PTY env; keep only the CHAN_-prefixed namespace.
     The CLI-flavoured aliases collide with user wrappers and
     CLI-specific config; chan should own its namespace and let
     each CLI consume CHAN_MCP_SERVER_JSON on its own terms.
     Plus per-tab toggle + info bubble + inject-command button
     so the user can opt out and see what's set. Filed
     [backend-3](./backend-3.md) (drop aliases + add `mcp_env`
     query-param) and [frontend-10](./frontend-10.md) (UI
     toggle + info + button). They coordinate on the query-
     param name before either ships. Note: the request
     checklist row "Embedded terminal sets ENV variables for
     claude / codex / gemini" stays ticked — the spirit
     (external agents auto-discover the chan MCP server) is
     preserved through CHAN_MCP_SERVER_JSON; the literal CLI-
     flavoured aliases are intentionally dropped.
  2. **Indexing progressive disclosure (question)**: answered
     inline. `ProgressStage::{GraphRebuild, IndexFile,
     EmbedBatch}` events already fire over the WS `progress`
     topic. File-tree is instant (Drive::open). chan-report is
     lazy + watch-warmed, not a boot stage. Order today is
     graph -> search (BM25 + embed) per
     [systacean-2](./systacean-2.md). If Alex wants chan-report
     as a first-class boot stage too, that's a small follow-up;
     waiting for the ask.
  3. **Bug: htop reload looks like it needs a reset**. Real
     bug. Full-screen TUIs don't repaint after reattach because
     systacean-5's byte-ring doesn't reconstruct alt-screen
     state. Filed [systacean-9](./systacean-9.md) with two
     layers: MVP is SIGWINCH the PTY child on attach (no-op
     resize on the master) so the TUI redraws from its
     internal model. Layer 2 nice-to-have: sniff
     `\x1b[?1049h/l` to track alt-screen mode and skip ring
     replay when active. Both small; both server-side in
     `crates/chan-server/src/terminal_sessions.rs`.

  [frontend-9](./frontend-9.md) (Alt-key word motions) is
  already REVIEW — @@Frontend shipped while round-14 was being
  drafted.
* 2026-05-17 @@Frontend: [frontend-10](./frontend-10.md) is REVIEW
  for the browser side. Terminal tabs now persist a default-on
  `mcpEnv` preference and a `sessionMcpEnv` sidecar in the per-window
  session blob; shareable hashes stay free of both. The terminal menu
  has the requested "Set MCP env vars" toggle, info popover, and
  "Show MCP env in terminal" action. Fresh opt-out sessions append
  `mcp_env=off`; reattaches omit the param. Added tests for the URL
  helper, descriptor serialization, and command injection. Verified
  `npm --prefix web run check`, `npm --prefix web test -- --run` (18
  files / 158 tests), and `npm --prefix web run build` (existing Vite
  warnings only). Live env behavior remains gated by
  [backend-3](./backend-3.md).

  Close-out chain unchanged in shape but with three more REVIEW
  lanes owed before the architect-2 commits fire. Webtest A
  picks up live verification on the next rebuilt bundle.
* 2026-05-17 @@Backend: [backend-3](./backend-3.md) REVIEW.
  Terminal PTY env construction is now CHAN-only:
  `CHAN_MCP_SERVER_NAME`, `CHAN_MCP_SOCKET`, `CHAN_MCP_COMMAND`,
  `CHAN_MCP_COMMAND_JSON`, and `CHAN_MCP_SERVER_JSON`; the
  `CLAUDE_` / `CODEX_` / `GEMINI_` aliases are gone. Added
  `mcp_env=on|off` parsing on `/api/terminal/ws`; fresh PTY
  creation honors it, while existing-session reattach leaves the
  already-exec'd environment unchanged. Added
  `routes::terminal::tests::mcp_env_off_omits_chan_mcp_vars` and
  tightened the real-PTY env probe to assert no third-party
  aliases leak. Updated README.md, CLAUDE.md, design.md,
  summary.md, architect-2, and systacean-7 to the CHAN-only env
  story. Verification green: `cargo fmt --check`,
  `cargo clippy -p chan-server --all-targets -- -D warnings`,
  `cargo clippy --all-targets -- -D warnings`,
  `cargo test -p chan-server`, `cargo build --no-default-features`,
  `cargo test`, `npm --prefix web run check`,
  `npm --prefix web test -- --run`, and
  `npm --prefix web run build`.
* 2026-05-17 @@Architect: round-15 housekeeping. All three round-14
  asks landed to REVIEW in a single wave:
  [backend-3](./backend-3.md) (CHAN_-only namespace + `mcp_env`
  WS param + docs sweep across README, CLAUDE.md, design.md,
  summary.md, architect-2.md, systacean-7.md;
  `routes::terminal::tests::mcp_env_off_omits_chan_mcp_vars`
  added);
  [frontend-10](./frontend-10.md) (per-tab "Set MCP env vars"
  toggle with default-ON + per-window persistence, info bubble,
  "Show MCP env in terminal" inject-command button,
  `terminalWsPath()` opt-out + URL/serialization/injection tests);
  [systacean-9](./systacean-9.md) (BUG-WT5-E close-out: post-
  replay redraw via PTY winsize re-apply through the controller
  thread + `\x1b[?1049h/l` sniffer that suppresses byte-ring
  replay while alt-screen is active; registry tests for both
  paths).
  Full pre-push gate green on each lane individually; @@Architect
  has not re-run the combined gate but the round-12 baseline
  was green and the three diffs are non-overlapping.
  Appended a round-8 re-smoke trigger to
  [webtest-1](./webtest-1.md) covering the four lanes A still
  has to live-validate ([frontend-9](./frontend-9.md) Alt-keys,
  [backend-3](./backend-3.md) env scope flip from eight vars to
  five + opt-out, [frontend-10](./frontend-10.md) toggle + info +
  inject button, [systacean-9](./systacean-9.md) htop / vim /
  less reload).
  Phase status: 21 of 22 implementation/docs tasks REVIEW. Only
  open lanes are [architect-1](./architect-1.md) and
  [architect-2](./architect-2.md) (mine, IN_PROGRESS),
  [architect-3](./architect-3.md) TODO awaiting Alex's sign-off
  on the commit-msg drafts, [webtest-1](./webtest-1.md)
  IN_PROGRESS awaiting the round-8 re-smoke, and
  [systacean-7](./systacean-7.md) + [webtest-3](./webtest-3.md)
  BLOCKED on commits.
* 2026-05-17 @@Architect: round-16 housekeeping. Webtest A's
  round-9 smoke landed with a corrected finding: **systacean-9
  FAIL**. Alex caught the partial-redraw via side-by-side
  screenshots — htop's dynamic data updates (so SIGWINCH IS
  firing) but the static chrome (CPU labels, header words, F-key
  footer, process-table header, column data) is missing because
  no-op `pty_master.resize(current_size)` doesn't trigger a
  structural redraw, only a cell refresh. backend-3 PASS,
  frontend-10 PASS confirmed in the same round.
  Flipped [systacean-9](./systacean-9.md) to REVIEW (insufficient)
  in the dispatch table — the original brief was delivered, but
  the chosen mechanism (no-op resize) doesn't actually accomplish
  what TUI repaint needs. Filed [systacean-10](./systacean-10.md)
  with the corrective fix: cross-chunk-safe alt-screen sniff
  (rolling tail buffer), replace no-op resize with a one-tick
  winsize wobble (rows-1 -> 50ms sleep -> rows) that forces the
  TUI's structural redraw, optional alt-screen prelude
  (`\x1b[?1049h\x1b[2J\x1b[H`) on the broadcast before the wobble.
  Acceptance criterion is now a side-by-side screenshot diff with
  a fresh-launch htop matching pixel-for-pixel after reload.
  Close-out chain stays blocked on this lane; commits cannot ship
  with the headline contract half-met. Webtest A holds service on
  PID 8248 for the next re-smoke after systacean-10 reaches
  REVIEW.
* 2026-05-17 @@Systacean: [systacean-10](./systacean-10.md)
  REVIEW. Implemented the BUG-WT5-E round-2 corrective path in
  `crates/chan-server/src/terminal_sessions.rs` and
  `crates/chan-server/src/routes/terminal.rs`: alt-screen state
  now uses a rolling tail so `\x1b[?1049h/l` matches across PTY
  read chunks, state transitions emit `alt_screen entered/exited`
  debug logs, alt-screen attaches send
  `\x1b[?1049h\x1b[2J\x1b[H` before redraw, and redraw performs a
  real rows-1 -> 50ms -> original winsize wobble on the
  per-session controller thread. Added
  `alt_screen_sniffer_matches_sequences_across_chunks`,
  `redraw_wobble_pattern_resizes_then_restores`, and
  `redraw_wobble_keeps_single_row_sessions_moving`; kept the
  systacean-9 replay-suppression test green. Verification green:
  `cargo fmt --check`, `cargo test -p chan-server
  terminal_sessions`, `cargo clippy -p chan-server --all-targets
  -- -D warnings`, `cargo build --no-default-features`, repo-wide
  `cargo clippy --all-targets -- -D warnings`, full `cargo test`,
  `npm --prefix web run check`, `npm --prefix web test -- --run`,
  and `npm --prefix web run build`. Webtest A still owns the live
  htop/vim/less screenshot-diff re-smoke on a rebuilt service.
* 2026-05-17 @@Architect: round-17 housekeeping.
  [systacean-10](./systacean-10.md) REVIEW — all three changes
  landed: cross-chunk-safe alt-screen sniff with rolling tail +
  debug logs on enter/exit, alt-screen attach prelude
  (`\x1b[?1049h\x1b[2J\x1b[H`), and the winsize wobble that
  replaces the no-op resize on the controller thread. Registry
  tests for split-escape sniff and resize order added. Local
  pre-push gate green.
  Appended a round-10 re-smoke trigger to
  [webtest-1](./webtest-1.md): the screenshot-diff bar is now
  the canonical acceptance for the htop reload (matches
  fresh-launch pixel-for-pixel), with vim, less, and a plain
  bash scrollback as additional checkpoints. Asked Webtest A to
  capture the `alt_screen entered/exited` debug lines from the
  server log so we have evidence the sniff is firing if the
  screenshot diff still misses.
  Phase status unchanged: 22 of 23 implementation/docs tasks
  REVIEW. Only open lanes are [architect-1](./architect-1.md) /
  [architect-2](./architect-2.md) (mine), [architect-3](./architect-3.md)
  TODO awaiting Alex's commit-msg sign-off, [webtest-1](./webtest-1.md)
  IN_PROGRESS awaiting the round-10 screenshot diff, and
  [systacean-7](./systacean-7.md) + [webtest-3](./webtest-3.md)
  BLOCKED on commits.
* 2026-05-17 @@Architect: round-18 — **phase-5 acceptance surface
  is GREEN**. Webtest A's round-10 smoke reports all PASS:
  htop / vim / less reload screenshot diff matches fresh-launch
  pixel-for-pixel; alt-screen enter/exit debug log fires six
  times across three TUIs (htop, vim, less) on the same session
  id; cross-chunk sniff verified. Ticked the persistent-terminal-
  sessions row in the request checklist.
  Webtest A also filed an open question (not blocking): plain
  bash reload shows empty buffer instead of recent scrollback.
  Pre-existing protocol behavior (`since=last_seq` slicing,
  empty xterm.js buffer after reload). Two possible spec changes
  documented in "Notes & decisions" above for a future phase.
  Close-out chain is now fully unblocked:
  1. Alex signs off on the three terminal-trio commit-message
     drafts in [architect-3](./architect-3.md).
  2. @@Architect fires the architect-3 non-interactive rebase
     (recipe + pre-staged message bodies at
     `/tmp/msg-{0f4614e,980fc3e,963bade}.txt`).
  3. @@Architect runs the four-commit groupings in
     [architect-2](./architect-2.md) (cleanup, enhancements,
     terminal + ux, release-close).
  4. @@Systacean fires [systacean-7](./systacean-7.md) (Chan.app
     build + install in /Applications).
  5. @@Webtest A fires [webtest-3](./webtest-3.md) (post-commit
     test service for click-around).
  6. @@Architect writes the final non-DRAFT
     [summary.md](./summary.md) with agent rankings + delivery
     notes.
* 2026-05-17 @@Architect: round-19 — **wrap fired**. Alex said
  "let's wrap!" after round-10 acceptance reported all PASS.
  Steps run by @@Architect in sequence:
  1. `make build-release` kicked in background (the chan
     release binary at `target/release/chan`).
  2. Stashed the working tree, ran the
     [architect-3](./architect-3.md) non-interactive rebase
     (`git rebase HEAD~3 --exec` with subject-line matching),
     popped the stash clean. The three terminal-trio commits
     now carry rewritten bodies in repo style (new hashes:
     `9c1ea91`, `455c5df`, `02be09c`).
  3. Wrote six commit-message bodies under
     `/tmp/msg-commit{1..6}-*.txt` matching the canonical
     "framed why → surface-bulleted body → Verification
     trailer" pattern.
  4. Staged + committed in order:
     * `c748484 chan-llm: pare to MCP-only surface` (17 files)
     * `58fe80a chan-drive: drop assistant blobs + vcs-aware
       indexing` (14 files)
     * `9e121d5 chan-server: prune agent surface + persistent
       terminal sessions` (21 files, +
       `crates/chan-server/src/terminal_sessions.rs`)
     * `790fd02 web: phase-5 frontend (overlay removal +
       persistent terminals + ux)` (~40 files, including
       deletes for agent components, new
       `web/src/terminal/{keymap, mcpEnv, session}{,.test}.ts`,
       and the new test files)
     * `9ecb27d docs: refresh phase-5 boundary` (3 files:
       CLAUDE.md, README.md, design.md)
     * `7da49f6 release: close phase 5 tasks` (22 files in
       `phase-5/`)
  5. Pre-push gate on the final HEAD: `cargo fmt --check`,
     `cargo clippy --all-targets -- -D warnings`,
     `cargo build --no-default-features`,
     `cargo test --workspace`, `npm run check`, `npm test`,
     `npm run build` — **7-for-7 green**.
  6. Kicked `cd desktop && make build` in background to
     produce `Chan.app` for the post-commit install via
     [systacean-7](./systacean-7.md). Still running at this
     log entry.
  7. Finalised [summary.md](./summary.md): flipped status from
     DRAFT to FINAL, filled in agent rankings with feedback,
     wrote the final-delivery section with the nine-commit
     summary table.
  Pending the last two actions:
  * `Chan.app` install at `/Applications/Chan.app` after the
    Tauri build completes (systacean-7).
  * Push `main` to `origin/main` — holding for Alex's explicit
    go since it's a shared-state action.
  Working tree clean. Local main at +9 commits over origin.
* 2026-05-17 @@Systacean: [systacean-7](./systacean-7.md)
  REVIEW. The desktop build did not leave an app bundle in
  `desktop/src-tauri/target/...`; reran `make build` from
  `desktop/`, which rebuilt the web bundle, release `chan`,
  staged `desktop/src-tauri/binaries/chan-aarch64-apple-darwin`,
  and produced
  `target/release/bundle/macos/Chan.app` plus
  `target/release/bundle/dmg/Chan_0.8.1_aarch64.dmg`. Installed
  the app at `/Applications/Chan.app`, cleared quarantine, and
  ad-hoc re-signed because the generated Developer ID signature
  failed `codesign --verify --deep --strict` locally. Installed
  verification: `CFBundleShortVersionString=0.8.1`,
  `CFBundleVersion=0.8.1`,
  `/Applications/Chan.app/Contents/MacOS/chan --version` prints
  `chan 0.8.1`, `codesign --verify --deep --strict --verbose=2
  /Applications/Chan.app` PASS, `com.apple.quarantine` absent,
  `open -a /Applications/Chan.app` launched PID 28637, and
  Accessibility reports window `Chan Desktop`. Alex still owns
  the hands-on GUI click-through inside the app.
* 2026-05-17 @@Architect: round-20 — phase release. Alex
  requested a cargo version bump + git tag + notarized DMG
  before the final push. Bumped workspace + `tauri.conf.json`
  from `0.8.1` to `0.9.0` (`97ad644 chore: bump version to
  0.9.0`); rebuilt the release `chan` binary at 0.9.0; ran
  `cd desktop && make app-notarized` end to end (after one
  retry against an updated `chan-notary` keychain password
  Alex refreshed). Apple notary returned `Accepted` for the
  `.app` and separately for the `.dmg`; both stapled and
  validated. Installed `/Applications/Chan.app` at 0.9.0
  (Gatekeeper `accepted, source=Notarized Developer ID`).
  Annotated tag `v0.9.0` created and pushed; `git push
  --follow-tags origin main` succeeded, taking origin from
  `06017f4` to `97ad644` (+12 commits) and publishing the
  v0.9.0 tag plus a handful of stale local tags from prior
  releases. The DMG at
  `target/release/bundle/dmg/Chan_0.9.0_aarch64.dmg` is the
  canonical distribution artifact.

  Note: the round-19 entry above + the original
  [systacean-7](./systacean-7.md) progress/completion notes
  describe an interim 0.8.1 ad-hoc re-signed build that was
  superseded by this release. systacean-7 carries a
  "Superseded by 0.9.0 release" addendum recording the actual
  shipped state.
