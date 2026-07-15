import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";
import { installWakeGapDetector } from "./wakeGap";

// The detector fires onWake only when the wall clock jumps far past the probe
// cadence between two ticks -- the signature of JS timers freezing during a
// machine sleep. The clock is injected so the test can freeze it relative to the
// (fake) interval and reproduce a sleep without real time passing.
describe("installWakeGapDetector", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  test("stays quiet while the clock advances in step with the probe", () => {
    let clock = 0;
    const wake = vi.fn();
    const dispose = installWakeGapDetector(wake, {
      probeMs: 2000,
      gapMs: 6000,
      now: () => clock,
    });
    // Three ordinary ticks: the injected clock moves in lockstep with the timer.
    for (let i = 0; i < 3; i++) {
      clock += 2000;
      vi.advanceTimersByTime(2000);
    }
    expect(wake).not.toHaveBeenCalled();
    dispose();
  });

  test("fires once when the wall clock jumps past the gap (a sleep)", () => {
    let clock = 0;
    const wake = vi.fn();
    const dispose = installWakeGapDetector(wake, {
      probeMs: 2000,
      gapMs: 6000,
      now: () => clock,
    });
    // One normal tick establishes the baseline.
    clock += 2000;
    vi.advanceTimersByTime(2000);
    expect(wake).not.toHaveBeenCalled();
    // Sleep: the wall clock jumps 10 minutes while JS timers were frozen. On
    // wake the coalesced tick fires once and sees the gap.
    clock += 600_000;
    vi.advanceTimersByTime(2000);
    expect(wake).toHaveBeenCalledTimes(1);
    dispose();
  });

  test("the disposer stops the probe", () => {
    let clock = 0;
    const wake = vi.fn();
    const dispose = installWakeGapDetector(wake, {
      probeMs: 2000,
      gapMs: 6000,
      now: () => clock,
    });
    dispose();
    clock += 600_000;
    vi.advanceTimersByTime(2000);
    expect(wake).not.toHaveBeenCalled();
  });
});
