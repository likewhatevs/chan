# Phase 26 - Windows-first chan-desktop: the Git BASH terminal, named-pipe control socket, and markdown iframe embeds

Status: released (round 1; **shipped as v0.36.0** — Alex lifted the original land+CI-green-only scope and elected to ship). `windows-latest` CI green (run 27548007231: build + NSIS + headless `/api/health` smoke), `make pre-push` + `cargo xwin` windows-msvc green at `7b2bfbdf`, merged to `origin/main`, and the v0.36.0 publish run (27561543753) was green end-to-end: signed+notarized macOS desktop, GitHub Release, chan.app `/dl` metadata. The Windows *runtime* is still **empirically unverified** by the team — Alex's real-hardware smoke is best-effort, post-release (see Verification).
Span: 2026-06-15.
Tags: #desktop #windows #terminal #git-bash #ipc #packaging #ci #shortcuts #markdown #embed #team

Round 1 of the `new-team-4` four-member team (Lead lead; LaneA, LaneB, LaneC), another full round on the `cs terminal team` tooling: generated bootstrap, append-only task files and journals, one-line pokes, an isolated-worktree gate, and `cs terminal survey` for the one host decision. The defining constraint shaped the whole round: **nobody on the team — and not Alex's Mac in a clean way — can run a Windows GUI.** `cargo-xwin` gives fast local *compile* proof; GitHub Actions `windows-latest` (build + NSIS bundle + headless `/api/health` smoke) is the authoritative build proof; the interactive runtime is deferred to Alex's hardware. Every Windows task therefore ended with the same empirical-deferral line, and the round was scoped to **land + CI-green only** — no public Windows release.

## Roadmap (the asks)

`dev/phase-26/plan.md` is the design of record (work breakdown steps 1-8).

