/// Wall-clock-gap sleep/wake detector.
///
/// Browsers throttle background tabs and, on macOS, WKWebView does not fire
/// focus / pageshow / visibilitychange across a display or system sleep -- the
/// window stays "visible" and focused through the sleep. So the only portable
/// wake signal is the wall clock itself: a coarse interval whose callback fires
/// far later than scheduled means JS timers were frozen (the machine slept) and
/// this tick is running late on wake. Both the terminal renderer recovery and
/// the /ws watcher redial key off it.
///
/// Pure and side-effect-light: it owns one interval and calls back on a jump.
/// The clock and timer are injectable so a test can drive a frozen-then-jumped
/// wall clock without real time passing.

export interface WakeGapOptions {
  /// How often to sample the wall clock. A sleep is only ever noticed on the
  /// first tick after wake, so this bounds the post-wake detection latency.
  probeMs?: number;
  /// A gap beyond this (several missed ticks) is a wake. Set it well above
  /// `probeMs` so ordinary timer jitter and a single dropped frame do not trip
  /// it.
  gapMs?: number;
  /// Wall-clock source. Defaults to `Date.now`; injectable for tests.
  now?: () => number;
}

/// The default probe cadence and wake threshold, shared by every caller so the
/// terminal recovery and the watcher redial react to the same sleep on the same
/// tick.
export const WAKE_PROBE_MS = 2000;
export const WAKE_GAP_MS = 6000;

/// Install a wake-gap detector. Calls `onWake()` once per detected wall-clock
/// jump (a gap greater than `gapMs` between two probes). Returns a disposer that
/// stops the probe; call it on unmount.
export function installWakeGapDetector(
  onWake: () => void,
  options: WakeGapOptions = {},
): () => void {
  const probeMs = options.probeMs ?? WAKE_PROBE_MS;
  const gapMs = options.gapMs ?? WAKE_GAP_MS;
  const now = options.now ?? (() => Date.now());

  let last = now();
  const timer = setInterval(() => {
    const t = now();
    const gap = t - last;
    last = t;
    if (gap > gapMs) onWake();
  }, probeMs);

  return () => clearInterval(timer);
}
