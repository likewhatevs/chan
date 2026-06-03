# Connecting-screen: 60-second WKWebView hand-smoke (for @@Alex)

@@LaneC verified every layer up to the WKWebView (build, wiring tests, probe
premise both directions, the page in Chrome). The ONE residual is watching the
real outbound window paint the connecting screen. An agent cannot drive it:
the launcher's "Open" button lives in a WKWebView (not Chrome-drivable), there
is no outbound deep-link / CLI / auto-open trigger, and synthetic mouse/key
events are blocked here (AXIsProcessTrusted = false; the agent process has no
macOS Accessibility permission). So this is a human-at-the-keyboard smoke,
consistent with the team norm (WKWebView smokes = @@Alex).

The app is already built: `./target/debug/chan-desktop` (debug, unsigned, runs
locally; built by @@LaneC at 2026-06-03 12:18 from the current working tree
with @@LaneB + @@LaneD changes).

## Case A: dead URL -> connecting screen (the bug fix)

1. Launch:  `./target/debug/chan-desktop`
2. In the launcher click "New workspace" -> "Remote outbound".
3. URL: `http://127.0.0.1:59999/`   Label: `conn-dead-test`   -> "Attach URL".
4. On the `conn-dead-test` row, click "Open".
5. EXPECT (PASS): the window shows the connecting surface IMMEDIATELY, not a
   blank white page:
   - spinner + "Connecting to workspace" + the `http://127.0.0.1:59999/` line;
   - a live "Trying for MM:SS . attempt N" (1s tick);
   - a growing log of RED, timestamped rows "attempt N: could not connect"
     (~2s apart), newest scrolled into view;
   - the window stays usable; rows keep appending until you close it (no
     silent give-up).
6. Close the window.
   FAIL would be: a blank white window (the old bug), no retries, or it stops
   retrying on its own.

## Case B: live URL -> navigates to the workspace

1. In a terminal, start a throwaway live server (copy the printed URL+token):
       ./target/debug/chan serve /tmp/chan-conn-live --port 8921
2. Launcher -> "New workspace" -> "Remote outbound" -> paste the
   `http://127.0.0.1:8921/?t=...` URL -> "Attach URL".
3. Click "Open" on that row.
4. EXPECT (PASS): a brief connecting screen -> a GREEN row "attempt 1:
   connected (HTTP 200)" -> "Opening workspace..." -> the window navigates to
   the live workspace (the editor / terminal UI loads in the same window).

## Cleanup

- Remove both test rows: row caret menu -> "Forget URL".
- Stop the throwaway server (Ctrl-C) and `rm -rf /tmp/chan-conn-live`.

## What @@LaneC already confirmed (so this smoke is the only open item)

- Stage-1 page (Chrome, standalone): spinner, live timer, one timestamped row
  per attempt accruing with no give-up, demo=ok success state, dark + light,
  zero console errors. All green.
- Integrated build: `chan-desktop` debug builds clean (exit 0); full
  `desktop/src-tauri` test suite green (81 + 7).
- Wiring pinned green by @@LaneB's tests:
  `outbound_windows_load_the_connecting_page_not_the_remote` (outbound loads
  connecting.html + the `__CHAN_CONNECTING__` handoff) and
  `invoke_handler_registers_probe_url` (probe_url is in the IPC handler).
- probe_url premise (what its reqwest GET observes), measured live:
  - live `http://127.0.0.1:8921/` -> HTTP 200 => reachable:true => navigate;
  - dead `http://127.0.0.1:59999/` -> connection refused (curl exit 7) =>
    reachable:false => retry. Both branches the screen depends on are correct.
