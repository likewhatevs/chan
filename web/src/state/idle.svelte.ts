// Window-level idle tracker. After IDLE_MS without a "user is doing
// something" event, `idle.active` flips to true and the floating pills
// (BottomPill, MobileFloatBar, the editor's top formatting bar) fade
// out so they don't sit on top of content while the user is reading.
// Any of the watched events flips it back to false and restarts the
// timer.
//
// Reset triggers: mousedown, click, wheel, scroll, keydown, touchstart,
// touchmove. Pointer hover (`mousemove`) is intentionally NOT a reset
// trigger: a user reading should be able to leave their cursor still
// and have the pills fade. The user reactivates by scrolling or
// tapping anywhere; both produce events on this list.

const IDLE_MS = 2500;

export const idle = $state<{ active: boolean }>({ active: false });

let idleTimer: ReturnType<typeof setTimeout> | null = null;

function arm(): void {
  if (idleTimer) clearTimeout(idleTimer);
  idleTimer = setTimeout(() => {
    idle.active = true;
  }, IDLE_MS);
}

function onActivity(): void {
  if (idle.active) idle.active = false;
  arm();
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
