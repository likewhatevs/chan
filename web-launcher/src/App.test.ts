// Smoke test: the launcher root mounts and renders its top bar with the
// theme toggle and the New-workspace button. Registry/feed rendering loads
// asynchronously from the backend and is covered by the state + component
// tests; this keeps the mount path itself green.

import { describe, it, expect, afterEach } from "vitest";
import { mount, unmount } from "svelte";
import App from "./App.svelte";

describe("launcher root", () => {
  let target: HTMLElement | null = null;
  let app: Record<string, unknown> | null = null;

  afterEach(() => {
    if (app) unmount(app);
    target?.remove();
    target = null;
    app = null;
  });

  it("renders the top bar and its actions", () => {
    target = document.createElement("div");
    document.body.appendChild(target);
    app = mount(App, { target });

    expect(target.querySelector(".topbar")).not.toBeNull();
    expect(target.querySelector('[aria-label="New workspace"]')).not.toBeNull();
    expect(target.querySelector('[aria-label="Toggle theme"]')).not.toBeNull();
  });
});
