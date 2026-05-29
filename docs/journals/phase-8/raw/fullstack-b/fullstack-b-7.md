# fullstack-b-7: chan-desktop external http/https links open at OS default browser

Owner: @@FullStackB
Date: 2026-05-20

## Goal

Inside `Chan.app` (the Tauri-bundled chan-desktop), clicking
an external `http://...` or `https://...` link in the
embedded webview must hand the URL off to the OS default
browser. Internal app routes stay inside the webview.

## Background

Source in [`../phase-8-bugs.md`](../phase-8-bugs.md):
"chan-desktop: external `http`/`https` links do not open at
all", flagged 2026-05-20 by Alex. Repro: clicking the
Round-1 test-server URL
(`http://127.0.0.1:8787/?t=...`) inside `Chan.app` is a
complete no-op — no in-webview navigation, no external
browser launch. This blocks Alex from clicking the
@@WebtestA test-server URL hand-off from inside the desktop
app.

Lives in chan-desktop / `desktop/src-tauri/` URL handling,
not the web SPA.

## Acceptance criteria

* Clicking an external `http(s)://...` link inside `Chan.app`
  opens the URL in the OS default browser.
* The chan-desktop webview does NOT navigate away from the
  embedded SPA on external link click.
* Internal app routes (`/`, in-app hash routes, etc.) still
  resolve inside the webview as today.
* `127.0.0.1` and `localhost` URLs are treated as external
  (the test-server URL is `127.0.0.1:8787`; that needs to
  open in the OS browser, not the embedded webview, so
  bearer-token sessions don't collide with the embedded
  chan-server session).
* Manual verification on `Chan.app` (built with
  `make app` / `npm run tauri build`): test-server URL
  click opens in Safari / Chrome / Firefox per OS default.

## How to start

1. Look at `desktop/src-tauri/tauri.conf.json` and any
   custom URL-handling code in `desktop/src-tauri/src/`.
   Tauri 2's default config blocks navigation to external
   origins from the webview but does not by itself trigger
   `shell.open` on the link click.
2. Two common patterns:
   * Add a `tauri-plugin-shell` permission for `open`, then
     intercept link clicks in the SPA (`<a target="_blank">`
     or a click handler) and call
     `invoke('plugin:shell|open', { path: url })`.
   * Or hook the navigation event in Rust
     (`WebviewWindow::on_navigation` / equivalent in Tauri 2)
     and call `tauri::api::shell::open` for non-app origins.
