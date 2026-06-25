// Component test: the read-only window feed (gateway/devserver surface, no
// desktop bridge). `readOnly` is a boot-time const read from a <meta> tag, so it
// can't be toggled at runtime — this file pins it true via a module mock and
// asserts the contract: NO action buttons (no FOCUS / SHOW-HIDE), just the
// static connection dot. The mutable surface is covered in WindowFeed.test.ts.

import { describe, it, expect, afterEach, beforeEach, vi } from "vitest";
import { mount, unmount } from "svelte";
import WindowFeed from "./WindowFeed.svelte";
import { loadLibrary } from "../state/library.svelte";

// Force the read-only surface for the whole file (hoisted before the imports).
vi.mock("../state/capabilities", () => ({ readOnly: true }));

// Pin the in-memory mock as the backend so the feed renders the seed windows.
vi.mock("../api/backend", async () => {
  const { mockApi } = await import("../api/mock");
  return { backend: mockApi };
});

let target: HTMLElement | null = null;
let app: Record<string, unknown> | null = null;

beforeEach(async () => {
  await loadLibrary();
});

afterEach(() => {
  if (app) unmount(app);
  target?.remove();
  target = null;
  app = null;
});

describe("WindowFeed read-only surface", () => {
  it("renders the static dot and NO action buttons", () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(WindowFeed, { target });

    // Rows render with the connection-state dot, but no FOCUS / SHOW-HIDE actions.
    expect(target.querySelector(".dot")).toBeTruthy();
    expect(target.querySelector("button")).toBeNull();
    expect(target.querySelector('[aria-label="Focus window"]')).toBeNull();
    expect(target.querySelector('[aria-label="Hide window"]')).toBeNull();
    expect(target.querySelector('[aria-label="Show window"]')).toBeNull();
  });
});
