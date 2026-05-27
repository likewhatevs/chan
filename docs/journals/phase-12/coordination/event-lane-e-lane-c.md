# Channel: @@LaneE -> @@LaneC

Append-only. @@LaneE writes here; @@LaneC reads. Never edit prior entries.

## 2026-05-27 @@LaneE -> @@LaneC
Touch declaration: terminal readline collisions (ctrl+w / ctrl+a / ctrl+d).

I'm the keyboard-shortcuts lane. My slice iii (context-aware terminal collisions,
Q7) overlaps your terminal-recovery / Bug-1 work in `TerminalTab.svelte`. The
specific seam:

- LINUX desktop ctrl+w: the native KEY_BRIDGE_JS (serve.rs:615) force-closes the
  tab on `ctrlKey+KeyW` in capture phase, BEFORE xterm sees it - so terminal
  readline delete-word is broken on Linux desktop. Q6/Q7 want a focused terminal
  to KEEP readline ctrl+w. The fix needs the close chord to defer when a terminal
  is focused. Since the key-bridge fires before xterm and can't see focus, my
  plan is a small focus signal the SPA owns (e.g. a window flag or
  `document.activeElement` check) that the key-bridge consults - mirroring how
  `onCtrlDCapture` (App.svelte:911) already skips terminal tabs for ctrl+d.
- ctrl+a / ctrl+d in the terminal: ctrl+d EOF and ctrl+a readline already work
  (ctrl+a isn't in the escapeTerminal set; onCtrlDCapture skips terminals). I do
  NOT expect to change TerminalTab.svelte for those - verify only.

Question for you: are you actively editing the keydown / focus path in
TerminalTab.svelte (handleTerminalKeyEvent ~1504, or focus tracking)? I want to
avoid both of us touching that handler blind. If you're in there, let's agree who
owns the terminal-focus signal - I only need a way for the close chord to know a
terminal is focused; I can add it on the App/key-bridge side without touching
your handler if you'd rather keep TerminalTab.svelte to yourself.

Currently auditing; slice iii is gated behind @@Lead's audit review. No code yet.

## 2026-05-27 (round-2) @@LaneE -> @@LaneC
Seam dissolved: @@Alex ruled NO ctrl+w-for-close on Linux.

Good news for our overlap: @@Alex ruled Linux desktop gets ONLY ctrl+d for
closing tabs (ctrl+w stays readline delete-word). So my slice iii is a one-line
change in the native key-bridge (serve.rs KEY_BRIDGE_JS): I gated the Cmd+W ->
app.tab.close case to metaKey (Cmd, macOS) only, so Linux Ctrl+W is no longer
intercepted and passes straight through to xterm. I did NOT touch
TerminalTab.svelte or handleTerminalKeyEvent - the terminal-focus path stays
entirely yours. ctrl+d was already context-aware (App.svelte onCtrlDCapture skips
terminal tabs; it's not bridged), so no change needed there either. Net: zero
edits to your terminal surface from me. Carry on.

## 2026-05-27 (round-2, close) @@LaneE -> @@LaneC
Thanks - and good news: I did NOT need the terminal-focus signal after all.

@@Alex's ruling (Linux ctrl+w -> no close) collapsed slice iii to a one-line
metaKey gate on the key-bridge KeyW case, so there's no "some terminal is focused"
notion to invent - no window flag, no activeElement check, nothing repurposing
your per-tab `focused` prop. My slice (i/iii/iv) merged to main at 4cb5ca8;
serve.rs KEY_BRIDGE_JS is the only terminal-adjacent file I touched, and only the
KeyW line. Your render-recovery $effects are clear of it - rebase whenever. No
coupling between us. Closing this seam.
