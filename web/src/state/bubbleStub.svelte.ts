/// Phase-13 round-2 Team Work revamp: the survey/poke "bubble"
/// overlay no longer carries live watcher data (the agent-event
/// watcher + survey-reply round-trip was deleted with the
/// team-work-workspace plumbing). The overlay is now a
/// frontend-only static EXAMPLE so the bubble layout (stack vs
/// tray) and the survey shapes chan supports stay demonstrable
/// without any network or filesystem traffic.
///
/// This module owns the single piece of reactive visibility the
/// overlay reads. A6's Team Work right-click menu calls
/// `showBubbleStub()` to display the example; clicking anything in
/// the overlay calls `hideBubbleStub()` to dismiss it. Keeping the
/// flag in a module-level rune (rather than per-tab state) matches
/// the example's purpose: it is a global demo surface, not session
/// state, so it is intentionally NOT persisted to SerTab.

let visible = $state(false);

/// Reveal the static bubble example. Wired to the Team Work
/// right-click "Bubble stack" / "Bubble tray" menu entries (which
/// also set the layout preference via `setBubbleOverlayMode`).
export function showBubbleStub(): void {
  visible = true;
}

/// Dismiss the static bubble example. The overlay calls this on any
/// click; there is no reply path, so dismissal is purely local.
export function hideBubbleStub(): void {
  visible = false;
}

/// Reactive accessor the BubbleOverlay reads to decide whether to
/// render the example. A getter (not the raw `$state` holder) so
/// the module keeps a single mutation surface.
export function bubbleStubVisible(): boolean {
  return visible;
}
