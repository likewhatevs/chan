# Phase 22 - window management: bury-on-close, remote windows, cs visibility

Status: closed with v0.31.0 + the v0.31.1 patch.
Span: 2026-06-11 to 2026-06-12.
Versions: v0.31.0, v0.31.1.
Tags: #desktop #windows #terminal #cs #bugfixes #release

A solo round (one Claude session + Alex reviewing and desktop-testing), run in a dedicated `window-mgmt` git worktree off main and merged back fast-forward at close. The driving document was `dev/request.md` (untracked; its asks are summarized below), followed by three remark-driven follow-up rounds from Alex's desktop testing.

## Roadmap (the asks)

**dev/request.md:** remove the mislabeled "Settings Cmd+," Window-menu item; Hybrid Nav in standalone terminals should stage terminals only; fix the split-pane bug where the original terminal showed only its last line until Cmd+R; closing a window via the OS button should "bury" it (keep shells running, reopenable from the Window menu, with an every-time info dialog) for standalone terminals and workspace windows alike; Cmd+Shift+N reopens the last buried window; an API to enumerate a remote session's windows so outbound/tunnel connections can repopulate the Window menu; `cs` visibility of open/hidden windows; and a control socket for standalone terminals (workspace-only subcommands refusing clearly). Item 10 arrived truncated and was resolved by survey: Cmd+Shift+N unburies the most recent window of the focused family, else opens a new one; the close dialog is informational and shows every time; the standalone control socket was implemented, not just investigated.

**Follow-ups (round 2):** `make clean` covering gateway and desktop artifacts; removing the dead "p Stage Team Work Terminal" cheatsheet row; tidying the `recordOutputBytes` seq bookkeeping (became a full removal of the dead `lastSeq` replay cursor); +5px of tab-title room so short names keep their tail.

**Remarks (round 3):** the CI DMG renders the legacy title bar while a local build looks modern (root cause: the linked SDK; CI now selects the newest Xcode); Cmd+Shift+N must stop multiplying the launcher and instead follow the focused connection (launcher → standalone terminal, remote → new window on that remote); a bounced remote serve left the window stale with stuck terminals until Cmd+R (now auto-reloads via a per-process instance id on `/api/health`); and a quit confirmation while any window is open or hidden.

## What shipped

Sixteen commits on `window-mgmt`, grouped:

- **Web fixes:** terminal-only Hybrid Nav cheatsheet filtering; the split-pane "only last line" fix. The root cause was NOT the assumed WKWebView render glitch: a layout restructure remounts the terminal component, `term.dispose()` kills the client-side scrollback, and a surviving `lastSeq` cursor made the server skip the replay. First fixed by clearing the cursor at mount, then (follow-up) the cursor was deleted end-to-end — a reattach always feeds an empty xterm, so `since` is now the constant 0 (kept explicit so ring-overflow loss still surfaces as the "replay missed N bytes" notice).
- **Bury-on-close (desktop):** prevent_close + hide with an every-time info dialog; a "Hidden Windows" Window-menu section (dynamic menu rebuild); window numbers survive burial; `WindowEvent::Destroyed` as the single cleanup point; the WindowConfig LRU captures at bury time and skips live labels on pop; the SPA's empty-window cascade uses `destroy()` to bypass burial; terminal windows with no live shells (registry query through the embedded host) really close.
- **Server/cs:** refcounted `/ws` window presence (`?w=` tag), `GET /api/windows` (`{id, connected, saved}`, byte-pinned), `cs window list`, a `ControlTenant` split so the standalone terminal tenant runs its own control socket ($CHAN_CONTROL_SOCKET in its PTYs) with a pinned workspace-only refusal, and the window/survey reply routes on the terminal router so blocking round-trips work.
- **Remote windows (desktop):** outbound/tunnel connections are polled for `saved && !connected` rows; reopening uses the remote-known label so the remote restores its session blob.
- **Round 3:** Cmd+Shift+N follows the focused connection (the launcher is a singleton titled "Chan Desktop"; the main-N spawner is gone); per-process instance id on `/api/health` + SPA auto-reload on change; CI Xcode selection for modern window chrome; health answering on workspace-less tenants (a 503-noise regression caught by Alex from the desktop console); quit confirmation via `RunEvent::ExitRequested`.
- **Hygiene:** `make clean` covers gateway/desktop/web stamp; the dead Team Work cheatsheet row is gone; tab-title fade headroom.

## Verification

Browser smokes covered everything reachable without a desktop build: split/swap/Esc-cancel/reload replay, overlay filtering in both modes, presence connect AND disconnect flips, `cs terminal list` / `cs window list` / blocking `cs pane` round-trips, tab-digit legibility, and the server-bounce auto-reload (marker command, ^C + re-run, the window reloaded itself with an interactive terminal). Alex desktop-verified the bury flow (dialog, menu, reopen), Cmd+, after the menu removal, real-close of shell-less terminal windows, the standalone-terminal `cs` matrix, and the outbound remote arc. Full `make pre-push` ran green in the worktree before each merge-back.

## Retrospective

**Highlights:**
- Root-causing the split-pane bug from source (replay cursor, not a render glitch) predicted Chrome reproducibility, which turned a desktop-only mystery into a fast browser-verifiable fix — and the follow-up deleted the whole cursor instead of patching its overshoot.
- The `connected`/`saved` vocabulary for window presence stayed honest about what the server can know (a buried desktop window keeps its socket alive), which kept the remote-windows design simple.
- Three rounds of Alex desktop testing each fed precise, actionable remarks; every regression they surfaced (503 noise, launcher multiplication) was reproduced, fixed, and pinned in the same round.

**Lowlights / carryovers:**
- GTK in-place Window-menu mutation is unverified on Linux; the fallback (full `set_menu` rebuild) is documented but not wired.
- The CI Xcode selection is verified only by the next release run's log; if the runner's newest Xcode is still pre-26 the runner image needs bumping.
- ~~A staged-then-cancelled split leaves an orphaned PTY~~ — fixed in v0.31.1 (cancel runs the staged terminals' close sinks while the draft still renders).
- `MAX_WINDOWS_PER_WORKSPACE` (10) counts buried windows.
- Buried windows keep their webviews alive — deliberate (warm state), but it is memory the user can't see.

## Notes

- Quit confirmation, the launcher-singleton change, and new-window-on-remote landed after Alex's main desktop pass; they ride v0.31.0 with compile + pin-test coverage and his release-build validation.
- v0.31.1 (same day) fixed what his v0.31.0 validation caught: the quit dialog never fired (macOS predefined Quit bypasses the ExitRequested hook; replaced with a custom Quit item that asks first) and connecting/retry windows were unclosable (red dot buried them; now red dot / Cmd+W / Ctrl+Shift+W / Ctrl+D all cancel for real, with the Cmd+W routing done in the key bridge, which consumes the chord before the menu accelerator). It also added Linux's Ctrl+Shift+W Close Window item, stripped the GTK menubar from the About dialog, closed the staged-split orphan-PTY carryover, and de-flaked the PTY shell probes whose end markers could match their own command echo (the v0.31.0 tag-run flake).
- The 400 GB stale `target/` discovered mid-round motivated the `make clean` expansion.
