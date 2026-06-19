// Compose the display label for a window-feed row WITHOUT parsing the
// library-composed `title` or the opaque `window_id`. The library titles
// every window from its own (local, 🏠) perspective; a remote library's rows
// must read ↗ from the launcher's perspective, so the launcher recomposes
// them from `kind`, `ordinal`, and `workspace_path` alone. The same fields
// drive both the per-library section header and each row.

import type { WindowKind, WindowRecord } from "../api/library";

/** The baked-in local-disk library's id; everything else is a remote library. */
export const LOCAL_LIBRARY_ID = "local";

/** Trailing path component, tolerant of a trailing slash. "" for "" or "/". */
export function basename(path: string): string {
  const trimmed = path.replace(/\/+$/, "");
  const slash = trimmed.lastIndexOf("/");
  return slash >= 0 ? trimmed.slice(slash + 1) : trimmed;
}

/** "🏠" for the local library, "↗" for a remote (devserver) library. */
export function libraryIcon(libraryId: string): string {
  return libraryId === LOCAL_LIBRARY_ID ? "🏠" : "↗";
}

/**
 * A row's label: the recomposed "<base> Window N" for a workspace, or
 * "Terminal Window N" for a standalone terminal. No icon here; the icon is
 * the library's (the section carries it), so a row reads the same under any
 * library.
 */
export function rowLabel(kind: WindowKind, ordinal: number, workspacePath: string | null): string {
  if (kind === "terminal") return `Terminal Window ${ordinal}`;
  const base = workspacePath ? basename(workspacePath) : "Workspace";
  return `${base} Window ${ordinal}`;
}

/** Convenience over a whole record. */
export function windowRowLabel(w: WindowRecord): string {
  return rowLabel(w.kind, w.ordinal, w.workspace_path);
}

/**
 * The section header for a library id: "🏠 Local" for the local library, or
 * "↗ <name>" for a remote one, where <name> is the user's devserver label
 * resolved via the library-id join. Falls back to a short id when no name is
 * known (e.g. a devserver that has not connected yet).
 */
export function librarySectionLabel(libraryId: string, remoteName: string | null): string {
  if (libraryId === LOCAL_LIBRARY_ID) return "🏠 Local";
  return `↗ ${remoteName ?? shortLibraryId(libraryId)}`;
}

/** A compact, non-parsed rendering of an opaque library id for fallback display. */
export function shortLibraryId(libraryId: string): string {
  const hex = libraryId.startsWith("lib-") ? libraryId.slice(4) : libraryId;
  return hex.length > 8 ? `${hex.slice(0, 8)}...` : hex;
}
