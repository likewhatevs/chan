# Phase 30 - making the devserver window lifecycle actually work: reattach, discard, and the FD leak

Status: code complete, gated full-tree green; **not yet landed/released** (awaiting Alex's land
decision for the `v0.40.0` branch). `make pre-push` green over the committed state (the authoritative
isolated full-tree gate at each checkpoint) and `cargo xwin` (`x86_64-pc-windows-msvc`, `-D warnings`)
green for chan-server/chan/chan-workspace/chan-desktop. The reliability core is **double-proven** -
headless against a `chan serve` oracle AND against a real lima devserver (the per-tenant
`session_dir` path = Alex's exact repro). Desktop-only WKWebView/native checks (the control-terminal
dialog, the launcher empty-state visual, the menu-reopen) are pending Alex's hand-smoke
(`dev/v0.40.0/team/followups/alex-desktop-residue.md`).
Span: 2026-06-19.
Tags: #devserver #terminal #reattach #fd-leak #window-lifecycle #discard-vs-bury #control-terminal #cli #ps #seam-first #windows #mcp #graph

The phase-29 `chan devserver` + window-persistence work (the v0.39.0 W10 launcher persistence) shipped
the right *intent* but was broken end to end: on a chan-desktop or browser reconnect the standalone
terminals **restarted** instead of resuming, closed windows never cleaned up out of `cs window list`,
and a long-running devserver climbed to **EMFILE**. This round made that design actually work, proven
by lima E2E. The single highest-value finding held: **reattach-not-respawn, discard-vs-bury, and the
FD leak are one root cause** - terminal sessions now live exactly as long as their window is
*persisted* (an explicit intent, not inferred). On top of the lifecycle core: **the devserver now
serves the host library** (decision D1 - fixing "comes up empty / stuck on Loading"), the
**control-terminal dialog** fires on a connected-phase exit, a **`chan workspace <…>`** CLI reorg, the
new `chan ps`, menu-reopen of closed devserver windows, and the deferred Windows/graph items. Ran as a
five-member team (Lead + Desktop / Devserver / @@webdev / Core), **seam-first**: three wire
contracts frozen up front, eight amendments ratified from the lanes' real implementations, all against
one `contracts.md`.

## Roadmap (the asks)

`dev/v0.40.0/plan.md` is the design of record (twelve workstreams L1-L12 + decisions D1/D2/D3):

- **L1 - Reconnect re-attaches, never respawns.** Live PTYs resume (same processes, scrollback) on a
  client disconnect→reconnect; parity with the known-good `chan serve` reload path.
- **L2 - Window discard-vs-bury is an explicit intent.** ^W / ^D / Ctrl+Shift+W / empty → discard
  (gone from `cs window list`); only buried windows save+hide.
- **L5 - FD / EMFILE leak.** Same root cause as L1 (busy detached sessions never pruned).
- **L3 - Control-terminal dialog.** Fires on a connected-phase exit/^C; Cmd+W while running; the OS
  close button buries when connected else discards (Ctrl+Shift+W in the shortcut store).
- **L4 - Devserver serves the host library (D1).** The host library is the single source of truth; the
  CLI routes through a control socket when one is present, else acts directly; all lock-safe.
- **L6 - Cross-window terminal drag-and-drop rules** (same-kind only).
- **L7 - CLI reorg + tagline** - a `chan workspace <…>` group; refresh the line-1 tagline.
- **L8 - fs-graph paged resume parent-less edges.**
- **L9 - MCP-on-Windows** (build-only per D2).
- **L10 - Menu-reopen of closed devserver windows** (high priority).
- **L11 - Windows leaked-handle lock-steal** (build-only per D2).
- **L12 - Phase docs** (this writeup + backfilling phases 28/29).

## What shipped

**Persistence & window lifecycle - the core (Seam A; L1+L2+L5 are one root cause):**
- **`7a4a4e6f`** - persistence-driven terminal session lifetime: sessions key their orphan timer off a
  detach timestamp (not output activity, which made busy detached sessions immortal); the pruner keeps
  a persisted window's sessions indefinitely and reaps a discarded window's; reattach-by-id rebinds the
  session's window on attach (Amendment 3A, the cross-window-move guard). **Kills the FD leak.**
- **`346c4cd4`** - SPA: persist reattachable terminal windows (was: delete them, the "shells restart"
  root cause) so a fresh page load reattaches by session id; an explicit synchronous discard DELETE
  replaces the `pagehide`-dependent flush.
- **`2a078374` / `498b27f9`** - a cross-window terminal move-out must not reap the moved session: the
  SPA sends `&moved=1` (deterministic at close-tab) and the server honors it (delete the blob, skip the
  reap), complemented by the server-side window_id rebind.

