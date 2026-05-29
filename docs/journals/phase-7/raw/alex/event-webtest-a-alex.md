# event-webtest-a-alex.md

From: @@WebtestA
To: @@Alex
Date: 2026-05-18

## 2026-05-18 11:34 BST - permission

Need approval for a setup and walkthrough batch: create and seed
`/tmp/chan-webtest-a-1/`, run `cargo build -p chan`, launch
`./target/debug/chan serve /tmp/chan-webtest-a-1/`, and launch
browser automation against the bearer-token URL. Expected duration:
30-45 minutes. See [../webtest-a/webtest-a-1.md](../webtest-a/webtest-a-1.md).

## 2026-05-18 12:15 BST - approved (transcribed by @@Architect)

@@Alex approved this permission batch verbally in chat. Proceed
with the full setup + walkthrough as scoped (drive create + seed,
`cargo build -p chan`, `chan serve`, browser automation).

Scope: the approval covers the named commands and the
walkthrough window above; further escalations (new commands,
significantly extended duration, new external surfaces) need a
fresh permission event.

— @@Architect, on behalf of @@Alex

## 2026-05-18 15:25 BST - permission

`webtest-a-3` scope was expanded to cover three external-link
scenarios (browser-served / Chan.app desktop / tunnel-loop).
Scenarios 2 + 3 need authorisations beyond the original grant:

1. Launching Chan.app desktop (the Tauri shell, PID 28810 is
   already running but I haven't been driving it).
2. Starting a second `chan serve --tunnel-url
   http://localhost:8801` process to fake the tunnel hop.
3. Driving the Tauri webview via the Chrome MCP if reachable;
   otherwise asking @@Alex to click-through the link manually
   and report what happened.

I'd like to do this as a survey reply (Round 2 protocol):

1. Approve scenarios 2 + 3 with me running everything.
2. Approve scenarios 2 + 3 but @@Alex performs the actual
   Chan.app click and reports.
3. Skip scenarios 2 + 3 - browser-only verdict is good enough
   for the round 1 commit on fullstack-2; revisit after
   @@FullStack lands the Tauri-aware dispatch.
4. Wait - @@FullStack's revised impl isn't landed yet; ping
   back when it is.

Reasonable default if no reply: option 4 (wait). Will keep
the 8801 server alive in the meantime.
