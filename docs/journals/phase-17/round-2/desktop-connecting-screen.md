# Task: chan-desktop connecting / retry screen for remote-workspace windows

Dispatched by @@LaneA (Lead) for @@LaneB / @@LaneC / @@LaneD. Identify your
role from `$CHAN_TAB_NAME`. Team process: read
`docs/journals/phase-17/team/bootstrap.md` first if you were just `/clear`ed.

## The ask (verbatim from @@Alex)

> in the chan-desktop when we click to open a pre-configured attached remote
> workspace and that URL is not available, the newly open window is just a
> white window with nothing in it. What I want: every time we click to open a
> window which is an outgoing connection to a URL, we print text on that
> window: `[spinner] connecting to {url}... (timer)` and if it times out, keep
> printing the retries until the user closes the window. Each row should have a
> timestamp so the user knows when / since when the connection has been
> attempted.

## Acceptance criteria

1. Opening a remote-workspace window (an outgoing connection to a URL) shows a
   connecting surface immediately, NOT a blank white webview, while the remote
   is being reached.
2. The surface shows `connecting to {url}...` with a live-updating elapsed
   timer and a spinner.
3. On failure / timeout it keeps RETRYING and appends one row PER attempt; each
   row carries a timestamp (wall-clock, so the user sees when each attempt ran
   and since when it has been trying). Retries continue until the user closes
   the window (no silent give-up).
4. On success the window navigates to the live workspace as normal.
5. Pure ASCII in any committed text/docs; no em dashes (repo writing rules).

## Roles (lowest-collision split; coordinate on the contract)

- **@@LaneB (scope + desktop/Tauri side)** OWNS `desktop/src-tauri/`. First,
  SCOPE how chan-desktop opens a pre-configured remote-workspace window today
  (where the blank-white WebviewWindow load happens), and POST a short
  window<->page CONTRACT into this file's "Contract" section below (how the
  connecting page is loaded, how it receives the target URL, how retry/success
  is signalled). Then implement the detection + retry driver against it.
- **@@LaneD (connecting-screen page/UI)** OWNS the connecting surface itself:
  spinner + `connecting to {url}...` + elapsed timer + the scrolling,
  timestamped retry-log rows. Build it against @@LaneB's posted contract; until
  the contract lands, stub the inputs so the page renders standalone.
- **@@LaneC (webtest / verify)** writes NO product code. Empirically verify the
  built behavior: point a window at an UNAVAILABLE remote URL and confirm the
  connecting screen shows (not blank white), the timer + timestamped retries
  accrue, retries continue until close, and a reachable URL still loads. Report
  findings; do a desktop build/run if needed (chan-desktop is `cargo
  tauri build` / `./target/release/chan-desktop`).

## Coordination

- @@LaneB posts the contract FIRST; @@LaneB (Tauri) and @@LaneD (page) then run
  in parallel against it; @@LaneC verifies once it builds.
- Own non-overlapping files. Report blockers / completion to @@LaneA via a
  1-line poke pointing at your journal or this file (lean poke bus: context
  lives in files, not pokes).
- Gate your own slice green (cargo fmt/clippy/test for Rust; the desktop crate
  builds via `make -C desktop check`); @@LaneA owns the full-tree gate before
  any push. Do NOT push.

## Contract (posted by @@LaneB)

### Scope: outbound only

Three webview flavours exist (serve.rs): `workspace-*` (local embedded
server, started + URL-awaited before the window opens, so never blank),
`tunnel-*` (remote dials IN; reached over a LOCAL per-tenant loopback
listener, only opened once that listener binds), and `outbound-*` (an
explicit "attach a remote by URL" row; the webview points straight at a
remote we do not own). The blank-white bug is the OUTBOUND case: serve.rs
`build_workspace_window` opens `WebviewUrl::External(remote)`, and when the
remote is unreachable WKWebView never finishes navigating and paints white
(the nil-URL panic guard at serve.rs ~443-447 documents this exact case).
The ask ("outgoing connection to a URL") IS the outbound flavour, so the
connecting screen wires `outbound-*` only. Local + tunnel keep their direct
`WebviewUrl::External` load.

### Decision: the retry loop runs in the PAGE (LaneD), not in Rust [RATIFIED by @@LaneA]

@@LaneA ratified the page-driven loop (lost-event-race rationale confirmed;
do NOT switch to Rust emit) and the outbound-only scope (local/tunnel await
their listener, never blank).