**Control-terminal dialog (Seam C; L3):**
- **`465e4dcf`** - a Rust control-PTY exit watcher fires the dialog on a connected-phase exit/^C
  (ACL-independent, robust to a buried/throttled WKWebView); the `control-terminal-*` capability grant;
  and the launcher empty-state on connect-success (L4 "Loading→(no workspaces)").
- **`d5467b50` / `06a846c8`** - Cmd+W in a connected control window drives the dialog (web/Linux/Windows
  keymap; and the macOS native-menu path: route to the survey, not bury).

**Devserver serves the host library (Seam B / D1; L4):**
- **`3928b66b`** - the devserver reads the host library live and lists every workspace (on/off),
  fixing "comes up empty / stuck on Loading"; Forget is destructive (= `chan workspace rm`, bins
  trash) under the single-registry model.

**CLI reorg + tagline (L7) and `chan ps`:**
- **`51c34f53` / `f250b128`** - group registry + content ops under `chan workspace <add|ls|rm|index|…>`
  (no back-compat aliases, pre-release); finalize the tagline ("an AI-native workspace for your
  Markdown notes and projects"). **`ddb6809e`** points stale `chan add` references at `chan workspace add`.
- **`d5488c68` / `8644b920` / `76a759ab`** - `chan ps` (serving state of registered workspaces);
  `ControlRequest::Identify` classifies the holder kind (standalone / desktop / devserver).

**Menu-reopen of closed devserver windows (L10):**
- **`eb00493b` / `661360dd`** - `GET /api/devserver/windows` (a persisted-window aggregate across
  tenants) + the desktop menu poll, filter (`saved && !connected`), and re-minted-token reopen.

**Backend + Windows (L8 / L11 / L9):**
- **`a9b58e5b`** - fs-graph paged-resume pages re-emit the ancestor chain (no parent-less `contains`
  edges).
- **`8e0cdc4f`** - steal a leaked Windows lock handle from a provably-dead holder (`FILE_SHARE_DELETE`
  + `windows-sys`; conservative steal-only-a-dead-holder rule).
- **`30adfa8b` / `ec25c131` / `7a3c1f09`** - MCP-on-Windows: extract the bridge over the cross-platform
  control-socket transport and un-cfg the proxy across chan-server / chan / chan-desktop (build-only).
- **`e9a21a08`** - rustfmt follow-up (the isolated full-tree gate caught a stale signature diff).

## Verification

- **Seam-first discipline.** Three wire contracts (Seam A persistence/reattach, Seam B devserver↔library,
  Seam C control dialog) frozen in `dev/v0.40.0/team/contracts.md` at t=0 after verifying every anchor
  against HEAD; **eight amendments** ratified from the lanes' real implementations - none reopened a
  frozen shape.
- **Reliability core, double-proven** (the harnesses under `dev/v0.40.0/team/scratch/`): Devserver's
  dual-platform FD soak (reattach holds the fd count flat, no respawn; discard reaps; the broken
  respawn path reproduces Alex's unbounded climb), @@webdev's headless continuation + headline-acceptance
  smokes (multi-terminal close→reopen resumes live PTYs + scrollback + identical layout; ^W/^D/empty
  vanish from `cs window list`; an adversarial no-session control proves the test discriminates), run
  on both a `chan serve` oracle AND the lima devserver's per-tenant `session_dir` path.
- **CLI E2E** (`scratch/l4-cli-e2e.sh`): `chan workspace ls` identical direct vs devserver-routed;
  `chan workspace rm` routes `Unserve` and unmounts only that tenant; `chan ps` classifies the kind.
- **Isolated full-tree gate** (Lead's separate worktree, immune to in-flight WIP): `make pre-push`
  (fmt, clippy `-D warnings`, all workspace tests, no-default + gateway builds, web vitest+svelte,
  marketing) + `cargo xwin`, green on the final tip.

## Deferred / follow-ups

- **Desktop-only hand-smoke** (WKWebView/native, not Chrome-automatable) handed to Alex:
  `dev/v0.40.0/team/followups/alex-desktop-residue.md` - the control-terminal dialog, the launcher
  empty-state visual, and the L10 menu-reopen.
- **Process-group kill on session close** - the conservative SIGKILL-the-direct-child teardown leaves a
  setsid'd / SIGHUP-ignoring grandchild edge case; the headline leak is fixed regardless
  (`dev/v0.40.0/team/followups/deferred-followups.md`).
- **`~/.chan/devserver/config.json` demotion** to serving-state-only (the devserver now reads the host
  library live, so its config is no longer a registry), and a possible `/api/devserver/terminals` ↔
  `/api/devserver/windows` consolidation.
- **The v0.40.0 release cut** (version pins + CHANGELOG + tag) is deferred to a follow-up round.
