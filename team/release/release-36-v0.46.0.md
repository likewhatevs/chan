# Phase 36 — v0.46.0: launcher polish, editor/graph fixes, desktop hardening

Status: released as `v0.46.0`. One dispatch round (5-agent team: Lead + Launcher / Editor /
Graph / Desktop, four parallel lanes) that opened on a small confirmed scope and then **roughly doubled
through Alex's live desktop hand-smoke** — the team stayed up and each thing he found became a scoped
follow-up that folded into the same tag. The planned scope: launcher served-workspace bulk management +
SquareTerminal icon, the v0.45.0 carryover follow-ups F1–F5, the devserver fd-leak fix, and a one-line
manual fix. The hand-smoke wave added: `cs upload` native picker, the Open-windows Focus/Show-Hide buttons,
in-flight spinners that survive reload, a dismissable error banner, `chan open` devserver live-refresh, a
FOCUS-raises-visible-window fix, and an un-zoomed app icon. Every lane reported a scoped own-gate-green;
Lead ran the full-tree `make pre-push` on the committed state (it caught a real failure — see lowlights)
and re-ran it green before the tag. WKWebView-native bits ship gated-green + Alex-hand-smoke-confirmed.
Span: 2026-06-23.

Tags: #web-launcher #devserver #editor #graph #desktop-dialog #fd-leak #app-icon #cs-upload
#window-focus #5-agent-team #hand-smoke-driven

Phase 35 shipped the v0.45.0 desktop release (launcher + devserver-in-launcher + lifecycle hardening).
Alex's end-of-round hand smoke left a follow-up backlog (F1–F6) and a diagnosed-but-unapplied devserver
fd-leak. Phase 36 closes F1–F3 + F5 + the fd-leak, finishes the launcher's served-workspace management, and
then — driven by a second, deeper hand-smoke of the assembled build — hardens the desktop upload path,
window focus, devserver live-refresh, launcher feedback, and the app icon.

## Shipped (all gate-green; desktop/native parts Alex-hand-smoke-confirmed)

- **Launcher — served workspaces managed like local** (`ffe4ee8c`): a `"served"` selection kind, one
  App-level bulk bar across local + served + devserver, ordered cross-kind delete, F6 fail-safe bulk-off.
- **Launcher — SquareTerminal top-bar icon** (`0dc418ca`).
- **Launcher — Open-windows Focus / Show-Hide buttons** (`dad5e9b9` + focus fix `25376aee`): replaced the
  click-to-toggle dot with a Focus button (`openWindow`) and an Eye/Eye-off toggle (`toggleWindow`). The
  initial build's Focus no-op'd on an already-visible window — `unbury_window` returned early before
  `show()`+`set_focus()`; fixed in both watcher branches.
- **Launcher — in-flight spinners surviving reload** (`2d22182c`): localStorage-persisted per-item pending
  markers, reconciled against the real registry on every refresh + `loadLibrary`, timer-free 45s self-clear.
- **Launcher — dismissable error banner** (`6bfc2a76`): an [X] → `clearError()`.
- **Launcher — `chan open <url>` live-refresh** (`6125e0a7`): the out-of-band CLI handoff now fires
  `signal_library_change()` (the launcher form already self-refreshed; removal already signalled).
- **Editor — wiki-link resolve before open (F1)** + **reopen-tab keeps expanded dirs (F3)** (`d2d1376c`).
- **Graph — file-node "Open" routes to the editor (F2)** (`45d8bb99`), via the shared `kinds.ts` predicate.
- **Desktop — native NSAlert confirms honor Return (F5)** (`153f0ce3`): a `cfg(macos)` objc2 shim with
  `setKeyEquivalent("\r")` + self-pumping `runModal()`.
- **Desktop — devserver fd-leak fixed (§7)** (`db26660d`): one `OnceLock`-cached `reqwest::Client`
  (was a fresh client per poll → ~22 ESTAB/min → 1024-fd cap → death ~40 min).
