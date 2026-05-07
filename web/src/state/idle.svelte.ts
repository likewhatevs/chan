// Window-level idle tracker. After IDLE_MS without a "user is doing
// something" event, `idle.active` flips to true and the floating pills
// (BottomPill, the editor's top formatting bar) fade out so they
// don't sit on top of content while the user is reading.
// Any of the watched events flips it back to false and restarts the
// timer.
//
// Reset triggers: mousedown, click, wheel, scroll, keydown, touchstart,
// touchmove. Pointer hover (`mousemove`) is intentionally NOT a reset
// trigger: a user reading should be able to leave their cursor still
// and have the pills fade. The user reactivates by scrolling or
// tapping anywhere; both produce events on this list.
//
// Pin mechanism: while any consumer holds a pin (typically because the
// mouse is hovering over an accessory bar), `idle.active` stays false
// and the timer is suspended. Each bar's mouseenter / mouseleave calls
// pinAccessory() and the returned release fn so the pill doesn't fade
// from under the user's cursor.

const IDLE_MS = 5000;

export const idle = $state<{ active: boolean }>({ active: false });

let idleTimer: ReturnType<typeof setTimeout> | null = null;
let pinCount = 0;

function arm(): void {
  if (idleTimer) clearTimeout(idleTimer);
  idleTimer = null;
  // Don't run the timer while something's pinned: the consumer
  // (the hovered bar) wants the pill visible until it releases.
  if (pinCount > 0) return;
  idleTimer = setTimeout(() => {
    idle.active = true;
  }, IDLE_MS);
}

function onActivity(): void {
  if (idle.active) idle.active = false;
  arm();
}

/// Hold the accessory pills visible until the returned release
/// function is called. Use this from a bar's mouseenter handler so
/// the pill doesn't fade while the user is pointing at it.
/// Refcounted: nested or overlapping pins all need to release
/// before the idle timer rearms.
export function pinAccessory(): () => void {
  pinCount += 1;
  if (idle.active) idle.active = false;
  if (idleTimer) {
    clearTimeout(idleTimer);
    idleTimer = null;
  }
  let released = false;
  return () => {
    if (released) return;
    released = true;
    pinCount = Math.max(0, pinCount - 1);
    if (pinCount === 0) arm();
  };
}

/// Install once at app startup. Returns a teardown for symmetry, but
/// the listener is intended to live for the entire app lifetime.
export function installIdleTracker(): () => void {
  if (typeof window === "undefined") return () => {};
  const events = [
    "mousedown",
    "click",
    "wheel",
    "scroll",
    "keydown",
    "touchstart",
    "touchmove",
  ] as const;
  for (const ev of events) {
    window.addEventListener(ev, onActivity, { passive: true, capture: true });
  }
  arm();
  return () => {
    for (const ev of events) {
      window.removeEventListener(
        ev,
        onActivity,
        { capture: true } as EventListenerOptions,
      );
    }
    if (idleTimer) {
      clearTimeout(idleTimer);
      idleTimer = null;
    }
  };
}
