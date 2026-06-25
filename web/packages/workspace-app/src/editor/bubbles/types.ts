// Shared types for the bubble controller.
//
// Keep separate from the heavier modules so the trigger / keymap
// modules can import without dragging in DOM-heavy bubble UI code.

export type BubbleKind = "wiki" | "image" | "tag" | "contact" | "mention";

/// Snapshot of the current bubble trigger as derived from the doc
/// state. The controller updateListener computes one of these on each
/// transaction and notifies the host (Wysiwyg.svelte) so it can open /
/// close the active bubble UI.
///
/// `triggerStart` is the position of the trigger's first character (the
/// `[`, `!`, `#`, or `@`). `triggerEnd` is the caret position. The
/// inclusive substring `[triggerStart .. triggerEnd]` is what gets
/// replaced when the bubble commits a selection.
export type BubbleSpec = {
  kind: BubbleKind;
  triggerStart: number;
  triggerEnd: number;
  query: string;
  /// Image-only: how the bubble formats its commit.
  ///   "wrap" (default): replace the trigger with `![](path)`. Used
  ///     when the user typed `![` from scratch.
  ///   "raw": replace the trigger with just `path`. Used when the
  ///     caret is inside an existing image's URL portion (the
  ///     surrounding `![alt](`...`)` is already there and we don't
  ///     want to nuke it).
  templateMode?: "wrap" | "raw";
};

/// Active-bubble handle exposed back to the host. Mirrors the legacy
/// editor/bubble.ts BubbleHandle (kept for shape parity); the host's
/// keymap routes keys through `handleKey` before CM6 defaults run.
export interface BubbleHandle {
  /// Process a keydown. Return true to consume; the host's keymap
  /// then preventDefaults the event.
  handleKey(event: KeyboardEvent): boolean;
  /// Update the bubble's typed query as the user keeps typing inside
  /// the trigger range. The bubble re-fetches / re-filters as needed.
  setQuery(query: string): void;
  /// Re-anchor under the caret (called when the viewport changes or
  /// the trigger position shifts due to upstream edits).
  reposition(): void;
  /// Tear down DOM + listeners. Idempotent.
  dismiss(): void;
}
