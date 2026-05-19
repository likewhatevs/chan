// @vitest-environment jsdom

import { mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, test, vi } from "vitest";

import TerminalRichPrompt from "./TerminalRichPrompt.svelte";
import { api } from "../api/client";
import {
  layout,
  type LeafNode,
  type TerminalRichPromptState,
} from "../state/tabs.svelte";
import {
  pathPromptState,
  resolvePathPrompt,
  ui,
} from "../state/store.svelte";
import { closeSpawnDialog } from "../state/spawnDialog.svelte";

const mounted: Array<Record<string, any>> = [];

afterEach(() => {
  for (const component of mounted.splice(0)) unmount(component);
  document.body.innerHTML = "";
  resolvePathPrompt(null);
  ui.status = null;
  resetLayout();
  closeSpawnDialog();
  vi.restoreAllMocks();
});

function resetLayout(): LeafNode {
  const pane: LeafNode = {
    kind: "leaf",
    id: "pane-rich-prompt-test",
    tabs: [],
    activeTabId: null,
  };
  layout.rootId = pane.id;
  layout.activePaneId = pane.id;
  layout.nodes = { [pane.id]: pane };
  return pane;
}

function installPointerCaptureStubs(): void {
  Object.defineProperty(HTMLElement.prototype, "setPointerCapture", {
    configurable: true,
    value: vi.fn(),
  });
  Object.defineProperty(HTMLElement.prototype, "releasePointerCapture", {
    configurable: true,
    value: vi.fn(),
  });
}

function pointerEvent(type: string, clientY: number): PointerEvent {
  const event = new MouseEvent(type, {
    bubbles: true,
    clientY,
  }) as PointerEvent;
  Object.defineProperty(event, "pointerId", { value: 1 });
  return event;
}

async function renderPrompt(prompt: TerminalRichPromptState) {
  installPointerCaptureStubs();
  const target = document.createElement("div");
  Object.assign(target.style, {
    position: "relative",
    height: "500px",
  });
  target.getBoundingClientRect = () =>
    ({
      x: 0,
      y: 0,
      top: 0,
      left: 0,
      right: 800,
      bottom: 500,
      width: 800,
      height: 500,
      toJSON: () => ({}),
    }) as DOMRect;
  document.body.append(target);

  const onSubmit = vi.fn();
  const onClose = vi.fn(() => {
    prompt.open = false;
  });
  const component = mount(TerminalRichPrompt, {
    target,
    props: { prompt, onSubmit, onClose },
  });
  mounted.push(component);
  await tick();
  const root = target.querySelector<HTMLElement>(".rich-prompt");
  if (!root) throw new Error("rich prompt not mounted");
  return { target, root, onSubmit, onClose };
}

function button(target: ParentNode, label: string): HTMLButtonElement {
  const el = target.querySelector<HTMLButtonElement>(`button[aria-label='${label}']`);
  if (!el) throw new Error(`button not found: ${label}`);
  return el;
}

async function waitFor(condition: () => boolean): Promise<void> {
  for (let i = 0; i < 20; i += 1) {
    if (condition()) return;
    await tick();
    await Promise.resolve();
  }
}