1. **Bring Windows support back to chan-desktop**, leaning on [Git for Windows](https://gitforwindows.org/) as a **hard dependency** so the desktop terminal is wired to **Git BASH** (a real POSIX-ish login shell), not `cmd`/PowerShell. The real blockers were Unix-only surfaces (`#[cfg(not(unix))]` stubs that *error*), not the GUI — Tauri/WebView2 already supports it. Windows previously reached only a compile-green CI baseline (phase-8) and was never shipped; the architecture has since changed (embedded chan-server, the first-party control socket, the `cs` poke-bus).
2. **Markdown iframe embeds** - `![](youtube|google-maps …)` URLs do not render as iframes today; add it.
3. **Launcher scroll/dropdown bug** - the registry window's sticky header scrolled away and last-row dropdowns rendered below the fold.

Decisions ratified with Alex before the round (locked, not re-litigated):
- **Shortcuts: Ctrl-based, Linux-like** - the clean app-vs-shell modifier separation expressed with `Ctrl` / `Ctrl+Shift` for shell-colliding chords. *Win-key-as-Cmd ruled out* (Windows reserves most `Win+<key>` chords globally).
- **Release scope: land + CI-green only.** `windows-latest` builds + bundles an NSIS installer + runs a headless smoke; the artifact is downloadable for Alex. **No Authenticode signing, no Windows updater feed, no public release this phase.**
- **Git for Windows: runtime-detect + a friendly in-app gate** (the NSIS installer also checks); the ~60 MB installer is **not** bundled.

## What shipped (14 commits; CI-validated on the `phase-26` branch, then merged to `origin/main` on green)

**Windows core (LaneA, the critical path):**
- **`f1b3da23`** - cross-platform control-socket **transport seam**. A thin `#[cfg]`-split module shared by the server (`control_socket.rs`) and client (`chan-shell/control.rs`): unix `tokio::net::Unix{Listener,Stream}` (today's code moved behind the seam) vs windows `tokio::net::windows::named_pipe` (`ServerOptions`/`ClientOptions`). No new dependency (`tokio` is already `features=["full"]`). The JSON line-framed `ControlRequest`/`ControlResponse` wire contract is **unchanged**; the "socket path" becomes a pipe name `\\.\pipe\chan-control-<pid>-<rand>` carried verbatim through `$CHAN_CONTROL_SOCKET`. The server holds the next idle pipe instance and `mem::replace`s a fresh one each accept (the canonical tokio create-next-before-spawn loop, avoiding the client `NotFound` race); the client retries `ERROR_PIPE_BUSY`/`NotFound` under a 5 s deadline so an absent server fails fast like unix `ENOENT`. Grounded in tokio 1.52.2 vendored source (named-pipe `poll_flush`/`poll_shutdown` are no-ops; `NamedPipeServer` Drop closes the handle so a written response drains before broken-pipe).
- **`ec660e22`** - **Git BASH login-shell terminal** + the missing-Git **structured signal**. `command_builder`'s windows arm spawns Git BASH as a login shell (`bash -l`, `-lc` for one-shots), replacing `cmd /C` and the `cmd.exe` default; `Session::spawn` prepends Git's `usr/bin` + `mingw64/bin` to `PATH` so `git`/coreutils/`cs` resolve. Discovery (`resolve_git_bash`, `OnceLock`-cached): `git --exec-path` (avoids the WSL `bash.exe` trap) → well-known Program Files / per-user dirs → registry `HKLM\…\GitForWindows\InstallPath` → filtered `where bash`. When Git BASH is absent, every spawn path returns `CreateError::GitBashMissing`, surfaced as **HTTP 424 Failed Dependency** (restart) and a **WS `ServerFrame::Error { reason: "git_bash_missing" }`** (attach-or-create), with the exact strings pinned by `git_bash_missing_contract_is_stable`.
- **Step 3 (dead `#[cfg(unix)]` relics in `main.rs`): verified a no-op.** The named relics (`reclaim_workspace`, `find_orphan_chan_serve_pids`) no longer exist; `open_workspace_from_handoff` is *live* (`#[cfg(unix)]`, `main.rs:986` → called `:1746` inside the unix handoff block) and correctly needs no Windows arm. Nothing to delete — confirmed against HEAD rather than deleted blindly.

**Desktop shell + packaging/CI (LaneB):**
- **`d511b27a`** - `tauri.conf.json` `bundle.windows` **wix→nsis** (lighter, no WiX toolset, `installMode currentUser`).
- **`44dfa812`** - `cs_install.rs` Windows **`.cmd` shim** (ARGV0 dispatch, install-root guard, best-effort `reg.exe` PATH append; marker-guarded/idempotent; pure helpers unit-tested).
- **`ee0aa1fc`** - `desktop/Makefile` **`cargo-xwin`** compile-check target; **`da6dbc73`** added the **LLVM** requirement (`clang-cl`/`lld-link`/`llvm-lib` for the `ring` C dep — auto-prepends Homebrew's keg `llvm` on macOS).
- **`4a10196f`** - `release-desktop.yml` **`windows-latest`** dry-run arm: web build + `cargo tauri build --bundles nsis` + headless `chan serve` → `/api/health` smoke + artifact upload.
- **`c4147f19`** - `linux_gui_stack.rs` Windows **no-op** contract (doc); **`6e007109`** - `docs/contributing/windows-and-linux.md`, the **WSL2 + sdme** Windows dev-loop design (design only, marked unvalidated).
- **`d360ab9c`** - `chan-workspace/src/fd_budget.rs` `#[cfg(unix)]`-gates the nofile/rlimit machinery (`EFFECTIVE_NOFILE_CEILING`, `effective_nofile_limit`) dead on Windows — the **core-xwin-green** unblock (see retrospective).
- **`main.rs`/`serve.rs`: zero Windows arms needed**, *proven* by `cargo xwin check -p chan-desktop` exit 0 (the whole desktop crate compiles for Windows as-is; the unix bits are already cfg-gated). No gratuitous arms added.

**Frontend (LaneC):**
- **`ff787251`** - launcher **single-scroll flex shell + flip-up last-row menu** (`desktop/src/{styles.css,main.js}`): one `main` scroll container (header non-scrolling, `th` sticks to `top:0`, magic `53px` offset deleted); `.split-menu` measures fit and toggles `open-up` for bottom rows.
- **`4fbd2cde`** - **YouTube/Maps iframe embeds** for `![](url)` (`web/src/api/{embed,markdown}.ts` + the editor `image.ts` widget): youtube/youtu.be → `youtube-nocookie.com/embed/<id>`, Maps → `…/maps/embed`, reusing the `#w=`/align hints; `iframe` allowed in DOMPurify behind a **tight host allowlist + `sandbox`**, with `frame-src` added to the Tauri WebView CSP.
- **`dabb35be`** - **Windows/Linux Rich Prompt chord** (no Win key): mirrors the Dashboard split — mac `Cmd+Shift+P`, native off-mac `Ctrl+Shift+P`, web `Alt+Shift+P`; native Windows now renders **zero `Cmd` labels**.
- **`7b2bfbdf`** - **missing-Git friendly in-app gate** (`TerminalTab.svelte`): consumes both halves of LaneA's contract (WS `reason:"git_bash_missing"` + HTTP 424), renders a gate card with an **Install Git for Windows** link instead of a raw error, clears on reconnect.

## Verification

- **Scoped own-gates per lane** (clippy/test/fmt under `RUSTFLAGS=-D warnings` + `make web-check`), re-run after last edits; the Rust lanes additionally ran `cargo xwin check --target x86_64-pc-windows-msvc` for their crates.
- **Lead isolated full-tree gate** (separate worktree, gates the committed state, immune to peers' WIP): `make pre-push` green at `7b2bfbdf` — fmt + clippy + `test --all-targets` (`-D warnings`, 27 ok / 0 failed) + `--no-default-features` build + the separate **gateway** workspace + `web-check` (**1810 vitest**, svelte-check 0/0, build OK) + marketing checks — **plus** an independent `cargo xwin check` of all three core crates (`chan-server`, `chan-shell`, `chan-workspace`) for `x86_64-pc-windows-msvc` under `-D warnings`, exit 0, zero warnings (LLVM on PATH).
- **Frontend browser-smokes (Chrome):** the iframe embeds *actually loaded* from the allowlisted hosts in a real `chan serve` (a non-allowlisted host stayed the normal `<img>` path) — allowlist enforcement, not just render; the launcher header holds with no gap on scroll and the last-row dropdown flips up fully visible; the missing-Git gate card + Install button render over the pane.
- **`windows-latest` CI: GREEN** (run 27548007231) — all three OS jobs (ubuntu-latest, macos-latest, windows-latest) succeeded; the Windows **build + NSIS bundle + headless `chan serve` → `/api/health` smoke** all passed on a real `windows-latest` runner. The authoritative Windows build proof; the embedded server boots and answers `/api/health` over the new named-pipe-backed stack.
- **Empirically UNVERIFIED by the team (the round's deliberate gap):** the Windows *runtime* — the named-pipe `cs` round-trip, Git BASH actually spawning with the right login-shell/PATH init, and the missing-Git → install → retry path end-to-end. No Windows GUI is runnable by the team (Wine doesn't run WebView2; a local VM is against the keep-it-clean constraint). Flagged for Alex's smoke on the CI NSIS artifact, mirroring the pre-release-merge-unverified discipline.

## Deferred / deviations

- **Server-side CSP.** The design (step 6) said add `frame-src` to "the server CSP header", but `static_assets.rs` sets **no CSP today** — the embed feature is fully satisfied by the Tauri WebView CSP. A net-new server-wide CSP (WSS/blob:/asset:/the SPA's own inline needs) is a riskier syseng change needing its own scope + a served-SPA browser test, so it was **deferred** and flagged to Alex (he can pull it back in).
- **Authenticode signing + Windows updater feed:** deferred by the locked release scope.
- **WSL2 dev-loop:** design only (`windows-and-linux.md`), unvalidated — the team has no Windows host.

## Retrospective

**Highlights:**
- The critical path executed exactly as planned: LaneA's transport seam (step 1) was the load-bearing unblock; it landed `cargo-xwin`-green *first*, which is precisely what let LaneB's CI smoke be expected to pass. The compile-window rule held — the seam plus both call sites (server listener, client connect) landed in one burst, re-checked green before pausing.
- **Evidence over speculation.** LaneB refused to add gratuitous Windows `cfg` arms to `main.rs`/`serve.rs`, instead *proving* none were needed (`cargo xwin check -p chan-desktop` exit 0); LaneA verified the step-3 "dead relics" didn't exist rather than deleting on the strength of a stale plan. Both avoided plausible-but-wrong changes.
- **Tokio-source grounding** of the named-pipe lifecycle (the accept loop's create-before-spawn, the drain-on-drop semantics) meant the one genuinely novel transport surface was reasoned from the vendored source, not hand-waved.
- **Frontend verification depth:** the embeds were smoked against a real server with the host allowlist actually exercised (allowlisted hosts load, others drop), not a render-only check.

**Lowlights / lessons:**
- **The Lead mis-scoped fd_budget's priority.** It was first routed as "low-priority, after step 2/3" on the basis that CI (no `-D warnings`) wouldn't break. That missed two things LaneB surfaced: under `-D warnings` (the gate posture) the dead code is a *hard error*, and the `xwin-check` **stops at chan-workspace** (chan-server/chan-shell are downstream), so it gated the entire core-xwin-green milestone. Re-prioritised and re-routed once the real severity was clear. Lesson: reason about the *gate's* flags, not just CI's.
- **A cross-agent severity disagreement that was really a flags difference.** LaneA reported fd_budget as "harmless warnings" (plain `cargo xwin check`); LaneB reported a "blocker" (`make xwin-check` = `-D warnings`). Both correct in-context — reconciled by checking what each command actually ran, not by trusting either label.
- **Two benign poke crossings** (the fd_budget reassignment to LaneB; the gate-UI nudge to LaneC): in both, a worker's completion poke crossed the Lead's re-poke in flight, reconciled cleanly from the on-disk task files. The lean-poke-bus discipline (context lives in task files, pokes are pointers) absorbed it exactly as designed.
- **A design assumption was wrong:** step 6's "server CSP header" doesn't exist. Caught by the implementing lane (LaneC) at implementation, not in review.

**Honest feedback, per member:**
- **LaneA:** the cleanest critical-path output. The transport seam was grounded in vendored tokio source, landed green on the first gate, and the missing-Git contract was test-pinned *for its downstream consumer* before that consumer existed. Flagged fd_budget (not their file) rather than absorbing it silently. No misses.
- **LaneB:** the broadest surface (8 commits across packaging, CI, the shell shim, and the core-xwin fix) and the round's best single judgment call — proving zero Windows arms were needed instead of adding them. Also correctly diagnosed the fd_budget blocker the Lead under-scoped, and built the `cargo-xwin` + LLVM harness the whole team's local Windows proof rides on.
- **LaneC:** the strongest verification story — every piece browser-smoked, the embed allowlist tested with a real server, and the Rich Prompt chord call mirrored an existing pattern (no Win key, zero Cmd labels on Windows) rather than inventing one. Surfaced the server-CSP design hole.
- **Lead:** the fd_budget priority miss is the lead lowlight (under-scoped a gate-blocker on a CI-only reading). What held the round together: the isolated full-tree gate plus an *independent* `cargo xwin` re-confirm (not relaying the lanes' claim), and catching two staleness gaps (re-poking LaneB/LaneC when their completion notes predated reassignments) so no lane idled thinking it was done. Holding the single Alex survey until the gate was actually green was the right call.
- **Alex:** the locked decisions up front (Ctrl shortcuts, no Win key, land+CI-green, runtime-detect gate, Git-for-Windows not bundled) removed all re-litigation and let the lanes start at t=0. The "branch → CI → merge on green" choice fit the no-local-Windows reality — validate on the branch before main.

**Carryover:**
- **Windows RUNTIME smoke on Alex's real hardware** via the CI NSIS artifact: terminal opens Git BASH; `cs` round-trips over the named pipe; the missing-Git gate fires when Git for Windows is absent and clears after install; shortcut feel. The round's empirical gap.
- **Server-side CSP** (deferred) — a scoped syseng task if Alex wants server-side embed hardening.
- **Authenticode signing + a Windows updater feed** — required before any *public* Windows release.
- **WSL2 dev-loop validation** — needs a Windows host to exercise the `windows-and-linux.md` design.

## Notes

- The round's coordination bus (`new-team-4/`: plan, tasks, journals) is the live dispatch during the round; per the convention it is committed alongside this report when the round fully closes (after the CI result and Alex's smoke land).
- Versions: **released as v0.36.0** (after v0.35.0). The round was scoped land+CI-green-only, but
  Alex elected to ship on his explicit go ("merge onto main and release", best-effort Windows
  smoke later); the version bump + tag + publish followed the standard release procedure (dry-run
  publish=false green first → tag → publish run 27561543753). **No Authenticode / Windows updater
  feed this cycle** (locked), so the Windows desktop installer is the CI dry-run artifact (run
  27548007231), not a published signed installer — the Windows *code* ships in the v0.36.0 binary.
  The one open item is Alex's real-hardware Windows runtime smoke (named-pipe `cs` round-trip +
  Git BASH spawn + the missing-Git gate); he re-reports if it breaks (pre-release-merge-unverified
  discipline).
