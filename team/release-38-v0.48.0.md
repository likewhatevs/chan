# Phase 38 — v0.48.0: devserver / launcher window lifecycle, identity & presentation

Round 2026-06-24 (version bump + cut pending a final Alex-requested experiment). Team:
Lead (architect/gate), Workspace (`crates/`), Desktop (`desktop/src-tauri/`),
@@webdev (`web/`), Launcher (`web-launcher/`) — disjoint trees, single cut on Alex's go.
27 commits, 41 files (+1845 / −258).

## Theme

The second pass on the devserver / chan-launcher connect surface Alex hand-smoked on a
live devserver, plus two pre-existing bugs (rich-prompt image paste, per-library pane
colour) and the same-basename-workspace collision. Opened as eight clusters (C1–C8); grew,
via four mid-round Alex asks, into a presentation + isolation round (the control-terminal
`running:` banner, the 🌐 devserver icon, never-hardcode-a-shell, `CHAN_HOME`). The C8 pane
colour took most of the round and is the round's lesson (below).

## What landed (by cluster / ask)

- **C1 — rich-prompt image paste** (`b5700684`): images deliver to the agent as
  workspace-rooted paths (not draft-file-relative), so the agent resolves them at `$CWD`.
- **C2 — control lifecycle + ACL** (`8df123ec` script-exit-disconnect always for
  script-based devservers; `154cf588` a dedicated **remote-scoped** `launcher-events.json`
  granting `core:event` — the literal "add to default.json" wouldn't have reached the
  loopback-served launcher). `2db62625` pins scrape_token against W5's banner.
- **C3 — keyed-pathspec mount + EADDRINUSE** (`79c4a9d6` prefix `/{slug}-{8hex}`,
  single-sourced `canonical_root_hash8`; `d6a4a124` actionable `:8787` hint; `04a5e789` L1
  launcher `{#each}` dedupe). Same-basename workspaces coexist; the launcher no longer
  crashes. *(C3 EADDRINUSE was v0.47 carryover.)*
- **C4 — terminal-window reap** (`c665cf80`): a standalone terminal row leaves the feed on
  PTY-exit-while-detached, respecting the no-natural-exit guard. *(v0.47 carryover.)*
- **C5 — control grouping** (`7bae45dc` seed library_id before the control mint;
  L2 never-blank header).
- **C6 — stale-window eye** (`a114c980` missing window → `Ok(())`/204 silent no-op, no ACL
  needed — pure HTTP→bridge, not a webview IPC; W4 route already 204/409; L3 catch;
  `0982cad6` pins the 204).
- **C7 — launcher row align** (in `04a5e789` / `18dc5ab5`).
- **C8 — per-library pane focus colour** — see the lesson below. Final fix = `96b97549`
  (W9, root route accepts a per-tenant surface token) + `1168f65c` (S5, SPA calls local-color
  on the ROOT path, not the tenant-prefixed one) + `2474fe44` (S4, watch→menu sync) on top of
  `bc72e287`/`ec81ef29` (D5/S2).
- **W5 — control-terminal banner** (`a1fba0de`): `running: {command}\r\n`, scoped to the
  command-carrying tenant (not naive `command.is_some()`, which would have bannered every
  team-agent terminal).
- **W6/D6 — shell resolver** (`582026d6`/`dc672b9f`): one `chan_workspace::user_shell()`
  (`$SHELL`→passwd→`/bin/sh`, reusing portable_pty); kills the `/bin/sh`/`/bin/zsh` hardcodes.
- **D7/L5 — 🌐 icon** (`4b24e5b5`/`18dc5ab5`): `ICON_OUTBOUND` 📤→🌐 + launcher glyph ↗→🌐.
- **S3 — terminal-blank revert** (`2555e447`): reverted the reattach reply-gating
  (`36fcbab5`+`9b44cef2`) that could stall and drop live CPR/DA replies → blank terminal
  under claude code. Alex chose the occasional leak over the breakage.
- **W7/D8, W8/D9, D10 — `CHAN_HOME`** (`681a2e4e`/`d3c8bae0`, `dcdfbeaf`/`d8be3c70`,
  `7d53c7b9`): a single `chan_home_override()` authority in `config_dir()`; every `.chan`
  store and the `.local/bin` shims honour it; the boot log names the real dir.

## Highlights

- **Correct deviations over literal instructions.** D2 (remote-scoped capability, not
  `default.json`), D4 (no ACL — the eye path is HTTP, not a webview IPC), and W5's
  tenant-default-command scoping all rejected the plan's literal wording for the right
  root-cause fix. Static plans get the layer wrong; the lanes caught it.
- **Single-source discipline.** `canonical_root_hash8` (C3), `user_shell()` (W6),
  `chan_home_override()` (W7/W8), `require_surface_bearer`/`any_tenant_token` (W9) each
  collapsed a scattered concern to one authority.
- **Lane hygiene held all round.** Atomic pathspec commits in the shared tree; the
  isolated gate; correct cfg(unix)/xwin handling; `cs window list --json` as the
  empirical token source for the final verification.

## Lowlights + honest feedback

- **Lead (me) — the C8 two-day miss, owned.** C8 was diagnosed **by static trace at the
  wrong layer, three times, without anyone reproducing it**: S2 declared the web side fine
  (test-only); D5 fixed only the *devserver* seed on the explicit assumption that local "needs
  no analog… always fresh"; W9 fixed the launcher-bearer *gate*. Every one was plausible on
  paper and every one shipped without a single endpoint hit — so Alex rebuilt and re-tested
  across long round-trips for two days while the colour simply never persisted. The actual bug
  was one layer none of the traces checked: a window loads under its tenant prefix, so
  `apiPath()` prepended that prefix to the local-color call → **404**, before the gate. I
  should have stood up an instance and `curl`ed the endpoint on day one — the moment I did
  (own isolated `CHAN_HOME`, real tenant token via `cs window list`), the prefixed PUT→404 /
  root PUT→204 / persisted-to-config split fell out in one command. **Lesson, full stop: a
  "doesn't persist / doesn't propagate" bug gets reproduced against the live endpoint before
  any fix is dispatched — static traces name suspects, they don't convict.**
- **The C8 fix needed all three layers anyway.** W9 (gate accepts the tenant token), S5 (call
  the root path), S4 (watch→menu sync) — none alone was sufficient, and W9/S2/D5 weren't
  wasted, but they were dispatched in the wrong order because the layer wasn't pinned first.
- **Workspace / Desktop / @@webdev / Launcher were excellent.** Sharp root-causing once
  pointed right (the apiPath prefix, the remote-capability scope, the no-ACL eye path), fast
  atomic turnarounds, and regression tests that pin the exact gaps (`localColorRootPath.test.ts`
  asserts ordinary paths still prefix while local-color resolves to root — the precise miss).
- **Alex** carried the empirical load this round — the W9-still-fails and "workspace windows
  too" reports were what finally killed the wrong-layer theory. That load should have been
  mine.

## Carryover (→ `dev/v0.48.0/carryover.md`)

- Devserver collaboration model; terminal scrollback-replay garbage (deferred, hard-repro);
  editor/graph F1–F6 (verify already fixed); `chan open` default-port decouple (option C);
  full watcher-management of the control window (Model A). The terminal-regression analysis is
  in `dev/terminal-regression/analysis.md`.

## Pending Alex (pre-cut)

- A medium-size experiment Alex will run before the version bump.
- Then: version bump `0.47.0 → 0.48.0` (all pins incl. web), final `make pre-push`, tag.
  Nothing pushed/tagged yet; 27 commits local.
