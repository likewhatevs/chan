// The backend the launcher talks to, split into two independently-sourced
// halves:
//
//   - The window feed (list + watch) is served live: the library's HTTP
//     handlers exist, so the launcher reads the authoritative open-window set
//     straight off the server.
//   - The registry CRUD (workspaces + devservers) is served by the in-memory
//     mock until its HTTP handlers are deployed; pointing REGISTRY at liveApi
//     moves it over with no other change.
//
// Each source implements the full LibraryApi; `backend` is composed from the
// two so the rest of the app stays a single LibraryApi consumer.

import { liveApi, type LibraryApi } from "./library";
import { mockApi } from "./mock";

const REGISTRY: LibraryApi = mockApi;
const WINDOW_FEED: LibraryApi = liveApi;

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
  createWindow: WINDOW_FEED.createWindow,
};
