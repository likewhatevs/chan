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
  ui,
} from "../state/store.svelte";
import { closeSpawnDialog } from "../state/spawnDialog.svelte";

const mounted: Array<Record<string, any>> = [];

afterEach(() => {
  for (const component of mounted.splice(0)) unmount(component);
  document.body.innerHTML = "";
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
  const component = mount(TerminalRichPrompt, {
    target,
    props: { prompt, onSubmit },
  });
  mounted.push(component);
  await tick();
  const root = target.querySelector<HTMLElement>(".rich-prompt");
  if (!root) throw new Error("rich prompt not mounted");
  return { target, root, onSubmit };
}

function button(target: ParentNode, label: string): HTMLButtonElement {
  const el = target.querySelector<HTMLButtonElement>(`button[aria-label='${label}']`);
  if (!el) throw new Error(`button not found: ${label}`);
  return el;
}

function buttonByText(target: ParentNode, text: string): HTMLButtonElement {
  const el = [...target.querySelectorAll<HTMLButtonElement>("button")].find((btn) =>
    btn.textContent?.includes(text),
  );
  if (!el) throw new Error(`button text not found: ${text}`);
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
  test("Escape closes the action menu without closing the prompt", async () => {
    const prompt: TerminalRichPromptState = {
      buffer: "## keep\n\nthis draft",
      heightPx: 320,
      open: true,
      mode: "source",
    };
    const { target, root } = await renderPrompt(prompt);

    button(target, "Rich Prompt actions").click();
    await tick();
    expect(target.querySelector(".ctx")).not.toBeNull();

    root.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await tick();

    expect(prompt.open).toBe(true);
    expect(prompt.buffer).toBe("## keep\n\nthis draft");
    expect(target.querySelector(".ctx")).toBeNull();
  });

  test("Cmd/Ctrl+Enter submits raw markdown and keeps the overlay state", async () => {
    const prompt: TerminalRichPromptState = {
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
    // Phase-13 bug 4: Shift+Enter must insert a newline in the
    // editor, never submit the prompt. The wrapper short-circuits
    // before the submit guard so a stray Shift+Enter that bubbles
    // up (e.g. editor not focused) cannot reach `submit()`.
    const prompt: TerminalRichPromptState = {
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
    // Phase-13 bug 4: chat-style send chord. The CM6-level handler
    // in Wysiwyg / Source claims the keystroke first when the editor
    // has focus; this test exercises the wrapper fallback that fires
    // when the keydown bubbles up unhandled (defaultPrevented=false).
    const prompt: TerminalRichPromptState = {
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

  test("Cmd+Enter with defaultPrevented does NOT re-submit (fullstack-a-20)", async () => {
    // `fullstack-a-18` threaded `onSubmit={submit}` to the Wysiwyg
    // child. Wysiwyg's CM6 keymap has its own Mod-Enter binding that
    // calls `submit()` and returns true; CM's keymap runner then
    // calls `preventDefault()` on the DOM event. Pre-`-a-20` the
    // wrapper's `onKeydown` ignored `defaultPrevented` and called
    // `submit()` again — `pwd` reached the PTY as `pwdpwd`. The
    // wrapper now bails on `defaultPrevented`; source mode is
    // unaffected because Source has no Mod-Enter binding.
    const prompt: TerminalRichPromptState = {
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
    // `fullstack-a-24`: the style toolbar's mode-toggle button
    // (aria-label="show rendered" / "show source") is the surface
    // the test clicks. Toolbar default flipped to off in -a-24, so
    // explicitly open it here — this test is exercising the mode-
    // toggle, not the toolbar's default visibility.
    const prompt: TerminalRichPromptState = {
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
    // Same `styleToolbarOpen: true` rationale as the mode-toggle
    // test above — `fullstack-a-24` default-off the toolbar.
    const first: TerminalRichPromptState = {
      buffer: "first draft",
      heightPx: 260,
      open: true,
      mode: "source",
      styleToolbarOpen: true,
    };
    const second: TerminalRichPromptState = {
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

  test("action menu drops prompt-local file and watcher controls", async () => {
    const prompt: TerminalRichPromptState = {
      buffer: "# reusable prompt\n\nbody",
      heightPx: 320,
      open: true,
      mode: "source",
    };
    const { target } = await renderPrompt(prompt);

    button(target, "Rich Prompt actions").click();
    await tick();

    expect(target.textContent).not.toContain("New File from here");
    expect(target.textContent).not.toContain("Watch directory");
    expect(target.textContent).not.toContain("Stop watching");
    expect(target.querySelector("button[aria-label='Close']")).toBeNull();
  });

  test("Spawn agents action opens the global team dialog", async () => {
    // `fullstack-a-78` repurposed the icon-btn from "Watch
    // directory" to Spawn agents. The dialog's bootstrap flow
    // lives in `-a-79` (orchestrator); slice 1 just opens the
    // dialog + hands off via the request bus.
    const { teamDialogState, closeTeamDialog } = await import(
      "../state/teamDialog.svelte"
    );
    closeTeamDialog();
    const prompt: TerminalRichPromptState = {
      buffer: "",
      heightPx: 320,
      open: true,
      mode: "source",
    };
    installPointerCaptureStubs();
    const target = document.createElement("div");
    Object.assign(target.style, { position: "relative", height: "500px" });
    document.body.append(target);
    const component = mount(TerminalRichPrompt, {
      target,
      props: {
        prompt,
        onSubmit: vi.fn(),
        terminalSessionId: "term_123",
        watcherPath: null,
      },
    });
    mounted.push(component);
    await tick();

    button(target, "Rich Prompt actions").click();
    await tick();
    buttonByText(target, "Spawn agents").click();
    await tick();
    expect(teamDialogState.request).not.toBeNull();
    expect(teamDialogState.request?.hostSessionId).toBe("term_123");
    closeTeamDialog();
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
        terminalSessionId: "orchestrator_session",
        onSpawned,
      },
    });
    mounted.push(component);
    await tick();

    const setSubmitMode = vi
      .spyOn(api, "setTerminalSubmitMode")
      .mockResolvedValue(undefined);
    const picker = target.querySelector<HTMLSelectElement>(".agent-picker");
    if (!picker) throw new Error("agent picker not found");
    picker.value = "codex";
    picker.dispatchEvent(new Event("change", { bubbles: true }));
    await waitFor(() => setSubmitMode.mock.calls.length === 1);
    expect(prompt.agentTarget).toBe("codex");
    expect(prompt.submitMode).toBe("agent");
    expect(setSubmitMode).toHaveBeenCalledWith("orchestrator_session", "agent");

    // `fullstack-a-4`: SpawnDialog mounts at the App root in
    // real life; in this test we mount it as a sibling so the
    // global state singleton workspaces it through to render.
    const SpawnDialog = (await import("./SpawnDialog.svelte")).default;
    const dialogHost = document.createElement("div");
    document.body.append(dialogHost);
    const dialogComponent = mount(SpawnDialog, { target: dialogHost, props: {} });
    mounted.push(dialogComponent);
    await tick();

    button(target, "Rich Prompt actions").click();
    await tick();
    buttonByText(target, "Spawn agent").click();
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
