<script lang="ts">
  // CodeMirror 6 WYSIWYG editor. The doc IS the markdown source per
  // editor-cm6/design.md spec #1; rendered appearance is a pure
  // decoration layer built from @lezer/markdown's syntax tree.
  //
  // Step 4 scope: bare mount + chanMarkdown grammar + decorations for
  // marks (bold/italic/strike/code/link/naked URL) and headings. No
  // pills, no bubbles, no autosave wiring yet — those land in later
  // steps. Not yet imported by App.svelte; the existing tiptap
  // editor remains the production surface until cutover (step 11).
  //
  // The component's prop contract MUST eventually match the legacy
  // editor/Wysiwyg.svelte (value, readonly, onSubmit, onSelectionChange,
  // wikiPickerPrefix, currentPath, plus the imperative API surface)
  // so cutover is a one-line import swap. v1 only ships `value` + the
  // findAdapter; the rest is added as later steps fill in the
  // corresponding behavior.

  import { onDestroy, onMount } from "svelte";
  import { EditorState } from "@codemirror/state";
  import { EditorView, keymap } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { drive, ui } from "../state/store.svelte";
  import {
    createValueSync,
    findField,
    makeFindAdapter,
    makeThemeCompartment,
  } from "./base";
  import { chanMarkdown } from "./markdown/grammar";
  import { chanDecorations } from "./decorations";
  import { tagDecorations } from "./widgets/tag";
  import { dateDecorations } from "./widgets/date";
  import {
    wikiLinkDecorations,
    type WikiLinkClickArgs,
  } from "./widgets/wikilink";
  import type { FindAdapter } from "../editor/find";

  let {
    value = $bindable(""),
    onTagClick = () => {},
    onWikiClick = () => {},
  }: {
    value: string;
    onTagClick?: (tag: string) => void;
    onWikiClick?: (args: WikiLinkClickArgs) => void;
  } = $props();

  const density = $derived(drive.info?.preferences?.line_spacing ?? "tight");

  let host: HTMLDivElement | undefined;
  let view: EditorView | undefined;
  const sync = createValueSync();
  const theme = makeThemeCompartment(ui.theme);

  /// Find-on-page adapter (same shape as Source.svelte and the legacy
  /// WYSIWYG; FileEditorTab passes whichever editor is mounted to
  /// FindBar). Step 4 satisfies the contract; later steps add the
  /// rest of the imperative API.
  export const findAdapter: FindAdapter = makeFindAdapter(() => view);

  onMount(() => {
    if (!host) return;
    const state = EditorState.create({
      doc: value,
      extensions: [
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        chanMarkdown(),
        theme.extension,
        EditorView.lineWrapping,
        findField,
        chanDecorations(),
        tagDecorations({ onTagClick }),
        dateDecorations(),
        wikiLinkDecorations({ onWikiClick }),
        EditorView.updateListener.of((u) => {
          sync.onDocChanged(u, (s) => (value = s));
        }),
      ],
    });
    view = new EditorView({ state, parent: host });
    view.dispatch({ selection: { anchor: 0 } });
    view.focus();
  });

  onDestroy(() => view?.destroy());

  $effect(() => {
    sync.applyExternal(view, value);
  });

  $effect(() => {
    if (!view) return;
    theme.reconfigure(view, ui.theme);
  });
</script>

<div class="md-wysiwyg-cm6" data-density={density} bind:this={host}></div>

