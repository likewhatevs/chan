// Component test: the shared WindowRow. The mutable surface renders [FOCUS] +
// [SHOW/HIDE] (Eye visible / EyeOff hidden); `icon` adds a leading kind glyph and
// the control terminal's amber "reconnecting…" pill + eye flash when its library
// needs attention. Exercises the real Svelte 5 runtime per jsdom. The read-only
// surface (static dot, no actions) is covered in LibraryReadOnly.test.ts.

import { describe, it, expect, afterEach, vi } from "vitest";
import { mount, unmount, flushSync } from "svelte";
import WindowRow from "./WindowRow.svelte";
import { controlAttention, clearAllControlAttention } from "../state/controlAttention.svelte";
import type { WindowRecord } from "../api/library";

vi.mock("../api/backend", async () => {
  const { mockApi } = await import("../api/mock");
  return { backend: mockApi };
});

function win(
  over: Partial<WindowRecord> & Pick<WindowRecord, "window_id" | "library_id">,
): WindowRecord {
  return {
    kind: "terminal",
    title: "",
    ordinal: 1,
    workspace_path: null,
    prefix: "",
    token: "",
    persisted: true,
    connected: true,
    control: false,
    ...over,
  };
}

let target: HTMLElement | null = null;
let app: Record<string, unknown> | null = null;

function render(w: WindowRecord, props: { icon?: boolean } = {}): HTMLElement {
  target = document.createElement("div");
  document.body.appendChild(target);
  app = mount(WindowRow, { target, props: { w, ...props } });
  return target;
}

afterEach(() => {
  if (app) unmount(app);
  target?.remove();
  target = null;
  app = null;
  clearAllControlAttention();
});

describe("WindowRow", () => {
  it("renders an iconless mutable row: focus + hide, no glyph, no dot, no pill", () => {
    const el = render(win({ window_id: "w", library_id: "local" }));
    expect(el.querySelector('[aria-label="Focus window"]')).not.toBeNull();
    expect(el.querySelector('[aria-label="Hide window"]')).not.toBeNull();
    expect(el.querySelector(".row-glyph")).toBeNull();
    expect(el.querySelector(".dot")).toBeNull();
    expect(el.textContent).not.toContain("reconnecting");
  });

  it("renders a leading glyph and a path-free 'Window N' label for a workspace window", () => {
    const el = render(
      win({ window_id: "w", library_id: "local", kind: "workspace", workspace_path: "/p", ordinal: 1 }),
      { icon: true },
    );
    expect(el.querySelector(".row-glyph")).not.toBeNull();
    // The window row no longer repeats the workspace path (the card carries it),
    // and the label drops the base prefix -- just "Window N".
    expect(el.querySelector(".row-sub")).toBeNull();
    expect(el.textContent).toContain("Window 1");
    expect(el.textContent).not.toContain("/p");
  });

  it("a hidden window shows the Show (EyeOff) toggle", () => {
    const el = render(win({ window_id: "h", library_id: "local", hidden: true }));
    expect(el.querySelector('[aria-label="Show window"]')).not.toBeNull();
    expect(el.querySelector('[aria-label="Hide window"]')).toBeNull();
  });

  it("flashes the control eye and shows the reconnecting pill when its library needs attention", () => {
    controlAttention.libs["lib-x"] = true;
    const el = render(win({ window_id: "c", library_id: "lib-x", control: true }), { icon: true });
    const eye = el.querySelector("button.icon-btn.attention");
    expect(eye).not.toBeNull();
    expect(eye!.getAttribute("aria-label")).toContain("needs attention");
    expect(el.querySelector(".reconnecting")).not.toBeNull();
    expect(el.querySelector(".row-glyph.control")).not.toBeNull();
  });

  it("FOCUS drives openWindow through the bridge", async () => {
    const { backend } = await import("../api/backend");
    const open = vi.spyOn(backend, "openWindow");
    const el = render(win({ window_id: "w-local-term-1", library_id: "local" }));
    (el.querySelector('[aria-label="Focus window"]') as HTMLButtonElement).click();
    await new Promise((r) => setTimeout(r, 0));
    flushSync();
    expect(open).toHaveBeenCalledTimes(1);
    open.mockRestore();
  });
});
