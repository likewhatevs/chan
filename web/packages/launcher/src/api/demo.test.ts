import { describe, expect, it } from "vitest";
import { createLauncherDemoApi } from "./demo";

describe("launcher demo api", () => {
  it("seeds a connected local and remote launcher tree", async () => {
    const api = createLauncherDemoApi();

    const [workspaces, devservers, windows] = await Promise.all([
      api.listWorkspaces(),
      api.listDevservers(),
      api.listWindows(),
    ]);

    expect(workspaces.filter((w) => w.devserver_id === null).length).toBeGreaterThanOrEqual(2);
    expect(devservers.filter((d) => d.status === "connected").length).toBeGreaterThanOrEqual(2);
    expect(windows.filter((w) => w.kind === "terminal" && w.library_id === "local").length).toBeGreaterThanOrEqual(2);
    expect(windows.some((w) => w.control)).toBe(true);
  });

  it("adds devservers and resets back to the seed", async () => {
    const api = createLauncherDemoApi();
    const before = await api.listDevservers();

    await api.addDevserver({ host: "demo.example.net", port: 8787, label: "demo" });
    expect(await api.listDevservers()).toHaveLength(before.length + 1);

    api.reset();
    expect(await api.listDevservers()).toHaveLength(before.length);
  });
});