The page calls a LaneB-owned `probe_url` IPC each attempt. Rationale: the
page is the only context that can start probing AFTER its own DOM + handlers
are ready, so there is no lost-event race. A Rust-driven emit loop can fire
`connecting-ready` / the first attempt event BEFORE the webview attaches its
listener (Tauri does not replay events to late listeners), which would
strand the screen or skip the success navigation. So LaneB owns the
DETECTION primitive (`probe_url`) + the window redirection that makes the
loop possible + the capability line; LaneD owns the loop cadence, the
elapsed timer, the rows, and the success navigation. (Flagged to @@LaneA:
if a Rust-driven loop is preferred, say so and I will switch to emit + a
page-ready handshake; the page contract below is otherwise unchanged.)

### 1. How the connecting page is loaded

`serve::spawn_outbound_workspace_window` no longer points the webview at the
remote. It loads `WebviewUrl::App("connecting.html")` (a bundled page under
`desktop/src/`, LaneD-owned, served from the app origin) and injects an init
script that runs BEFORE any page script:

    window.__CHAN_CONNECTING__ = { "url": "<display>", "target": "<navigate>" };

- `url`    = the clean remote URL to display ("connecting to {url}...") and
             to hand to `probe_url`. The user-facing attachment URL verbatim.
- `target` = the full URL to navigate to on success. LaneB pre-assembles it
             exactly as the old direct load did: remote URL + `?w=<window-
             label>` + any restored `#fragment`, so per-window SPA state and
             the window-config restore still work after navigation.

### 2. How the page receives the target URL

Via `window.__CHAN_CONNECTING__` (the injected init script above), NOT a
query string. Avoids URL-encoding pitfalls and matches the existing
`initialization_script(KEY_BRIDGE_JS)` pattern. Read it synchronously on
load:

    const { url, target } = window.__CHAN_CONNECTING__ || {};

### 3. How retry / success is signalled

- DETECTION (LaneB IPC): `invoke('probe_url', { url })` resolves to
      { reachable: boolean, status: number | null, detail: string }
  `reachable` is true when the remote returned ANY HTTP response (even
  401 / 404: the server is up and serving). It is false ONLY on a transport
  failure (connection refused / DNS / TLS / timeout), which is exactly the
  blank-white case to retry past. `detail` is a short ASCII reason for the
  row; `status` is the HTTP code when reachable. A rejected promise (IPC
  error) should be treated as a failed attempt.
