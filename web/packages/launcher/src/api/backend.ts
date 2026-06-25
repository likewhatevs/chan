// The backend the launcher talks to: the library's live HTTP client. Registry
// CRUD (workspaces + devservers) and the window feed are all served by the
// library's `/api/library/*` handlers, so the launcher reads and mutates real
// state on every surface (desktop loopback + devserver) through one client.
//
// `mock.ts` stays an in-memory test double the vitest suites import directly (or
// pin via `vi.mock("./backend")`); it is no longer wired into the runtime path.

import { liveApi, type LibraryApi } from "./library";

export const backend: LibraryApi = liveApi;
