# Release v0.70.0 - first-class gateways in chan-desktop

The v0.70.0 feature wave makes gateways first-class in chan-desktop: add a gateway by URL, sign in once for your account, and every devserver you own or that is shared with you appears in the launcher live. Coordination artifacts live in the untracked `dev/v0.70.0/` tree of the round host's checkout; this report is the consolidated front door.

## What shipped

- **First-class gateways in chan-desktop.** The Computers title flips to a new Gateways screen: add a gateway by URL, Connect to sign in once for your account, and the gateway's devservers (yours and the ones shared with you) surface under Computers automatically, appearing, disappearing, and flipping online state within seconds. Rosters poll every 10s with ETag, so a quiet gateway costs almost nothing. Rows read "via <gateway>"; connect and disconnect work per row exactly like plain devservers, and bulk select covers gateways too (deleted last, after their rows). The old flow (a gateway URL pasted into the Add devserver form, picking one devserver at sign-in) is gone; existing picked rows migrate into gateway entries at first startup.
- **Launcher notification bubbles.** Corner bubbles styled after the workspace notices replace the launcher error banner: each names its source (gateway, devserver, or desktop), expands on click, and dismisses. Gateway life-cycle events narrate there.
- **`chan open <gateway-url>` registers the gateway** against a running desktop instead of failing as a devserver dial; plain devserver URLs behave as before, and old CLIs are unaffected.

## Changed

- **One sign-in per gateway account.** The consent page authorizes your whole account with no per-devserver pick. Existing sign-ins keep working for already-connected rows but cannot list the account roster, so the first gateway Connect after upgrading asks you to sign in once more.
- **Full command vocabulary on self-hosted gateways.** Windows served from any gateway's proxy origin get the same IPC grants as `*.devserver.chan.app` windows (upload/download, clipboard, zoom chords, open-in-browser): the desktop mints a runtime capability scoped to that gateway's proxy wildcard at first connect, live on already-open windows. One Tauri-design caveat: a removed gateway's grant persists until the app exits.
- **New terminal from a standalone terminal window** via the pane menu (Cmd+T), matching the workspace window.

## Fixed

- **Terminal tabs on gateway-backed devservers no longer go dead after idle.** Two layers conspired: the gateway WebSocket bridge cut any connection quiet in one direction for 300s (a terminal streaming output still died 300s after the last keystroke), and the terminal socket was the only one with neither a heartbeat nor a reconnect. The terminal socket now heartbeats (20s ping, 45s read-deadline) and reconnects with capped backoff into the same session (scrollback preserved, no reload); the bridge cuts only when both directions are idle and always sends a real Close frame. Doc and scene sync sockets gain the same protection.
- **Cmd+Shift+S no longer opens a dead Search overlay in a standalone terminal window**; the chord is inert there, matching every other search entry point.

## Operators

New PAT scope `desktop.account` and roster endpoint `GET /desktop/v1/devservers` (Bearer PAT, ETag/304, degraded-safe); discovery advertises `roster_url` (additive, `api_version` stays 1). The devserver-proxy bridged-WebSocket idle cut is now both-directions-idle (default 300s) and announces itself with a WS Close frame. A PAT mint registers a devserver row only when the token carries the `tunnel` scope, so a non-tunnel PAT no longer creates a phantom offline row.
