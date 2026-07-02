// Dev harness entry for the frontend-only workspace demo. Loads the snapshot
// asset with the real fetch (before the mock transport is installed), then
// mounts WorkspaceDemo which installs the mock and renders the app.
//
// Served by index.demo.html under `vite dev`. The production marketing embed
// (Phase 7) loads the same WorkspaceDemo the same way, lazily.

import { mount } from "svelte";
import WorkspaceDemo from "../WorkspaceDemo.svelte";
import type { MockWorkspaceData } from "./data";

async function main(): Promise<void> {
  const res = await fetch("/demo-workspace.json");
  if (!res.ok) throw new Error(`failed to load demo snapshot: ${res.status}`);
  const data = (await res.json()) as MockWorkspaceData;

  const target = document.getElementById("app");
  if (!target) throw new Error("missing #app element");
  mount(WorkspaceDemo, { target, props: { data } });
}

main().catch((err) => {
  document.body.innerHTML = `<pre style="padding:2rem;color:#f85149">${String(err?.stack ?? err)}</pre>`;
});
