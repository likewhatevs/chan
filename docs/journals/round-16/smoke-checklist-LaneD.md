# @@Alex smoke checklist: chan-desktop launcher redesign (MODAL path)

From: @@LaneD (verify-lane)  For: @@Alex  Via: @@LaneA
Source of truth: new-team-1/desktop-redesign-design.md (LOCKED: D2 MODAL,
D3 connection dot, D4 no tagline, D1 keep add-time toggles).

## Why a human drives this

chan-desktop renders in WKWebView (macOS). Chrome MCP is Blink and
cannot reach it, so this is a hand-driven click-through. @@LaneD ran the
full static gate (fmt / clippy / build / test) + built the app; the
boxes below are the runtime behaviors only a real launch can confirm.

Built app (unsigned, local):
  target/release/bundle/macos/Chan.app
Launch: open that .app (or `open target/release/bundle/macos/Chan.app`).
First launch may need right-click -> Open (unsigned local build).

## 1. Header

[ ] Header shows enso + "Workspaces" + a single [New] button + theme
    toggle. NOTHING else (no "Open workspace", no "Attach").
[ ] NO italic tagline ("what are we working on today?") anywhere. (D4)
[ ] Theme toggle flips light <-> dark; the modal (opened below) matches
    the current theme.

@@Alex: I want to swap the position of the NEW and SUN/MOON ICON: [ICON] [NEW]

## 2. [New] opens the modal (D2: in-launcher overlay, not a window)

[ ] Click [New] -> an overlay modal opens; the workspace list behind it
    is dimmed by a backdrop (it does NOT open a separate OS window).
[ ] Title reads "New workspace".
[ ] A segmented switch at the top has three choices:
    [ Local directory | Remote outbound | Remote inbound ].
[ ] Clicking each segment swaps the body + the footer button; the active
    segment is highlighted. Default selected = Local.

## 3. Local directory

[ ] "Choose folder..." opens the native folder picker. Cancel -> stays
    on the Local body (modal stays open).
[ ] After picking a folder, the body shows IN PLACE:
    - the chosen path (monospace),
    - a scan report (files / markdown / size / media / source-control;
      plus an "already registered" or "read-only" warning if applicable),
    - two feature toggles: "Semantic search" and "Reports". (D1)
[ ] Both toggles default OFF (unchecked).
[ ] [ Open ] registers the workspace -> the modal CLOSES -> a new row
    appears in the list with the home/computer (local) icon in Where.
[ ] (Optional) [ Back ] returns to the Choose-folder step.

## 4. Remote outbound ("we connect to a URL")

[ ] Body shows a URL field + a Name field + intro copy.
[ ] Enter a URL + Name, [ Attach URL ] -> modal closes -> a new row
    appears with the OUTBOUND direction icon in Where.
[ ] Pressing Enter in either field submits (same as Attach URL).
[ ] Empty/invalid URL is rejected (no row added, error surfaced).

@@Alex: we need to include an example of "run `chan serve ./path/to/repo` and paste the URL here, or run chan remotely via ssh, e.g. `ssh user@host -L 8787:localhost:8787 chan serve ./path/to/repo`

When we show these commands, use a proper code block.

## 5. Remote inbound ("we listen for an incoming connection")

[ ] NOT listening: body shows Port (placeholder auto) + Label +
    Workspace fields + helper line + [ Start listening ].
[ ] [ Start listening ] -> body switches IN PLACE to "Listening on
    127.0.0.1:<port>" with a snippet block.
[ ] A Local | Tunnel segmented toggle switches WHICH snippet shows
    (Tunnel adds the `ssh -R` line; Local shows the chan serve command).
[ ] Click-to-copy on a snippet copies it (paste elsewhere to confirm).
[ ] [ Stop ] returns to the form (listener stopped).
[ ] Start again, then [ Done ] -> modal closes. The listener KEEPS
    running: reopen [New] -> Remote inbound -> it still shows "Listening
    on ...:<same port>" (NOT back to the form).
[ ] CRITICAL: with a live listener, press ESC to dismiss the modal ->
    the modal closes but the listener KEEPS running (reopen confirms).
    ESC must NOT stop the listener.

@@Alex: about the text:

> Bind a loopback port to accept an incoming `chan serve --tunnel-url` from another machine over an SSH reverse forward (we listen).

Becomes:

Listen for incoming connections on a configurable port, or use 0 to let the OS pick one. Then connect to it:
```
chan serve ./path/to/repo --tunnel-url={chan-desktop-listener}
```


## 6. Row redesign (On | Where)

[ ] Table header columns read "On" and "Where".
[ ] Remote (outbound URL) rows show a connection DOT in the On cell:
    green when present/connected, grey otherwise. (D3)
[ ] Remote rows show NO url/tunnel TEXT tag next to the name; the
    inbound vs outbound direction is conveyed by the Where-column icon.
[ ] NO per-row gear (settings cog) on any row. (gear removed; both its
    settings live in the SPA per @@LaneD's gap finding)
[ ] Local row: clicking the Where cell reveals the folder in Finder.
[ ] Open split-button still works: Open, the caret -> Open in Browser,
    and Forget Workspace all function.

## 7. Modal dismissal

[ ] ESC closes the modal. (does NOT stop a live inbound listener; see 5)
[ ] Clicking the backdrop (outside the dialog) closes the modal.
[ ] The [X] close control closes the modal.

## 8. Empty state + first run

[ ] With zero workspaces, the empty-state primary button ("New
    workspace") opens the modal on the Local choice.
[ ] First run (no workspaces registered yet) auto-opens the modal on
    Local rather than the old native picker.

## 9. Out of launcher scope (verify-only, SPA, optional)

[ ] ESC-on-Cmd+P was already fixed (commit 6100ec84). This is the chan
    SPA's quick-open, NOT the launcher. No launcher code touches it.
    Confirm only if convenient: in an open workspace, Cmd+P then ESC
    closes quick-open without side effects.

---
If anything above fails, report the step number to @@LaneA. @@LaneD
cannot drive WKWebView, so this hand-smoke is the last gate before the
redesign is called done.
