// @vitest-environment jsdom

import { afterEach, describe, expect, test } from "vitest";
import { sessionPath, sessionWindowId } from "./client";

afterEach(() => {
  window.history.replaceState(null, "", "/");
  window.sessionStorage.clear();
});

describe("sessionWindowId", () => {
  test("uses per-tab sessionStorage without a window id", () => {
    window.history.replaceState(null, "", "/?t=token");

    window.sessionStorage.setItem("chan.session.window", "tab-a1b2c3d4");

    expect(sessionWindowId()).toBe("tab-a1b2c3d4");
    expect(sessionPath()).toBe("/api/session?w=tab-a1b2c3d4");
  });

  test("generates and reuses a per-tab sessionStorage id", () => {
    window.history.replaceState(null, "", "/?t=token");

    const first = sessionWindowId();
    const second = sessionWindowId();

    expect(first).toMatch(/^[0-9a-f]{8}$/);
    expect(second).toBe(first);
  });

  test("uses the chan-desktop window id from the URL", () => {
    window.history.replaceState(null, "", "/?t=token&w=drive-notes-7");

    expect(sessionWindowId()).toBe("drive-notes-7");
    expect(sessionPath()).toBe("/api/session?w=drive-notes-7");
  });

  test("encodes unusual window labels before calling the session API", () => {
    window.history.replaceState(null, "", "/?w=tunnel%20a/drive%201");

    expect(sessionWindowId()).toBe("tunnel a/drive 1");
    expect(sessionPath()).toBe("/api/session?w=tunnel%20a%2Fdrive%201");
  });
});
