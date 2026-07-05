// @vitest-environment jsdom

import { mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

// Isolate from the real catalog registrations (install pulls every lane's
// action module); register a small known set instead.
vi.mock("../state/commands/install", () => ({}));

import CommandLauncher from "./CommandLauncher.svelte";
import appRaw from "../App.svelte?raw";
import launcherRaw from "./CommandLauncher.svelte?raw";
import overlayShellRaw from "./OverlayShell.svelte?raw";
import { launcherPanel } from "../state/store.svelte";
import { registerCommands } from "../state/commands";
import { layout, type BrowserTab, type LeafNode } from "../state/tabs.svelte";

// jsdom does not implement scrollIntoView; the launcher calls it on the
// highlighted row.
Element.prototype.scrollIntoView = vi.fn();

const runSearch = vi.fn();
const runNewFile = vi.fn();
const runBrowserAlpha = vi.fn();
const runBrowserZoom = vi.fn();
const runHidden = vi.fn();

// Register once. allCommands de-dups by (id, category, title), so a
// re-register under a re-run collapses rather than stacking.
registerCommands([
  {
    // Reuses a kept, chorded SHORTCUTS id (app.window.reload) so chordFor
    // resolves a real chord for the "shows the current chord" test. The
    // displayed title is arbitrary for the fixture; the id only feeds the
    // chord lookup (the launcher dispatches run(), not the id).
    id: "app.window.reload",
    title: "Search",
    category: "Global",
    keywords: ["find"],
    available: () => true,
    run: runSearch,
  },
  {
    id: "app.file.new",
    title: "New file",
    category: "Editor",
    keywords: ["create"],
    available: () => true,
    run: runNewFile,
  },
  {
    id: "app.browser.zoom",
    title: "Zoom browser",
    category: "File Browser",
    keywords: ["files"],
    available: () => true,
    run: runBrowserZoom,
  },
  {
    id: "app.browser.alpha",
    title: "Alpha browser",
    category: "File Browser",
    keywords: ["files"],
    available: () => true,
    run: runBrowserAlpha,
  },
  {
    id: "app.hidden.one",
    title: "Hidden command",
    category: "Global",
    available: () => false,
    run: runHidden,
  },
]);

const mounted: Array<Record<string, unknown>> = [];

// Let the open effect's `tick().then(focus)` and any queued microtasks run.
async function flush(): Promise<void> {
  await tick();
  await tick();
}

async function typeLauncherQuery(query = "r"): Promise<void> {
  launcherPanel.query = query;
  await tick();
}

function openLauncher(): HTMLElement {
  const target = document.createElement("div");
  document.body.append(target);
  mounted.push(mount(CommandLauncher, { target }) as Record<string, unknown>);
  launcherPanel.open = true;
  return target;
}

function rowTitles(target: HTMLElement): (string | null)[] {
  return [...target.querySelectorAll(".row .title")].map((e) => e.textContent);
}

function groupLabels(target: HTMLElement): (string | null)[] {
  return [...target.querySelectorAll(".group-label")].map((e) => e.textContent);
}

function rowTitlesInGroup(target: HTMLElement, label: string): (string | null)[] {
  const group = [...target.querySelectorAll(".group")].find(
    (g) => g.querySelector(".group-label")?.textContent === label,
  );
  return group ? rowTitles(group as HTMLElement) : [];
}

function resetLayout(): LeafNode {
  const pane: LeafNode = {
    kind: "leaf",
    id: "command-launcher-pane",
    tabs: [],
    activeTabId: null,
  };
  layout.rootId = pane.id;
  layout.activePaneId = pane.id;
  layout.nodes = { [pane.id]: pane };
  layout.focusColor = "blue";
  return pane;
}

function setActiveBrowserTab(): void {
  const pane = resetLayout();
  const tab: BrowserTab = {
    kind: "browser",
    id: "browser-test",
    title: "Files",
    inspectorOpen: false,
  };
  pane.tabs = [tab];
  pane.activeTabId = tab.id;
}

beforeEach(() => {
  resetLayout();
  launcherPanel.open = false;
  launcherPanel.query = "";
});

afterEach(() => {
  for (const c of mounted.splice(0)) unmount(c);
  document.body.innerHTML = "";
  launcherPanel.open = false;
  launcherPanel.query = "";
  vi.clearAllMocks();
});

describe("command launcher overlay", () => {
  test("command launcher chords route through the app keymap", () => {
    expect(appRaw).toMatch(
      /isTauriDesktop\(\) && currentOS\(\) === "mac"[\s\S]{1,220}e\.metaKey[\s\S]{1,220}e\.ctrlKey && !e\.metaKey && e\.altKey[\s\S]{1,240}toggleCommandLauncher\(\);/,
    );
  });

  test("launcher backdrop is a plain dark scrim with no blur while launcher chrome stays opaque", () => {
    expect(launcherRaw).toMatch(
      /\.search-row \{[\s\S]{1,240}background: color-mix\(in srgb, var\(--bg-elev\) 86%, transparent\);/,
    );
    expect(launcherRaw).toMatch(
      /\.results \{[\s\S]{1,240}background: color-mix\(in srgb, var\(--bg-card\) 82%, var\(--bg-elev\) 18%\);/,
    );
    expect(overlayShellRaw).toMatch(
      /\.overlay\.top,[\s\S]{1,80}\.overlay\.center \{[\s\S]{1,160}background: rgba\(0, 0, 0, 0\.4\);/,
    );
    expect(overlayShellRaw).not.toMatch(/blur\(10px\)/);
    expect(overlayShellRaw).toMatch(
      /\.overlay\.top \.panel,[\s\S]{1,80}\.overlay\.center \.panel \{[\s\S]{1,320}background: color-mix\(in srgb, var\(--bg-elev\) 82%, transparent\);[\s\S]{1,240}backdrop-filter: blur\(24px\) saturate\(1\.12\);/,
    );
    expect(overlayShellRaw).toMatch(
      /@keyframes spotlight-pop \{[\s\S]{1,240}filter: blur\(14px\);[\s\S]{1,240}filter: blur\(0\);/,
    );
  });

  test("renders nothing while closed", async () => {
    const target = document.createElement("div");
    document.body.append(target);
    mounted.push(mount(CommandLauncher, { target }) as Record<string, unknown>);
    await flush();
    expect(target.querySelector(".launcher")).toBeNull();
  });

  test("opens focused with only the search box, then lists available commands", async () => {
    const target = openLauncher();
    await flush();
    const input = target.querySelector("input.search") as HTMLInputElement;
    expect(input).not.toBeNull();
    expect(document.activeElement).toBe(input);
    expect(input.getAttribute("aria-expanded")).toBe("false");
    expect(rowTitles(target)).toEqual([]);

    await typeLauncherQuery();
    expect(input.getAttribute("aria-expanded")).toBe("true");
    const titles = rowTitles(target);
    expect(titles).toContain("Search");
    expect(titles).toContain("New file");
    expect(titles).toContain("Alpha browser");
    expect(titles).not.toContain("Hidden command");
  });

  test("a non-matching query still shows the full catalog, sorted", async () => {
    const target = openLauncher();
    await flush();
    // No command matches "zzz", so there is no Results section and the whole
    // available catalog stays visible for discovery, category-sorted.
    await typeLauncherQuery("zzz");
    expect(groupLabels(target)).toEqual(["Editor", "File Browser", "Global"]);
    expect(rowTitlesInGroup(target, "File Browser")).toEqual([
      "Alpha browser",
      "Zoom browser",
    ]);
  });

  test("pins the active tab category before the alphabetical sections", async () => {
    setActiveBrowserTab();
    const target = openLauncher();
    await flush();
    await typeLauncherQuery("zzz");
    expect(groupLabels(target)).toEqual(["File Browser", "Editor", "Global"]);
  });

  test("keeps the active surface pinned below the query matches", async () => {
    setActiveBrowserTab();
    const target = openLauncher();
    await flush();
    await typeLauncherQuery("new");
    // Matches lead under "Results"; the active surface (File Browser) is
    // pinned ahead of the other discovery sections even though none of its
    // commands matched the query.
    expect(groupLabels(target)).toEqual(["Results", "File Browser", "Global"]);
    expect(rowTitlesInGroup(target, "Results")).toEqual(["New file"]);
  });

  test("shows the current chord next to a chorded command", async () => {
    const target = openLauncher();
    await flush();
    await typeLauncherQuery("search");
    const searchRow = [...target.querySelectorAll(".row")].find(
      (r) => r.querySelector(".title")?.textContent === "Search",
    ) as HTMLElement;
    // The chord renders inside the assign affordance; a chorded command shows
    // its resolved chord (not the "Assign" prompt).
    const chord = searchRow.querySelector(".chord-btn");
    expect(chord).not.toBeNull();
    const text = chord?.textContent?.trim() ?? "";
    expect(text.length).toBeGreaterThan(0);
    expect(text).not.toBe("Assign");
  });

  test("a query promotes matches to a Results section but hides nothing", async () => {
    const target = openLauncher();
    await flush();
    launcherPanel.query = "new";
    await tick();
    // "New file" matches and leads the list under a "Results" section...
    expect(groupLabels(target)[0]).toBe("Results");
    expect(rowTitlesInGroup(target, "Results")).toEqual(["New file"]);
    expect(rowTitles(target)[0]).toBe("New file");
    // ...but every other available command stays discoverable below, in its
    // own category rather than dropped.
    expect(rowTitlesInGroup(target, "File Browser")).toEqual([
      "Alpha browser",
      "Zoom browser",
    ]);
    expect(rowTitlesInGroup(target, "Global")).toEqual(["Search"]);
    expect(rowTitles(target)).not.toContain("Hidden command");
  });

  test("clearing the query closes results and returns to the centered state", async () => {
    const target = openLauncher();
    await flush();
    const input = target.querySelector("input.search") as HTMLInputElement;

    await typeLauncherQuery("new");
    expect(target.querySelector(".results")).not.toBeNull();
    expect(target.querySelector(".panel.lifted")).not.toBeNull();
    expect(input.getAttribute("aria-expanded")).toBe("true");

    await typeLauncherQuery("");
    expect(target.querySelector(".results")).toBeNull();
    expect(target.querySelector(".panel.lifted")).toBeNull();
    expect(input.getAttribute("aria-expanded")).toBe("false");
  });

  test("arrows move the highlight and aria-activedescendant", async () => {
    const target = openLauncher();
    await flush();
    await typeLauncherQuery();
    const input = target.querySelector("input.search") as HTMLInputElement;
    const launcher = target.querySelector(".launcher") as HTMLElement;
    const rows = () => [...target.querySelectorAll(".row")];
    expect(rows()[0].getAttribute("aria-selected")).toBe("true");
    expect(input.getAttribute("aria-activedescendant")).toBe(rows()[0].id);

    launcher.dispatchEvent(
      new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }),
    );
    await tick();
    expect(rows()[0].getAttribute("aria-selected")).toBe("false");
    expect(rows()[1].getAttribute("aria-selected")).toBe("true");
    expect(input.getAttribute("aria-activedescendant")).toBe(rows()[1].id);
  });

  test("Enter runs the highlighted command and closes", async () => {
    const target = openLauncher();
    await flush();
    await typeLauncherQuery("new");
    const launcher = target.querySelector(".launcher") as HTMLElement;
    launcher.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
    );
    await tick();
    expect(runNewFile).toHaveBeenCalledTimes(1);
    expect(runSearch).not.toHaveBeenCalled();
    expect(launcherPanel.open).toBe(false);
  });

  test("clicking a row runs it and closes", async () => {
    const target = openLauncher();
    await flush();
    await typeLauncherQuery("new");
    const newFileRow = [...target.querySelectorAll(".row")].find(
      (r) => r.querySelector(".title")?.textContent === "New file",
    ) as HTMLElement;
    newFileRow.click();
    await tick();
    expect(runNewFile).toHaveBeenCalledTimes(1);
    expect(launcherPanel.open).toBe(false);
  });
});
