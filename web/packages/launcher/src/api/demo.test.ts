import { describe, expect, it } from "vitest";
import { createLauncherDemoApi } from "./demo";

describe("launcher demo api", () => {
  it("seeds connected remotes plus an attention control row", async () => {
    const api = createLauncherDemoApi();

    const [workspaces, devservers, windows] = await Promise.all([
      api.listWorkspaces(),
      api.listDevservers(),
      api.listWindows(),
    ]);

    expect(workspaces.filter((w) => w.devserver_id === null).length).toBeGreaterThanOrEqual(2);
    expect(devservers.filter((d) => d.status === "connected").length).toBeGreaterThanOrEqual(1);
    // The attention remote is the disconnected one whose dead control row
    // stays mounted and visible so it can flash.
    expect(devservers.find((d) => d.id === api.attentionDevserverId)?.status).toBe("disconnected");
    const attentionRow = windows.find(
      (w) => w.control && w.window_id === `control-terminal-${api.attentionDevserverId}`,
    );
    expect(attentionRow?.hidden).toBe(false);
    // lima-vm connected cleanly with auto-hide on, so its control row is hidden
    // and its two workspaces surface through the connected-devserver merge.
    expect(windows.find((w) => w.control && w.library_id === "lib-lima")?.hidden).toBe(true);
    expect(workspaces.filter((w) => w.devserver_id === "ds-lima")).toHaveLength(2);
    expect(windows.filter((w) => w.kind === "terminal" && w.library_id === "local").length).toBeGreaterThanOrEqual(2);
    expect(workspaces.map((w) => w.path)).not.toContainEqual(expect.stringMatching(/^\\\\\?\\/));
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

describe("launcher demo api: empty variants", () => {
  it.each(["empty", "devserver"] as const)("%s seeds nothing and flags no attention devserver", async (variant) => {
    const api = createLauncherDemoApi({ variant });

    expect(api.attentionDevserverId).toBeNull();
    const [workspaces, devservers, windows] = await Promise.all([
      api.listWorkspaces(),
      api.listDevservers(),
      api.listWindows(),
    ]);
    expect(workspaces).toHaveLength(0);
    expect(devservers).toHaveLength(0);
    expect(windows).toHaveLength(0);
  });

  it("creates terminals and workspaces from empty, and resets back to empty", async () => {
    const api = createLauncherDemoApi({ variant: "empty" });

    await api.createWindow("terminal");
    const picked = await api.pickFolder();
    expect(picked).toBe("/Users/you/dev/your-project");
    await api.addLocalWorkspace(picked!, "");

    expect(await api.listWindows()).toHaveLength(1);
    const workspaces = await api.listWorkspaces();
    expect(workspaces).toHaveLength(1);
    expect(workspaces[0]!.on).toBe(true);

    api.reset();
    expect(await api.listWindows()).toHaveLength(0);
    expect(await api.listWorkspaces()).toHaveLength(0);
  });
});
