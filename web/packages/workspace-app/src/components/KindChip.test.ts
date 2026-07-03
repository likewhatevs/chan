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

  test("a path routes file kinds through the extension bucket colour", async () => {
    // A `.rs` file rides wire kind `text`; the graph paints it blue
    // (source), so the bubble must be blue too, not the wire kind's
    // orange. A `.txt` on the same wire kind stays orange.
    const rs = render({ kind: "text", path: "src/lib.rs" });
    await tick();
    expect(rs.querySelector(".kind-chip")!.getAttribute("style")).toContain("var(--g-source)");

    const txt = render({ kind: "text", path: "notes.txt" });
    await tick();
    expect(txt.querySelector(".kind-chip")!.getAttribute("style")).toContain("var(--g-doc)");

    const png = render({ kind: "media", path: "pic.png" });
    await tick();
    expect(png.querySelector(".kind-chip")!.getAttribute("style")).toContain("var(--g-img)");
  });

  test("without a path a file kind keeps its wire-kind colour", async () => {
    // The `text` regression: a pathless text chip stays orange (doc).
    const target = render({ kind: "text" });
    await tick();
    expect(target.querySelector(".kind-chip")!.getAttribute("style")).toContain("var(--g-doc)");
  });

  test("a path never overrides a non-file kind", async () => {
    const target = render({ kind: "tag", path: "irrelevant.rs" });
    await tick();
    expect(target.querySelector(".kind-chip")!.getAttribute("style")).toContain("var(--g-tag)");
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
