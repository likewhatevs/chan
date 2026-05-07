<script lang="ts">
  // CodeMirror 6 source mode. Same backing buffer as the WYSIWYG view; the
  // user toggles per-tab. Markdown grammar gives basic highlighting.
  //
  // The CM theme follows the app theme via a `Compartment` so we can
  // reconfigure on toggle without rebuilding the editor.

  import { onDestroy, onMount } from "svelte";
  import { Compartment, EditorState } from "@codemirror/state";
  import { EditorView, keymap, lineNumbers } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
  import { markdown } from "@codemirror/lang-markdown";
  import { syntaxHighlighting, defaultHighlightStyle } from "@codemirror/language";
  import { oneDark } from "@codemirror/theme-one-dark";
  import {
    SearchQuery,
    findNext as cmFindNext,
    findPrevious as cmFindPrev,
    getSearchQuery,
    search,
    setSearchQuery,
  } from "@codemirror/search";
  import { drive, ui } from "../state/store.svelte";

  // Editor density follows the user's line_spacing pref. Same hook
  // the Wysiwyg side uses (see Wysiwyg.svelte:820), exposed here as
  // a `data-density` attribute on .md-source so the CSS rules below
  // can dial line-height between tight (gdocs-like) and standard
  // (older roomier) without rebuilding the CodeMirror editor.
  const density = $derived(drive.info?.preferences?.line_spacing ?? "tight");

  let { value = $bindable("") }: { value: string } = $props();

  let host: HTMLDivElement | undefined;
  let view: EditorView | undefined;
  let applyingExternal = false;
  const themeCompartment = new Compartment();

  /// In-document find API. Mirrors the Wysiwyg side so FileEditorTab
  /// can drive either editor through a single FindBar. CodeMirror's
  /// own panel UI is suppressed (we don't add searchKeymap to the
  /// extensions); we just feed it `setSearchQuery` and step through
  /// matches with cmFindNext / cmFindPrev. CM6 owns the highlight
  /// rendering via the `search()` extension.
  ///
  /// Returns `{ matches, current }` for the FindBar's "n of total"
  /// indicator. CM6 doesn't expose total match count directly, so
  /// we count by scanning the doc with the same query the panel
  /// would have used.
  export type FindSnapshot = { matches: number; current: number };

  function snapshot(): FindSnapshot {
    if (!view) return { matches: 0, current: 0 };
    const q = getSearchQuery(view.state);
    if (!q.search) return { matches: 0, current: 0 };
    const re = q.caseSensitive
      ? new RegExp(escapeRe(q.search), "g")
      : new RegExp(escapeRe(q.search), "gi");
    const text = view.state.doc.toString();
    let total = 0;
    let current = 0;
    const cursorPos = view.state.selection.main.from;
    let m: RegExpExecArray | null;
    while ((m = re.exec(text)) !== null) {
      total += 1;
      if (m.index <= cursorPos) current = total;
      // Defend against zero-length matches (unlikely with plain
      // text; `setSearchQuery` rejects them) by advancing manually.
      if (m.index === re.lastIndex) re.lastIndex += 1;
    }
    return { matches: total, current: total === 0 ? 0 : Math.max(1, current) };
  }

  function escapeRe(s: string): string {
    return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  }

  export function findSetQuery(query: string, caseSensitive: boolean): FindSnapshot {
    if (!view) return { matches: 0, current: 0 };
    view.dispatch({
      effects: setSearchQuery.of(
        new SearchQuery({
          search: query,
          caseSensitive,
          // Plain text only; FindBar exposes a single Aa toggle
          // and no regex switch yet.
          regexp: false,
          // Wrap so Enter at the bottom comes back around to top.
          wholeWord: false,
        }),
      ),
    });
    if (query) {
      // Move the selection to the first match without opening
      // CM6's own search panel. cmFindNext returns true if a
      // match was found.
      cmFindNext(view);
    }
    return snapshot();
  }

  export function findStep(delta: number): FindSnapshot {
    if (!view) return { matches: 0, current: 0 };
    if (delta > 0) cmFindNext(view);
    else cmFindPrev(view);
    return snapshot();
  }

  export function findClear(): void {
    if (!view) return;
    view.dispatch({
      effects: setSearchQuery.of(new SearchQuery({ search: "" })),
    });
  }

  export function findFocus(): void {
    view?.focus();
  }

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

  function themeExtensions(theme: "light" | "dark") {
    // Bg paints on the outer host (`.md-source`) only; the CM
    // internals stay transparent so a short doc doesn't show two
    // different darks where `.cm-content` (sized to longest line)
    // ends and the parent's bg takes over. Theme ordering inside
    // CM's class injection isn't reliable across versions, so
    // the actual transparency rule lives in plain CSS at the
    // bottom of this file with `!important`; the theme block
    // below only carries non-bg styling.
    if (theme === "dark") return [oneDark];
    return [
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
      EditorView.theme({
        "&": { color: "var(--text)" },
        ".cm-gutters": {
          backgroundColor: "var(--bg-card)",
          color: "var(--text-secondary)",
          border: "none",
        },
        ".cm-activeLineGutter": { backgroundColor: "var(--hover-bg)" },
        ".cm-cursor": { borderLeftColor: "var(--text)" },
      }),
    ];
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
        // Highlight machinery for FindBar. We deliberately
        // omit `searchKeymap` and don't show CM6's built-in
        // panel: the bar lives in FileEditorTab and routes Cmd+F
        // through `findSetQuery` etc. to keep the WYSIWYG and
        // Source experiences visually identical.
        search(),
        themeCompartment.of(themeExtensions(ui.theme)),
        EditorView.lineWrapping,
        EditorView.updateListener.of((u) => {
          if (applyingExternal) return;
          if (u.docChanged) value = u.state.doc.toString();
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
    if (!view) return;
    const cur = view.state.doc.toString();
    if (cur !== value) {
      applyingExternal = true;
      view.dispatch({
        changes: { from: 0, to: cur.length, insert: value },
        selection: { anchor: 0 },
      });
      applyingExternal = false;
      view.focus();
    }
  });

  // Reconfigure the theme compartment whenever the app theme flips.
  $effect(() => {
    if (!view) return;
    view.dispatch({
      effects: themeCompartment.reconfigure(themeExtensions(ui.theme)),
    });
  });
</script>

<div class="md-source" data-density={density} bind:this={host}></div>

<style>
  /* Keep the CodeMirror chrome wrapper themed. The CM6 editor itself
     uses its default light highlight style for now (see v1.1 polish). */
  .md-source {
    height: 100%;
    overflow: auto;
    background: var(--bg);
    /* Reserve room for the mobile floating bar on whichever edge
       it currently sits; vars set on `.mobile-shell`, default to
       0px on desktop. */
    padding-top: var(--mobile-bar-pad-top, 0px);
    padding-bottom: var(--mobile-bar-pad-bottom, 0px);
    box-sizing: border-box;
  }
  /* Source mode uses the drive's "code" font preference (it is
     a code editor, after all). */
  :global(.md-source .cm-editor) {
    height: 100%;
    font-size: var(--chan-font-code-size, 14px);
  }
  :global(.md-source .cm-content) {
    font-family: var(--chan-font-code-family);
    /* Center content within the cap when --chan-page-max-width is
       set (per-device pref written by state/pageWidth). When unset,
       max-width: none restores the original full-width behavior. */
    max-width: var(--chan-page-max-width, none);
    margin-inline: auto;
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
</style>
