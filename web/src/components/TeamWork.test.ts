// @vitest-environment jsdom

import { mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, test, vi } from "vitest";

import TeamWork from "./TeamWork.svelte";
import {
  layout,
  type LeafNode,
  type TeamWorkState,
} from "../state/tabs.svelte";
import {
  ui,
} from "../state/store.svelte";

const mounted: Array<Record<string, any>> = [];

afterEach(() => {
  for (const component of mounted.splice(0)) unmount(component);
  document.body.innerHTML = "";
  ui.status = null;
  resetLayout();
  vi.restoreAllMocks();
});

function resetLayout(): LeafNode {
  const pane: LeafNode = {
    kind: "leaf",
    id: "pane-team-work-test",
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

async function renderPrompt(prompt: TeamWorkState) {
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
  const component = mount(TeamWork, {
    target,
    props: { prompt, onSubmit },
  });
  mounted.push(component);
  await tick();
  const root = target.querySelector<HTMLElement>(".team-work");
  if (!root) throw new Error("team work prompt not mounted");
  return { target, root, onSubmit };
}

function button(target: ParentNode, label: string): HTMLButtonElement {
  const el = target.querySelector<HTMLButtonElement>(`button[aria-label='${label}']`);
  if (!el) throw new Error(`button not found: ${label}`);
  return el;
}

describe("TeamWork", () => {
  test("Escape closes the action menu without closing the prompt", async () => {
    const prompt: TeamWorkState = {
      buffer: "## keep\n\nthis draft",
      heightPx: 320,
      open: true,
      mode: "source",
    };
    const { target, root } = await renderPrompt(prompt);

    button(target, "Team Work actions").click();
    await tick();
    expect(target.querySelector(".ctx")).not.toBeNull();

    root.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await tick();

    expect(prompt.open).toBe(true);
    expect(prompt.buffer).toBe("## keep\n\nthis draft");
    expect(target.querySelector(".ctx")).toBeNull();
  });

  test("Cmd/Ctrl+Enter submits raw markdown and keeps the overlay state", async () => {
    const prompt: TeamWorkState = {
      buffer: "one **two**\n![alt](attachments/a.png)",
      heightPx: 320,
      open: true,
      mode: "source",
    };
    const { root, onSubmit } = await renderPrompt(prompt);

    root.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "Enter",
        metaKey: true,
        bubbles: true,
      }),
    );
    await tick();

    expect(onSubmit).toHaveBeenCalledWith("one **two**\n![alt](attachments/a.png)");
    expect(prompt.open).toBe(true);
    expect(prompt.buffer).toBe("one **two**\n![alt](attachments/a.png)");
  });

  test("Shift+Enter never submits (chat-style newline chord)", async () => {
    // Shift+Enter inserts a newline. The wrapper short-circuits before
    // the submit guard so a stray Shift+Enter that bubbles up cannot
    // reach submit().
    const prompt: TeamWorkState = {
      buffer: "draft",
      heightPx: 320,
      open: true,
      mode: "source",
    };
    const { root, onSubmit } = await renderPrompt(prompt);

    root.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "Enter",
        shiftKey: true,
        bubbles: true,
        cancelable: true,
      }),
    );
    await tick();

    expect(onSubmit).not.toHaveBeenCalled();
  });

  test("plain Enter submits the prompt at the wrapper fallback path", async () => {
    // The CM6-level handler claims Enter when the editor has focus.
    // This exercises the wrapper fallback that fires when the keydown
    // bubbles up unhandled (defaultPrevented=false).
    const prompt: TeamWorkState = {
      buffer: "hi",
      heightPx: 320,
      open: true,
      mode: "source",
    };
    const { root, onSubmit } = await renderPrompt(prompt);

    root.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: "Enter",
        bubbles: true,
        cancelable: true,
      }),
    );
    await tick();

    expect(onSubmit).toHaveBeenCalledWith("hi");
  });

  test("Cmd+Enter with defaultPrevented does NOT re-submit", async () => {
    // Wysiwyg's CM6 keymap calls submit() and sets preventDefault().
    // The wrapper bails on defaultPrevented to avoid a double-submit
    // (e.g. `pwd` reaching the PTY as `pwdpwd`).
    const prompt: TeamWorkState = {
      buffer: "pwd",
      heightPx: 320,
      open: true,
      mode: "wysiwyg",
    };
    const { root, onSubmit } = await renderPrompt(prompt);

    const event = new KeyboardEvent("keydown", {
      key: "Enter",
      metaKey: true,
      bubbles: true,
      cancelable: true,
    });
    event.preventDefault();
    root.dispatchEvent(event);
    await tick();

    expect(onSubmit).not.toHaveBeenCalled();
  });

  test("send button submits the same raw source as the keyboard path", async () => {
    const prompt: TeamWorkState = {
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
    const prompt: TeamWorkState = {
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
    // The style toolbar is off by default, so it is opened explicitly here
    // to exercise the mode-toggle, not the toolbar's default visibility.
    const prompt: TeamWorkState = {
      buffer: "draft",
      heightPx: 320,
      open: true,
      mode: "source",
      styleToolbarOpen: true,
    };
    const { target } = await renderPrompt(prompt);

    button(target, "show rendered").click();
    await tick();

    expect(prompt.mode).toBe("wysiwyg");
  });

  test("mounted terminal prompts keep draft and submit state isolated", async () => {
    // Same styleToolbarOpen: true rationale as the mode-toggle test.
    const first: TeamWorkState = {
      buffer: "first draft",
      heightPx: 260,
      open: true,
      mode: "source",
      styleToolbarOpen: true,
    };
    const second: TeamWorkState = {
      buffer: "second draft",
      heightPx: 360,
      open: true,
      mode: "source",
      styleToolbarOpen: true,
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

  test("action menu drops prompt-local file, watcher, and spawn controls", async () => {
    // Agent-spawn entry points, copy-config helpers, and older
    // file/watcher controls are all gone from the right-click menu.
    const prompt: TeamWorkState = {
      buffer: "# reusable prompt\n\nbody",
      heightPx: 320,
      open: true,
      mode: "source",
    };
    const { target } = await renderPrompt(prompt);

    button(target, "Team Work actions").click();
    await tick();

    expect(target.textContent).not.toContain("New File from here");
    expect(target.textContent).not.toContain("Watch directory");
    expect(target.textContent).not.toContain("Stop watching");
    expect(target.textContent).not.toContain("Spawn agent");
    expect(target.textContent).not.toContain("Spawn agents");
    expect(target.textContent).not.toContain("Copy metadata dir");
    expect(target.textContent).not.toContain("Copy Spawn agents config");
    expect(target.querySelector("button[aria-label='Close']")).toBeNull();
  });

  test("action menu lists the items in order", async () => {
    const prompt: TeamWorkState = {
      buffer: "draft",
      heightPx: 320,
      open: true,
      mode: "wysiwyg",
    };
    const { target } = await renderPrompt(prompt);

    button(target, "Team Work actions").click();
    await tick();

    const labels = [...target.querySelectorAll<HTMLButtonElement>(".ctx button")].map(
      (btn) => btn.textContent?.trim(),
    );
    expect(labels).toEqual([
      "Show source code",
      "Show style toolbar",
      "Collapse prompt",
    ]);
  });
});
