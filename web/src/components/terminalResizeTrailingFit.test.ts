import { describe, expect, test } from "vitest";
import terminal from "./TerminalTab.svelte?raw";

// `fullstack-a-93`: terminal columns don't widen after a
// pane/window resize (bug-list root cause: FIRST resize
// transition's ResizeObserver fire is missed/swallowed, so
// the terminal stays at the size from the leading-edge fit).
// Palliative-first fix: add a trailing-edge debounced fit that
// converges on the steady-state size 120ms after the last
// observed change. Leading-edge rAF stays for snappy initial
// response.

describe("fullstack-a-93: trailing-edge fit converges after resize", () => {
  test("queueFit schedules both the leading rAF fit AND the trailing fit", () => {
    expect(terminal).toMatch(
      /function queueFit\(\): void \{[\s\S]*?requestAnimationFrame\(\(\) => \{[\s\S]*?fit\?\.fit\(\)[\s\S]*?scheduleTrailingFit\(\);/,
    );
  });

  test("scheduleTrailingFit debounces via setTimeout(...120) coalescing on the trailing edge", () => {
    expect(terminal).toMatch(
      /function scheduleTrailingFit\(\): void \{[\s\S]*?if \(trailingFitTimer\) clearTimeout\(trailingFitTimer\);[\s\S]*?trailingFitTimer = setTimeout\(\(\) => \{[\s\S]*?fit\?\.fit\(\)[\s\S]*?\}, 120\);/,
    );
  });

  test("trailingFitTimer state declared at module scope of the component", () => {
    expect(terminal).toMatch(
      /let trailingFitTimer: ReturnType<typeof setTimeout> \| null = null;/,
    );
  });

  test("teardown clears the trailing-fit timer (no race against dispose)", () => {
    expect(terminal).toMatch(
      /function teardown\(\): void \{[\s\S]*?if \(trailingFitTimer\) \{[\s\S]*?clearTimeout\(trailingFitTimer\);[\s\S]*?trailingFitTimer = null;/,
    );
  });

  test("ResizeObserver still wired to queueFit (leading-edge path preserved)", () => {
    expect(terminal).toMatch(
      /resizeObserver = new ResizeObserver\(queueFit\);[\s\S]*?resizeObserver\.observe\(host\);/,
    );
  });

  test("rationale comment explains the leading vs trailing split + idempotence", () => {
    expect(terminal).toMatch(/fullstack-a-93/);
    expect(terminal).toMatch(/trailing-edge fit/i);
    expect(terminal).toMatch(/Idempotent[\s\S]{1,80}size hasn't drifted/i);
  });
});

describe("fullstack-a-93: PTY resize propagation preserved", () => {
  test("xterm onResize still sends `{ type: 'resize', cols, rows }` to chan-server", () => {
    expect(terminal).toMatch(
      /term\.onResize\(\(\{ cols, rows \}\) => send\(\{ type: "resize", cols, rows \}\)\);/,
    );
  });

  test("WebSocket open handler still sends the initial resize frame", () => {
    expect(terminal).toMatch(
      /if \(term\) send\(\{ type: "resize", cols: term\.cols, rows: term\.rows \}\);/,
    );
  });
});
