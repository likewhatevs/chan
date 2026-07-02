// Lazy loader for the frontend-only workspace demo overlay. This module (and
// the whole workspace-app graph behind it) lives in its own chunk: the
// launcher embed dynamic-imports it on the first window-tile click, so the
// marketing landing page never pays for the editor/graph/terminal bundle.

import { mount } from "svelte";
import type { MockWorkspaceData } from "@chan/workspace-app/demo-data";
import WorkspaceDemoOverlay from "./WorkspaceDemoOverlay.svelte";

let snapshot: MockWorkspaceData | null = null;
let overlay: { show: () => void } | null = null;

export async function openWorkspaceDemo(): Promise<void> {
  if (overlay) {
    overlay.show();
    return;
  }
  if (!snapshot) {
    const res = await fetch("/assets/demo-workspace.json");
    if (!res.ok) throw new Error(`demo snapshot failed to load: ${res.status}`);
    snapshot = (await res.json()) as MockWorkspaceData;
  }
  const host = document.createElement("div");
  document.body.appendChild(host);
  overlay = mount(WorkspaceDemoOverlay, { target: host, props: { data: snapshot } });
}