<style>
  /* Step 4 styles. Each rule is scoped to .md-wysiwyg-cm6 so we don't
     bleed into Source mode or the legacy WYSIWYG. CSS variables come
     from the app theme (theme.css). */

  .md-wysiwyg-cm6 {
    flex: 1;
    min-height: 0;
    height: 100%;
    overflow: auto;
    background: var(--bg);
    box-sizing: border-box;
  }

  :global(.md-wysiwyg-cm6 .cm-editor) {
    height: 100%;
    font-size: var(--chan-font-text-size, 16px);
    max-width: var(--chan-page-max-width, none);
    margin-inline: auto;
  }
  :global(.md-wysiwyg-cm6 .cm-content) {
    font-family: var(--chan-font-text-family);
  }
  :global(.md-wysiwyg-cm6 .cm-editor),
  :global(.md-wysiwyg-cm6 .cm-editor .cm-scroller),
  :global(.md-wysiwyg-cm6 .cm-editor .cm-content),
  :global(.md-wysiwyg-cm6 .cm-editor .cm-line),
  :global(.md-wysiwyg-cm6 .cm-editor .cm-activeLine) {
    background-color: transparent !important;
  }
  :global(.md-wysiwyg-cm6[data-density="tight"] .cm-line) { line-height: 1.5; }
  :global(.md-wysiwyg-cm6[data-density="standard"] .cm-line) { line-height: 1.8; }

  /* ---- mark decoration classes ---- */
  :global(.md-wysiwyg-cm6 .cm-md-bold) { font-weight: 700; }
  :global(.md-wysiwyg-cm6 .cm-md-italic) { font-style: italic; }
  :global(.md-wysiwyg-cm6 .cm-md-strike) { text-decoration: line-through; }
  :global(.md-wysiwyg-cm6 .cm-md-code) {
    font-family: var(--chan-font-code-family, monospace);
    font-size: 0.92em;
    background: var(--bg-card, rgba(0,0,0,0.06));
    padding: 0.05em 0.25em;
    border-radius: 3px;
  }
  :global(.md-wysiwyg-cm6 .cm-md-link) {
    color: var(--link, #0a64c8);
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  /* URL-when-revealed: dimmed so the user knows it's the URL portion
     they're editing, not the label. */
  :global(.md-wysiwyg-cm6 .cm-md-link-url) {
    color: var(--text-secondary, #888);
    opacity: 0.75;
  }

  /* ---- block-level line classes ---- */
  :global(.md-wysiwyg-cm6 .cm-md-quote) {
    border-left: 3px solid var(--text-secondary, #888);
    padding-left: 0.75em;
    color: var(--text-secondary, #888);
    font-style: italic;
  }
  :global(.md-wysiwyg-cm6 .cm-md-hr) {
    border-bottom: 1px solid var(--border, #ddd);
    margin: 0.5em 0;
    height: 0.5em;
    color: transparent;
  }
  :global(.md-wysiwyg-cm6 .cm-md-fence-opener),
  :global(.md-wysiwyg-cm6 .cm-md-fence-closer) {
    color: var(--text-secondary, #888);
    font-family: var(--chan-font-code-family, monospace);
    font-size: 0.92em;
  }
  :global(.md-wysiwyg-cm6 .cm-md-code-block) {
    font-family: var(--chan-font-code-family, monospace);
    font-size: 0.92em;
    background: var(--bg-card, rgba(0, 0, 0, 0.04));
    padding-left: 0.75em;
  }
  :global(.md-wysiwyg-cm6 .cm-md-fence-info) {
    color: var(--link, #0a64c8);
    font-weight: 500;
  }
  :global(.md-wysiwyg-cm6 .cm-md-task-checkbox) {
    margin: 0 0.4em 0 0;
    vertical-align: middle;
    cursor: pointer;
  }
  :global(.md-wysiwyg-cm6 .cm-md-tag) {
    background: var(--tag-bg, rgba(106, 168, 255, 0.18));
    color: var(--tag-fg, #2563b8);
    padding: 0.05em 0.4em;
    border-radius: 999px;
    font-size: 0.92em;
    cursor: pointer;
  }
  :global(.md-wysiwyg-cm6 .cm-md-tag:hover) {
    background: var(--tag-bg-hover, rgba(106, 168, 255, 0.28));
  }
  :global(.md-wysiwyg-cm6 .cm-md-date-pill) {
    background: var(--date-bg, rgba(120, 200, 120, 0.18));
    color: var(--date-fg, #2a7d2a);
    padding: 0.05em 0.4em;
    border-radius: 4px;
    font-size: 0.92em;
    cursor: text;
  }
  :global(.md-wysiwyg-cm6 .cm-md-wiki-pill) {
    background: var(--wiki-bg, rgba(168, 130, 255, 0.18));
    color: var(--wiki-fg, #6831c8);
    padding: 0.05em 0.4em;
    border-radius: 4px;
    font-size: 0.95em;
    cursor: pointer;
  }
  :global(.md-wysiwyg-cm6 .cm-md-wiki-pill:hover) {
    background: var(--wiki-bg-hover, rgba(168, 130, 255, 0.28));
  }

  /* ---- heading line classes ---- */
  :global(.md-wysiwyg-cm6 .cm-md-h1) { font-size: 2.0em; font-weight: 700; line-height: 1.25; }
  :global(.md-wysiwyg-cm6 .cm-md-h2) { font-size: 1.6em; font-weight: 700; line-height: 1.3; }
  :global(.md-wysiwyg-cm6 .cm-md-h3) { font-size: 1.3em; font-weight: 600; line-height: 1.35; }
  :global(.md-wysiwyg-cm6 .cm-md-h4) { font-size: 1.15em; font-weight: 600; line-height: 1.4; }
  :global(.md-wysiwyg-cm6 .cm-md-h5) { font-size: 1.0em; font-weight: 600; line-height: 1.4; }
  :global(.md-wysiwyg-cm6 .cm-md-h6) { font-size: 0.95em; font-weight: 600; line-height: 1.4; color: var(--text-secondary); }

  /* find-on-page (mirror of Source/WYSIWYG) */
  :global(.md-wysiwyg-cm6 .find-match) {
    background: var(--find-match-bg, rgba(255, 213, 0, 0.45));
    border-radius: 2px;
  }
  :global(.md-wysiwyg-cm6 .find-match--current) {
    background: var(--find-match-current-bg, rgba(255, 140, 0, 0.65));
    outline: 1px solid var(--find-match-current-border, rgba(180, 80, 0, 0.9));
  }
</style>
