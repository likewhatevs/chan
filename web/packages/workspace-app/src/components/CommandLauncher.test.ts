// @vitest-environment jsdom

import { mount, tick, unmount } from "svelte";
import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

// Isolate from the real catalog registrations (install pulls every lane's
// action module); register a small known set instead.
vi.mock("../state/commands/install", () => ({}));

import CommandLauncher from "./CommandLauncher.svelte";
import { launcherPanel } from "../state/store.svelte";
import { registerCommands } from "../state/commands";

// jsdom does not implement scrollIntoView; the launcher calls it on the
// highlighted row.
// @ts-expect-error test stub
Element.prototype.scrollIntoView = vi.fn();

const runSearch = vi.fn();
const runNewFile = vi.fn();
const runHidden = vi.fn();

// Register once. allCommands de-dups by (id, category, title), so a
// re-register under a re-run collapses rather than stacking.
registerCommands([
  {
    id: "app.search.toggle",
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

beforeEach(() => {
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
  test("renders nothing while closed", async () => {
    const target = document.createElement("div");
    document.body.append(target);
    mounted.push(mount(CommandLauncher, { target }) as Record<string, unknown>);
    await flush();
    expect(target.querySelector(".launcher")).toBeNull();
  });

  test("opens focused and lists only available commands", async () => {
    const target = openLauncher();
    await flush();
    const input = target.querySelector("input.search") as HTMLInputElement;
    expect(input).not.toBeNull();
    expect(document.activeElement).toBe(input);
    const titles = rowTitles(target);
    expect(titles).toContain("Search");
    expect(titles).toContain("New file");
    expect(titles).not.toContain("Hidden command");
  });

  test("shows the current chord next to a chorded command", async () => {
    const target = openLauncher();
    await flush();
    const searchRow = [...target.querySelectorAll(".row")].find(
      (r) => r.querySelector(".title")?.textContent === "Search",
    ) as HTMLElement;
    const chord = searchRow.querySelector(".chord");
    expect(chord).not.toBeNull();
    expect(chord?.textContent?.length).toBeGreaterThan(0);
  });

  test("type-ahead filters the list", async () => {
    const target = openLauncher();
    await flush();
    launcherPanel.query = "new";
    await tick();
    expect(rowTitles(target)).toEqual(["New file"]);
  });

  test("arrows move the highlight and aria-activedescendant", async () => {
    const target = openLauncher();
    await flush();
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
    const launcher = target.querySelector(".launcher") as HTMLElement;
    launcher.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Enter", bubbles: true }),
    );
    await tick();
    expect(runSearch).toHaveBeenCalledTimes(1);
    expect(runNewFile).not.toHaveBeenCalled();
    expect(launcherPanel.open).toBe(false);
  });

  test("clicking a row runs it and closes", async () => {
    const target = openLauncher();
    await flush();
    const newFileRow = [...target.querySelectorAll(".row")].find(
      (r) => r.querySelector(".title")?.textContent === "New file",
    ) as HTMLElement;
    newFileRow.click();
    await tick();
    expect(runNewFile).toHaveBeenCalledTimes(1);
    expect(launcherPanel.open).toBe(false);
  });
});
