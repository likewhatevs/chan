// Bubble controller: wires trigger detection to bubble lifecycle.
//
// Pattern:
//   - bubbleListener({ onSpec }) returns an Extension whose
//     updateListener computes a BubbleSpec on every transaction and
//     fires onSpec(spec | null). The host (Wysiwyg.svelte) decides
//     when to open / close the actual bubble UI in response.
//   - bubbleKeymap(getActive) returns a high-precedence keymap that
//     consults the active bubble's handleKey before CM6's defaults.
//     The host updates the closure variable when bubbles open / close.
//
// Why split this from the bubble UIs themselves: the trigger detection
// + state field is pure CM6, but the bubble UIs build DOM that lives
// in Svelte's lifecycle. Keeping the controller here means the Svelte
// host can decide whether to mount a fresh bubble or reuse an existing
// one (e.g., the user types more chars within the same `[[query`).

import { type Extension, Prec } from "@codemirror/state";
import { EditorView, keymap, type ViewUpdate } from "@codemirror/view";
import type { BubbleHandle, BubbleSpec } from "./types";
import { computeBubbleSpec } from "./triggers";

export function bubbleListener(opts: {
  onSpec: (spec: BubbleSpec | null) => void;
  getCurrentPath?: () => string | null;
  isInlineCodeFileLink?: (text: string, currentPath: string | null) => boolean;
  /// Extra recompute trigger beyond doc/selection changes. The host wires
  /// this to the kind-resolve broadcast (an effect-only transaction that is
  /// neither docChanged nor selectionSet) so an inline-code link that
  /// resolves while the caret is already inside it opens the picker at once.
  recomputeOn?: (u: ViewUpdate) => boolean;
}): Extension {
  let prev: BubbleSpec | null = null;
  // The inline-code change region currently armed, position-mapped across
  // edits. Lets the trigger detector keep matching that one inline `code`
  // span structurally while the user edits its token (the picker only
  // OPENS on a resolved file; see triggers.inlineCodeChangeSpec).
  let armed: { from: number; to: number } | null = null;
  return EditorView.updateListener.of((u) => {
    if (armed && u.docChanged) {
      armed = {
        from: u.changes.mapPos(armed.from),
        to: u.changes.mapPos(armed.to, 1),
      };
    }
    if (!(u.docChanged || u.selectionSet || opts.recomputeOn?.(u))) return;
    const spec = computeBubbleSpec(u.state, {
      getCurrentPath: opts.getCurrentPath,
      isInlineCodeFileLink: opts.isInlineCodeFileLink,
      armedInlineCode: armed,
    });
    armed =
      spec?.origin === "inline-code"
        ? { from: spec.triggerStart, to: spec.triggerEnd }
        : null;
    if (specEqual(prev, spec)) return;
    prev = spec;
    opts.onSpec(spec);
  });
}

function specEqual(a: BubbleSpec | null, b: BubbleSpec | null): boolean {
  if (a === b) return true;
  if (!a || !b) return false;
  return (
    a.kind === b.kind &&
    a.triggerStart === b.triggerStart &&
    a.triggerEnd === b.triggerEnd &&
    a.query === b.query
  );
}

export function bubbleKeymap(
  getActive: () => BubbleHandle | null,
): Extension {
  // High precedence so we beat CM6 defaults (Enter / Escape / arrow
  // keys all have built-in handlers that would otherwise eat the
  // event before the bubble sees it).
  return Prec.highest(
    keymap.of([
      {
        any: (_view, event) => {
          const handle = getActive();
          if (!handle) return false;
          return handle.handleKey(event);
        },
      },
    ]),
  );
}
