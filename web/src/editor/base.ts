// Shared CodeMirror 6 plumbing for both editors (Source + WYSIWYG).
//
// Lifted out of the legacy editor/Source.svelte so the WYSIWYG rewrite
// reuses the same theme handling, find-on-page state, density attribute,
// and external-sync guard. See web/src/editor-cm6/design.md for the
// invariants this module helps enforce (in particular #1: the doc IS the
// markdown source, and #8: find shape is identical across modes).

import {
  Compartment,
  RangeSetBuilder,
  StateEffect,
  StateField,
  type Extension,
} from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
} from "@codemirror/view";
import { syntaxHighlighting } from "@codemirror/language";
import { githubDarkHighlight, githubLightHighlight } from "./highlight";
import {
  scanMatches,
  type FindAdapter,
  type FindOptions,
  type FindRange,
} from "./find";

export type ChanTheme = "light" | "dark";

/// Build the theme extensions for the given app theme. The CM theme block
/// only carries non-bg styling; the actual transparency rule lives in the
/// host CSS (see Source.svelte's <style> block) because CM injects theme
/// rules as generated classes whose ordering we cannot rely on.
export function themeExtensions(theme: ChanTheme): Extension[] {
  // The editor-theme dimension (github / google_docs / word) flows
  // through CSS vars on documentElement, which both .cm-content
  // (typography) and the host CSS (chrome) read. The syntax-
  // highlight palette is deliberately NOT per-editor-theme - all
  // three editor themes share a single GitHub Primer palette,
  // branching only on light / dark. This keeps code snippets
  // reading the same regardless of which document chrome is
  // active (gh is the de-facto reference for syntax color).
  const palette = theme === "dark" ? githubDarkHighlight : githubLightHighlight;
  return [
    syntaxHighlighting(palette, { fallback: true }),
    EditorView.theme({
      // Ink follows the active editor theme; falls back to the
      // app's --text so a partial theme override still reads.
      "&": { color: "var(--chan-editor-body-color, var(--text))" },
      // Caret color. The browser's native caret on .cm-content
      // honors `caret-color`; CM6's synthetic cursor (used for
      // multi-select etc.) reads `borderLeftColor` on .cm-cursor.
      // Both pull from --chan-editor-body-color so the caret
      // flips with the active editor theme + color scheme. The
      // previous oneDark extension set both; dropping it without
      // restoring caret-color left the caret black on dark.
      ".cm-content": {
        caretColor: "var(--chan-editor-body-color, var(--text))",
      },
      ".cm-cursor, .cm-dropCursor": {
        borderLeftColor: "var(--chan-editor-body-color, var(--text))",
      },
      ".cm-gutters": {
        backgroundColor: "var(--bg-card)",
        color: "var(--text-secondary)",
        border: "none",
      },
      ".cm-activeLineGutter": { backgroundColor: "var(--hover-bg)" },
    }),
  ];
}

/// Compartment factory for the theme so callers can `reconfigure()` on
/// app-theme flips without rebuilding the editor. Returned together so the
/// caller can both seed the initial extensions AND keep the handle for
/// later reconfiguration.
export function makeThemeCompartment(initial: ChanTheme): {
  compartment: Compartment;
  extension: Extension;
  reconfigure(view: EditorView, theme: ChanTheme): void;
} {
  const compartment = new Compartment();
  return {
    compartment,
    extension: compartment.of(themeExtensions(initial)),
    reconfigure(view, theme) {
      view.dispatch({
        effects: compartment.reconfigure(themeExtensions(theme)),
      });
    },
  };
}

// ---- find-on-page state field ---------------------------------------------
// Mirror of the legacy WYSIWYG findHighlight plugin and the Source.svelte
// state field. The FindBar dispatches setFindEffect with the latest ranges
// + the active index; the StateField turns those into a Decoration.mark set
// so CodeMirror paints `.find-match` / `.find-match--current`.

export type FindFieldState = {
  ranges: FindRange[];
  currentIndex: number;
  decos: DecorationSet;
};

export const setFindEffect = StateEffect.define<{
  ranges: FindRange[];
  currentIndex: number;
}>();

const findMarkMatch = Decoration.mark({ class: "find-match" });
const findMarkCurrent = Decoration.mark({
  class: "find-match find-match--current",
});

export function buildFindDecos(
  ranges: FindRange[],
  currentIndex: number,
  docLen: number,
): DecorationSet {
  if (ranges.length === 0) return Decoration.none;
  const b = new RangeSetBuilder<Decoration>();
  for (let i = 0; i < ranges.length; i++) {
    const r = ranges[i]!;
    if (r.from >= r.to) continue;
    if (r.from < 0 || r.to > docLen) continue;
    b.add(r.from, r.to, i === currentIndex ? findMarkCurrent : findMarkMatch);
  }
  return b.finish();
}

