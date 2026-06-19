// The backend the launcher talks to, split into two independently-sourced
// halves so each can move to the live client as its handlers ship:
//
//   - The window feed (list + watch) is the authoritative open-window set.
//     Its handlers exist, so it can be served live independently.
//   - The registry CRUD (workspaces + devservers) is served by the in-memory
//     mock until its HTTP handlers are deployed.
//
// To move a half to the live server, import `liveApi` from "./library" and
// point that half's source at it. Each source implements the full LibraryApi;
// `backend` is composed from the two so the rest of the app stays a single
// LibraryApi consumer.

import type { LibraryApi } from "./library";
import { mockApi } from "./mock";

const REGISTRY: LibraryApi = mockApi;
const WINDOW_FEED: LibraryApi = mockApi;

export const backend: LibraryApi = {
  // Registry CRUD.
  listWorkspaces: REGISTRY.listWorkspaces,
  addLocalWorkspace: REGISTRY.addLocalWorkspace,
  setWorkspaceOn: REGISTRY.setWorkspaceOn,
  removeWorkspace: REGISTRY.removeWorkspace,
  listDevservers: REGISTRY.listDevservers,
  addDevserver: REGISTRY.addDevserver,
  updateDevserver: REGISTRY.updateDevserver,
  removeDevserver: REGISTRY.removeDevserver,
  // Window feed.
  listWindows: WINDOW_FEED.listWindows,
  watchWindows: WINDOW_FEED.watchWindows,
};
