// Install the frontend-only demo backend: point the transport's fetch and
// WebSocket factory at the in-memory mock. Call before the app mounts (see
// WorkspaceDemo.svelte). The real transport is unchanged until this runs.

import { setFetchImpl, setSocketFactory } from "../api/transport";
import type { MockWorkspaceData } from "./data";
import { DemoGraph } from "./graph";
import { createDemoFetch } from "./router";
import { demoSocketFactory } from "./socket";
import { MockWorkspaceStore } from "./store";

export function installDemoWorkspace(data: MockWorkspaceData): MockWorkspaceStore {
  const store = new MockWorkspaceStore(data);
  const graph = new DemoGraph(store);
  setFetchImpl(createDemoFetch(store, graph));
  setSocketFactory(demoSocketFactory);
  return store;
}

export function uninstallDemoWorkspace(): void {
  setFetchImpl(null);
  setSocketFactory(null);
}