export const findField = StateField.define<FindFieldState>({
  create(): FindFieldState {
    return { ranges: [], currentIndex: -1, decos: Decoration.none };
  },
  update(prev, tr): FindFieldState {
    let ranges = prev.ranges;
    let currentIndex = prev.currentIndex;
    let dirty = false;
    for (const e of tr.effects) {
      if (e.is(setFindEffect)) {
        ranges = e.value.ranges;
        currentIndex = e.value.currentIndex;
        dirty = true;
      }
    }
    if (!dirty && !tr.docChanged) return prev;
    if (tr.docChanged && !dirty) {
      // Map existing ranges through the edit so highlights track local
      // insertions without a synchronous rescan. The FindBar's debounced
      // rescan replaces them shortly after.
      const mapped: FindRange[] = [];
      for (const r of ranges) {
        const from = tr.changes.mapPos(r.from, 1);
        const to = tr.changes.mapPos(r.to, -1);
        if (to > from) mapped.push({ from, to });
      }
      ranges = mapped;
    }
    return {
      ranges,
      currentIndex,
      decos: buildFindDecos(ranges, currentIndex, tr.state.doc.length),
    };
  },
  provide: (f) => EditorView.decorations.from(f, (s) => s.decos),
});

/// Build a FindAdapter against an EditorView accessor. The accessor pattern
/// (rather than a direct view ref) lets Svelte components hand in their
/// ref-bound view without a stale closure when the editor remounts.
export function makeFindAdapter(getView: () => EditorView | undefined): FindAdapter {
  return {
    scan(query: string, opts: FindOptions): FindRange[] {
      const view = getView();
      if (!view) return [];
      return scanMatches(view.state.doc.toString(), query, opts);
    },
    highlightAll(matches: FindRange[], currentIndex: number): void {
      const view = getView();
      if (!view) return;
      view.dispatch({
        effects: setFindEffect.of({ ranges: matches, currentIndex }),
      });
    },
    clearHighlights(): void {
      const view = getView();
      if (!view) return;
      view.dispatch({
        effects: setFindEffect.of({ ranges: [], currentIndex: -1 }),
      });
    },
    scrollIntoView(currentIndex: number): void {
      const view = getView();
      if (!view) return;
      const f = view.state.field(findField, false);
      if (!f) return;
      const r = f.ranges[currentIndex];
      if (!r) return;
      view.dispatch({
        effects: EditorView.scrollIntoView(r.from, { y: "center" }),
      });
    },
    placeCursor(currentIndex: number): void {
      const view = getView();
      if (!view) return;
      const f = view.state.field(findField, false);
      if (!f) return;
      const r = f.ranges[currentIndex];
      if (!r) return;
      // Set a zero-width selection at the match start. Skipping
      // `view.focus()` keeps the FindBar input focused so the user
      // can step through more matches; Esc later returns focus to
      // the editor and the caret is already on the match.
      view.dispatch({
        selection: { anchor: r.from, head: r.from },
      });
    },
  };
}

/// Two-way sync helper for `$bindable` Svelte props. The caller wraps both
/// the updateListener (doc → prop) and the $effect (prop → doc) so this
/// helper centralizes the `applyingExternal` guard and the scoped flag.
///
/// Usage:
///   const sync = createValueSync();
///   // in EditorView extensions:
///   EditorView.updateListener.of((u) => sync.onDocChanged(u, (s) => value = s)),
///   // in $effect:
///   $effect(() => sync.applyExternal(view, value));
///
/// The guard prevents the prop-write triggered by the user's own typing
/// from re-applying as an "external" change (which would clobber the cursor
/// and re-render).
export function createValueSync(): {
  onDocChanged(
    update: { docChanged: boolean; state: { doc: { toString(): string } } },
    write: (s: string) => void,
  ): void;
  applyExternal(
    view: EditorView | undefined,
    value: string,
    opts?: { focus?: boolean },
  ): void;
} {
  let applying = false;
  return {
    onDocChanged(update, write) {
      if (applying) return;
      if (update.docChanged) write(update.state.doc.toString());
    },
    applyExternal(view, value, opts) {
      if (!view) return;
      const cur = view.state.doc.toString();
      if (cur === value) return;
      applying = true;
      try {
        // Preserve the user's selection across the external replace.
        // Forcing the caret to position 0 ("first line jump") was the
        // old behavior; it surfaced as a cursor-yank during typing if
        // a sibling write briefly desynced `value` from the live doc.
        // We clamp to the new doc length so a shorter incoming value
        // cannot place the caret past the end.
        const prev = view.state.selection.main;
        const lim = value.length;
        view.dispatch({
          changes: { from: 0, to: cur.length, insert: value },
          selection: {
            anchor: Math.min(prev.anchor, lim),
            head: Math.min(prev.head, lim),
          },
        });
      } finally {
        applying = false;
      }
      if (opts?.focus !== false) view.focus();
    },
  };
}
