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
  import { Compartment, EditorState, Prec } from "@codemirror/state";
  import { EditorView, keymap } from "@codemirror/view";
  import { syntaxTree } from "@codemirror/language";
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
  import {
    imageCaretRedirect,
    imageDecorations,
    type ImageClickArgs,
  } from "./widgets/image";
  import { tableDecorations } from "./widgets/table";
  import { bubbleKeymap, bubbleListener } from "./bubbles/controller";
  import type { BubbleHandle, BubbleSpec } from "./bubbles/types";
  import { openWikiBubble } from "./bubbles/wiki";
  import { openTagBubble } from "./bubbles/tag";
  import { openContactBubble } from "./bubbles/contact";
  import { openImageBubble } from "./bubbles/image";
  import { imageDropHandlers } from "./bubbles/image_drop";
  import { htmlPasteHandler } from "./paste_html";
  import { openImageZoom } from "../state/imageZoom";
  import { headingFold } from "./fold";
  import * as fmt from "./commands/format";
  import type { BlockKind } from "./commands/format";
  import type { FindAdapter } from "./find";

  let {
    value = $bindable(""),
    readonly = false,
    currentPath = null,
    wikiPickerPrefix = null,
    onSubmit,
    onSelectionChange,
    onTagClick = () => {},
    onWikiClick = () => {},
    onImageClick = () => {},
  }: {
    value: string;
    readonly?: boolean;
    currentPath?: string | null;
    wikiPickerPrefix?: string | null;
    onSubmit?: () => void;
    onSelectionChange?: () => void;
    onTagClick?: (tag: string) => void;
    onWikiClick?: (args: WikiLinkClickArgs) => void;
    onImageClick?: (args: ImageClickArgs) => void;
  } = $props();

  const density = $derived(drive.info?.preferences?.line_spacing ?? "tight");

  let host: HTMLDivElement | undefined;
  let view: EditorView | undefined;
  const sync = createValueSync();
  const theme = makeThemeCompartment(ui.theme);
  const editableCompartment = new Compartment();

  /// Active bubble handle (or null when no bubble is open). Updated by
  /// the controller's onSpec callback; the keymap reads it via the
  /// `() => activeBubble` accessor so each keydown sees the live
  /// reference (no stale closure).
  let activeBubble: BubbleHandle | null = null;
  let activeKind: BubbleSpec["kind"] | null = null;

  /// Image atom Cmd/Ctrl-click handler. The image widget only fires
  /// onImageClick on Cmd/Ctrl-click now (plain click drops the caret
  /// inside the URL so the image bubble auto-opens via the
  /// imageUrlAtCaret trigger). We treat Cmd/Ctrl-click as "open" —
  /// route to the existing image-zoom modal.
  function handleImageClick(args: ImageClickArgs): void {
    openImageZoom(args.src, currentPath);
    onImageClick(args);
  }

  function handleSpec(spec: BubbleSpec | null): void {
    if (!view) return;
    if (spec === null) {
      if (activeBubble) {
        activeBubble.dismiss();
        activeBubble = null;
        activeKind = null;
      }
      return;
    }
    // Same bubble kind already open: update its query / trigger end
    // in place. Different kind or no bubble open: dismiss the old
    // and mount fresh.
    if (activeBubble && activeKind === spec.kind) {
      activeBubble.setQuery(spec.query);
      // Cast: only wiki bubble carries setTriggerEnd today; harmless
      // for others until they implement the same shape.
      const ext = activeBubble as BubbleHandle & {
        setTriggerEnd?: (end: number) => void;
      };
      ext.setTriggerEnd?.(spec.triggerEnd);
      return;
    }
    if (activeBubble) {
      activeBubble.dismiss();
      activeBubble = null;
      activeKind = null;
    }
    const onDismiss = () => {
      activeBubble = null;
      activeKind = null;
    };
    if (spec.kind === "wiki") {
      activeBubble = openWikiBubble({
        view,
        triggerStart: spec.triggerStart,
        triggerEnd: spec.triggerEnd,
        initialQuery: spec.query,
        prefix: wikiPickerPrefix,
        templateMode: spec.templateMode ?? "wrap",
        onOpenLink: (target, anchor) =>
          onWikiClick({
            target,
            label: target,
            anchor: anchor ?? "",
            wasAbs: target.startsWith("/"),
            openInNewPane: false,
          }),
        onDismiss,
      });
      activeKind = "wiki";
    } else if (spec.kind === "tag") {
      activeBubble = openTagBubble({
        view,
        triggerStart: spec.triggerStart,
        triggerEnd: spec.triggerEnd,
        initialQuery: spec.query,
        onDismiss,
      });
      activeKind = "tag";
    } else if (spec.kind === "contact") {
      activeBubble = openContactBubble({
        view,
        triggerStart: spec.triggerStart,
        triggerEnd: spec.triggerEnd,
        initialQuery: spec.query,
        onDismiss,
      });
      activeKind = "contact";
    } else if (spec.kind === "image") {
      activeBubble = openImageBubble({
        view,
        triggerStart: spec.triggerStart,
        triggerEnd: spec.triggerEnd,
        initialQuery: spec.query,
        uploadDir: dirOf(currentPath),
        currentPath,
        templateMode: spec.templateMode ?? "wrap",
        onOpenLink: (path) => openImageZoom(path, currentPath),
        onDismiss,
      });
      activeKind = "image";
    }
  }

  function dirOf(p: string | null): string | null {
    if (!p) return null;
    const idx = p.lastIndexOf("/");
    return idx <= 0 ? null : p.slice(0, idx);
  }

  /// Find-on-page adapter (same shape as Source.svelte and the legacy
  /// WYSIWYG; FileEditorTab passes whichever editor is mounted to
  /// FindBar).
  export const findAdapter: FindAdapter = makeFindAdapter(() => view);

  /// Style-toolbar contract. Each method routes to the corresponding
  /// editor-cm6/commands/format function with the live view ref.
  /// Mirrors the legacy editor's exported imperative API so
  /// StyleToolbar.svelte works at cutover with no edits.
  export function toggleBold(): void { if (view) fmt.toggleBold(view); }
  export function toggleItalic(): void { if (view) fmt.toggleItalic(view); }
  export function toggleStrike(): void { if (view) fmt.toggleStrike(view); }
  export function toggleInlineCode(): void { if (view) fmt.toggleInlineCode(view); }
  export function toggleBulletList(): void { if (view) fmt.toggleBulletList(view); }
  export function toggleOrderedList(): void { if (view) fmt.toggleOrderedList(view); }
  export function toggleTaskList(): void { if (view) fmt.toggleTaskList(view); }
  export function setBlockKind(kind: BlockKind): void {
    if (view) fmt.setBlockKind(view, kind);
  }
  export function insertHorizontalRule(): void {
    if (view) fmt.insertHorizontalRule(view);
  }
  export function insertImage(): void { if (view) fmt.insertImage(view); }
  export function toggleLink(url?: string): void {
    if (view) fmt.toggleLink(view, url);
  }
  export function isActive(name: string): boolean {
    return view ? fmt.isActive(view, name) : false;
  }
  export function currentBlockKind(): BlockKind {
    return view ? fmt.currentBlockKind(view) : "normal";
  }
  /// Place caret at end of doc and focus. Used by InlineAssist after
  /// content insertion / paste so the user can keep typing.
  export function focusEnd(): void {
    if (!view) return;
    const end = view.state.doc.length;
    view.dispatch({ selection: { anchor: end } });
    view.focus();
  }

  onMount(() => {
    if (!host) return;
    const state = EditorState.create({
      doc: value,
      extensions: [
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        chanMarkdown(),
        headingFold(),
        theme.extension,
        EditorView.lineWrapping,
        findField,
        chanDecorations(),
        tagDecorations({ onTagClick }),
        dateDecorations(),
        wikiLinkDecorations({
          onWikiClick,
          getCurrentPath: () => currentPath,
        }),
        imageDecorations({
          getCurrentPath: () => currentPath,
          onImageClick: handleImageClick,
        }),
        imageCaretRedirect(),
        tableDecorations(),
        bubbleListener({ onSpec: handleSpec }),
        bubbleKeymap(() => activeBubble),
        imageDropHandlers({
          getUploadDir: () => dirOf(currentPath),
          getCurrentPath: () => currentPath,
        }),
        // HTML-paste handler runs ahead of CM6's default plain-text
        // paste so rich pastes get converted to markdown. Image-file
        // pastes (clipboard with image/* MIME) are owned by
        // imageDropHandlers; this handler skips them.
        htmlPasteHandler(),
        editableCompartment.of(EditorView.editable.of(!readonly)),
        EditorView.updateListener.of((u) => {
          sync.onDocChanged(u, (s) => (value = s));
          if (u.docChanged || u.selectionSet) {
            onSelectionChange?.();
          }
        }),
        // Cmd/Ctrl+Enter -> onSubmit (assistant prompt). Registered
        // via Prec.high so it beats CM6 default Enter (which would
        // insert a newline first). Returning true consumes the event.
        Prec.high(
          keymap.of([
            {
              key: "Mod-Enter",
              run: () => {
                onSubmit?.();
                return true;
              },
            },
          ]),
        ),
      ],
    });
    view = new EditorView({ state, parent: host });
    view.dispatch({ selection: { anchor: 0 } });
    view.focus();
  });

  onDestroy(() => {
    if (activeBubble) activeBubble.dismiss();
    view?.destroy();
  });

  $effect(() => {
    sync.applyExternal(view, value);
  });

  $effect(() => {
    if (!view) return;
    theme.reconfigure(view, ui.theme);
  });

  // Reconfigure editability when the readonly prop flips. Runs in its
  // own effect so theme reconfigs don't bundle with it.
  $effect(() => {
    if (!view) return;
    view.dispatch({
      effects: editableCompartment.reconfigure(
        EditorView.editable.of(!readonly),
      ),
    });
  });

  /// Scroll to the i-th heading (0-based, document order). Called by
  /// the inspector outline when the user picks a heading.
  export function scrollToHeading(i: number): void {
    if (!view) return;
    const headings: number[] = [];
    syntaxTree(view.state).iterate({
      enter(node) {
        if (
          node.name === "ATXHeading1" ||
          node.name === "ATXHeading2" ||
          node.name === "ATXHeading3" ||
          node.name === "ATXHeading4" ||
          node.name === "ATXHeading5" ||
          node.name === "ATXHeading6"
        ) {
          headings.push(node.from);
        }
      },
    });
    const target = headings[Math.max(0, Math.min(i, headings.length - 1))];
    if (target === undefined) return;
    view.dispatch({
      selection: { anchor: target },
      effects: EditorView.scrollIntoView(target, { y: "start" }),
    });
    view.focus();
  }
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
  /* CM6 paints `outline: 1px dotted` on .cm-editor.cm-focused as a
     focus indicator. We don't want it — the cursor itself is
     indicator enough, and the dotted outline spans the editor's
     entire bounding box including the gutter, which looks like a
     vertical divider in the gutter column. */
  :global(.md-wysiwyg-cm6 .cm-editor.cm-focused) {
    outline: none !important;
  }
  /* Fold gutter: drop the default greyish gutter background. The
     chevron itself stays clickable; only the container goes
     transparent so the editor bg shows through. */
  :global(.md-wysiwyg-cm6 .cm-gutters),
  :global(.md-wysiwyg-cm6 .cm-foldGutter),
  :global(.md-wysiwyg-cm6 .cm-foldGutter .cm-gutterElement) {
    background: transparent !important;
    border: none !important;
  }
  :global(.md-wysiwyg-cm6 .cm-foldGutter .cm-gutterElement) {
    color: var(--text-secondary, #888);
    cursor: pointer;
    padding: 0 0.25em;
  }
  :global(.md-wysiwyg-cm6 .cm-foldGutter .cm-gutterElement:hover) {
    color: var(--text);
  }
  /* Folded-heading inline placeholder ("..." dropped at EOL when a
     heading collapses). CM6's baseTheme paints it as a bordered chip
     with #eee bg + #ddd border which reads as too heavy. Switch to a
     light, borderless `…` that hints at the fold without competing
     with the heading text. */
  :global(.md-wysiwyg-cm6 .cm-foldPlaceholder) {
    background: transparent;
    border: none;
    color: var(--text-secondary, #aaa);
    margin: 0 0.25em;
    padding: 0 0.2em;
    font-weight: normal;
    cursor: pointer;
  }
  :global(.md-wysiwyg-cm6 .cm-foldPlaceholder:hover) {
    color: var(--text);
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
  :global(.md-wysiwyg-cm6 .cm-md-frontmatter) {
    color: var(--text-secondary, #888);
    font-family: var(--chan-font-code-family, monospace);
    font-size: 0.88em;
    opacity: 0.7;
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
  /* Kind variants. data-refkind populates after the async resolve
     lands; pills default to file styling until then. */
  :global(.md-wysiwyg-cm6 .cm-md-wiki-pill[data-refkind="contact"]) {
    background: var(--contact-bg, rgba(255, 170, 100, 0.20));
    color: var(--contact-fg, #b35f10);
  }
  :global(.md-wysiwyg-cm6 .cm-md-wiki-pill[data-refkind="image"]) {
    background: var(--image-bg, rgba(120, 200, 120, 0.20));
    color: var(--image-fg, #2a7d2a);
  }
  :global(.md-wysiwyg-cm6 .cm-md-wiki-pill-image) {
    display: inline-block;
    padding: 0;
    background: none;
    line-height: 0;
    vertical-align: middle;
    border-radius: 4px;
    overflow: hidden;
    max-width: 160px;
    max-height: 96px;
  }
  :global(.md-wysiwyg-cm6 .cm-md-wiki-pill-image img) {
    max-width: 160px;
    max-height: 96px;
    object-fit: contain;
    display: block;
  }
  :global(.md-wysiwyg-cm6 .cm-md-wiki-pill[data-refkind="broken"]) {
    background: var(--broken-bg, rgba(220, 80, 80, 0.18));
    color: var(--broken-fg, #b32020);
    text-decoration: line-through;
  }
  :global(.md-wysiwyg-cm6 .cm-md-image-wrap) {
    display: inline-block;
    position: relative;
    line-height: 0;
    max-width: 100%;
  }
  /* Inline mode (image mixed with paragraph text). Alignment makes
     the image float so surrounding text wraps around it. */
  :global(.md-wysiwyg-cm6 .cm-md-image-wrap:not([data-standalone])[data-align="left"]) {
    display: block;
    float: left;
    margin-right: 1em;
  }
  :global(.md-wysiwyg-cm6 .cm-md-image-wrap:not([data-standalone])[data-align="right"]) {
    display: block;
    float: right;
    margin-left: 1em;
  }
  /* Standalone mode (image alone on its source line). Alignment
     positions the image within the line via flex justify-content —
     no float, no text wrap. Default is centered. */
  :global(.md-wysiwyg-cm6 .cm-md-image-wrap[data-standalone="true"]) {
    display: flex;
    width: 100%;
    justify-content: center;
  }
  :global(.md-wysiwyg-cm6 .cm-md-image-wrap[data-standalone="true"][data-align="left"]) {
    justify-content: flex-start;
  }
  :global(.md-wysiwyg-cm6 .cm-md-image-wrap[data-standalone="true"][data-align="right"]) {
    justify-content: flex-end;
  }
  :global(.md-wysiwyg-cm6 .cm-md-image-wrap img) {
    max-width: 100%;
    height: auto;
    display: block;
    border-radius: 4px;
  }
  :global(.md-wysiwyg-cm6 .cm-md-image-handle) {
    position: absolute;
    right: -4px;
    bottom: -4px;
    width: 12px;
    height: 12px;
    background: var(--text-secondary, #888);
    border: 2px solid var(--bg, #fff);
    border-radius: 50%;
    cursor: nwse-resize;
    opacity: 0;
    transition: opacity 0.15s;
  }
  :global(.md-wysiwyg-cm6 .cm-md-image-wrap:hover .cm-md-image-handle) {
    opacity: 1;
  }
  /* Hover action overlay (Edit / Zoom). Visible only when the mouse
     is over the image wrap; uses the same bouncy reveal as the other
     bubble surfaces. */
  :global(.md-wysiwyg-cm6 .cm-md-image-actions) {
    position: absolute;
    top: 4px;
    right: 4px;
    display: flex;
    gap: 4px;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.15s, transform 200ms cubic-bezier(0.34, 1.56, 0.64, 1);
    transform: scale(0.95);
    transform-origin: top right;
    line-height: 1;
  }
  :global(.md-wysiwyg-cm6 .cm-md-image-wrap:hover .cm-md-image-actions) {
    opacity: 1;
    pointer-events: auto;
    transform: scale(1);
  }
  :global(.md-wysiwyg-cm6 .cm-md-image-action) {
    background: var(--bg-card, #fff);
    border: 1px solid var(--border, #ddd);
    border-radius: 4px;
    padding: 3px 10px;
    font-family: inherit;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.12);
    transition: transform 200ms cubic-bezier(0.34, 1.56, 0.64, 1),
      background 0.12s;
  }
  :global(.md-wysiwyg-cm6 .cm-md-image-action:hover) {
    background: var(--hover-bg, rgba(0, 0, 0, 0.06));
    transform: scale(1.05);
  }

  /* ---- table widget ---- */
  :global(.md-wysiwyg-cm6 .cm-md-table-wrap) {
    overflow-x: auto;
    margin: 0.5em 0;
  }
  :global(.md-wysiwyg-cm6 .cm-md-table) {
    border-collapse: collapse;
    font-size: 0.95em;
  }
  :global(.md-wysiwyg-cm6 .cm-md-table th),
  :global(.md-wysiwyg-cm6 .cm-md-table td) {
    border: 1px solid var(--border, #ddd);
    padding: 0.3em 0.6em;
    text-align: left;
    vertical-align: top;
  }
  :global(.md-wysiwyg-cm6 .cm-md-table th) {
    background: var(--bg-card, rgba(0, 0, 0, 0.04));
    font-weight: 600;
  }
  :global(.md-wysiwyg-cm6 .cm-md-table tr:nth-child(even) td) {
    background: var(--bg-card, rgba(0, 0, 0, 0.02));
  }

  /* ---- bubble shells ---- */
  :global(.md-bubble.cm-bubble) {
    background: var(--bg-card, #fff);
    border: 1px solid var(--border, #ddd);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
    min-width: 240px;
    max-width: 480px;
    padding: 4px;
    font-family: var(--chan-font-text-family);
    font-size: 14px;
    /* Bouncy reveal + hover wobble — matches the tab-menu bubble's
       easeOutBack motion so the editor's pickers feel of-a-piece. */
    transform-origin: top left;
    animation: cm-bubble-pop 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
    transition: transform 200ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  :global(.md-bubble.cm-bubble:hover) {
    transform: scale(1.015);
  }
  @keyframes cm-bubble-pop {
    0% {
      opacity: 0;
      transform: scale(0.92);
    }
    100% {
      opacity: 1;
      transform: scale(1);
    }
  }
  :global(.md-bubble .md-bubble-list) {
    display: flex;
    flex-direction: column;
  }
  :global(.md-bubble .md-bubble-row) {
    padding: 6px 8px;
    border-radius: 4px;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  :global(.md-bubble .md-bubble-row:hover) {
    background: var(--hover-bg, rgba(0, 0, 0, 0.04));
  }
  :global(.md-bubble .md-bubble-row-selected) {
    background: var(--accent-bg, rgba(106, 168, 255, 0.18));
    color: var(--accent, #2563b8);
  }
  :global(.md-bubble .md-bubble-status) {
    padding: 4px 8px;
    color: var(--text-secondary, #888);
    font-size: 12px;
    border-top: 1px solid var(--border, #eee);
    margin-top: 2px;
  }
  :global(.md-bubble .md-bubble-row-sub) {
    color: var(--text-secondary, #888);
    font-size: 12px;
  }
  :global(.md-bubble .md-bubble-row-level) {
    display: inline-block;
    min-width: 2em;
    margin-right: 0.5em;
    color: var(--text-secondary, #888);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
  }
  :global(.md-bubble .md-bubble-actions) {
    border-bottom: 1px solid var(--border, #eee);
    margin-bottom: 4px;
    padding-bottom: 4px;
  }
  :global(.md-bubble .md-bubble-action) {
    color: var(--accent, #2563b8);
    font-weight: 500;
  }
  :global(.md-image-preview) {
    display: block;
    max-width: 100%;
    margin-bottom: 4px;
    border-bottom: 1px solid var(--border, #eee);
    padding-bottom: 4px;
    text-align: center;
    line-height: 0;
  }
  :global(.md-image-preview img) {
    max-width: 100%;
    max-height: 140px;
    object-fit: contain;
    border-radius: 4px;
  }
  :global(.md-image-align-row) {
    display: flex;
    gap: 4px;
    padding: 4px;
    border-bottom: 1px solid var(--border, #eee);
    margin-bottom: 4px;
  }
  :global(.md-image-align-btn) {
    flex: 1;
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 4px 6px;
    font-family: inherit;
    font-size: 14px;
    color: var(--text);
    cursor: pointer;
  }
  :global(.md-image-align-btn:hover) {
    background: var(--hover-bg, rgba(0, 0, 0, 0.06));
    border-color: var(--border, #ddd);
  }
  :global(.md-image-align-btn-active) {
    background: var(--accent-bg, rgba(106, 168, 255, 0.18));
    color: var(--accent, #2563b8);
    border-color: var(--accent, #2563b8);
  }
  :global(.md-image-action-overlay) {
    display: flex;
    gap: 4px;
    background: var(--bg-card, #fff);
    border: 1px solid var(--border, #ddd);
    border-radius: 6px;
    padding: 4px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }
  :global(.md-image-action-btn) {
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 4px 10px;
    font-family: inherit;
    font-size: 12px;
    color: var(--text);
    cursor: pointer;
  }
  :global(.md-image-action-btn:hover) {
    background: var(--hover-bg, rgba(0, 0, 0, 0.06));
    border-color: var(--border, #ddd);
  }

  /* ---- link action popover ---- */
  :global(.md-link-action) {
    background: var(--bg-card, #fff);
    border: 1px solid var(--border, #ddd);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
    padding: 6px;
    min-width: 220px;
    max-width: 360px;
    font-family: var(--chan-font-text-family);
    font-size: 13px;
  }
  :global(.md-link-action-target) {
    padding: 4px 6px 6px;
    color: var(--text-secondary, #666);
    font-family: var(--chan-font-code-family, monospace);
    font-size: 12px;
    border-bottom: 1px solid var(--border, #eee);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  :global(.md-link-action-buttons) {
    display: flex;
    flex-direction: column;
    margin-top: 4px;
  }
  :global(.md-link-action-btn) {
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 6px 8px;
    text-align: left;
    font-family: inherit;
    font-size: 13px;
    color: var(--text);
    cursor: pointer;
  }
  :global(.md-link-action-btn:hover) {
    background: var(--hover-bg, rgba(0, 0, 0, 0.06));
  }

  /* ---- date popover ---- */
  :global(.md-date-popover) {
    background: var(--bg-card, #fff);
    border: 1px solid var(--border, #ddd);
    border-radius: 6px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
    padding: 8px;
    min-width: 240px;
    font-family: var(--chan-font-text-family);
    font-size: 13px;
    transform-origin: top left;
    animation: cm-bubble-pop 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
    transition: transform 200ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  :global(.md-date-popover:hover) {
    transform: scale(1.015);
  }
  :global(.md-date-header) {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 4px 6px;
    border-bottom: 1px solid var(--border, #eee);
    margin-bottom: 6px;
  }
  :global(.md-date-title) {
    font-weight: 600;
  }
  :global(.md-date-nav) {
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 2px 8px;
    font-size: 16px;
    line-height: 1;
    color: var(--text);
    cursor: pointer;
  }
  :global(.md-date-nav:hover) {
    background: var(--hover-bg, rgba(0, 0, 0, 0.06));
    border-color: var(--border, #ddd);
  }
  :global(.md-date-grid) {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 2px;
  }
  :global(.md-date-dow) {
    text-align: center;
    color: var(--text-secondary, #888);
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    padding: 4px 0;
  }
  :global(.md-date-blank) {
    visibility: hidden;
  }
  :global(.md-date-day) {
    background: none;
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 6px 0;
    font-family: inherit;
    font-size: 13px;
    color: var(--text);
    cursor: pointer;
    text-align: center;
  }
  :global(.md-date-day:hover) {
    background: var(--hover-bg, rgba(0, 0, 0, 0.06));
  }
  :global(.md-date-day-today) {
    border-color: var(--accent, #2563b8);
  }
  :global(.md-date-day-selected) {
    background: var(--accent, #2563b8);
    color: var(--bg-card, #fff);
  }
  :global(.md-date-format-row) {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
    padding-top: 6px;
    border-top: 1px solid var(--border, #eee);
  }
  :global(.md-date-format-label) {
    color: var(--text-secondary, #888);
    font-size: 12px;
  }
  :global(.md-date-format-select) {
    flex: 1;
    background: var(--bg-card, #fff);
    border: 1px solid var(--border, #ddd);
    border-radius: 4px;
    padding: 4px 6px;
    color: var(--text);
    font-family: inherit;
    font-size: 12px;
  }

  /* ---- heading line classes ---- */
  :global(.md-wysiwyg-cm6 .cm-md-h1) { font-size: 2.0em; font-weight: 700; line-height: 1.25; }
  :global(.md-wysiwyg-cm6 .cm-md-h2) { font-size: 1.6em; font-weight: 700; line-height: 1.3; }
  :global(.md-wysiwyg-cm6 .cm-md-h3) { font-size: 1.3em; font-weight: 600; line-height: 1.35; }
  :global(.md-wysiwyg-cm6 .cm-md-h4) { font-size: 1.15em; font-weight: 600; line-height: 1.4; }
  :global(.md-wysiwyg-cm6 .cm-md-h5) { font-size: 1.0em; font-weight: 600; line-height: 1.4; }
  :global(.md-wysiwyg-cm6 .cm-md-h6) { font-size: 0.95em; font-weight: 600; line-height: 1.4; color: var(--text-secondary); }
  /* defaultHighlightStyle paints tags.heading with text-decoration:
     underline. Strip it on every descendant of the heading line so
     the styled spans CM6 injects don't show the underline either. */
  :global(.md-wysiwyg-cm6 .cm-md-h1),
  :global(.md-wysiwyg-cm6 .cm-md-h2),
  :global(.md-wysiwyg-cm6 .cm-md-h3),
  :global(.md-wysiwyg-cm6 .cm-md-h4),
  :global(.md-wysiwyg-cm6 .cm-md-h5),
  :global(.md-wysiwyg-cm6 .cm-md-h6) {
    text-decoration: none;
  }
  :global(.md-wysiwyg-cm6 [class*="cm-md-h"] > span) {
    text-decoration: none !important;
  }

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