3. Pick the option that keeps the SPA build target-agnostic
   (the webview-side intercept is preferable: same SPA bundle
   runs in chan-desktop and in a plain browser; the intercept
   no-ops when Tauri's `__TAURI__` global is absent).
4. Confirm internal hash routes + `tauri://localhost` (or
   whatever Tauri's local origin is) still resolve in-app.

## Coordination

* This task does not require runtime walkthrough by @@WebtestB
  (build + manual launch + click → opens / does not open is
  a binary check). @@FullStackB self-verifies the manual
  launch portion using the `Chan.app` build path they own;
  if standing permission scope does not cover a Tauri build
  + launch from this lane, fire a `permission` event to
  @@Alex via `event-fullstack-b-alex.md` before proceeding.
* @@CI is independently working on the tag-triggered
  chan-desktop release pipeline in `ci-2`; coordinate with
  them on any `tauri.conf.json` edits that might affect the
  bundle output shape.

## 2026-05-20 - root cause + capability fix

Diagnosis: the SPA's external-link path is already correct end to
end. `web/src/editor/external_links.ts::openExternalUrl` detects
the Tauri global, calls `__TAURI__.opener.openUrl(url)` if the JS
binding is exposed, otherwise falls back to
`invoke("plugin:opener|open_url", { url })`. Both editor link
clicks (`externalLinkClickHandler` via CodeMirror) and bubble
overlay clicks (`BubbleOverlay.svelte` button onclick) route
through it. SPA grep confirms there are no bare `<a href>` tags
that bypass the helper.

The break is on the Tauri side: `capabilities/drive.json` only
granted `core:webview:allow-set-webview-zoom` to `drive-*` /
`tunnel-*` windows. Without `opener:allow-open-url` on that
capability, the `plugin:opener|open_url` IPC denies on every
drive/tunnel webview. `openExternalUrl` catches the rejection,
falls through to `copyAndNotifyFailure` (clipboard write +
`notify(...)` toast), and looks like a no-op to a user who
doesn't notice the toast. The repro Alex hit
(`http://127.0.0.1:8787/?t=...` click inside Chan.app) lives
inside a drive webview, so it landed on this code path.

Bonus drift found alongside the primary bug: `capabilities/default.json`
targeted `["main"]` only, so the Cmd+N "additional launcher"
windows (`main-N`, added by `fullstack-83`) inherited NO
capability. Same opener denial would hit any external link
clicked from a second-or-later launcher window. The drive bug
just made it visible first because launcher-window external
clicks are rare in practice.

Fix:

* `desktop/src-tauri/capabilities/drive.json` — add
  `opener:default` + `opener:allow-open-url` next to the
  existing webview-zoom permission. drive-* and tunnel-* both
  inherit so local and tunneled drive webviews behave
  identically.
* `desktop/src-tauri/capabilities/default.json` — widen
  `"windows": ["main"]` to `["main", "main-*"]` so launchers
  spawned via File > New Window inherit the same plugin set as
  the singleton.

No SPA changes. The fix is a strict widening of the trust
surface: any drive webview can now call
`plugin:opener|open_url`. The SPA is shipped embedded in the
chan binary (rust-embed) so we control what reaches that IPC;
the worst that can happen from a malicious note is "the OS
opens the URL the note pointed at", which is the same outcome
the user explicitly opted into by clicking.

`127.0.0.1` / `localhost` URLs satisfy the acceptance criterion
naturally because `isOpenableExternalUrl` only checks the URL
scheme (`http`/`https`/`mailto`/`tel`). The OS browser picks the
URL up and opens it in a separate tab there — the SPA never
calls `window.open` inside a Tauri webview, so the embedded
chan-server session in the originating drive window stays
untouched (no bearer-token cross-session collision).

Tests:

* Two new structural tests in `desktop/src-tauri/src/serve.rs`
  (`drive_capability_grants_opener_to_drive_and_tunnel_windows`,
  `default_capability_covers_extra_launcher_windows`) that parse
  the capability JSON via `include_str!` and assert both the
  window globs and the `opener:allow-open-url` permission. They
  fail loudly if a future capability edit drops the permissions.
* No SPA test changes — `web/src/editor/external_links.test.ts`
  already covers both the bound-JS-API path and the
  `invoke("plugin:opener|open_url", ...)` fallback path with the
  matching mock shape; the runtime fix is purely about removing
  the IPC denial.

Pre-push gate green:
* `cargo fmt --check` — clean.
* `cargo clippy --workspace --all-targets -- -D warnings` —
  clean.
* `cargo test --workspace` — every suite passes; chan-desktop
  itself reports 19/19 including the two new tests.
* `cargo build --workspace --no-default-features` — clean.
* `npm run check` (svelte-check) — 0 errors, 0 warnings.
* `npx vitest run` — 475/475 (unchanged from -2..-6 baseline).
* `npm run build` — clean.

## 2026-05-20 - permission ask for runtime verification

Acceptance criterion 5 ("manual verification on `Chan.app`
built with `make app` / `npm run tauri build`: test-server URL
click opens in Safari / Chrome / Firefox") needs a Tauri build
+ launch. Standing scope from `fullstack-b-1` covered chan-
desktop crate edits + `cargo test -p chan-desktop`, not a full
`make run` / `make app`.

Firing a `permission` event to @@Alex via
`event-fullstack-b-alex.md` for either:

* @@Alex runs `make run` from `desktop/`, opens the lane-A or
  lane-B test-server URL inside a drive webview, clicks it,
  confirms the OS default browser launches at the URL; OR
* @@Alex approves @@FullStackB to run `make run` (debug; ~3-5
  min build + launch on first invocation) for the same check,
  with the standing tear-down rule (kill chan-desktop process
  on completion).

Approval bar: either form per `process.md` "Approving a
permission event". Code review for @@Architect proceeds in
parallel; the runtime confirmation only gates the
"Acceptance criteria satisfied" claim on this task, not the
review itself.

## 2026-05-20 - commit readiness

Files changed (proposed single commit):

* `desktop/src-tauri/capabilities/drive.json` — add
  `opener:default` + `opener:allow-open-url`.
* `desktop/src-tauri/capabilities/default.json` — widen
  `windows` to `["main", "main-*"]`; updated description.
* `desktop/src-tauri/src/serve.rs` — two new tests pinning the
  capability shape via `include_str!`.

Tests run: full pre-push gate green (see implementation note
above).

Known risks: widens IPC surface in drive/tunnel webviews to
include `plugin:opener|open_url`. SPA is trusted (we ship it
embedded); see implementation note for the threat-model
argument.

Push waits for Round-1 close per the standing rule.

Proposed commit subject:
`Grant opener IPC to drive/tunnel/main-N windows so external links open in OS browser (fullstack-b-7)`

## 2026-05-20 — @@Architect: approved + commit clearance (code review only)

Reviewer: @@Architect.

Sharp diagnosis. Tracing the symptom from "click is a no-op"
to "opener IPC denied in drive/tunnel webviews because
`capabilities/drive.json` only granted webview-zoom" is the
load-bearing piece — the SPA was already correct, the trust
surface was the gap. The bonus finding on `default.json`'s
`["main"]`-only glob (so Cmd+N launchers from `fullstack-83`
inherit no capabilities) is the right kind of opportunistic
fix: same root-cause family, ships in the same single commit,
spares a future task.

The two structural tests in `serve.rs` reading the capability
JSON via `include_str!` and asserting both the window globs +
`opener:allow-open-url` are the right pin shape — they catch
a future capability edit that drops the permission, which is
exactly the regression family we'd otherwise miss in code
review. Good engineering instinct.

Threat-model framing is correct: widening the IPC surface in
drive/tunnel webviews to `plugin:opener|open_url` is fine
because we ship the SPA embedded (rust-embed) — the worst
case is "the OS opens the URL the user clicked on" which is
the intended behavior. Recording the reasoning in the task
note is exactly what we want for the audit trail.

Pre-push gate green: fmt + clippy + workspace tests (19/19
chan-desktop including the two new pins) + no-default-
features build + svelte-check + vitest 475/475 + npm build.
That's the full stack.

**Commit clearance (code review)**: approved. Use your
proposed subject as-is. Push waits for Round-1 close.

**Runtime walkthrough**: parked until @@Alex returns. @@Alex
has stepped away for an extended window and explicitly held
the build cut for their return; the manual `Chan.app` URL-
click verification falls into the same hold. They'll likely
combine the click-check with the build cut in one session.
The code review + structural tests + the SPA-side
`external_links.test.ts` coverage are sufficient interim
confidence; the runtime click is the final empirical seal,
not a gate on the commit itself.

Your permission ask in
[event-fullstack-b-alex.md](../alex/event-fullstack-b-alex.md)
stays open. I am NOT transcribing approval — both options
in your ask need @@Alex's interactive participation (option
A is them running it themselves; option B is them
authorising you to). When they return they'll pick.

After commit you can carry on with `fullstack-b-8`
(Cmd+Enter first-char swallow) and `fullstack-b-9` (Cmd+T
web alternate chord). Queue order unchanged.