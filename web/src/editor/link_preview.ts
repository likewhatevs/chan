// Internal-link markdown preview for the body-context menu.
// Resolves the wiki link ([[target]]) under a click point and opens the
// shared read-only preview popover (marked + DOMPurify render) for its
// target file. Handles wiki pills only, which carry their resolved
// target in `data-target`. Relative markdown links ([text](./other.md))
// are not handled (they would need a syntax-tree walk + normalizeHref
// rather than a rendered widget with the target attached).

import { openPreviewPopover } from "./overlays/preview_popover";

export interface InternalLinkHit {
  /// Workspace-rooted target file the wiki link resolves to (a bare
  /// stem is expanded to its note path, see resolvePreviewTarget).
  target: string;
  /// The pill element, used to anchor the preview popover.
  anchorEl: HTMLElement;
}

/// File extensions a wiki link may already carry. A target without one
/// is a note stem (`[[bullets]]`), so it expands to `<stem>.md`; reading
/// or opening the bare stem 404s.
const KNOWN_EXTS = new Set([
  ".md",
  ".markdown",
  ".txt",
  ".png",
  ".jpg",
  ".jpeg",
  ".webp",
  ".gif",
  ".svg",
  ".pdf",
]);

/// Resolve a wiki target to a readable/openable workspace path: drop any
/// `#heading` anchor and append the default note extension when the
/// target is a bare stem.
export function resolvePreviewTarget(target: string): string {
  const path = target.split("#", 1)[0]!;
  const dot = path.lastIndexOf(".");
  const hasKnownExt = dot > 0 && KNOWN_EXTS.has(path.slice(dot).toLowerCase());
  return hasKnownExt ? path : `${path}.md`;
}

/// The internal-link target under the viewport point, or null. MUST be
/// called BEFORE a menu portal covers the click point: this reads the
/// topmost element via elementFromPoint, so once the menu bubble is on
/// screen it would resolve the bubble instead of the pill underneath.
export function internalLinkAtPoint(x: number, y: number): InternalLinkHit | null {
  if (typeof document === "undefined") return null;
  const el = document.elementFromPoint(x, y);
  const pill = el?.closest?.(".cm-md-wiki-pill");
  if (pill instanceof HTMLElement && pill.dataset.target) {
    return {
      target: resolvePreviewTarget(pill.dataset.target),
      anchorEl: pill,
    };
  }
  return null;
}

/// Open the read-only markdown preview for an internal-link hit, reusing
/// the shared preview popover. `onOpen` commits to fully opening the
/// target (the popover's Open button / Cmd+Enter).
export function openLinkPreview(opts: {
  hit: InternalLinkHit;
  fromPath: string | null;
  onOpen: (openInNewPane: boolean) => void;
}): { dismiss: () => void } {
  return openPreviewPopover({
    anchor: opts.hit.anchorEl,
    path: opts.hit.target,
    fromPath: opts.fromPath,
    onOpen: opts.onOpen,
  });
}
