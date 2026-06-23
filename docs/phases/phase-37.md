# Phase 37 — v0.47.0: devserver / launcher connect lifecycle

Released 2026-06-23. Team: @@Lead (architect/gate), @@Devserver (`crates/`), @@Desktop
(`desktop/src-tauri/`), @@Launcher (`web/` + `web-launcher/`). Single cut on @@Alex's go.

## Theme

The devserver/chan-launcher connect lifecycle — the window-feed and control-terminal bugs
@@Alex hit hand-smoking a live devserver, plus the `chan devserver` tunnel/listen behaviour for
gateway use. It opened as four problem clusters and grew, via two of @@Alex's mid-round catches,
into a deeper window/library-model round.

## What landed (by theme)

- **Theme 1 — `chan devserver` tunnel/LISTEN** (`17f8b83f`) **+ `--stop`/`--restart`** (`3745771a`):
  tunnel token present ⇒ no local bind by default; `CHAN_DEVSERVER_LISTEN` override; supervised
  start/stop/restart.
- **Bug-A + Bug-B, fixed at the ROOT via the ARCH unification.** @@Alex's principle — "no terminals
  outside chan-library" — turned the planned desktop-side workarounds into a re-architecture: the
  control terminal became a first-class chan-library `WindowRegistry` window (`e81ee7bc` library_id
  in DevserverInfo, `8b6ba341` chan-library mint/reap, `fbeb7034` desktop rewire). The control
  record now rides the registry snapshot (shows on a zero-window connect, Bug-A) and is reaped by
  `WindowRegistry::remove` on PTY exit (Bug-B-iii). Bug-B-i survey ported to the SPA
  (`3c2d5e3e`/`5a44eab2`); Bug-B-ii connected-flip (`eeb00c06`).
- **Theme 5 — window visibility** (`85aa2c0f` chan-library `hidden`, `c06610db`→`231171f0` SPA,
  `26ed5207` desktop): hidden state persists + mirrors on connect; launcher shows hidden inline
  with an eye toggle (the Open/Hidden *section* was reverted per @@Alex — icon-only).
- **Theme 6 — live per-library focus colour** (`0a14d8a9` server watch + dedicated notify,
  `051e3608` pane subscribe, `a274bf9d` desktop consume): colour change broadcasts to all open
  windows live; replaced the 5s poll.
- **Cleanup:** dead Tauri devserver CRUD removed (`b1c64e94`).
- **Hand-smoke fixes:** colour null-push no longer clobbers the `?pane=` seed (`f407f2eb`);
  devserver stays connected on a benign setup-script exit (`95b9b64f`); closed workspace windows
  don't re-open on restart (`36bda0d5`); `pane_color` build trace (`811816ab`).

## Highlights

- **Ratify-before-build earned its keep.** On the ARCH re-architecture alone it caught five
  would-be cross-lane defects *before* any mismatched code shipped: the mechanically-impossible §5
  opener override, the open/close fork (Model B), the reap-trigger ownership, the control-dot
  semantics, and the hide-persist mechanism. None were compile errors — all were runtime/WKWebView
  bugs that would otherwise have slipped to hand-smoke.
- **Lane discipline.** Atomic pathspec commits in the shared tree; peers verified each other's
  WIP before building on it (@@Launcher confirmed @@Devserver's uncommitted `hidden` field matched
  its TS; @@Desktop isolated-worktree-gated around a peer's transient broken-main). Wire contracts
  were pinned and the WindowRecord HTTP wire kept byte-identical, so the whole control-terminal
  re-architecture was invisible on the wire (zero launcher change).
- **Empirical seam-settling.** When B/C ping-ponged ("my side's correct"), an actual WS-client
  round-trip + a disk-persist harness settled them as server-correct, redirecting to the real roots.

## Lowlights + honest feedback

- **@@Lead (me).** Two real misses: (1) I made the launcher's Open/Hidden *section* a default
  @@Alex never asked for — a build-then-revert cost; UX defaults the host hasn't requested should
  be surveyed upfront, not default-and-flag. (2) I **declined** adding a WS-client e2e test to the
  Theme-6 harness ("it mirrors the proven window watch"), and that exact gap let the devserver-colour
  seam pass gate-green and surface only in hand-smoke. Lesson: a novel WS path gets its own e2e
  test, full stop.
- **Gate-green ≠ done for WKWebView/runtime/persistence.** The colour new-window read, the
  setup-script connected-flip, and the restart-restore were all unit-untested; the hand-smoke was
  the only thing that caught them. Honest framing held — better caught now than shipped.
- **Build-provenance hazard.** Version skew between the terminal `chan`, the AppImage, and the
  devserver caused both the `chan open` :8787 nit and the *apparent* devserver-colour deadness.
  Consistent-build hygiene matters for the Linux/AppImage hand-smoke.
- **@@Devserver/@@Desktop/@@Launcher** were excellent — sharp root-causing (the §3 flip over-eager
  on setup-script exit; the boot re-serve re-minting closed windows; the null-push clobbering the
  seed), and each cleared its own lane decisively before pointing elsewhere.

## Carryover (in `dev/v0.47.0/carryover.md`)

- Full watcher-management of the control window (Model A) — purer, no user-visible win this round.
- Regular terminal-window reap on PTY exit (exited+detached), respecting the no-natural-exit guard.
- `chan open` raw `EADDRINUSE` on :8787 when a handoff falls through — decouple the default port (C)
  or a clearer error (A); needs @@Alex's live repro to pin the fall-through.
- Plus the prior backlog (devserver collaboration model, rich-prompt image paste, editor F1–F6 +
  native-dialog, terminal scrollback-replay, `cs upload` file picker).

## Pending @@Alex (post-release)

- Survey-semantics for the control-closed prompt: only-on-unreachable (current) vs any-control-exit
  (a 1-line change) — his UX call.
- Re-smoke confirmation of the colour + Bug-A/C fixes against consistent current-main builds.
