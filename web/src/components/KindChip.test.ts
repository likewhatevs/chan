// @vitest-environment jsdom

import { mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, test, vi } from "vitest";

import KindChip from "./KindChip.svelte";

const mounted: Array<Record<string, any>> = [];

afterEach(() => {
  for (const component of mounted.splice(0)) unmount(component);
  document.body.innerHTML = "";
});

function render(props: Record<string, unknown>): HTMLElement {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(KindChip, { target, props: props as any });
  mounted.push(component);
  return target;
}

describe("KindChip", () => {
  test("without onClick renders as a presentational span", async () => {
    const target = render({ kind: "document" });
    await tick();
    const chip = target.querySelector(".kind-chip");
    expect(chip).not.toBeNull();
    expect(chip!.tagName).toBe("SPAN");
    expect(chip!.classList.contains("clickable")).toBe(false);
  });

  test("with onClick renders as a button and forwards the click", async () => {
    const onClick = vi.fn();
    const target = render({ kind: "tag", onClick });
    await tick();
    const chip = target.querySelector<HTMLButtonElement>(".kind-chip");
    expect(chip).not.toBeNull();
    expect(chip!.tagName).toBe("BUTTON");
    expect(chip!.classList.contains("clickable")).toBe(true);
    expect(chip!.type).toBe("button");
    chip!.click();
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  test("style modifiers carry through both render branches", async () => {
    const spanTarget = render({ kind: "folder", block: true, compact: true, ghost: true, dim: true });
    await tick();
    const span = spanTarget.querySelector(".kind-chip")!;
    expect(span.tagName).toBe("SPAN");
    for (const cls of ["block", "compact", "ghost", "dim"]) {
      expect(span.classList.contains(cls)).toBe(true);
    }

    const buttonTarget = render({
      kind: "folder",
      block: true,
      compact: true,
      ghost: true,
      dim: true,
      onClick: () => {},
    });
    await tick();
    const button = buttonTarget.querySelector(".kind-chip")!;
    expect(button.tagName).toBe("BUTTON");
    for (const cls of ["block", "compact", "ghost", "dim"]) {
      expect(button.classList.contains(cls)).toBe(true);
    }
  });
});
