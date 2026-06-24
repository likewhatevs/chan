import { afterEach, describe, expect, test, vi } from "vitest";
import { createTrailingFitScheduler, runTerminalFit } from "./resize";

describe("terminal resize helpers", () => {
  afterEach(() => vi.useRealTimers());

  test("runs fit and reports the current terminal size", () => {
    const details: string[] = [];
    const fit = vi.fn();
    expect(runTerminalFit({ fit }, { cols: 80, rows: 24 }, (detail) => details.push(detail))).toBe(
      true,
    );
    expect(fit).toHaveBeenCalledTimes(1);
    expect(details).toEqual(["80x24"]);
  });

  test("absorbs fit exceptions while layout settles", () => {
    expect(
      runTerminalFit(
        {
          fit() {
            throw new Error("not measurable");
          },
        },
        { cols: 80, rows: 24 },
        () => {},
      ),
    ).toBe(false);
  });

  test("coalesces trailing-edge fits", () => {
    vi.useFakeTimers();
    const runFit = vi.fn();
    const scheduler = createTrailingFitScheduler(runFit, 120);

    scheduler.schedule();
    vi.advanceTimersByTime(80);
    scheduler.schedule();
    vi.advanceTimersByTime(119);
    expect(runFit).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(runFit).toHaveBeenCalledTimes(1);
  });
});