describe("TerminalRichPrompt", () => {
  test("Escape hides without clearing the draft", async () => {
    const prompt: TerminalRichPromptState = {
      buffer: "## keep\n\nthis draft",
      heightPx: 320,
      open: true,
      mode: "source",
    };
    const { root, onClose } = await renderPrompt(prompt);

    root.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await tick();

    expect(onClose).toHaveBeenCalledTimes(1);
    expect(prompt.open).toBe(false);
    expect(prompt.buffer).toBe("## keep\n\nthis draft");
  });

  test("Cmd/Ctrl+Enter submits raw markdown and keeps the overlay state", async () => {
    const prompt: TerminalRichPromptState = {
      buffer: "one **two**\n![alt](attachments/a.png)",
      heightPx: 320,
      open: true,
      mode: "source",
    };
    const { root, onSubmit, onClose } = await renderPrompt(prompt);

    root.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "Enter",
        metaKey: true,
        bubbles: true,
      }),
    );
    await tick();

    expect(onSubmit).toHaveBeenCalledWith("one **two**\n![alt](attachments/a.png)");
    expect(onClose).not.toHaveBeenCalled();
    expect(prompt.open).toBe(true);
    expect(prompt.buffer).toBe("one **two**\n![alt](attachments/a.png)");
  });

  test("send button submits the same raw source as the keyboard path", async () => {
    const prompt: TerminalRichPromptState = {
      buffer: "# prompt\n\nbody",
      heightPx: 280,
      open: true,
      mode: "source",
    };
    const { target, onSubmit } = await renderPrompt(prompt);

    button(target, "Send prompt").click();
    await tick();

    expect(onSubmit).toHaveBeenCalledWith("# prompt\n\nbody");
  });

  test("height drag clamps to minimum height and top gap", async () => {
    const prompt: TerminalRichPromptState = {
      buffer: "",
      heightPx: 320,
      open: true,
      mode: "source",
    };
    const { target } = await renderPrompt(prompt);
    const handle = button(target, "resize prompt");

    handle.dispatchEvent(pointerEvent("pointerdown", 250));
    handle.dispatchEvent(pointerEvent("pointermove", 490));
    expect(prompt.heightPx).toBe(150);

    handle.dispatchEvent(pointerEvent("pointermove", 10));
    expect(prompt.heightPx).toBe(464);

    handle.dispatchEvent(pointerEvent("pointerup", 10));
  });

  test("mode toggle stores source/render state on the terminal prompt", async () => {
    const prompt: TerminalRichPromptState = {
      buffer: "draft",
      heightPx: 320,
      open: true,
      mode: "source",
    };
    const { target } = await renderPrompt(prompt);

    button(target, "show rendered").click();
    await tick();

    expect(prompt.mode).toBe("wysiwyg");
  });

  test("mounted terminal prompts keep draft and submit state isolated", async () => {
    const first: TerminalRichPromptState = {
      buffer: "first draft",
      heightPx: 260,
      open: true,
      mode: "source",
    };
    const second: TerminalRichPromptState = {
      buffer: "second draft",
      heightPx: 360,
      open: true,
      mode: "source",
    };
    const a = await renderPrompt(first);
    const b = await renderPrompt(second);

    button(a.target, "show rendered").click();
    button(b.target, "Send prompt").click();
    await tick();

    expect(first.mode).toBe("wysiwyg");
    expect(second.mode).toBe("source");
    expect(a.onSubmit).not.toHaveBeenCalled();
    expect(b.onSubmit).toHaveBeenCalledWith("second draft");
    expect(first.buffer).toBe("first draft");
    expect(second.buffer).toBe("second draft");
  });

  test("New File from here seeds the create prompt and writes the draft", async () => {
    const create = vi.spyOn(api, "create").mockResolvedValue(undefined);
    vi.spyOn(api, "list").mockResolvedValue([]);
    vi.spyOn(api, "read").mockResolvedValue({
      content: "persisted",
      mtime: 12,
      repo_root: null,
      writable: true,
    } as Awaited<ReturnType<typeof api.read>>);
    resetLayout();
    const prompt: TerminalRichPromptState = {
      buffer: "# reusable prompt\n\nbody",
      heightPx: 320,
      open: true,
      mode: "source",
    };
    const { target } = await renderPrompt(prompt);

    button(target, "New file from here").click();
    await tick();
    expect(pathPromptState.open).toBe(true);
    expect(pathPromptState.defaultValue).toBe("prompt.md");
    expect(pathPromptState.kind).toBe("file");
    expect(pathPromptState.mode).toBe("create");

    resolvePathPrompt("saved-prompt");
    await waitFor(() => ui.status === "Created saved-prompt.md");

    expect(create).toHaveBeenCalledWith("saved-prompt.md", false, "# reusable prompt\n\nbody");
    expect(ui.status).toBe("Created saved-prompt.md");
    const pane = layout.nodes[layout.activePaneId];
    expect(pane?.kind).toBe("leaf");
    if (pane?.kind !== "leaf") return;
    expect(pane.tabs.some((tab) => tab.kind === "file" && tab.path === "saved-prompt.md")).toBe(true);
  });

  test("Watch directory uses the path prompt and terminal watcher endpoint", async () => {
    const setWatcher = vi.spyOn(api, "setTerminalWatcher").mockResolvedValue(undefined);
    const prompt: TerminalRichPromptState = {
      buffer: "",
      heightPx: 320,
      open: true,
      mode: "source",
    };
    const onWatcherStarted = vi.fn();
    installPointerCaptureStubs();
    const target = document.createElement("div");
    Object.assign(target.style, { position: "relative", height: "500px" });
    document.body.append(target);
    const component = mount(TerminalRichPrompt, {
      target,
      props: {
        prompt,
        onSubmit: vi.fn(),
        onClose: vi.fn(),
        terminalSessionId: "term_123",
        watcherPath: null,
        onWatcherStarted,
      },
    });
    mounted.push(component);
    await tick();

    button(target, "Watch directory").click();
    await tick();
    expect(pathPromptState.open).toBe(true);
    expect(pathPromptState.kind).toBe("folder");
    expect(pathPromptState.allowAbsolute).toBe(true);

    resolvePathPrompt("/tmp/events");
    await waitFor(() => onWatcherStarted.mock.calls.length === 1);

    expect(setWatcher).toHaveBeenCalledWith("term_123", "/tmp/events");
    expect(onWatcherStarted).toHaveBeenCalledWith("/tmp/events");
  });

  test("Spawn agent opens dialog and posts terminal control request", async () => {
    const spawn = vi.spyOn(api, "spawnTerminal").mockResolvedValue({
      session: "spawn_session",
      tab_label: "@@Pair",
    });
    const prompt: TerminalRichPromptState = {
      buffer: "",
      heightPx: 320,
      open: true,
      mode: "source",
    };
    const onSpawned = vi.fn();
    installPointerCaptureStubs();
    const target = document.createElement("div");
    Object.assign(target.style, { position: "relative", height: "500px" });
    document.body.append(target);
    const component = mount(TerminalRichPrompt, {
      target,
      props: {
        prompt,
        onSubmit: vi.fn(),
        onClose: vi.fn(),
        terminalSessionId: "orchestrator_session",
        onSpawned,
      },
    });
    mounted.push(component);
    await tick();

    // `fullstack-a-4`: SpawnDialog mounts at the App root in
    // real life; in this test we mount it as a sibling so the
    // global state singleton drives it through to render.
    const SpawnDialog = (await import("./SpawnDialog.svelte")).default;
    const dialogHost = document.createElement("div");
    document.body.append(dialogHost);
    const dialogComponent = mount(SpawnDialog, { target: dialogHost, props: {} });
    mounted.push(dialogComponent);
    await tick();

    button(target, "Spawn agent").click();
    await tick();
    const dialog = document.body.querySelector<HTMLElement>(".spawn-dialog");
    if (!dialog) throw new Error("spawn dialog not mounted");
    const inputs = [...dialog.querySelectorAll<HTMLInputElement>("input")];
    const textareas = [...dialog.querySelectorAll<HTMLTextAreaElement>("textarea")];
    inputs[0]!.value = "@@Pair";
    inputs[0]!.dispatchEvent(new Event("input", { bubbles: true }));
    await tick();
    textareas[0]!.value = "codex --model gpt-5";
    textareas[0]!.dispatchEvent(new Event("input", { bubbles: true }));
    await tick();
    textareas[1]!.value = "FOO=bar";
    textareas[1]!.dispatchEvent(new Event("input", { bubbles: true }));
    await tick();

    [...dialog.querySelectorAll("button")].find((el) => el.textContent?.includes("Spawn"))!.click();
    await waitFor(() => onSpawned.mock.calls.length === 1);

    expect(spawn).toHaveBeenCalledWith({
      name: "@@Pair",
      command: "codex --model gpt-5",
      env: { FOO: "bar" },
      orchestrator_session: "orchestrator_session",
    });
    expect(onSpawned).toHaveBeenCalledWith({ session: "spawn_session", tab_label: "@@Pair" }, "@@Pair");
  });
});
