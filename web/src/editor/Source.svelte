<script lang="ts">
  // CodeMirror 6 source mode. Same backing buffer as the WYSIWYG view; the
  // user toggles per-tab. Markdown grammar gives basic highlighting.
  //
  // The CM theme follows the app theme via a `Compartment` so we can
  // reconfigure on toggle without rebuilding the editor.

  import { onDestroy, onMount } from "svelte";
  import { EditorState } from "@codemirror/state";
  import { EditorView, keymap, lineNumbers } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { markdown } from "@codemirror/lang-markdown";
  import { drive, ui } from "../state/store.svelte";
  import {
    createValueSync,
    findField,
    makeFindAdapter,
    makeThemeCompartment,
  } from "../editor-cm6/base";
  import type { FindAdapter } from "./find";

  // Editor density follows the user's line_spacing pref. Same hook
  // the Wysiwyg side uses (see Wysiwyg.svelte:820), exposed here as
  // a `data-density` attribute on .md-source so the CSS rules below
  // can dial line-height between tight (gdocs-like) and standard
  // (older roomier) without rebuilding the CodeMirror editor.
  const density = $derived(drive.info?.preferences?.line_spacing ?? "tight");

  let { value = $bindable("") }: { value: string } = $props();

  let host: HTMLDivElement | undefined;
  let view: EditorView | undefined;
  const sync = createValueSync();
  const theme = makeThemeCompartment(ui.theme);

  /// Find-on-page adapter. FileEditorTab passes whichever editor is
  /// currently visible to FindBar; the bar drives matches + decorations
  /// through this surface. Shared shape with the WYSIWYG adapter via
  /// editor-cm6/base.ts.
  export const findAdapter: FindAdapter = makeFindAdapter(() => view);

  /// Scroll to a specific line (0-based). Called by the inspector
  /// (outline view) when the user picks a heading and this tab is
  /// in source mode.
  export function scrollToLine(line: number): void {
    if (!view) return;
    const total = view.state.doc.lines;
    const target = Math.min(Math.max(0, line), Math.max(0, total - 1));
    const pos = view.state.doc.line(target + 1).from;
    view.dispatch({
      selection: { anchor: pos },
      effects: EditorView.scrollIntoView(pos, { y: "start" }),
    });
    view.focus();
  }

  onMount(() => {
    if (!host) return;
    const state = EditorState.create({
      doc: value,
      extensions: [
        lineNumbers(),
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        markdown(),
        theme.extension,
        EditorView.lineWrapping,
        findField,
        EditorView.updateListener.of((u) => {
          sync.onDocChanged(u, (s) => (value = s));
        }),
      ],
    });
    view = new EditorView({ state, parent: host });
    // Drop cursor at start of doc and focus so the editor is ready to
    // type immediately after opening / switching tabs.
    view.dispatch({ selection: { anchor: 0 } });
    view.focus();
  });

  onDestroy(() => view?.destroy());

  $effect(() => {
    sync.applyExternal(view, value);
  });

  // Reconfigure the theme compartment whenever the app theme flips.
  $effect(() => {
    if (!view) return;
    theme.reconfigure(view, ui.theme);
  });
</script>

<div class="md-source" data-density={density} bind:this={host}></div>

<style>
  /* Keep the CodeMirror chrome wrapper themed. The CM6 editor itself
     uses its default light highlight style for now (see v1.1 polish). */
  .md-source {
    /* `flex: 1` so the wrapper always spans the full pane width
       (matches `.md-wysiwyg`). Without it, the wrapper shrinks to
       its content's intrinsic width — and once we cap `.cm-editor`
       via `--chan-page-max-width`, that intrinsic width becomes
       the cap, leaving the source view left-aligned in the pane
       instead of centered within the page-width column. */
    flex: 1;
    min-height: 0;
    height: 100%;
    overflow: auto;
    background: var(--bg);
    box-sizing: border-box;
  }
  /* Source mode uses the drive's "code" font preference (it is
     a code editor, after all). */
  :global(.md-source .cm-editor) {
    height: 100%;
    font-size: var(--chan-font-code-size, 14px);
    /* Center the whole CM editor (gutter + content together) within
       the cap when --chan-page-max-width is set (per-device pref
       written by state/pageWidth). Putting the cap on .cm-content
       instead would only narrow where lines wrap, leaving the
       gutter glued to the left edge and an empty band on the
       right. The scroll container .md-source stays full-width so
       the scrollbar sits at the viewport edge, matching the
       Wysiwyg side. */
    max-width: var(--chan-page-max-width, none);
    margin-inline: auto;
  }
  :global(.md-source .cm-content) {
    font-family: var(--chan-font-code-family);
  }
  /* Force every CM internal that could paint a background to
     transparent so `.md-source`'s `var(--bg)` shows uniformly,
     even past the longest line. CM injects theme rules as
     generated classes whose ordering we can't depend on; an
     `!important` at this static layer wins regardless. The
     gutter keeps its own bg (set on `.cm-gutters` in the theme
     extension) because its rule has higher specificity than
     these. */
  :global(.md-source .cm-editor),
  :global(.md-source .cm-editor .cm-scroller),
  :global(.md-source .cm-editor .cm-content),
  :global(.md-source .cm-editor .cm-line),
  :global(.md-source .cm-editor .cm-activeLine) {
    background-color: transparent !important;
  }
  /* Line-spacing pref. Mirrors the Wysiwyg's data-density rules so
     toggling between tight (default, gdocs-like) and standard
     (older, roomier) flips both editors in lockstep. CodeMirror's
     default line-height (1.4) becomes the tight value; standard
     bumps to 1.7 to match the WYSIWYG's normal-mode feel. */
  :global(.md-source[data-density="tight"] .cm-line) { line-height: 1.4; }
  :global(.md-source[data-density="standard"] .cm-line) { line-height: 1.7; }

  /* Find-on-page highlight (mirror of the Wysiwyg rule). The
     CodeMirror Decoration.mark wraps each match in a <span> with
     these classes; the active match also picks up the orange ring.
     Same CSS variables as the WYSIWYG side so both modes look the
     same across a Wysiwyg <-> Source toggle. */
  :global(.md-source .find-match) {
    background: var(--find-match-bg, rgba(255, 213, 0, 0.45));
    border-radius: 2px;
  }
  :global(.md-source .find-match--current) {
    background: var(--find-match-current-bg, rgba(255, 140, 0, 0.65));
    outline: 1px solid var(--find-match-current-border, rgba(180, 80, 0, 0.9));
  }
</style>
