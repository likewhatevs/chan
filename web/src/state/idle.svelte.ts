// Window-level idle tracker. After IDLE_MS without a "user is doing
// something" event, `idle.active` flips to true and the floating pills
// (BottomPill, the editor's top formatting bar) fade out so they
// don't sit on top of content while the user is reading.
// Any of the watched events flips it back to false and restarts the
// timer.
//
// Reset triggers: mousemove, mousedown, click, wheel, scroll,
// touchstart, touchmove. Keyboard input (`keydown`) is intentionally
// NOT a reset trigger: typing or arrow-key caret motion should leave
// the floating pills hidden so the writing surface stays uncluttered.
// Mouse motion IS a reset trigger so the user "reaching for the
// chrome" (any cursor twitch) brings the pill back into view; once
// the mouse stops moving for IDLE_MS the pill fades again.
//
// Boot behavior: idle.active starts false, so the pill is visible
// when the app loads / a new tab opens; the very first arm() starts
// the fade timer, so if the user never moves the mouse the pill
// fades on its own after IDLE_MS.
//
// Pin mechanism: while any consumer holds a pin (typically because the
// mouse is hovering over an accessory bar), `idle.active` stays false
// and the timer is suspended. Each bar's mouseenter / mouseleave calls
// pinAccessory() and the returned release fn so the pill doesn't fade
// from under the user's cursor.

const IDLE_MS_DEFAULT = 5000;
/// Idle window in read-only mode. Half the write-mode default so
/// the floating pills get out of the way faster while the user is
/// reading.
const IDLE_MS_READMODE = 2500;

export const idle = $state<{ active: boolean }>({ active: false });

/// Window-level read-mode flag. True only when *every* visible
/// file tab is read-only (user-toggled or filesystem-locked); a
/// mixed write/read layout keeps this false so the bottom pill
/// stays full-color. Driven by a single $effect in App.svelte
/// that derives the value from layout state, so there's exactly
/// one writer to this signal regardless of pane count.
export const readMode = $state<{ active: boolean }>({ active: false });

let currentIdleMs = IDLE_MS_DEFAULT;
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
  }, currentIdleMs);
}

/// Flip the global read-mode flag. Re-arms the idle timer at the
/// new window so the bottom pill auto-hides faster while reading.
export function setReadMode(active: boolean): void {
  if (readMode.active === active) return;
  readMode.active = active;
  currentIdleMs = active ? IDLE_MS_READMODE : IDLE_MS_DEFAULT;
  // Re-arm so the new window kicks in immediately rather than
  // waiting for the next user activity event.
  arm();
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
    "mousemove",
    "mousedown",
    "click",
    "wheel",
    "scroll",
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
