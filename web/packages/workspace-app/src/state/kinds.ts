// Unified file + entity kind taxonomy. Single source of truth for
// chip labels, palette tokens, and tree icons across the file tree,
// inspector, search overlay, graph overlay, and tag inspector.
//
// Three families of kind:
//   - FileKind: things that exist as files in the workspace.
//   - EntityKind: graph-only entities (tokens extracted from
//     markdown bodies); no file backing.
//   - ContainerKind: the lone "folder" kind for directory rows.
//
// The server projects a `kind` discriminator on every regular file in
// `/api/files`. The path-only fallback below only runs for paths held
// outside the tree listing (graph ghost rows, broken-link targets). It
// mirrors chan-workspace's classifier via `classifyPath` so the editor
// and the workspace agree on what counts as text.

import { FileText, User, FileCode, Image, File, Hash, Calendar, Folder } from "lucide-svelte";

import type { TreeEntry } from "../api/types";
import { classifyPath } from "./fileTypes";

export type FileKind =
  | "document"
  | "contact"
  | "text"
  | "media"
  | "binary"
  // Unknown extension whose content hasn't been sniffed yet. The
  // server resolves this to "text"/"binary" for per-directory file-
  // browser listings; it only reaches the SPA from the recursive
  // whole-tree listing (image picker), so it renders neutrally.
  | "pending";
export type EntityKind = "tag" | "mention" | "date";
export type ContainerKind = "folder";
export type Kind = FileKind | EntityKind | ContainerKind;

/// Classify a tree entry. Folder entries return "folder"; otherwise
/// the server-provided `kind === "contact"` discriminator wins,
/// followed by image-by-extension, the editable-text gate, and
/// finally "binary" for everything that can't yet round-trip
/// through the editor (PDFs, archives, fonts, ...).
export function classifyEntry(entry: TreeEntry): Kind {
  if (entry.is_dir) return "folder";
  return classifyFile(entry.path, entry.kind);
}

/// Path + optional server kind hint. Useful when we hold the path
/// and the wire kind but not a full TreeEntry (graph nodes, search
/// rows). When `serverKind` is one of the wire values
/// (`document` | `contact` | `text` | `media` | `binary`) we trust
/// it directly; that's the path most callers take. The path-based
/// fallback (`classifyPath`) only runs when `serverKind` is absent
/// -- e.g. a graph ghost row whose target file isn't in the current
/// tree listing. It cannot distinguish a markdown-class .md (with
/// contact frontmatter) from a regular document, so callers that
/// need the contact distinction must come through the server.
export function classifyFile(
  path: string,
  serverKind?: TreeEntry["kind"],
): FileKind {
  if (serverKind) return serverKind;
  return classifyPath(path);
}

/// True for file kinds the editor opens as text: markdown documents,
/// contacts (markdown notes flagged `chan.kind: contact`), and plain
/// source / config / shell text. Gates the inspector's Open-vs-Download
/// pill and the File Browser's open-selection off the SERVER-provided
/// content kind. The per-directory listing content-sniffs an unknown
/// extension to `text` / `binary`, so an odd-suffix plaintext file lands
/// as `text` here and gets "Open" like the tree's double-click instead
/// of the extension-gated "Download". Media, binary, and the not-yet-
/// sniffed `pending` kind are not openable as text. Accepts the broad
/// `Kind` (what `classifyEntry` returns) so callers can pass it directly;
/// folder / tag / mention / date all fall through to false.
export function isOpenableTextKind(kind: Kind): boolean {
  return kind === "document" || kind === "text" || kind === "contact";
}

/// Human label used in kind chips. Lowercased; chip CSS handles
/// uppercasing so callers can compose strings with the label.
export function labelFor(kind: Kind): string {
  if (kind === "folder") return "directory";
  return kind;
}

/// CSS color variable for the chip background. Wraps the canonical
/// palette tokens defined in App.svelte; see web/packages/workspace-app/src/design.md for
/// the cross-surface mapping. `text` aliases `--g-doc` for now
/// because the two share the document hue family until we pick a
/// separate tone (the visual distinction is icon + label, not hue).
///
/// Kind palette:
///   document/text    -> orange  (--g-doc)
///   contact/mention  -> yellow  (--warn-text)
///   media            -> purple  (--g-img)
///   tag              -> green   (--g-tag)
///   binary           -> blue    (--g-binary)
///   folder           -> grey    (--g-folder)
///   date             -> grey    (--text-secondary, low-emphasis neutral)
/// Flows through every surface (file tree, inspector, search, graph)
/// via this single switch instead of being hardcoded per component.
export function colorVarFor(kind: Kind): string {
  switch (kind) {
    case "document":
    case "text":
      return "var(--g-doc)";
    case "contact":
    case "mention":
      return "var(--warn-text)";
    case "media":
      return "var(--g-img)";
    case "tag":
      return "var(--g-tag)";
    case "binary":
      return "var(--g-binary)";
    case "pending":
      // Low-emphasis neutral until the sniff resolves it.
      return "var(--text-secondary)";
    case "date":
      return "var(--text-secondary)";
    case "folder":
      return "var(--g-folder)";
  }
}

/// Lucide icon component for the kind. Used by the file tree (one
/// glyph per row) and by future inspector / search row glyphs.
/// `Image` is the Lucide icon name; the import collides harmlessly
/// with the DOM Image constructor because consumers use the
/// imported binding.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function iconFor(kind: Kind): any {
  switch (kind) {
    case "document":
      return FileText;
    case "contact":
    case "mention":
      return User;
    case "text":
      return FileCode;
    case "media":
      return Image;
    case "binary":
    case "pending":
      // Generic file glyph; "pending" is transient and rarely shown.
      return File;
    case "tag":
      return Hash;
    case "date":
      return Calendar;
    case "folder":
      return Folder;
  }
}
