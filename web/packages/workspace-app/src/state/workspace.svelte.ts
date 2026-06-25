// Workspace info singleton + draft-path helpers.
//
// This is a LEAF module with no eager side effects so both
// `store.svelte.ts` (which re-exports `workspace`) and `tabs.svelte.ts`
// can import it without triggering the store/tabs draft-promotion-sink
// init-order cycle (see the note in tabs.svelte.ts). Keep it dependency
// -light: only the `WorkspaceInfo` type, nothing that runs at import.

import type { WorkspaceInfo } from "../api/types";

export const workspace = $state<{ info: WorkspaceInfo | null }>({ info: null });

/// Single source of truth for the configured Drafts directory. The
/// backend surfaces `WorkspaceInfo.drafts_dir` (a real in-workspace
/// relpath, e.g. `.Drafts`) read-only on `/api/workspace`. Default
/// `.Drafts` until the info round-trip lands. Never hardcode the
/// literal anywhere; key all draft-path logic off this accessor.
export function draftsDir(): string {
  return workspace.info?.drafts_dir ?? ".Drafts";
}

/// A path is a draft path when it is the drafts dir itself or sits
/// under it. Drafts are real relpaths (`.Drafts/untitled/draft.md`).
export function isDraftPath(path: string): boolean {
  const dir = draftsDir();
  return path === dir || path.startsWith(`${dir}/`);
}
