// The backend the launcher talks to.
//
// The /api/library/* handlers are not deployed yet, so the launcher runs
// against an in-memory mock that implements the same wire. Set USE_MOCK to
// false (or delete this indirection) to point the SPA at the live HTTP
// client once the handlers exist; nothing else in the app changes.

import { liveApi, type LibraryApi } from "./library";
import { mockApi } from "./mock";

const USE_MOCK = true;

export const backend: LibraryApi = USE_MOCK ? mockApi : liveApi;
