import { describe, it, expect } from "vitest";
import { mockApi } from "./mock";

describe("createWindow — terminal mint", () => {
  it("mints a local terminal window and adds it to the feed", async () => {
    const before = await mockApi.listWindows();
    const rec = await mockApi.createWindow("terminal");
    expect(rec.kind).toBe("terminal");
    expect(rec.library_id).toBe("local");
    expect(rec.workspace_path).toBeNull();
    expect(rec.title).toMatch(/Terminal Window \d+/);
    const after = await mockApi.listWindows();
    expect(after.length).toBe(before.length + 1);
    expect(after.some((w) => w.window_id === rec.window_id)).toBe(true);
  });

  it("notifies watch subscribers when a window is minted", async () => {
    const counts: number[] = [];
    const unsub = mockApi.watchWindows((s) => counts.push(s.windows.length));
    const seen = counts.length; // the on-connect snapshot
    await mockApi.createWindow("terminal");
    expect(counts.length).toBeGreaterThan(seen);
    unsub();
  });
});
