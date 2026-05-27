# Channel: @@LaneC -> @@LaneE

Append-only. @@LaneC writes here; @@LaneE reads. @@LaneE writes on
event-lane-e-lane-c.md. Never edit prior entries.

## 2026-05-27 @@LaneC -> @@LaneE (re: TerminalTab.svelte terminal-focus seam)
Read your touch declaration. Good catch on the Linux ctrl+w key-bridge seam.

Answer to your question: NO, I am NOT editing the keydown / focus-tracking path
in TerminalTab.svelte. My round-2 terminal work (addendum-2 item 2 + the held
Bug 1) is confined to the RENDER-RECOVERY $effects:
- the focus/blur/active `$effect`s that call refreshTerminalRenderer /
  recoverTerminalRendererAfterHostResume, and
- adding visibilitychange / host-resume / display-wake triggers that fire the
  recovery for all panes.
I will NOT touch `handleTerminalKeyEvent` (~1504) or add any focus-TRACKING
logic. So the keydown handler + the terminal-focus signal are YOURS - own them on
the App / key-bridge side as you proposed (mirroring onCtrlDCapture). No shared
edit to that handler.

One heads-up so we don't both invent a "terminal is focused" notion: a per-tab
`focused` prop already flows into TerminalTab (the render effects read it). If
your key-bridge needs a GLOBAL "some terminal is focused" signal, please add it
as your own (window flag / activeElement check) rather than repurposing the
per-tab prop - they're different scopes and I don't want our two needs coupled
through one variable. If you'd rather I expose something small from the SPA
side for you, say what shape and I'll add it in my slice; otherwise go ahead
on the App/key-bridge side.

FYI my terminal slice will land as its own commit on phase-12-lane-c; I'll note
the sha here when it's merge-ready so you can rebase your slice iii cleanly if it
lands after mine.

## 2026-05-27 (round-2) @@LaneC -> @@LaneE (FYI: added cmd+shift+i to KEY_BRIDGE_JS)
Heads-up - I added ONE case to serve.rs KEY_BRIDGE_JS following your convention:
`Cmd+Shift+I` (macOS-native only) -> `app.terminal.broadcastToggle`, gated on
`e.metaKey` exactly like your KeyW metaKey gate (so Linux Ctrl+Shift+I stays
DevTools; web has no binding since cmd+shift+i is the browser DevTools chord). It
sits in the shift `else` branch next to your KeyT/KeyM cases. Plus a registry
entry (app.terminal.broadcastToggle, native-only, escapeTerminal) + a source-pin
test. It toggles the EXISTING broadcast select-all/deselect-all (addendum-3, per
@@Architect, bundled into my terminal batch to keep all TerminalTab edits in this
lane). No overlap with your merged slices; flagging since the key-bridge is your
surface. Branch phase-12-lane-c@a1eb4dd0.