- LOOP (LaneD page): on load, render "connecting to {url}..." + spinner + a
  live elapsed timer (setInterval). Then loop: await `probe_url`; if
  `reachable`, `window.location.replace(target)` (done); else append ONE
  timestamped row (wall-clock `new Date()`, attempt #, and `detail`) and wait
  ~2s before the next attempt. Never give up: the loop ends only when the
  user closes the window (closing tears down the page + loop for free).
- The probe carries a 5s server-side timeout, so a black-hole host cannot
  hang the loop. Pace attempts ~2s apart AFTER each probe resolves (no
  overlapping in-flight probes).

### 4. CSP / capability notes for LaneD

- `connecting.js` MUST be an EXTERNAL file (`<script src="connecting.js">`):
  the app CSP is `script-src 'self'`, so inline `<script>` is blocked.
  Inline styles are allowed (`style-src 'unsafe-inline'`); a `connecting.css`
  is cleaner but optional.
- The page CANNOT `fetch()` the remote directly: CSP `default-src 'self'`
  blocks cross-origin connect. That is the whole reason detection goes
  through the `probe_url` IPC (Rust has no CORS / CSP).
- `window.__TAURI__.core.invoke` is available (`withGlobalTauri: true`), the
  same surface the launcher's main.js uses (`const { invoke } =
  window.__TAURI__.core;`). LaneB adds `allow-probe-url` to the
  `workspace-window` permission set so `outbound-*` windows may call it (the
  connecting page runs in an `outbound-*` window, which already carries that
  set).

### Files

- LaneB: `desktop/src-tauri/src/{serve.rs,main.rs}`,
  `desktop/src-tauri/permissions/app.toml`. (No tauri.conf / capability JSON
  change needed: `outbound-*` is already in the `workspace` capability, and
  that capability applies to app-origin pages because `local` defaults true.)
- LaneD: `desktop/src/connecting.html`, `desktop/src/connecting.js`
  (+ optional `desktop/src/connecting.css`). NEW files; no overlap with the
  launcher's index.html / main.js / styles.css.
- LaneC: verify only, no product files.

### Status

- [x] Scoped + contract posted (LaneB).
- [x] `probe_url` + outbound redirection + permission landed (LaneB).
      serve.rs 6908fe4f, main.rs c6f428b3, app.toml 3b3296b2. Own-gate GREEN:
      cargo fmt --check + clippy --all-targets -D warnings + test --all-targets
      (81 + 7 passed, incl. probe_url + connecting-page + ACL pins).
- [x] connecting.html / .js (+ .css) built against the contract, smoked
      standalone in dark + light, all states (LaneD). Awaiting LaneB's
      `probe_url` + redirection for the live end-to-end.
- [~] empirical verify (LaneC): Stage-1 page (Chrome) + integrated build +
      @@LaneB wiring tests + probe premise ALL green. The live WKWebView visual
      (window paints the connecting screen, not blank white) is a 60-second
      @@Alex hand-smoke: an agent cannot drive it here (AXIsProcessTrusted=
      false -> no synthetic clicks; WKWebView not Chrome-drivable; the launcher
      "Open" button is the only outbound trigger, no deep-link/CLI/auto-open).
      Findings below; recipe: connecting-screen-handsmoke.md.

### Stage-2 verify findings (LaneC, 2026-06-03)

GREEN (every agent-verifiable layer):
- Stage-1 connecting page in Chrome (standalone): immediate paint (never blank
  white), spinner, live MM:SS timer, ONE timestamped row per attempt accruing
  to attempt 10 with no give-up, demo=ok success state (green "connected (HTTP
  200)" + "Opening workspace..."), dark + light, zero console errors.
- Integrated build: `chan-desktop` debug builds clean (exit 0). Full
  desktop/src-tauri test suite green: 81 + 7 passed, 0 failed.
- @@LaneB wiring pinned green, confirmed BY NAME:
  serve::tests::outbound_windows_load_the_connecting_page_not_the_remote (ok),
  serve::tests::invoke_handler_registers_probe_url (ok).
- probe_url premise (its reqwest GET), measured against real servers:
  live  http://127.0.0.1:8921/  -> HTTP 200            (reachable:true  -> navigate)
  dead  http://127.0.0.1:59999/ -> refused, curl exit 7 (reachable:false -> retry)
  Both branches the page loops on are correct.

OPEN (human-at-keyboard, by design - see connecting-screen-handsmoke.md):
- The live WKWebView visual: dead-URL outbound row -> connecting screen (not
  blank white) + accruing retries; live-URL row -> navigates to the workspace.
  Not agent-drivable on this macOS setup (no Accessibility perm; WKWebView not
  Chrome-drivable; no non-UI open trigger).

### Stage-1 standalone verify recipe (for @@LaneC, from @@LaneD)

The page renders + runs its whole retry loop WITHOUT Tauri (no `probe_url`,
no init script yet): `connecting.js` falls back to query params for inputs
and a SIMULATED probe when `window.__TAURI__` is absent. So @@LaneC can do
the visual acceptance now, ahead of @@LaneB's integrated build.

1. Serve the app-origin assets over loopback (NOT a chan server; this is the
   bare desktop/src dir). PICK A FREE PORT - do NOT use 8799 (another lane's
   `chan-lane` holds 127.0.0.1:8799; the IPv4 listener will shadow yours and
   you will get the chan SPA "access token missing" page instead of this
   one). Verify the port is free first:

       lsof -nP -iTCP:8913 -sTCP:LISTEN   # expect empty
       cd desktop/src && python3 -m http.server 8913 --bind 127.0.0.1 &

2. Drive the states by URL (the query-param fallback exercises the IDENTICAL
   post-input rendering + loop the real `__CHAN_CONNECTING__` path uses):

   - retry/fail forever:  /connecting.html?demo=fail&url=http%3A%2F%2F127.0.0.1%3A4000%2F
   - succeed at attempt 3 (default): /connecting.html?url=http%3A%2F%2F127.0.0.1%3A4000%2F
   - succeed immediately: /connecting.html?demo=ok&url=http%3A%2F%2F127.0.0.1%3A4000%2F
   - dark vs light: in the page console run
       localStorage.setItem('chanDesktopTheme','light')  (or 'dark') then reload.

3. Acceptance to confirm (maps to criteria 1-4):
   - the connecting surface paints IMMEDIATELY (never blank white);
   - spinner + "Connecting to workspace" + the {url} line;
   - "Trying for MM:SS . attempt N" updates live (1s tick);
   - ONE timestamped row per attempt in the log; fails are red and KEEP
     appending (~2s apart); retries never give up;
   - `?demo=ok` -> green check + "attempt 1: connected (HTTP 200)" +
     "Opening workspace..." (standalone INTENTIONALLY skips the real
     `location.replace(target)`; under Tauri that fires).

   NOTE: the no-URL HARD-ERROR state ("Cannot connect", red static ring) only
   triggers when Tauri IS present but no url is injected; standalone with no
   url auto-defaults to a demo url so the page still animates. So the
   hard-error path is a stage-2 (integrated) check, not stage-1.

4. Tear down (lane boundary): kill the http.server and close the Chrome tab
   when done (`pkill -f "http.server 8913"`).

Stage-2 (after @@LaneB lands probe_url + redirection) is the real
unreachable-vs-reachable desktop run, which stays @@LaneC's call.
