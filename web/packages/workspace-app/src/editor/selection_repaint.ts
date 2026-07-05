import { EditorView, ViewPlugin, type ViewUpdate } from "@codemirror/view";
import type { Extension } from "@codemirror/state";

// WKWebView leaves a stale, full-content-width `.cm-selectionBackground`
// band on screen when a word selection on a wrapped, hanging-indent (list)
// line collapses back to a caret: CM6's drawSelection drops the marker
// from its layer, but WKWebView does not repaint `.cm-selectionLayer`, so
// the old band lingers even though the real selection is an empty caret.
// This is the same repaint-staleness class the `browser.webkit` display
// hack in CM6's own LayerView already targets, and that the block/walker
// decoration code notes manifests under WKWebView, not Blink.
//
// Nudge the layer to repaint on a real collapse (a non-empty selection
// becoming a caret). Blink repaints natively, so there it is at most one
// forced reflow; gating on the collapse keeps this off the per-keystroke
// caret-move path entirely.
export function selectionLayerRepaintFix(): Extension {
  return ViewPlugin.fromClass(
    class {
      update(u: ViewUpdate): void {
        if (!u.selectionSet) return;
        // Only a non-empty selection collapsing to a caret can strand a
        // band; a caret move from an already-empty selection never drew
        // one, so skip it.
        if (u.startState.selection.main.empty) return;
        if (!u.state.selection.main.empty) return;
        u.view.requestMeasure({
          key: "chan-selection-layer-repaint",
          read: () => null,
          write: (_measured, view: EditorView) => {
            const layer = view.scrollDOM.querySelector<HTMLElement>(
              ".cm-selectionLayer",
            );
            if (!layer) return;
            const prev = layer.style.display;
            layer.style.display = "none";
            // Read layout to force a synchronous reflow so WKWebView drops
            // the stale band before the layer is shown again.
            void layer.offsetHeight;
            layer.style.display = prev;
          },
        });
      }
    },
  );
}
