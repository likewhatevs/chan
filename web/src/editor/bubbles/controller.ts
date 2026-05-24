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
import { EditorView, keymap } from "@codemirror/view";
import type { BubbleHandle, BubbleSpec } from "./types";
import { computeBubbleSpec } from "./triggers";

export function bubbleListener(opts: {
  onSpec: (spec: BubbleSpec | null) => void;
}): Extension {
  let prev: BubbleSpec | null = null;
  return EditorView.updateListener.of((u) => {
    if (!(u.docChanged || u.selectionSet)) return;
    const spec = computeBubbleSpec(u.state);
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