- **Desktop — `cs upload` native picker** (`0df8647b`): WKWebView blocks a programmatic file-input click,
  so desktop branches to a native `pick_upload_files` (ACL-scoped to local windows, excludes `outbound-*`)
  feeding the unchanged upload pipeline.
- **App icon un-zoomed** (`e5ddfd4b`): regenerated from source with the enso back at its original margin;
  added `scripts/gen-app-icon.py` as the reproducible pipeline.
- **Manual intro bullet fix (§3)** (`70b82aab`) + the ACL-parity test grant (`e0f79eff`).

## Deferred to carryover (next phase)

F4 (editor caret can't click a line inside an *expanded* fenced code block — a WKWebView `posAtCoords`
hit-test quirk; both candidate fixes disproven, needs on-device iteration); browser-served `cs upload`
(Chromium gesture gate, needs a different affordance); the §7 connect-failure feed/window reap; the dead
Tauri `add/update/remove_devserver` commands cleanup; the tokei "Unknown extension" log silence; the
terminal scrollback-replay garbage-on-reload recurrence; rich-prompt image paste; `chan devserver
--stop/--restart` for the launchd/systemd service.

## Retrospective

**Highlights.**
- The hand-smoke-driven model worked: Alex hand-smoking the assembled build caught real, user-facing
  bugs (Focus no-op, `cs upload`, devserver live-refresh, un-dismissable banner, over-zoomed icon) that no
  static gate or headless test would have — and the live team turned each around same-round.
- Lane discipline held: every commit was pathspec-scoped and own-gate-green; Editor's refusal to ship a
  plausible-but-wrong F4 fix (disproving BOTH candidates) avoided shipping a regression; Desktop's
  corrected traces (HTTP-not-Tauri devserver CRUD; the one out-of-band path) tightened fixes to their
  minimum.
- The integrated gate earned its keep (see lowlights) and the bump/lock handling stayed correct
  (external `windows-sys 0.45.0` was *not* mis-bumped because locks were updated via cargo, never sed).

**Lowlights / what to fix.**
- **Own-gate ran `clippy --all-targets` but not `cargo test`**, so a runtime test assertion
  (`app_acl_grants_every_registered_command`, tripped by the new `pick_upload_files` command) compiled but
  was never *run* — it slipped to Lead's integrated gate. Partly Lead's fault: the task-spec own-gate
  list omitted `cargo test`. Fixed + lesson locked (Desktop ran `cargo test` on the next fix).
- A compound `make pre-push; echo "EXIT=$?"` background command let a trailing echo mask make's real exit,
  so the harness reported "exit 0" on a gate that actually failed (`EXIT=2`). Caught by reading the log, not
  the notification — but the wrapper should propagate the real exit (fixed on the re-gate).
- Scope grew large mid-round (≈8 post-hand-smoke additions). Productive, but a single round carrying a
  doubling scope strains tracking; a clean cut between "planned" and "hand-smoke follow-ups" (or a fast
  v0.46.1) is worth considering next time.

**Feedback to Alex.** The live hand-smoke + immediate-dispatch loop is high-value and you drove it well
(clear repros, screenshots, decisive "ship it"). Two notes: (1) several finds were pre-existing bugs, not
this round's regressions — batching a "hand-smoke pass" before opening a round (vs during the close) would
let them be planned scope rather than close-time adds; (2) no in-app way to verify the launcher's live paths
without a real desktop kept every frontend smoke on you — the in-app driver / a connected Chrome would
offload that.

**Feedback to Lead (self).** Good: anchored on git/logs over notifications; reviewed every diff including
ACL scoping and the corrected traces; round-closer fixed the trivial gate-caught nit rather than spinning a
worker up. To fix: own-gate task-specs must list `cargo test` explicitly (the omission caused the only red),
and never let a verifying command's exit be masked by a trailing echo.
