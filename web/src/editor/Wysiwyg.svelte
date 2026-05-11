<script lang="ts">
  // TipTap-based WYSIWYG editor with smart-node extensions for
  // dates, wiki-links, and images. Two-way bound to the parent's
  // `value` (markdown text). Round-trips through tiptap-markdown.
  //
  // Trigger handling: we listen for input events. When the buffer
  // gains a known trigger, we insert the corresponding node and
  // clean up the trigger text:
  //
  //   `!/today`  -> date pill prefilled with today's date
  //   `!/date`   -> calendar picker; commits to a date pill
  //   `[[`       -> wiki-link picker (file autocomplete)
  //   `![`       -> image picker
  //
  // The `!/` two-char prefix for command-style insertions is the
  // chan convention (see project memory): collision-free with
  // prose, leaves single chars (`@`, `:`, `;`) free for other
  // surfaces. `@` is reserved for the contacts picker.

  import { onDestroy, onMount } from "svelte";
  import { Editor } from "@tiptap/core";
  import { NodeSelection, TextSelection } from "@tiptap/pm/state";
  import StarterKit from "@tiptap/starter-kit";
  import TaskList from "@tiptap/extension-task-list";
  import TaskItem from "@tiptap/extension-task-item";
  import Link from "@tiptap/extension-link";
  import { Markdown } from "tiptap-markdown";
  import { DateNode, showCalendar, type DatePick } from "./extensions/date";
  import {
    findDateMatches,
    formatDate,
    isoOf,
    type DateFormatId,
  } from "./dateFormats";
  import { createImageNode, resolveImageSrc } from "./extensions/image";
  import { openImageBubble, type ImageBubble } from "./extensions/imageBubble";
  import {
    createWikiLinkNode,
    handleWikiClick,
    openWikiBubble,
    type WikiBubble,
  } from "./extensions/wikiLink";
  import { openTagBubble, type TagBubble } from "./extensions/tagPicker";
  import {
    openContactBubble,
    type ContactBubble,
  } from "./extensions/contactPicker";
  import { type BubbleHandle } from "./bubble";
  import { CodeBlockFenced } from "./extensions/codeBlockFenced";
  import { FoldHeadingExtension } from "./extensions/foldHeading";
  import { LiveSourceExtension } from "./extensions/liveSource";
  import { createTagDecorationExtension } from "./extensions/tagDecoration";
  import { openGraphAtNode } from "../state/store.svelte";
  import { api } from "../api/client";
  import { normalizeHref, relativizePath, resolveRelativePath } from "./links";
  import { drive } from "../state/store.svelte";

  let {
    value = $bindable(""),
    readonly = false,
    onSubmit,
    onSelectionChange,
    wikiPickerPrefix = null,
    currentPath = null,
  }: {
    value: string;
    readonly?: boolean;
    /// When set, Cmd/Ctrl+Enter inside the editor calls onSubmit
    /// instead of inserting a hard break. Used by the assistant
    /// prompt; left undefined for the file editor where Cmd+Enter
    /// has no special meaning.
    onSubmit?: () => void;
    /// Fires whenever the selection or document changes. Lets the
    /// host (FileEditorTab) bump a local `$state` counter so its
    /// formatting-toolbar derivations re-run with the latest mark
    /// / node activity. We don't push the active-state map directly
    /// to keep coupling thin: the host calls back into our
    /// `isActive` / `currentBlockKind` helpers for whatever it
    /// needs.
    onSelectionChange?: () => void;
    /// Optional path prefix passed to the wiki-link autocomplete:
    /// when set, file suggestions are scoped to that subdirectory
    /// of the drive. The file editor passes the source file's
    /// `repoRoot` (when the file lives inside a git repo) so
    /// `[[note]]` autocomplete stays project-bound rather than
    /// spanning the whole drive.
    wikiPickerPrefix?: string | null;
    /// Drive-rooted POSIX path of the file this editor is bound
    /// to (e.g. `Recipes/Pasta.md`). Used by the wiki-link
    /// serializer to emit file-relative URLs (`./foo.md`) and by
    /// the parser to resolve them back to canonical drive-rooted
    /// targets. Null for editors with no source file (assistant
    /// prompt), in which case wiki-link URLs stay drive-rooted.
    currentPath?: string | null;
  } = $props();

  let host: HTMLDivElement | undefined;
  let editor: Editor | undefined;

  // Reactive read-only: tiptap caches `editable` from the initial
  // EditorOptions, so once-only constructor wiring doesn't follow
  // a parent prop flip. Mirror it through setEditable on every
  // prop change. Also dismiss the wiki bubble: it cannot commit
  // edits in a non-editable doc.
  //
  // Two subtle points:
  //
  // 1. Read `readonly` BEFORE the early-return so Svelte 5 always
  //    subscribes the effect to it. The first run can happen before
  //    onMount instantiates the editor; bailing on `!editor` first
  //    would leave `readonly` untracked and Tiptap stuck in its
  //    initial `editable` state.
  //
  // 2. Pass `emitUpdate=false` (and short-circuit when nothing
  //    needs to change) so setEditable doesn't dispatch a Tiptap
  //    update transaction. The default `emitUpdate=true` fires
  //    onUpdate, which writes `value` back through the bindable;
  //    that round-trip ripples into App.svelte's autosave effect
  //    and trips Svelte's effect-update-depth guard.
  $effect(() => {
    const ro = readonly;
    if (!editor) return;
    if (editor.isEditable === !ro) return;
    editor.setEditable(!ro, false);
    if (ro) {
      dismissWikiBubble();
      dismissImageBubble();
      dismissImageOverlay();
      dismissTagBubble();
      dismissContactBubble();
      clearCursorDecorations();
    }
  });

  /// Wiki-link bubble. Open while the caret sits between an
  /// auto-paired `[[ ]]` in the editor. The bubble is informational
  /// (no focus); the caret stays inside the brackets and the user's
  /// typing IS the search query. Wysiwyg owns the keyboard and
  /// selection-tracking; the bubble owns its own DOM and result
  /// list. Cleared when the caret leaves the bracket range, on
  /// Escape, on accept, or on editor destroy.
  let wikiBubble: WikiBubble | undefined;

  /// `#tag` autocomplete bubble. Opens while the caret sits at the
  /// end of a `#word` token in a non-heading, non-codeblock textblock.
  /// Same non-focus-stealing pattern as the wiki bubble: keyboard is
  /// owned by Wysiwyg; the bubble owns its own DOM and result list.
  let tagBubble: TagBubble | undefined;

  /// `@contact` picker bubble. Opens on a fresh `@` keystroke at
  /// start-of-word; replaces the `@<query>` range with a wiki-link
  /// to the chosen contact's note. Dismisses on Esc, on `@` then
  /// space (empty query), on the caret leaving the trigger range
  /// (different line / different block), or on accept.
  let contactBubble: ContactBubble | undefined;

  /// Elements currently carrying caret-driven `data-cursor-*` attrs.
  /// Tracked so `updateCursorDecorations` can wipe the previous set
  /// in O(N) without scanning the whole editor DOM.
  let cursorDecorated: HTMLElement[] = [];

  /// Last atom (image or wikiLink) whose edit popover was opened
  /// from a click / selection on it. One-shot guard so a single
  /// `NodeSelection` on the atom doesn't keep re-opening the
  /// popover every time `onSelectionUpdate` fires (the dismiss
  /// path leaves PM's selection where it was). Cleared as soon
  /// as the selection moves off the atom.
  let lastAtomEditPos: number | null = null;

  /// Direction the user came from when entering wiki edit mode
  /// via NodeSelection. ArrowLeft → entered from the right side
  /// of the pill, so on dismiss the caret should land BEFORE the
  /// restored atom (continuing leftward). ArrowRight → entered
  /// from the left, caret AFTER. Click and other paths default to
  /// "after". Cleared after each restore so a stale value can't
  /// influence the next edit.
  let wikiEditEntryDir: "before" | "after" = "after";
  /// Last horizontal arrow keypress, captured in `handleKeyDown`
  /// to decide `wikiEditEntryDir`. Reset to null on any non-arrow
  /// keystroke so a click-driven entry doesn't inherit a stale
  /// direction.
  let lastHorizontalArrow: "left" | "right" | null = null;

  /// Wiki link atom currently in edit mode. Click on the pill (or
  /// ArrowLeft/Right adjacency) replaces the atom with `[[label]]`
  /// text and lets the wiki bubble take over (the same flow the
  /// user gets when they type `[[`). Original atom attrs are saved
  /// here so a dismiss without accept can restore the link.
  let editingWikiOriginal:
    | { target: string; label: string; anchor: string; wasAbs: boolean }
    | null = null;
  /// Doc position of the opening `[` of the bracket pair we
  /// inserted on entry. Tracked so `restoreWikiEditOriginal` can
  /// find the brackets even after the user navigates the caret
  /// out of them (which used to defeat the selection-based
  /// `findBracketRange` lookup and leave the brackets unrestored
  /// in the doc, where they would round-trip as `\[\[…\]\]`).
  /// Mapped through every transaction in `onUpdate` so typing
  /// inside the brackets doesn't desync the saved position.
  let editingWikiBracketStart: number | null = null;

  /// Image bubble. Same non-focus-stealing pattern as the wiki
  /// bubble: open while the caret sits inside `![alt](src)` source
  /// text, dismiss when the caret leaves the range. Two sub-modes
  /// (path / alt) track which half of the markdown the caret is in.
  let imageBubble: ImageBubble | undefined;

  /// Snapshot of the image atom we replaced when entering edit
  /// mode. Mirrors `editingWikiOriginal`: a dismiss without accept
  /// restores the original atom rather than leaving stray
  /// `![alt](src)` text in the doc.
  let editingImageOriginal: { src: string; alt: string } | null = null;
  /// Doc position of the leading `!` of the `![alt](src)` text
  /// inserted on entry. Mapped through every transaction so typing
  /// inside the range doesn't desync the saved offset.
  let editingImageBracketStart: number | null = null;
  /// Alt text we pre-populated on edit entry. The accept path uses
  /// it to decide whether to auto-fill from the filename: an
  /// unchanged default counts as "no user input" and gets replaced
  /// by the picked file's basename.
  let editingImageDefaultAlt: string = "";

  /// Cleanup for the floating image action overlay (zoom + edit
  /// buttons shown when a rendered image is clicked). `undefined`
  /// when no overlay is open. The function tears down the DOM and
  /// removes the global listeners that drive its lifetime.
  let imageOverlayDismiss: (() => void) | undefined;

  /// Scroll the editor to the i-th heading (0-based, document order).
  /// Called by the inspector (outline view) via `bind:this` from
  /// the parent Pane.
  export function scrollToHeading(index: number): void {
    if (!host) return;
    const el = host.querySelector(`[data-heading-id="h-${index}"]`) as
      | HTMLElement
      | null;
    if (!el) return;
    el.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  // ---- formatting API ---------------------------------------------------
  // Thin pass-throughs over TipTap's chain commands. The toolbar in
  // FileEditorTab calls these on click and reads back state via
  // `isActive` / `currentBlockKind`.

  // No `.focus()` in these chains: the toolbar buttons are
  // expected to use onmousedown=preventDefault to keep the editor
  // focused, so the implicit `focus()` would just re-trigger
  // ProseMirror's scroll-into-view (the source of issue: clicking
  // inline-code on a selection used to scroll the page).
  export function toggleBold(): void { editor?.chain().toggleBold().run(); }
  export function toggleItalic(): void { editor?.chain().toggleItalic().run(); }
  export function toggleStrike(): void { editor?.chain().toggleStrike().run(); }
  export function toggleInlineCode(): void { editor?.chain().toggleCode().run(); }
  /// Lists are node-type toggles, not marks. TipTap collapses an
  /// existing list back to paragraphs on a second toggle and
  /// converts between bullet / ordered when the other is active,
  /// so a single chain command per direction is enough.
  export function toggleBulletList(): void { editor?.chain().toggleBulletList().run(); }
  export function toggleOrderedList(): void { editor?.chain().toggleOrderedList().run(); }
  export function toggleTaskList(): void { editor?.chain().toggleTaskList().run(); }

  /// Set the current block to a heading (1..6), a paragraph
  /// ("normal"), an inline-code-rich code block, or a blockquote.
  /// Idempotent: re-applying the same kind is a no-op in TipTap.
  export type BlockKind = "h1" | "h2" | "h3" | "normal" | "code" | "quote";
  export function setBlockKind(kind: BlockKind): void {
    if (!editor) return;
    const c = editor.chain();
    switch (kind) {
      case "h1": c.setHeading({ level: 1 }).run(); break;
      case "h2": c.setHeading({ level: 2 }).run(); break;
      case "h3": c.setHeading({ level: 3 }).run(); break;
      case "normal": c.setParagraph().run(); break;
      case "code": c.setCodeBlock().run(); break;
      case "quote": c.setBlockquote().run(); break;
    }
  }

  /// True when the named mark or node-type is active at the current
  /// selection. Wraps `editor.isActive()` so the toolbar doesn't
  /// have to import TipTap. Accepts mark names (bold / italic /
  /// strike / code) and node-type names (bulletList / orderedList /
  /// blockquote / codeBlock).
  export function isActive(name: string): boolean {
    return editor?.isActive(name) ?? false;
  }

  /// Identify the block at the cursor for the heading dropdown's
  /// current value. Falls back to "normal" when no block-level node
  /// matches (covers list items, doc root, etc.).
  export function currentBlockKind(): BlockKind {
    if (!editor) return "normal";
    if (editor.isActive("heading", { level: 1 })) return "h1";
    if (editor.isActive("heading", { level: 2 })) return "h2";
    if (editor.isActive("heading", { level: 3 })) return "h3";
    if (editor.isActive("blockquote")) return "quote";
    if (editor.isActive("codeBlock")) return "code";
    return "normal";
  }


  /// Two-flag guard against bind:value feedback loops:
  ///
  /// `applyingExternal` is true while we push the parent's `value`
  /// into the editor; `onUpdate` reads it and skips the
  /// `value = getMarkdown()` write-back so the parent's state
  /// doesn't echo right back into us.
  ///
  /// `lastSyncedValue` is the string we most recently pushed (or
  /// emitted on user edit). The sync $effect compares against
  /// this instead of `getMarkdown()` because tiptap-markdown's
  /// serializer is non-idempotent (it adds an extra `\n` after H1,
  /// among other things). Without the pin, a parse-then-serialize
  /// round-trip changes the byte string, `current !== value` stays
  /// permanently true, and the $effect re-runs setContent forever
  /// until Svelte's depth guard fires.
  let applyingExternal = false;
  let lastSyncedValue = "";

  onMount(() => {
    if (!host) return;
    editor = new Editor({
      element: host,
      editable: !readonly,
      extensions: [
        // Disable StarterKit's built-in CodeBlock so our
        // `CodeBlockFenced` (always-visible fences + editable
        // language) is the only code-block node in the schema.
        StarterKit.configure({ codeBlock: false }),
        CodeBlockFenced,
        // `nested: true` lets a task list contain another task list
        // when the user indents (Tab inside a task item). Mirrors
        // GitHub-flavored markdown task list semantics.
        TaskList,
        TaskItem.configure({ nested: true }),
        // `isAllowedUri` overridden so relative URLs land as Link
        // marks: the default validator only accepts known schemes
        // (http, https, mailto, etc.) and silently drops the mark
        // for anything else. Without this override, tiptap-markdown
        // parses `[Brazilian Rice](Recipes/Brazilian%20Rice.md)`
        // back from disk as plain text on tab swap because
        // `Recipes/...` has no protocol; `decorateWikiLinks` then
        // finds nothing to convert into a wikiLink pill, and the
        // editor renders the link as flat text. Accepting any
        // URI here is safe because we drive-rooted internal links
        // do not embed in `<a href>` for users to click — they
        // round-trip into wikiLink atom nodes whose own click
        // handler routes through `openInActivePane`.
        Link.configure({
          openOnClick: false,
          autolink: false,
          isAllowedUri: () => true,
          validate: () => true,
        }),
        Markdown.configure({ html: false, linkify: false, breaks: true }),
        DateNode,
        // Per-instance wikiLink extension. The factory closes over
        // `currentPath` (the prop, captured by reference each call)
        // so the markdown serializer always sees the latest path
        // even when the user swaps tabs into a new file.
        createWikiLinkNode(() => currentPath),
        // Same per-instance factory shape as wikiLink: the closure
        // gives the node view + renderHTML a live read on
        // `currentPath`, so a relative src like `../logo.png`
        // resolves against the editing file's directory.
        createImageNode(() => currentPath),
        // Heading fold/unfold via a chevron widget + a node-class
        // decoration that hides following blocks until the next
        // heading of equal-or-higher level. Pure UI state; the
        // markdown source is never touched.
        FoldHeadingExtension,
        // Live-preview decorations: heading prefix + bold / italic /
        // strike markers shown only when the caret is on / in the
        // element. PM-managed so re-renders by other plugins don't
        // wipe them.
        LiveSourceExtension,
        // `#tag` rendering as clickable pills. Click opens the
        // graph inspector pre-selected at the tag node so users
        // can see which documents share the tag. The id on a tag
        // graph node is `#<name>` (chan-server emits it that way),
        // so we rebuild it from the bare name passed back here.
        createTagDecorationExtension({
          onTagClick: (name) => openGraphAtNode(`#${name}`),
        }),
      ],
      content: value,
      // Cmd/Ctrl+Enter -> parent's onSubmit (assistant prompt
      // case). Drop / paste hooks funnel image files and image
      // URLs through `handleImageInsert` so the picker, drag-drop,
      // and clipboard paste flows all share one upload + node-
      // insert path.
      editorProps: {
        handleKeyDown: (_view, event) => {
          // Bubble keyboard routing: each adapter owns its Enter /
          // Escape / Arrow semantics behind `BubbleHandle.handleKey`.
          // Only one bubble is ever open at a time (the open paths
          // guard with `if (otherBubble) return`), so first-match
          // wins is enough; the host stays out of per-bubble accept
          // / dismiss logic.
          const activeBubble: BubbleHandle | undefined =
            wikiBubble ?? imageBubble ?? tagBubble ?? contactBubble;
          if (activeBubble?.handleKey(event)) {
            event.preventDefault();
            return true;
          }
          if (
            onSubmit &&
            (event.metaKey || event.ctrlKey) &&
            event.key === "Enter"
          ) {
            event.preventDefault();
            onSubmit();
            return true;
          }
          // Capture horizontal arrow direction so the wiki edit-
          // existing entry path can place the caret on the correct
          // side of the pill on dismiss.
          if (event.key === "ArrowLeft") lastHorizontalArrow = "left";
          else if (event.key === "ArrowRight") lastHorizontalArrow = "right";
          else lastHorizontalArrow = null;
          return false;
        },
        handleDrop: () => {
          return false;
        },
        handlePaste: (view, event) => {
          // Route clipboard images through the attachments endpoint
          // instead of letting Tiptap's `allowBase64` inline them as
          // a data: URI. The base64 path is fine for previewing in
          // memory but bloats the markdown source and never reaches
          // the drive, so the link breaks on the next reload.
          const cd = event.clipboardData;
          if (!cd) return false;
          const imageFiles = Array.from(cd.files).filter((f) =>
            f.type.startsWith("image/"),
          );
          if (imageFiles.length === 0) return false;
          event.preventDefault();
          const dir = dirOfPath(currentPath ?? null);
          const fromPath = currentPath ?? null;
          // Snapshot the insertion point at paste time. Subsequent
          // uploads are async; capturing the position now keeps the
          // images landing where the user pasted instead of wherever
          // the caret has wandered to by the time the first response
          // returns.
          const insertAt = view.state.selection.from;
          const imgType = view.state.schema.nodes.image;
          if (!imgType) return false;
          void (async () => {
            let cursor = insertAt;
            for (const file of imageFiles) {
              try {
                const { path } = await api.uploadAttachment(file, dir);
                // Drive-rooted path from the server; relativize
                // against the editing file so the markdown reads
                // `./name.png` like the bubble-driven insert.
                const src = fromPath ? relativizePath(path, fromPath) : path;
                const last = path.split("/").pop() ?? path;
                const alt = last.replace(/\.[^./]+$/, "");
                const tr = view.state.tr.insert(
                  cursor,
                  imgType.create({ src, alt }),
                );
                view.dispatch(tr);
                // Image atom is one position; advance the cursor so
                // a second pasted image lands AFTER the first.
                cursor += 1;
              } catch (e) {
                // eslint-disable-next-line no-console
                console.error("[paste] upload failed", e);
              }
            }
          })();
          return true;
        },
      },
      onUpdate: ({ editor, transaction }) => {
        // Keep the wiki edit-existing bracket-start position in
        // sync with the doc as the user types inside (or near)
        // the brackets. Without this, `restoreWikiEditOriginal`
        // would walk the doc from a stale offset and miss the
        // brackets, leaving them to round-trip as escaped text.
        if (editingWikiBracketStart !== null) {
          editingWikiBracketStart = transaction.mapping.map(
            editingWikiBracketStart,
          );
        }
        // Same mapping for the image bubble's saved bracket start.
        // Without it, `restoreImageEditOriginal` would walk the doc
        // from a stale offset and miss the `![alt](src)` text.
        if (editingImageBracketStart !== null) {
          editingImageBracketStart = transaction.mapping.map(
            editingImageBracketStart,
          );
        }
        if (applyingExternal) return;
        const raw = (editor.storage.markdown as { getMarkdown(): string }).getMarkdown();
        // Strip the NBSP-paragraph markers we injected on parse so
        // the file on disk stays clean (plain blank lines, no
        // invisible characters). The next reload re-injects them
        // through `preserveBlankParagraphs`.
        const md = stripBlankParagraphs(raw);
        // Pin lastSyncedValue to the same string we're writing to
        // value, so the external-sync $effect (which fires from the
        // bind:value round-trip) sees no work to do and skips
        // setMarkdownContent. Without this pin the $effect would
        // re-parse the user's just-typed markdown and reset the
        // selection.
        lastSyncedValue = md;
        value = md;
        tagHeadings();
        syncWikiBubble();
        syncImageBubble();
        syncTagBubble();
        syncContactBubble();
        updateCursorDecorations();
        maybeOpenAtomEditAtSelection();
        onSelectionChange?.();
      },
      onSelectionUpdate: () => {
        syncWikiBubble();
        syncImageBubble();
        syncTagBubble();
        syncContactBubble();
        updateCursorDecorations();
        maybeOpenAtomEditAtSelection();
        onSelectionChange?.();
      },
    });
    // Override the paragraph node's markdown serializer so empty
    // paragraphs round-trip. prosemirror-markdown's default rule
    // writes nothing for an empty <p></p> (renderInline emits no
    // content, and the block separator is only flushed before the
    // *next* block), so a doc like [A, empty, B] collapses to
    // "A\n\nB" on serialize and the blank line is gone after a tab
    // swap. Emitting an NBSP turns empty paragraphs into the
    // " \n\n" markers `stripBlankParagraphs` and
    // `preserveBlankParagraphs` already understand. NBSP, not a
    // regular space, because markdown-it treats a line containing
    // only ASCII whitespace as a blank line and drops the
    // paragraph on the next reparse.
    //
    // The override has to land on the resolved extension's storage
    // *and* shadow the inherited `storage` getter via
    // `Object.defineProperty`. tiptap 2.x's Extendable base class
    // exposes `extension.storage` as a getter that returns a fresh
    // empty object on every access for any extension that didn't
    // declare `addStorage()` (StarterKit's paragraph hasn't), so a
    // plain `extension.storage.markdown = ...` writes to a
    // throwaway and tiptap-markdown's `getMarkdownSpec` keeps
    // reading the default prosemirror serializer. Defining an own
    // data property shadows the getter so the spec sticks.
    type PMState = {
      write: (content?: string) => void;
      renderInline: (node: { content: { size: number } }) => void;
      closeBlock: (node: unknown) => void;
    };
    type PMNode = { content: { size: number } };
    const paraExt = editor.extensionManager.extensions.find(
      (e: { name: string }) => e.name === "paragraph",
    );
    if (paraExt) {
      Object.defineProperty(paraExt, "storage", {
        value: {
          markdown: {
            serialize(state: PMState, node: PMNode) {
              if (node.content.size === 0) {
                state.write("\u00A0");
              } else {
                state.renderInline(node);
              }
              state.closeBlock(node);
            },
            parse: {},
          },
        },
        writable: true,
        enumerable: true,
        configurable: true,
      });
    }
    // Set initial markdown explicitly (StarterKit treats `content` as HTML
    // by default). Markdown extension exposes setContent via commands.
    // Wrapped in the same guard the sync $effect uses: setContent's
    // own emitUpdate=false flag suppresses the SetContent transaction's
    // onUpdate, but decorateSmartNodes() dispatches a follow-up
    // transaction whose onUpdate would fire and write `value` back,
    // creating a re-render loop with the bind:value-driven $effect.
    applyingExternal = true;
    try {
      setMarkdownContent(value);
      lastSyncedValue = value;
      editor.commands.focus("start");
      tagHeadings();
      updateCursorDecorations();
    } finally {
      applyingExternal = false;
    }
    host.addEventListener("input", onInput);
    host.addEventListener("click", onClick);
  });

  /// Walk the rendered ProseMirror DOM and assign `data-heading-id` to
  /// every h1..h6, in document order. The inspector's outline view
  /// uses these as scroll targets; the index matches the order in
  /// which the outline regex finds headings, so clicks line up.
  function tagHeadings(): void {
    if (!host) return;
    const all = host.querySelectorAll(
      ".ProseMirror h1, .ProseMirror h2, .ProseMirror h3, .ProseMirror h4, .ProseMirror h5, .ProseMirror h6",
    );
    all.forEach((el, i) => el.setAttribute("data-heading-id", `h-${i}`));
  }

  onDestroy(() => {
    dismissWikiBubble();
    dismissImageBubble();
    dismissImageOverlay();
    dismissTagBubble();
    dismissContactBubble();
    editor?.destroy();
  });

  // Keep editor in sync when parent rewrites `value` (e.g. switching tabs
  // or async load completing). Compare against `lastSyncedValue` rather
  // than the editor's getMarkdown() output, since the round-trip is
  // non-idempotent (see lastSyncedValue's docstring).
  $effect(() => {
    if (!editor) return;
    if (lastSyncedValue === value) return;
    applyingExternal = true;
    try {
      setMarkdownContent(value);
      lastSyncedValue = value;
      tagHeadings();
      updateCursorDecorations();
      // External content change = tab switch or fresh load. Refocus
      // so the user can keep typing without clicking. Skip when
      // the editor is non-editable: refocusing a contenteditable=
      // false editor can leave ProseMirror's selection in a state
      // that suppresses the post-setContent paint, which is why
      // filesystem updates appeared to stop landing once the lamp
      // was flipped to read.
      if (editor.isEditable) editor.commands.focus("start");
    } finally {
      applyingExternal = false;
    }
  });

  function setMarkdownContent(md: string): void {
    if (!editor) return;
    // tiptap-markdown registers `setMarkdown` via storage.markdown.parser
    // but the cleanest invocation is editor.commands.setContent(md).
    // Second positional arg is `emitUpdate: boolean` in this tiptap
    // version; the older `{ emitUpdate: false }` object form was
    // dropped.
    editor.commands.setContent(preserveBlankParagraphs(md), false);
    decorateSmartNodes();
    decorateWikiLinks();
  }

  /// markdown-it (and CommonMark in general) treats blank lines as
  /// block separators, not as content. Two paragraphs separated by
  /// any number of blank lines parse to two adjacent paragraph
  /// nodes; the editor's bullet-list-then-paragraph rendering loses
  /// the visual gap the user typed. We can't change the parser
  /// from outside, so we pre-process: every run of 3+ newlines (a
  /// paragraph break plus N blank-paragraph rows) is replaced with
  /// a sequence of NBSP paragraphs that markdown-it parses as
  /// real paragraph nodes. The NBSP renders as a thin invisible
  /// gap, restoring the spacing.
  ///
  /// On save we run `stripBlankParagraphs` so the file on disk stays
  /// clean (plain blank lines, no NBSP characters); the next
  /// re-parse re-injects the NBSPs.
  function preserveBlankParagraphs(md: string): string {
    return md.replace(/\n{3,}/g, (m) => {
      const empties = m.length - 2;
      return "\n\n" + " \n\n".repeat(empties);
    });
  }

  /// Inverse of `preserveBlankParagraphs`. Removes NBSP-only
  /// paragraphs (the editor's internal gap markers) so the
  /// markdown going to disk has plain blank lines instead of
  /// invisible characters. Each ` \n\n` substring (an NBSP
  /// paragraph followed by its block separator) collapses to a
  /// single newline, which when added to the prior block's `\n\n`
  /// yields the 3-newline pattern the user originally typed.
  function stripBlankParagraphs(md: string): string {
    return md.replace(/ \n\n/g, "\n");
  }

  /// Restore wiki-link pills after a markdown round-trip.
  ///
  /// The wikiLink node serializes to `[label](path)` (a standard
  /// markdown link, so files on disk stay portable to any reader).
  /// On re-parse, tiptap-markdown turns that back into a plain
  /// `Link` mark since wikiLink has no markdown->node parser. The
  /// pill styling vanishes and, in cases where the label contains
  /// markdown-significant characters (`[`, `]`, `(`), markdown-it
  /// can drop the link entirely and leave plain text behind. Both
  /// failures triggered the "links disappear after switching tabs"
  /// bug: every tab switch unmounts + remounts the editor, which
  /// re-parses the buffer and runs the round-trip.
  ///
  /// We rebuild the pills here. For every text node carrying a
  /// `link` mark, the href is run through `normalizeHref`; a
  /// non-null result replaces the marked range with a fresh
  /// `wikiLink` atom node carrying the canonical drive-rooted
  /// target. External http(s)/mailto links (normalizeHref returns
  /// null) are left as Link marks. Idempotent: a doc with only
  /// existing wikiLink nodes (no Link marks) walks to no
  /// replacements.
  function decorateWikiLinks(): void {
    if (!editor) return;
    const wikiType = editor.schema.nodes.wikiLink;
    const linkMarkType = editor.schema.marks.link;
    if (!wikiType || !linkMarkType) return;

    type Range = {
      from: number;
      to: number;
      target: string;
      label: string;
      anchor: string;
      wasAbs: boolean;
    };
    const ranges: Range[] = [];

    editor.state.doc.descendants((node, pos) => {
      if (!node.isText || !node.text) return;
      const linkMark = node.marks.find((m) => m.type === linkMarkType);
      if (!linkMark) return;
      const href = (linkMark.attrs.href as string | null) ?? "";
      if (!href) return;
      // Decode once (chan-shared encodes spaces / parens when
      // serializing), then split off `#anchor` so normalizeHref
      // operates on the path portion alone. The atom carries the
      // canonical drive-rooted path on `target`, the section on
      // its own `anchor` attr, and `wasAbs` if the source markdown
      // used a leading slash (so the serializer can round-trip
      // `/path` instead of relativizing it). normalizeHref returns
      // null for externals / fragment-only refs, in which case the
      // Link mark is left untouched and the browser default fires.
      let decoded: string;
      try {
        decoded = decodeURIComponent(href);
      } catch {
        decoded = href;
      }
      const hashIdx = decoded.indexOf("#");
      const pathPart = hashIdx === -1 ? decoded : decoded.slice(0, hashIdx);
      const anchor = hashIdx === -1 ? "" : decoded.slice(hashIdx + 1);
      const sourceDir = currentPath
        ? currentPath.split("/").slice(0, -1).join("/")
        : "";
      const normalized = normalizeHref(pathPart, sourceDir);
      if (normalized === null) return;
      ranges.push({
        from: pos,
        to: pos + node.text.length,
        target: normalized,
        label: node.text,
        anchor,
        wasAbs: pathPart.startsWith("/"),
      });
    });

    if (ranges.length === 0) return;
    const tr = editor.state.tr;
    // Apply in reverse so earlier positions stay valid as later
    // ones are replaced. Each Link-mark range collapses to a
    // single atomic node, so positions after `r.from` shift, but
    // applying right-to-left avoids the invalidation.
    for (const r of ranges.reverse()) {
      tr.replaceWith(
        r.from,
        r.to,
        wikiType.create({
          target: r.target,
          label: r.label,
          anchor: r.anchor,
          wasAbs: r.wasAbs,
        }),
      );
    }
    // Same flags as decorateSmartNodes: out of undo, out of the
    // bind:value loop. preventUpdate keeps tiptap's onUpdate from
    // firing, so the post-decoration markdown serialization
    // doesn't bounce back into the parent's `value` and re-fire
    // the $effect. Decoration is applied to the editor view
    // synchronously regardless of the meta flag, so the wikiLink
    // pill renders immediately.
    editor.view.dispatch(
      tr.setMeta("addToHistory", false).setMeta("preventUpdate", true),
    );
  }

  /// Round-trip recovery for smart nodes that markdown can't carry.
  /// Date nodes serialize to plain text in their chosen format; on
  /// re-parse they come back as text nodes and lose their styled
  /// appearance. Walk the doc (or the current paragraph in `local`
  /// mode) and replace every match for any catalog regex with a
  /// `date` node so the WYSIWYG view stays consistent across
  /// source-mode round-trips, AND so dates the user just typed
  /// auto-pill as the trailing word boundary lands.
  ///
  /// `local` scopes the walk to the cursor's parent text-block so
  /// per-keystroke calls don't pay for a whole-doc scan. Skips text
  /// inside code blocks / inline code so e.g. `2026-05-02` inside a
  /// snippet stays plain.
  function decorateSmartNodes(scope: "all" | "local" = "all"): void {
    if (!editor) return;
    const dateNodeType = editor.schema.nodes.date;
    if (!dateNodeType) return;
    type Range = { from: number; to: number; iso: string; format: DateFormatId };
    const ranges: Range[] = [];

    // Determine the walk range. Local = the parent block of the
    // current selection (a paragraph / list-item / heading); falls
    // back to whole-doc when the resolver fails (degenerate doc).
    let walkStart = 0;
    let walkEnd = editor.state.doc.content.size;
    if (scope === "local") {
      const resolved = editor.state.selection.$from;
      const depth = resolved.depth;
      if (depth >= 1) {
        walkStart = resolved.before(depth);
        walkEnd = resolved.after(depth);
      }
    }

    editor.state.doc.nodesBetween(walkStart, walkEnd, (node, pos, parent) => {
      if (!node.isText || !node.text) return;
      const parentName = parent?.type.name ?? "";
      if (parentName === "codeBlock") return false;
      if (node.marks.some((m) => m.type.name === "code")) return;
      for (const m of findDateMatches(node.text)) {
        ranges.push({
          from: pos + m.start,
          to: pos + m.end,
          iso: isoOf(m.date),
          format: m.formatId,
        });
      }
    });
    if (ranges.length === 0) return;
    const tr = editor.state.tr;
    // Apply in reverse so earlier positions stay valid as later
    // ones are replaced.
    for (const r of ranges.reverse()) {
      tr.replaceWith(
        r.from,
        r.to,
        dateNodeType.create({ date: r.iso, format: r.format }),
      );
    }
    // preventUpdate stops tiptap from emitting `update`, so onUpdate's
    // `value = md` round-trip can't fire during sync. Without it, the
    // decoration transaction lands inside the bind:value loop and
    // tiptap-markdown's non-idempotent serialization (it adds an extra
    // \n after headings on every reparse) makes current !== value
    // permanently true, blowing past Svelte's effect-depth guard.
    editor.view.dispatch(
      tr.setMeta("addToHistory", false).setMeta("preventUpdate", true),
    );
  }

  /// User's preferred default date format. Falls back to ISO if
  /// the drive prefs haven't loaded yet (boot race) or the stored
  /// id no longer exists in the catalog (the catalog lookup
  /// itself falls back to ISO too, so this is belt-and-suspenders).
  function defaultDateFormat(): DateFormatId {
    const v = drive.info?.preferences?.date_format;
    if (v === "iso" || v === "medium" || v === "short") return v;
    return "iso";
  }

  function onInput(e: Event): void {
    if (!editor) return;
    const inputData = (e as InputEvent).data ?? "";
    // Defer the rest of the work to the next tick. The browser's
    // `input` event can fire BEFORE ProseMirror's mutation observer
    // has applied the transaction for the just-typed character, so
    // reading `editor.state` here would miss it. By the next macro-
    // task PM has caught up and `endsWith("![")` etc. see the right
    // doc.
    setTimeout(() => onInputDeferred(inputData), 0);
  }

  function onInputDeferred(inputData: string): void {
    if (!editor) return;
    // Tag bubble opens only on a literal `#` keystroke. Distinguishes
    // a fresh `#` from caret merely passing over an existing `#tag`.
    // `InputEvent.data` is the typed character for plain insertions
    // and null for everything else (backspace, composition state
    // changes, paste, etc.), so the check is safe.
    if (!tagBubble && inputData === "#") {
      const range = findTagRange(editor);
      if (range) openTagBubbleForCurrentCaret(range.query);
    }
    // Contact bubble opens on a fresh `@` keystroke at start-of-
    // word. Same input-event rationale as the tag bubble: caret
    // moving across an existing `@foo` should NOT pop the picker;
    // only a freshly-typed `@` should. `@@` (the existing mention
    // syntax) auto-dismisses via syncContactBubble because the
    // range regex won't match a doubled `@`.
    if (!contactBubble && inputData === "@") {
      const range = findContactRange(editor);
      if (range) openContactBubbleForCurrentCaret(range.query);
    }
    // `@<space>` (bare `@` then space) dismisses the picker. The
    // user signaled "not a contact lookup, just an `@` in prose."
    // Spaces inside a non-empty query are allowed (contact display
    // names like "Jane Doe" must be typeable).
    if (contactBubble && inputData === " ") {
      const range = findContactRange(editor);
      if (!range || range.query.trim() === "") {
        dismissContactBubble();
      }
    }
    // Look at text immediately before the cursor (up to 16 chars).
    const { from } = editor.state.selection;
    const before = editor.state.doc.textBetween(Math.max(0, from - 16), from, "\n", "\n");

    // `!/today` and `!/date`: command-style trigger for inline
    // insertions. The two-char `!/` prefix is collision-free with
    // prose (`Done!`, `:smile:`, `/usr/local/bin`), so the picker
    // never flickers mid-typing. Reserved as the convention for any
    // future inline command (`!/table`, `!/hr`, etc.); see the
    // chan_command_trigger memory.
    if (before.endsWith("!/today")) {
      replaceTrailingTrigger("!/today", () => {
        // !/today and !/date both produce dates; we use the same
        // node type for both so the styling is consistent and a
        // markdown round-trip doesn't change the appearance.
        // Format follows the user's default date-format pref.
        const fmt = defaultDateFormat();
        const iso = isoOf(new Date());
        editor!
          .chain()
          .focus()
          .insertContent({ type: "date", attrs: { date: iso, format: fmt } })
          .insertContent(" ")
          .run();
      });
      return;
    }
    if (before.endsWith("!/date")) {
      replaceTrailingTrigger("!/date", () => {
        const anchor = caretAnchorHost();
        showCalendar(
          anchor,
          (picked) => {
            if (!picked || !editor) return;
            editor
              .chain()
              .focus()
              .insertContent({
                type: "date",
                attrs: { date: picked.iso, format: picked.format },
              })
              .insertContent(" ")
              .run();
          },
          defaultDateFormat(),
        );
      });
      return;
    }
    if (before.endsWith("![") && !imageBubble) {
      // Auto-pair: complete the markdown image shape to `![](|)`
      // with the caret parked between the parens. The user's typing
      // becomes the `(src)` query; the bubble opens in path mode
      // and the host's sync hook keeps it pinned while the caret
      // stays inside.
      const pos = editor.state.selection.from;
      editor
        .chain()
        .insertContentAt(pos, "]()", { updateSelection: false })
        .setTextSelection(pos + 2)
        .run();
      editingImageBracketStart = pos - 2;
      editingImageOriginal = null;
      editingImageDefaultAlt = "";
      openImageBubbleForCurrentCaret();
      return;
    }
    if (before.endsWith("[[") && !wikiBubble) {
      // Auto-pair: insert `]]` after the caret and step the caret
      // back into the middle. The leading `[[` the user typed stays;
      // their next keystroke goes between the brackets and IS the
      // search query. The bubble below the caret renders results
      // without taking focus.
      const pos = editor.state.selection.from;
      editor
        .chain()
        .insertContentAt(pos, "]]", { updateSelection: false })
        .setTextSelection(pos)
        .run();
      openWikiBubbleForCurrentCaret();
      return;
    }
    // Live date detection: scan the cursor's parent block for any
    // catalog match and convert it to a pill. The catalog regex
    // requires a non-word, non-dash sentinel after the date, so
    // typing "2026-05-05" doesn't pill until the user types the
    // following space / punctuation (or the date sits at end of
    // block). Scoped to the local block to keep per-keystroke cost
    // bounded.
    decorateSmartNodes("local");
  }

  /// Locate the `[[ ... ]]` text range that surrounds the current
  /// caret, if any. Returns positions in the prosemirror document
  /// (start = `[`, end = position AFTER second `]`) plus the query
  /// text between the brackets. Constraints:
  ///   - Selection must be collapsed (no range select).
  ///   - The brackets must live in the same textblock as the caret.
  ///   - There must be no other `]]` between the open `[[` and the
  ///     caret, and no other `[[` between the caret and the close
  ///     `]]`. This handles the common case of a single in-progress
  ///     wiki entry without false matches across nearby brackets.
  function findBracketRange(
    ed: Editor,
  ): { start: number; end: number; query: string } | null {
    const sel = ed.state.selection;
    if (!sel.empty) return null;
    const resolved = ed.state.doc.resolve(sel.from);
    const block = resolved.parent;
    if (!block.isTextblock) return null;
    const blockStart = resolved.start();
    const offset = sel.from - blockStart;
    const text = block.textContent;
    const leftIdx = text.lastIndexOf("[[", Math.max(0, offset - 1));
    if (leftIdx === -1) return null;
    const between = text.slice(leftIdx + 2, offset);
    if (between.includes("]]")) return null;
    const rightIdx = text.indexOf("]]", offset);
    if (rightIdx === -1) return null;
    const after = text.slice(offset, rightIdx);
    if (after.includes("[[")) return null;
    return {
      start: blockStart + leftIdx,
      end: blockStart + rightIdx + 2,
      query: text.slice(leftIdx + 2, rightIdx),
    };
  }

  /// Mount the wiki bubble anchored at the caret's actual screen
  /// position. The selection-update hook keeps it in sync; this
  /// function just handles the open path. Caller must ensure the
  /// editor has the `[[ ]]` brackets in place (the caret should sit
  /// inside them).
  function openWikiBubbleForCurrentCaret(): void {
    if (!editor || wikiBubble) return;
    // When this open is part of an "edit existing link" flow,
    // surface the original target as a `>` follow button on the
    // bubble so the user can navigate without dismissing first.
    const followExisting = editingWikiOriginal
      ? {
          target: editingWikiOriginal.target,
          anchor: editingWikiOriginal.anchor,
        }
      : undefined;
    // Initial query reflects whatever sits between the brackets
    // RIGHT NOW. For typed `[[` the brackets are empty; for the
    // edit-existing path enterWikiEditAt has just inserted
    // `[[label]]`, so `range.query === label` and the search runs
    // pre-populated with the existing link's text.
    const range = findBracketRange(editor);
    const initialQuery = range?.query ?? "";
    wikiBubble = openWikiBubble({
      host: caretAnchorHost(),
      prefix: wikiPickerPrefix,
      onClickAccept: () => acceptWikiBubble(),
      onCommit: () => acceptWikiBubble(),
      onDismiss: () => dismissWikiBubble(),
      followExisting,
      onFollowExisting: (target, anchor) => {
        // Treat this as "navigate AND keep the link": restore the
        // original atom in place of the temporary `[[label]]`
        // brackets first, so the surrounding markdown round-trips
        // unchanged once the new file is opened. Use the saved
        // snapshot rather than `target`/`anchor` alone so the
        // original label is preserved.
        const orig = editingWikiOriginal;
        editingWikiOriginal = null;
        editingWikiBracketStart = null;
        if (editor && orig) {
          const range = findBracketRange(editor);
          const wikiType = editor.schema.nodes.wikiLink;
          if (range && wikiType) {
            editor.view.dispatch(
              editor.state.tr.replaceWith(
                range.start,
                range.end,
                wikiType.create({
                  target: orig.target,
                  label: orig.label,
                  anchor: orig.anchor,
                  wasAbs: orig.wasAbs,
                }),
              ),
            );
          }
        }
        dismissWikiBubble();
        const fullTarget = anchor ? `${target}#${anchor}` : target;
        handleWikiClick(fullTarget);
      },
    });
    wikiBubble.setQuery(initialQuery);
  }

  /// Build a synthetic "host" element that reports the caret's
  /// viewport-relative bounding rect. `positionPopover` only ever
  /// reads `getBoundingClientRect()` from the host, so we can
  /// shim the result without attaching the element to the DOM.
  ///
  /// Why not pass a real DOM element: the previous implementation
  /// used `window.getSelection().focusNode.parentElement`, which
  /// returns the paragraph (or block) containing the caret rather
  /// than a per-character rect. Long lines pulled the picker to
  /// the line's leftmost edge; an unreliable selection (right
  /// after the `[[` autopair flushes a transaction) returned the
  /// editor host itself, landing the picker at the editor's
  /// top-left corner — which is roughly the top-left of the
  /// viewport in a single-pane layout.
  ///
  /// `editor.view.coordsAtPos(pos)` gives us the cursor's actual
  /// viewport rect; we wrap it in an element shim so the
  /// shared positioning helper does not need to learn a second
  /// shape.
  function caretAnchorHost(): HTMLElement {
    if (!editor) return host!;
    const pos = editor.state.selection.from;
    let coords: { left: number; right: number; top: number; bottom: number };
    try {
      coords = editor.view.coordsAtPos(pos);
    } catch {
      // Position out of range can throw; fall back to the editor
      // container so the bubble still appears, just less precisely.
      return host!;
    }
    // A 0-width rect at the caret. `positionPopover` flips above
    // the rect when below would clip; using the actual line
    // bottom as `bottom` keeps the popover from overlapping the
    // caret line.
    const rect = {
      left: coords.left,
      right: coords.left,
      top: coords.top,
      bottom: coords.bottom,
      width: 0,
      height: coords.bottom - coords.top,
      x: coords.left,
      y: coords.top,
      toJSON() {
        return rect;
      },
    } as DOMRect;
    const shim: HTMLElement = {
      getBoundingClientRect: () => rect,
    } as unknown as HTMLElement;
    return shim;
  }

  /// Pull the current bracket query off the doc, ask the bubble to
  /// commit, and replace the entire `[[query]]` range with a
  /// wikiLink atom node. No-op when the bubble has no result to
  /// commit (empty query, no matches): the user must type or
  /// dismiss with Escape. Block picks may carry a pending file
  /// write (the chosen block had no `^id` yet); we persist it
  /// before committing the link so the on-disk anchor exists by
  /// the time the user clicks through.
  function acceptWikiBubble(): void {
    if (!editor || !wikiBubble) return;
    const range = findBracketRange(editor);
    if (!range) {
      dismissWikiBubble();
      return;
    }
    const picked = wikiBubble.accept();
    if (!picked) return;
    // Accept supersedes the edit-existing snapshot: the user
    // explicitly chose a new target, so dismiss must NOT restore
    // the prior atom. Clear before dismissWikiBubble runs.
    editingWikiOriginal = null;
    editingWikiBracketStart = null;
    dismissWikiBubble();
    // anchor is "" for file picks; only heading / block picks
    // populate it. The wikiLink node carries it onto the markdown
    // serialization so the on-disk link is `[label](path#anchor)`.
    const anchor = picked.kind === "file" ? "" : picked.anchor;
    const pending =
      picked.kind === "block" ? picked.pendingFileWrite : null;
    const ed = editor;
    const insertNode = (): void => {
      ed.chain()
        .focus()
        .deleteRange({ from: range.start, to: range.end })
        .insertContent({
          type: "wikiLink",
          attrs: { target: picked.target, label: picked.label, anchor },
        })
        .insertContent(" ")
        .run();
    };
    if (pending) {
      // CAS-write the rewritten target file body, then insert the
      // link. On 409 (external edit beat us), drop the link rather
      // than committing a dangling anchor; the user can retype the
      // bracket once they have re-resolved the conflict.
      void api
        .write(picked.target, pending.content, pending.expectedMtime)
        .then(() => insertNode())
        .catch((e: unknown) => {
          // eslint-disable-next-line no-console
          console.error("wiki block write failed:", e);
        });
      return;
    }
    insertNode();
  }

  function dismissWikiBubble(): void {
    wikiBubble?.dismiss();
    wikiBubble = undefined;
    // If the bubble was opened in edit-existing mode and the user
    // walked away without accepting, restore the original atom so
    // the document doesn't end up with stray bracket text.
    // `acceptWikiBubble` clears the snapshot before calling
    // `dismissWikiBubble` so this branch only fires on true
    // dismissals (Escape, click out, caret leaving the brackets).
    if (editingWikiOriginal) restoreWikiEditOriginal();
  }

  /// Re-evaluate bubble lifecycle on every selection / doc update.
  /// Open: keep alive while caret stays between the brackets, push
  /// the latest query in. Closed: open if a `[[ ]]` range now
  /// surrounds the caret (covers undo / redo into bracket state).
  function syncWikiBubble(): void {
    if (!editor) return;
    const range = findBracketRange(editor);
    if (wikiBubble) {
      if (!range) {
        dismissWikiBubble();
        return;
      }
      wikiBubble.setQuery(range.query);
    }
  }

  // ---- tag bubble ------------------------------------------------------

  /// Detect the `#word` token immediately to the left of the caret, if
  /// any. Mirrors `findBracketRange`'s contract: returns positions in
  /// the prosemirror document plus the typed query (without the `#`).
  /// Constraints:
  ///   - Selection must be collapsed (no range select).
  ///   - Caret must sit in a textblock that is not a heading or
  ///     codeBlock (those treat `#` literally).
  ///   - Caret must NOT be inside a `[[ ]]` range; the wiki bubble
  ///     owns `#` inside its bracket flow.
  ///   - The `#` must be at block-start or preceded by whitespace,
  ///     and only `[A-Za-z0-9_-]` may follow it up to the caret.
  function findTagRange(
    ed: Editor,
  ): { start: number; end: number; query: string } | null {
    const sel = ed.state.selection;
    if (!sel.empty) return null;
    if (findBracketRange(ed)) return null;
    const fromPos = ed.state.doc.resolve(sel.from);
    const parent = fromPos.parent;
    if (!parent.isTextblock) return null;
    if (parent.type.name === "heading") return null;
    if (parent.type.name === "codeBlock") return null;
    const blockStart = fromPos.start();
    // textBetween with NBSP for atom leaves keeps offsets aligned to
    // doc positions even when the block contains wikiLink / image
    // atoms ahead of the caret.
    const before = ed.state.doc.textBetween(blockStart, sel.from, "\n", " ");
    const m = before.match(/(?:^|\s)#([A-Za-z0-9_-]*)$/);
    if (!m) return null;
    const query = m[1] ?? "";
    const hashPos = sel.from - query.length - 1;
    return { start: hashPos, end: sel.from, query };
  }

  function openTagBubbleForCurrentCaret(query: string): void {
    if (!editor || tagBubble) return;
    tagBubble = openTagBubble({
      host: caretAnchorHost(),
      onClickAccept: () => acceptTagBubble(),
      onCommit: () => acceptTagBubble(),
      onDismiss: () => dismissTagBubble(),
    });
    tagBubble.setQuery(query);
  }

  function acceptTagBubble(): void {
    if (!editor || !tagBubble) return;
    const range = findTagRange(editor);
    if (!range) {
      dismissTagBubble();
      return;
    }
    const picked = tagBubble.accept();
    if (!picked) return;
    dismissTagBubble();
    // Replace `#typed` (the entire trigger range) with the chosen
    // tag plus a trailing space so the cursor lands at a clean break.
    editor
      .chain()
      .focus()
      .deleteRange({ from: range.start, to: range.end })
      .insertContent(`#${picked}`)
      .insertContent(" ")
      .run();
  }

  function dismissTagBubble(): void {
    tagBubble?.dismiss();
    tagBubble = undefined;
  }

  /// Re-evaluate an OPEN tag bubble's lifecycle on every selection /
  /// doc update. Open: keep alive while the caret stays in the
  /// trigger range; push the latest query in. Dismiss when the
  /// caret leaves the range. We deliberately do NOT auto-open here:
  /// opening is triggered only by a fresh `#` keystroke (`onInput`)
  /// so the bubble doesn't pop when the caret merely passes over an
  /// existing `#tag` in the document.
  function syncTagBubble(): void {
    if (!editor) return;
    if (!editor.isEditable) {
      dismissTagBubble();
      return;
    }
    if (!tagBubble) return;
    const range = findTagRange(editor);
    if (!range) {
      dismissTagBubble();
      return;
    }
    tagBubble.setQuery(range.query);
  }

  /// Locate the trigger range for the contact `@` picker: an `@`
  /// at start-of-word (preceded by whitespace or block start),
  /// followed by zero-or-more name-friendly chars. Returns the
  /// range to replace on accept and the current query (without the
  /// leading `@`). Skipped in headings + code blocks for the same
  /// reasons as the tag bubble: pills don't belong in either.
  /// Spaces ARE allowed in the query so display names like
  /// "Jane Doe" are typeable; the `@<space>` early-dismiss lives
  /// in `onInput` (it's an input-event signal, not a range check).
  function findContactRange(
    ed: Editor,
  ): { start: number; end: number; query: string } | null {
    const sel = ed.state.selection;
    if (!sel.empty) return null;
    if (findBracketRange(ed)) return null;
    const fromPos = ed.state.doc.resolve(sel.from);
    const parent = fromPos.parent;
    if (!parent.isTextblock) return null;
    if (parent.type.name === "heading") return null;
    if (parent.type.name === "codeBlock") return null;
    const blockStart = fromPos.start();
    const before = ed.state.doc.textBetween(blockStart, sel.from, "\n", " ");
    // Allow letters, digits, underscore, hyphen, period, and SINGLE
    // spaces inside the query (no consecutive spaces - that's a
    // strong signal the user is no longer composing a name). The
    // leading `(?:^|\s)` ensures the `@` is at start-of-word so
    // `email@host` doesn't trigger.
    const m = before.match(/(?:^|\s)@([A-Za-z0-9_.-]*(?:\s[A-Za-z0-9_.-]+)*)$/);
    if (!m) return null;
    const query = m[1] ?? "";
    const atPos = sel.from - query.length - 1;
    return { start: atPos, end: sel.from, query };
  }

  function openContactBubbleForCurrentCaret(query: string): void {
    if (!editor || contactBubble) return;
    contactBubble = openContactBubble({
      host: caretAnchorHost(),
      onClickAccept: () => acceptContactBubble(),
      onCommit: () => acceptContactBubble(),
      onDismiss: () => dismissContactBubble(),
    });
    contactBubble.setQuery(query);
  }

  function acceptContactBubble(): void {
    if (!editor || !contactBubble) return;
    const range = findContactRange(editor);
    if (!range) {
      dismissContactBubble();
      return;
    }
    const picked = contactBubble.accept();
    if (!picked) return;
    dismissContactBubble();
    // Insert the picked contact as a wiki-link to its note. The
    // `[[` parser strips the `.md` suffix; we strip here too so
    // the on-disk markdown stays clean. The decorator pass on the
    // next render will pill it like any other wiki-link.
    const target = picked.path.replace(/\.md$/i, "");
    editor
      .chain()
      .focus()
      .deleteRange({ from: range.start, to: range.end })
      .insertContent(`[[${target}]]`)
      .insertContent(" ")
      .run();
  }

  function dismissContactBubble(): void {
    contactBubble?.dismiss();
    contactBubble = undefined;
  }

  /// Same lifecycle pattern as syncTagBubble: keep an open contact
  /// bubble alive while the caret stays in the trigger range;
  /// dismiss when the range is gone (caret moved to a different
  /// line / different block / out of the `@<query>` slice).
  function syncContactBubble(): void {
    if (!editor) return;
    if (!editor.isEditable) {
      dismissContactBubble();
      return;
    }
    if (!contactBubble) return;
    const range = findContactRange(editor);
    if (!range) {
      dismissContactBubble();
      return;
    }
    contactBubble.setQuery(range.query);
  }

  // ---- date edit-existing flow ----------------------------------------

  /// When the caret arrives on an editable atom (date pill, wiki
  /// link) via arrow-key NodeSelection, open the corresponding
  /// edit popover. Mirrors the click path; the one-shot guard
  /// `lastAtomEditPos` prevents the dismiss-refocus loop from
  /// re-opening for the same atom. Clears the guard as soon as the
  /// selection moves off any atom so a later re-entry reopens.
  function maybeOpenAtomEditAtSelection(): void {
    if (!editor) return;
    const sel = editor.state.selection;
    if (!(sel instanceof NodeSelection)) {
      lastAtomEditPos = null;
      return;
    }
    const node = sel.node;
    const name = node.type.name;
    if (name !== "date" && name !== "wikiLink" && name !== "image") {
      lastAtomEditPos = null;
      return;
    }
    if (lastAtomEditPos === sel.from) return;
    lastAtomEditPos = sel.from;
    if (name === "image") {
      // Image atoms route into the source-text edit flow rather than
      // a separate popover; `enterImageEditAt` reads attrs off the
      // node directly so it doesn't need the DOM element.
      enterImageEditAt(sel.from, node);
      return;
    }
    const dom = editor.view.nodeDOM(sel.from);
    if (!(dom instanceof HTMLElement)) return;
    if (name === "date") {
      openDateEditAt(sel.from, dom);
      return;
    }
    // Wiki: record entry direction so the dismiss path lands the
    // caret on the correct side of the restored atom (continuing
    // the user's arrow motion). Left-arrow entry means the user
    // came from the right side; restore caret BEFORE the atom.
    wikiEditEntryDir = lastHorizontalArrow === "left" ? "before" : "after";
    enterWikiEditAt(dom);
  }

  /// Open the calendar pre-filled with the date atom at `pos`.
  /// Shared by both the click handler and the NodeSelection
  /// (arrow-key) trigger so the two paths behave identically.
  function openDateEditAt(pos: number, host: HTMLElement): void {
    if (!editor) return;
    const node = editor.state.doc.nodeAt(pos);
    if (!node || node.type.name !== "date") return;
    const existingFormat = (node.attrs.format as DateFormatId) ?? "iso";
    showCalendar(
      host,
      (picked: DatePick | null) => {
        if (!editor) return;
        if (!picked) {
          // Dismiss: refocus the editor so the caret lands back
          // on the pill (the calendar stole DOM focus). Mirrors
          // the image / wiki dismiss path.
          editor.commands.focus();
          return;
        }
        const dateType = editor.schema.nodes.date;
        if (!dateType) return;
        editor.view.dispatch(
          editor.state.tr.replaceWith(
            pos,
            pos + 1,
            dateType.create({ date: picked.iso, format: picked.format }),
          ),
        );
        editor.commands.focus();
      },
      existingFormat,
    );
  }

  // ---- wiki edit-existing flow ----------------------------------------

  /// Enter wiki edit mode by replacing the clicked atom with
  /// `[[label]]` text and dropping the caret inside the brackets;
  /// the existing `[[ ]]` bubble flow takes over from there. The
  /// original atom attrs are saved so a dismiss-without-accept can
  /// restore the link rather than leaving stray brackets.
  /// Build the inner `[[ ]]` query text for an existing wikiLink
  /// atom on edit-entry. Mirrors the bubble's input grammar:
  ///   - heading anchor (bare slug) → `target#slug`
  ///   - block anchor (leading `^`) → `target^id`
  ///   - alias differs from default file label → append `|alias`
  /// The default label is the file basename without `.md` (same
  /// derivation `fileLabel` uses inside the bubble), so a link
  /// whose alias matches the natural label doesn't pick up a
  /// redundant `|name` on every edit.
  function wikiEditQuery(target: string, label: string, anchor: string): string {
    let query = target;
    if (anchor) {
      query += anchor.startsWith("^") ? anchor : `#${anchor}`;
    }
    const defaultLabel =
      (target.split("/").pop() ?? target).replace(/\.md$/, "");
    if (label && label !== defaultLabel) {
      query += `|${label}`;
    }
    return query;
  }

  function enterWikiEditAt(wrap: HTMLElement): void {
    if (!editor) return;
    const stash = (wrap as unknown as { __wikiGetPos?: () => number | undefined })
      .__wikiGetPos;
    const pos = typeof stash === "function" ? stash() : undefined;
    if (typeof pos !== "number") return;
    const atom = editor.state.doc.nodeAt(pos);
    if (!atom || atom.type.name !== "wikiLink") return;
    const target = (atom.attrs.target as string) || "";
    const label = (atom.attrs.label as string) || target;
    const anchor = (atom.attrs.anchor as string) || "";
    const wasAbs = (atom.attrs.wasAbs as boolean) || false;
    editingWikiOriginal = { target, label, anchor, wasAbs };
    editingWikiBracketStart = pos;
    // Rebuild the inner query so the visible source matches what
    // the user originally typed: anchor (`#heading` or `^block`)
    // and `|alias` are restored when present. The bubble parses
    // the same shape when reopened, so the user can edit any
    // component in place.
    const inner = wikiEditQuery(target, label, anchor);
    const insertText = `[[${inner}]]`;
    editor
      .chain()
      .focus()
      .insertContentAt({ from: pos, to: pos + atom.nodeSize }, insertText)
      .setTextSelection(pos + 2 + inner.length)
      .run();
    // syncWikiBubble fires from onUpdate; openWikiBubbleForCurrent
    // -Caret won't because the caret was already inside brackets,
    // but our onInput trigger only catches `[[` keystrokes. Open
    // explicitly here so the bubble is alive on first paint with
    // the follow button populated.
    openWikiBubbleForCurrentCaret();
  }

  /// Restore the wiki atom we replaced when entering edit mode.
  /// Called by the bubble's dismiss path. Looks up the current
  /// `[[ ]]` range surrounding the caret (it might have been
  /// edited but not accepted) and replaces it with the original
  /// atom; if no bracket range survives, nothing to do.
  function restoreWikiEditOriginal(): void {
    if (!editor || !editingWikiOriginal) return;
    const orig = editingWikiOriginal;
    const start = editingWikiBracketStart;
    editingWikiOriginal = null;
    editingWikiBracketStart = null;
    if (start === null) return;
    // Locate the closing `]]` by scanning the parent textblock
    // from the saved bracket-start. Selection is unreliable here
    // because the user may have arrowed out of the brackets
    // (which is what triggered this dismiss); we walk the doc
    // explicitly so the brackets get replaced even when the caret
    // has moved away.
    const doc = editor.state.doc;
    if (start < 0 || start >= doc.content.size) return;
    let resolvedStart;
    try {
      resolvedStart = doc.resolve(start);
    } catch {
      return;
    }
    const blockStart = resolvedStart.start();
    const blockEnd = resolvedStart.end();
    if (start < blockStart || start >= blockEnd) return;
    const text = doc.textBetween(blockStart, blockEnd, "\n", " ");
    const offset = start - blockStart;
    if (text.slice(offset, offset + 2) !== "[[") return;
    const closeIdx = text.indexOf("]]", offset + 2);
    if (closeIdx === -1) return;
    const end = blockStart + closeIdx + 2;
    const wikiType = editor.schema.nodes.wikiLink;
    if (!wikiType) return;
    const dir = wikiEditEntryDir;
    wikiEditEntryDir = "after";
    const atomNode = wikiType.create({
      target: orig.target,
      label: orig.label,
      anchor: orig.anchor,
      wasAbs: orig.wasAbs,
    });
    let tr = editor.state.tr.replaceWith(start, end, atomNode);
    // After the replace, `start` points to the atom and
    // `start + 1` points right after it. Place the caret on the
    // side the user came from so arrow nav continues smoothly.
    const caretPos = dir === "before" ? start : start + 1;
    try {
      const r = tr.doc.resolve(caretPos);
      tr = tr.setSelection(
        TextSelection.near(r, dir === "before" ? -1 : 1),
      );
    } catch {
      // Position out of range: leave the selection where the
      // replace mapped it.
    }
    editor.view.dispatch(tr);
  }

  // ---- image edit-existing flow ----------------------------------------

  /// Drive-relative dirname for `path`. Used to scope uploads next
  /// to the editing file. Null path -> null (let the server fall
  /// back to its configured attachments_dir); root-level file ->
  /// empty string (drive root); nested file -> dirname segment.
  function dirOfPath(p: string | null): string | null {
    if (p === null) return null;
    const slash = p.lastIndexOf("/");
    if (slash < 0) return "";
    return p.slice(0, slash);
  }

  /// Locate the `![alt](src)` text range surrounding the caret and
  /// report which slot (alt / path / outside) the caret sits in.
  /// Mirrors `findBracketRange`'s contract for the wiki bubble; the
  /// host's sync hook uses the `mode` to drive the bubble between
  /// path-search and alt-echo modes.
  ///
  /// Mode boundaries (offsets relative to the leading `!`):
  ///   - 0..1                    -> outside (between `!` and `[`)
  ///   - 2..2 + altLen + 1       -> alt (covers `[`, alt text, `]`,
  ///                                and one boundary char so a
  ///                                keystroke crossing the divider
  ///                                doesn't immediately dismiss)
  ///   - 2 + altLen + 2..end - 1 -> path (inside `(...)`)
  ///   - >= fullLen              -> outside
  function findImageRange(
    ed: Editor,
  ): {
    start: number;
    end: number;
    alt: string;
    src: string;
    mode: "alt" | "path" | "outside";
  } | null {
    const sel = ed.state.selection;
    if (!sel.empty) return null;
    const resolved = ed.state.doc.resolve(sel.from);
    const block = resolved.parent;
    if (!block.isTextblock) return null;
    const blockStart = resolved.start();
    const offset = sel.from - blockStart;
    const text = block.textContent;
    const bangIdx = text.lastIndexOf("![", Math.max(0, offset));
    if (bangIdx === -1) return null;
    const rest = text.slice(bangIdx);
    const m = /^!\[([^\]]*)\]\(([^)]*)\)/.exec(rest);
    if (!m) return null;
    const alt = m[1] ?? "";
    const src = m[2] ?? "";
    const fullLen = m[0].length;
    const rel = offset - bangIdx;
    if (rel < 0 || rel >= fullLen) return null;
    let mode: "alt" | "path" | "outside";
    if (rel <= 1) {
      mode = "outside";
    } else if (rel <= 2 + alt.length + 1) {
      mode = "alt";
    } else if (rel >= 2 + alt.length + 2 && rel <= fullLen - 1) {
      mode = "path";
    } else {
      mode = "outside";
    }
    return {
      start: blockStart + bangIdx,
      end: blockStart + bangIdx + fullLen,
      alt,
      src,
      mode,
    };
  }

  /// Fallback range lookup for commit / restore paths. When the OS
  /// file picker dropped focus, the live selection check inside
  /// `findImageRange` fails (sel.empty is false because PM lost the
  /// cursor entirely); we scan the textblock starting from the saved
  /// bracket-start and parse the `![alt](src)` shape from there.
  /// Returns positions plus parsed alt / src; no mode field — the
  /// callers don't need to disambiguate.
  function findImageRangeAt(
    ed: Editor,
    start: number,
  ): { start: number; end: number; alt: string; src: string } | null {
    const doc = ed.state.doc;
    if (start < 0 || start >= doc.content.size) return null;
    let resolved;
    try {
      resolved = doc.resolve(start);
    } catch {
      return null;
    }
    const blockStart = resolved.start();
    const blockEnd = resolved.end();
    if (start < blockStart || start >= blockEnd) return null;
    const text = doc.textBetween(blockStart, blockEnd, "\n", " ");
    const offset = start - blockStart;
    const rest = text.slice(offset);
    const m = /^!\[([^\]]*)\]\(([^)]*)\)/.exec(rest);
    if (!m) return null;
    return {
      start,
      end: start + m[0].length,
      alt: m[1] ?? "",
      src: m[2] ?? "",
    };
  }

  /// Mount the image bubble anchored at the caret's screen position.
  /// Caller must ensure `![alt](src)` text is already present with
  /// the caret inside it (either freshly autopaired by `onInput` or
  /// inserted by `enterImageEditAt`). Same caret-anchor shim as the
  /// wiki bubble; sync hook keeps it in step with the caret.
  function openImageBubbleForCurrentCaret(): void {
    if (!editor || imageBubble) return;
    imageBubble = openImageBubble({
      host: caretAnchorHost(),
      uploadDir: dirOfPath(currentPath ?? null),
      onClickPick: (src) => {
        replaceImagePathInSource(src);
      },
      onUpload: (src) => {
        // Relativize against the editing file so the markdown reads
        // `./name.png` like the paste path does. Server returns a
        // drive-rooted path; without this, the bubble upload would
        // emit `[](file.png)` while paste emits `[](./file.png)`.
        const rel = currentPath ? relativizePath(src, currentPath) : src;
        replaceImagePathInSource(rel);
        // Pass the path explicitly so accept doesn't pick up the
        // list's currently-highlighted catalog entry instead.
        acceptImageBubble(rel);
      },
      onCommit: () => acceptImageBubble(),
      onDismiss: () => dismissImageBubble(),
    });
    // Seed the bubble's mode + query / alt from the current range so
    // the first paint reflects what the user has already typed.
    const range = findImageRange(editor);
    if (range) {
      imageBubble.setMode(range.mode === "alt" ? "alt" : "path");
      if (range.mode === "alt") {
        imageBubble.setAlt(range.alt);
      } else {
        imageBubble.setPathQuery(cleanSrc(range.src));
      }
    }
  }

  /// Strip a `#w=N` (or any `#...`) fragment from a markdown image
  /// src. The width is rendered by the image node, not searched on,
  /// so the path-mode filter sees the path portion only.
  function cleanSrc(src: string): string {
    const hash = src.indexOf("#");
    return hash < 0 ? src : src.slice(0, hash);
  }

  /// Replace the `(src)` portion of the surrounding `![alt](src)`
  /// markdown range with `newSrc`, leaving `[alt]` intact. No-op
  /// when no range surrounds the caret AND the saved bracket-start
  /// can't be located.
  function replaceImagePathInSource(newSrc: string): void {
    if (!editor) return;
    const ed = editor;
    let range = findImageRange(ed);
    if (!range && editingImageBracketStart !== null) {
      const fallback = findImageRangeAt(ed, editingImageBracketStart);
      if (fallback) {
        range = { ...fallback, mode: "path" };
      }
    }
    if (!range) return;
    const replacement = `![${range.alt}](${newSrc})`;
    ed.view.dispatch(
      ed.state.tr.insertText(replacement, range.start, range.end),
    );
  }

  /// Commit the bubble: replace the `![alt](src)` text with an image
  /// atom carrying the chosen src + alt. The alt auto-fills from the
  /// picked file's basename when the user hasn't typed (or has left
  /// the default we pre-populated on edit-entry). When the picked
  /// src matches the saved original after fragment-stripping, we
  /// keep the original verbatim so things like `#w=120` survive a
  /// round-trip through the bubble.
  ///
  /// `overrideSrc` short-circuits `imageBubble.accept()`: callers
  /// that already know which path to commit (notably the upload
  /// flow's `onUpload`, where the bubble's list-highlight is stale)
  /// pass the path explicitly so it doesn't get overridden by the
  /// currently-highlighted catalog entry.
  function acceptImageBubble(overrideSrc?: string): void {
    if (!editor || !imageBubble) return;
    const ed = editor;
    let range = findImageRange(ed);
    if (!range && editingImageBracketStart !== null) {
      const fb = findImageRangeAt(ed, editingImageBracketStart);
      if (fb) range = { ...fb, mode: "path" };
    }
    if (!range) {
      dismissImageBubble();
      return;
    }
    const picked = overrideSrc ?? imageBubble.accept() ?? range.src;
    if (!picked) {
      dismissImageBubble();
      return;
    }
    // Auto-fill alt from the picked file's basename (without ext)
    // when the user hasn't supplied one (or has left the default
    // we pre-populated on edit-entry, which counts as untouched).
    let alt = range.alt;
    if (alt === "" || alt === editingImageDefaultAlt) {
      const base = picked.split("/").pop() ?? picked;
      const dot = base.lastIndexOf(".");
      alt = dot > 0 ? base.slice(0, dot) : base;
    }
    // Preserve the original src verbatim when the user's pick
    // resolves to the same drive-rooted path; this keeps `#w=N`
    // fragments and `./` style prefixes intact across the edit.
    let finalSrc = picked;
    if (editingImageOriginal) {
      const origClean = cleanSrc(editingImageOriginal.src);
      const origNormalized = origClean.startsWith("./") || origClean.startsWith("../")
        ? currentPath
          ? resolveRelativePath(origClean, currentPath)
          : origClean
        : origClean;
      if (origNormalized === picked) {
        finalSrc = editingImageOriginal.src;
      }
    }
    const imgType = ed.schema.nodes.image;
    if (!imgType) {
      dismissImageBubble();
      return;
    }
    const orig = editingImageOriginal;
    editingImageOriginal = null;
    editingImageBracketStart = null;
    editingImageDefaultAlt = "";
    dismissImageBubble();
    const insertNode = imgType.create({ src: finalSrc, alt });
    ed.chain()
      .focus()
      .deleteRange({ from: range.start, to: range.end })
      .insertContent({
        type: "image",
        attrs: { src: finalSrc, alt },
      })
      .run();
    // Silence the unused-var warning when no original snapshot was
    // captured (the typed-`![` flow). The node was inserted above;
    // we only kept `orig`/`insertNode` for branch symmetry.
    void orig;
    void insertNode;
  }

  function dismissImageBubble(): void {
    imageBubble?.dismiss();
    imageBubble = undefined;
    // Same contract as the wiki dismiss: if the bubble was opened
    // in edit-existing mode, restore the original atom. The typed-
    // `![` flow has no original snapshot; in that case we walk the
    // doc and delete the leftover `![]()` markup.
    if (editingImageOriginal || editingImageBracketStart !== null) {
      restoreImageEditOriginal();
    }
  }

  function restoreImageEditOriginal(): void {
    if (!editor) return;
    const ed = editor;
    const orig = editingImageOriginal;
    const start = editingImageBracketStart;
    editingImageOriginal = null;
    editingImageBracketStart = null;
    editingImageDefaultAlt = "";
    if (start === null) return;
    const range = findImageRangeAt(ed, start);
    if (!range) return;
    const imgType = ed.schema.nodes.image;
    if (orig && imgType) {
      const atom = imgType.create({ src: orig.src, alt: orig.alt });
      ed.view.dispatch(ed.state.tr.replaceWith(range.start, range.end, atom));
    } else {
      // Typed-`![` flow: nothing to restore. Delete the literal
      // `![](|)` markup so the user doesn't end up with stray
      // brackets on dismiss.
      ed.view.dispatch(ed.state.tr.delete(range.start, range.end));
    }
  }

  /// Re-evaluate the open bubble on every selection / doc update.
  /// Track caret movement across the `[alt]` / `(src)` divide by
  /// flipping modes, and dismiss when the caret leaves the range.
  function syncImageBubble(): void {
    if (!editor || !imageBubble) return;
    // Suspend the dismiss path while an upload is in flight. The
    // OS file picker steals focus and PM's selection updates can
    // fire as focus returns; without this guard, syncImageBubble
    // would dismiss the bubble (and `restoreImageEditOriginal`
    // would delete the typed `![]()` markup) before the upload's
    // onUpload callback can land the new path.
    if (imageBubble.isUploading()) return;
    const range = findImageRange(editor);
    if (!range || range.mode === "outside") {
      dismissImageBubble();
      return;
    }
    imageBubble.setMode(range.mode);
    if (range.mode === "alt") {
      imageBubble.setAlt(range.alt);
    } else {
      // Normalize relative srcs against the editing file so the
      // catalog filter (drive-rooted entries) can match `./foo.png`
      // typed from a nested doc.
      let q = cleanSrc(range.src);
      if ((q.startsWith("./") || q.startsWith("../")) && currentPath) {
        q = resolveRelativePath(q, currentPath);
      }
      imageBubble.setPathQuery(q);
    }
  }

  /// Enter image edit mode by replacing the atom at `pos` with
  /// `![alt](src)` source text, then opening the bubble in path
  /// mode. Mirrors `enterWikiEditAt`. The original src + alt are
  /// snapshotted so a dismiss-without-accept can restore the atom.
  /// Tear down any open image action overlay. Idempotent; safe to
  /// call from places that don't know whether one is showing.
  function dismissImageOverlay(): void {
    if (imageOverlayDismiss) {
      imageOverlayDismiss();
      imageOverlayDismiss = undefined;
    }
  }

  /// Show a small floating overlay anchored to the clicked image's
  /// top-right corner with two actions: "Zoom" opens the image in a
  /// fullscreen viewer; "Edit" reveals the markdown source via
  /// `enterImageEditAt`. Click outside or Escape dismisses without
  /// committing. Arrow-key entry into an image bypasses this overlay
  /// and goes straight to edit, per the EDITOR.md spec.
  function openImageActionOverlay(
    imgEl: HTMLElement,
    pos: number,
    node: { attrs: Record<string, unknown>; nodeSize: number },
  ): void {
    dismissImageOverlay();
    const wrap = document.createElement("div");
    wrap.className = "md-image-actions";
    const makeBtn = (label: string, run: () => void): HTMLButtonElement => {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "md-image-action";
      btn.textContent = label;
      // Use mousedown + preventDefault so the editor's selection
      // survives the click. Click events on a document.body-mounted
      // overlay would otherwise race with PM's blur/refocus and the
      // action's editor commands would land in a stale state.
      btn.addEventListener("mousedown", (ev) => {
        ev.preventDefault();
        ev.stopPropagation();
        run();
      });
      return btn;
    };
    const zoomBtn = makeBtn("Zoom", () => {
      const src = (node.attrs.src as string) || "";
      dismissImageOverlay();
      openImageZoom(src);
    });
    const editBtn = makeBtn("Edit", () => {
      dismissImageOverlay();
      enterImageEditAt(pos, node);
    });
    wrap.appendChild(zoomBtn);
    wrap.appendChild(editBtn);
    document.body.appendChild(wrap);
    // Position over the image's top-right corner with a small inset
    // so the overlay sits ON the image, not floating in space.
    const reposition = (): void => {
      const r = imgEl.getBoundingClientRect();
      const w = wrap.offsetWidth || 120;
      wrap.style.top = `${r.top + window.scrollY + 8}px`;
      wrap.style.left = `${r.right + window.scrollX - w - 8}px`;
    };
    reposition();
    const onScroll = (): void => reposition();
    const onResize = (): void => reposition();
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", onResize);
    const onDocMouseDown = (ev: MouseEvent): void => {
      const target = ev.target as Node | null;
      if (target && wrap.contains(target)) return;
      dismissImageOverlay();
    };
    const onKey = (ev: KeyboardEvent): void => {
      if (ev.key === "Escape") {
        ev.preventDefault();
        dismissImageOverlay();
      }
    };
    document.addEventListener("mousedown", onDocMouseDown, true);
    document.addEventListener("keydown", onKey, true);
    imageOverlayDismiss = (): void => {
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", onResize);
      document.removeEventListener("mousedown", onDocMouseDown, true);
      document.removeEventListener("keydown", onKey, true);
      wrap.remove();
    };
  }

  /// Fullscreen image viewer. Renders the image centered on a dark
  /// backdrop. Click anywhere or press Escape to dismiss.
  function openImageZoom(src: string): void {
    if (!src) return;
    const resolved = resolveImageSrc(src, currentPath ?? null);
    const backdrop = document.createElement("div");
    backdrop.className = "md-image-zoom";
    const img = document.createElement("img");
    img.src = resolved;
    img.alt = "";
    img.draggable = false;
    backdrop.appendChild(img);
    document.body.appendChild(backdrop);
    const dismiss = (): void => {
      document.removeEventListener("keydown", onKey, true);
      backdrop.remove();
    };
    const onKey = (ev: KeyboardEvent): void => {
      if (ev.key === "Escape") {
        ev.preventDefault();
        dismiss();
      }
    };
    backdrop.addEventListener("click", () => dismiss());
    document.addEventListener("keydown", onKey, true);
  }

  function enterImageEditAt(pos: number, atomNode: { attrs: Record<string, unknown>; nodeSize: number }): void {
    if (!editor) return;
    const ed = editor;
    const src = (atomNode.attrs.src as string) ?? "";
    const alt = (atomNode.attrs.alt as string) ?? "";
    editingImageOriginal = { src, alt };
    editingImageBracketStart = pos;
    editingImageDefaultAlt = alt;
    const insertText = `![${alt}](${src})`;
    // Caret position inside the `(src)` parens: `![alt](` is
    // 2 + alt.length + 2 chars; we want the caret just after the
    // opening paren so the bubble opens in path mode pre-populated
    // with the current src.
    const caretInsideSrc = pos + 2 + alt.length + 2 + src.length;
    ed.chain()
      .focus()
      .insertContentAt({ from: pos, to: pos + atomNode.nodeSize }, insertText)
      .setTextSelection(caretInsideSrc)
      .run();
    openImageBubbleForCurrentCaret();
  }

  // ---- caret-driven decorations ----------------------------------------

  /// Strip the `data-cursor-*` attributes from any element decorated
  /// on the previous pass. Cheap to call on every selection change
  /// because the tracked list is small (at most a few elements).
  ///
  /// Elements whose subtree contains the active focus (the user is
  /// editing the atom's source span) are kept decorated so the CSS
  /// reveal doesn't flicker mid-edit; a follow-up pass clears them
  /// once focus returns to the editor.
  function clearCursorDecorations(): void {
    const active = document.activeElement;
    const kept: HTMLElement[] = [];
    for (const el of cursorDecorated) {
      if (active && el.contains(active)) {
        kept.push(el);
        continue;
      }
      el.removeAttribute("data-cursor-in");
      el.removeAttribute("data-cursor-prefix");
      el.removeAttribute("data-cursor-href");
      el.removeAttribute("data-cursor-md");
    }
    cursorDecorated = kept;
  }

  /// Tag the `<a>` element under the caret with `data-cursor-in` so
  /// the link-URL suffix renders via `attr(href)`. Headings and
  /// inline marks are handled by the `liveSource` PM-decoration
  /// extension; this function only covers the plain Link mark
  /// because its CSS uses `attr(href)` on the live `<a>` element and
  /// a PM-managed decoration would wrap that in a span and break
  /// the selector.
  function updateCursorDecorations(): void {
    if (!editor || !host) return;
    clearCursorDecorations();
    if (!editor.isEditable) return;
    const sel = editor.state.selection;
    if (!sel.empty) return;
    const cursor = sel.from;
    let fromPos;
    try {
      fromPos = editor.state.doc.resolve(cursor);
    } catch {
      return;
    }
    const view = editor.view;

    // Plain `<a>` Link mark covering the caret. Marks have no
    //    node DOM, so we walk up from the caret's DOM ancestor to
    //    the anchor element; CSS uses the native `href` attribute
    //    for the suffix so no extra attr-setting is needed.
    const linkType = editor.schema.marks.link;
    if (linkType) {
      const inLink =
        fromPos.nodeBefore?.marks.some((m) => m.type === linkType) ||
        fromPos.nodeAfter?.marks.some((m) => m.type === linkType);
      if (inLink) {
        try {
          const result = view.domAtPos(cursor);
          let el: HTMLElement | null =
            result.node instanceof HTMLElement
              ? result.node
              : (result.node.parentElement ?? null);
          while (el && el.tagName !== "A" && el !== host) {
            el = el.parentElement;
          }
          if (el && el.tagName === "A") {
            el.setAttribute("data-cursor-in", "");
            cursorDecorated.push(el);
          }
        } catch {
          // domAtPos can throw on invalid positions during rapid
          // updates; the next selection event will retry.
        }
      }
    }
  }

  function replaceTrailingTrigger(marker: string, after: () => void): void {
    if (!editor) return;
    // Delete `marker.length` characters before the cursor.
    const pos = editor.state.selection.from;
    const from = Math.max(0, pos - marker.length);
    editor.chain().focus().deleteRange({ from, to: pos }).run();
    after();
  }

  function onClick(e: MouseEvent): void {
    const t = e.target as HTMLElement | null;
    if (!t) return;
    // Click on the empty area below the last block (e.g. user taps
    // way down in the canvas with no text near the cursor): place
    // the caret at the end of the document, appending a fresh empty
    // paragraph if the last block already has content. Fixes the
    // common mobile gripe of "I tap below the text and nothing
    // happens" because ProseMirror won't synthesize a position when
    // the click misses every content node. Equivalent of Apple
    // Notes' tap-to-extend behaviour. Desktop benefits too.
    //
    // We only intercept when the target is the .md-wysiwyg host
    // itself: clicks inside the .ProseMirror content root are
    // already handled by ProseMirror's selection logic and we don't
    // want to override that.
    if (editor && t === host) {
      e.preventDefault();
      const { doc } = editor.state;
      const lastNode = doc.lastChild;
      const isEmptyParagraph =
        !!lastNode &&
        lastNode.type.name === "paragraph" &&
        lastNode.content.size === 0;
      const chain = editor.chain().focus("end");
      if (!isEmptyParagraph) {
        chain.insertContent({ type: "paragraph" });
      }
      chain.run();
      return;
    }
    // Click on a wiki pill: enter "edit existing link" mode. We do
    // NOT navigate on click (the user requested the bubble-style
    // editor instead, with a `>` follow button). The atom is
    // replaced with `[[label]]` text and the bubble auto-opens via
    // the existing `[[ ]]` flow; the bubble's `>` button covers
    // the previous click-to-navigate behaviour.
    const wikiEl = t.closest("[data-md-wiki]") as HTMLElement | null;
    if (wikiEl) {
      e.preventDefault();
      enterWikiEditAt(wikiEl);
      return;
    }
    // Click on an inline image atom: open the action overlay
    // (zoom + edit). Arrow-key entry still jumps straight to edit
    // via `maybeOpenAtomEditAtSelection`; click is the slow path
    // because clicks frequently land on an image as part of a
    // resize / select gesture rather than an explicit "edit" intent.
    // Image atom click target: the rendered <img>, or the wrap span
    // the node view inserts to anchor the drag-resize handle. Walk
    // up if the click landed on the handle so we still resolve the
    // image atom's position. Clicks on the handle itself preventDefault
    // up in the handle's mousedown listener so they never reach here.
    const imgEl =
      t.tagName === "IMG"
        ? (t as HTMLImageElement)
        : (t.closest(".md-image-wrap")?.querySelector("img") as HTMLImageElement | null);
    if (editor && imgEl && host && host.contains(imgEl)) {
      const ed = editor;
      let pos: number;
      try {
        pos = ed.view.posAtDOM(imgEl, 0);
      } catch {
        return;
      }
      let node = ed.state.doc.nodeAt(pos);
      if (!node || node.type.name !== "image") {
        const alt = pos - 1;
        const altNode = alt >= 0 ? ed.state.doc.nodeAt(alt) : null;
        if (altNode && altNode.type.name === "image") {
          pos = alt;
          node = altNode;
        } else {
          return;
        }
      }
      e.preventDefault();
      openImageActionOverlay(imgEl, pos, node);
      return;
    }
    if (t.matches("[data-md-date]")) {
      e.preventDefault();
      if (!editor) return;
      const ed = editor;
      // Resolve the atom's doc position via PM rather than scanning
      // by data-attrs (the old path); `openDateEditAt` is the same
      // entrypoint the NodeSelection trigger uses, so click and
      // arrow-key onto the pill behave identically.
      let pos: number;
      try {
        pos = ed.view.posAtDOM(t, 0);
      } catch {
        return;
      }
      const node = ed.state.doc.nodeAt(pos);
      if (!node || node.type.name !== "date") {
        // Some browsers report posAtDOM at the position before the
        // wrap; step back one and re-check.
        const alt = pos - 1;
        const altNode = alt >= 0 ? ed.state.doc.nodeAt(alt) : null;
        if (altNode && altNode.type.name === "date") pos = alt;
        else return;
      }
      openDateEditAt(pos, t);
      return;
    }
    // Standard markdown links saved as <a href>. Hold Cmd/Ctrl to
    // fall through to default browser behavior. Otherwise the href
    // goes through `normalizeHref`, the same resolver chan-drive
    // uses when writing graph edges, so `/abs`, `../rel`, `./rel`,
    // and bare `rel` all converge on the canonical drive-rooted
    // path. A null result means external / fragment-only / escapes
    // the drive, in which case the browser default applies.
    const a = t.closest("a") as HTMLAnchorElement | null;
    if (a && !e.metaKey && !e.ctrlKey) {
      const href = a.getAttribute("href") ?? "";
      if (!href) return;
      let decoded: string;
      try {
        decoded = decodeURIComponent(href);
      } catch {
        decoded = href;
      }
      const hashIdx = decoded.indexOf("#");
      const pathPart = hashIdx === -1 ? decoded : decoded.slice(0, hashIdx);
      const fragment = hashIdx === -1 ? "" : decoded.slice(hashIdx);
      const sourceDir = currentPath
        ? currentPath.split("/").slice(0, -1).join("/")
        : "";
      const normalized = normalizeHref(pathPart, sourceDir);
      if (normalized === null) return;
      e.preventDefault();
      handleWikiClick(normalized + fragment);
    }
  }

  // Editor density follows the user's line_spacing pref. Default
  // tight matches Google Docs spacing; standard keeps the older
  // roomier layout. Bound as a data-attribute so the CSS rules
  // below can scope to either density without rebuilding the DOM.
  const density = $derived(drive.info?.preferences?.line_spacing ?? "tight");
</script>

<div
  class="md-wysiwyg"
  class:is-readonly={readonly}
  bind:this={host}
  data-density={density}
></div>

<style>
  /* Fill the parent flex slot and scroll internally. We deliberately
     avoid `min-height` based on viewport units: in a split-pane layout
     that pushes the pane's intrinsic minimum past its allocated share
     and starves the sibling pane (its tab bar collapses to 0px). */
  .md-wysiwyg {
    flex: 1;
    min-height: 0;
    /* Extra bottom slack so the last line can scroll above the
       floating bottom pill (~92px tall counting offset + chrome).
       8rem clears it with breathing room without feeling empty. */
    padding: 1rem 1.25rem 8rem;
    line-height: 1.6;
    /* Body text uses the drive's "normal" font preference. */
    font-family: var(--chan-font-normal-family);
    font-size: var(--chan-font-normal-size, 15px);
    color: var(--text);
    background: var(--bg);
    overflow-y: auto;
  }
  :global(.md-wysiwyg .ProseMirror) {
    outline: none;
    /* Center content within the cap when --chan-page-max-width is
       set (per-device pref written by state/pageWidth). When unset,
       max-width: none restores the original full-width behavior.
       The scroll container .md-wysiwyg stays full-width so the
       scrollbar and overlays remain at the viewport edges. */
    max-width: var(--chan-page-max-width, none);
    margin-inline: auto;
  }
  /* Heading text uses the drive's heading-{1,2,3} prefs. h4..h6
     fall through to the normal text style; calling them out
     individually would just expand the settings UI without much
     practical benefit. */
  :global(.md-wysiwyg h1) {
    font-family: var(--chan-font-heading1-family);
    font-size: var(--chan-font-heading1-size, 28px);
  }
  :global(.md-wysiwyg h2) {
    font-family: var(--chan-font-heading2-family);
    font-size: var(--chan-font-heading2-size, 22px);
  }
  :global(.md-wysiwyg h3) {
    font-family: var(--chan-font-heading3-family);
    font-size: var(--chan-font-heading3-size, 18px);
  }
  /* Headings anchor the fold chevron (absolute-positioned into the
     left gutter). Without `position: relative` the chevron would
     anchor to the editor root, missing the per-line gutter. */
  :global(.md-wysiwyg :is(h1, h2, h3, h4, h5, h6)) {
    position: relative;
  }
  :global(.md-wysiwyg ::selection) { background: var(--selection-bg); }
  /* Read-only mode: hide the caret entirely (the user toggled into
     "maximize for reading"). ProseMirror still lets you click to
     position selection for copy-paste; only the visible caret is
     suppressed. */
  :global(.md-wysiwyg.is-readonly .ProseMirror) { caret-color: transparent; }

  /* Smart nodes inherit the text caret from .ProseMirror by default;
     override unconditionally. !important wins over the inherited cursor
     without us having to fight specificity for every nested element.
     `user-select: none` is critical: with `all` the browser tries to
     select the entire `.md-smart` element as a single DOM unit, and
     ProseMirror normalizes that into a much wider TextSelection,
     so a single shift+arrow next to a date node ended up selecting
     all the way back to the start of the line. Atoms manage their
     own selection via NodeSelection (TipTap sets contenteditable
     false for them); the browser's range selection should not
     participate. */
  :global(.md-wysiwyg .md-smart) {
    background: var(--smart-bg);
    border-radius: 3px;
    padding: 0 4px;
    cursor: pointer !important;
    user-select: none;
  }
  /* Date pill: same chip shape as the wiki pill, in `--warn-text`
     so the user can tell dates and links apart at a glance. The
     base `.md-smart` rules supply background / cursor / user-
     select; this rule overrides shape + size only. */
  :global(.md-wysiwyg .md-smart-date) {
    color: var(--warn-text);
    border-radius: 999px;
    padding: 0.05em 0.55em;
    font-size: 0.95em;
  }
  :global(.md-wysiwyg .md-smart-date:hover) {
    filter: brightness(1.1);
  }
  /* Wiki link rendered as a rounded chip pill (Google Docs-style
     mention chip): accent text on a soft background, no underline,
     pill border-radius, slight horizontal padding. The base
     `.md-smart` rules take care of background/cursor/user-select;
     we only override the radius / padding / decoration here. */
  :global(.md-wysiwyg .md-smart-wiki) {
    color: var(--link);
    text-decoration: none;
    border-radius: 999px;
    padding: 0.05em 0.55em;
    font-size: 0.95em;
  }
  :global(.md-wysiwyg .md-smart-wiki:hover) {
    filter: brightness(1.1);
  }
  /* Plain markdown links: pointer too (we hijack internal ones in onClick). */
  :global(.md-wysiwyg a) {
    cursor: pointer !important;
    color: var(--link);
  }
  /* Tight (default, gdocs-like) and standard (older) density rules.
     ProseMirror wraps each <li> content in a <p>; the default
     paragraph margins make lists look double-spaced unless we
     zero them. The body line-height drops too in tight mode so
     paragraphs of prose match the list cadence. */
  :global(.md-wysiwyg[data-density="tight"]) { line-height: 1.4; }
  :global(.md-wysiwyg[data-density="tight"] p) { margin: 0; }
  :global(.md-wysiwyg[data-density="tight"] ul),
  :global(.md-wysiwyg[data-density="tight"] ol) {
    margin: 0;
    padding-left: 1.5em;
  }
  :global(.md-wysiwyg[data-density="tight"] li) { margin: 0; }
  :global(.md-wysiwyg[data-density="tight"] li > p) { margin: 0; }
  :global(.md-wysiwyg[data-density="tight"] li > ul),
  :global(.md-wysiwyg[data-density="tight"] li > ol) {
    margin: 0;
  }

  :global(.md-wysiwyg[data-density="standard"] ul),
  :global(.md-wysiwyg[data-density="standard"] ol) {
    margin: 0.5em 0;
    padding-left: 1.5em;
  }
  :global(.md-wysiwyg[data-density="standard"] li) { margin: 0; }
  :global(.md-wysiwyg[data-density="standard"] li > p) { margin: 0; }
  :global(.md-wysiwyg[data-density="standard"] li > ul),
  :global(.md-wysiwyg[data-density="standard"] li > ol) {
    margin: 0.15em 0 0.15em 0;
  }
  /* Task lists: GitHub-flavored markdown checkboxes. The list is a
     plain <ul data-type="taskList"> with no bullet markers; each
     <li data-checked="..."> hosts a checkbox label + the content.
     Layout: checkbox flush left, content flowing to its right.
     Checked items get a strikethrough on the inner <p>. */
  :global(.md-wysiwyg ul[data-type="taskList"]) {
    list-style: none;
    padding-left: 0;
  }
  :global(.md-wysiwyg ul[data-type="taskList"] li) {
    display: flex;
    align-items: flex-start;
    gap: 0.4em;
  }
  :global(.md-wysiwyg ul[data-type="taskList"] li > label) {
    flex-shrink: 0;
    user-select: none;
    margin-top: 0.15em;
  }
  :global(.md-wysiwyg ul[data-type="taskList"] li > div) {
    flex: 1;
    min-width: 0;
  }
  :global(.md-wysiwyg ul[data-type="taskList"] li[data-checked="true"] > div) {
    color: var(--text-secondary);
    text-decoration: line-through;
  }
  :global(.md-wysiwyg blockquote) {
    border-left: 3px solid var(--border);
    padding-left: 0.75rem;
    color: var(--text-secondary);
    margin: 0.5em 0;
    font-family: var(--chan-font-quote-family);
    font-size: var(--chan-font-quote-size, 15px);
  }
  :global(.md-wysiwyg pre) {
    background: var(--code-bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 0.5rem 0.75rem;
    overflow-x: auto;
  }
  :global(.md-wysiwyg code),
  :global(.md-wysiwyg pre) {
    font-family: var(--chan-font-code-family);
    font-size: var(--chan-font-code-size, 14px);
  }
  :global(.md-wysiwyg code) {
    background: var(--code-bg);
    padding: 0 0.25rem;
    border-radius: 3px;
  }

  /* Floating popovers (calendar, file picker) live at document.body, so
     they need their own theming hook. */
  :global(.md-cal) {
    background: var(--bg-elev);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,.4);
    padding: .5rem;
    font-size: 15px;
  }
  /* Format dropdown row at the top of the calendar popover. The
     preview span on the right shows what the cursor's date will
     look like once picked, so the user can sanity-check before
     clicking. */
  :global(.md-cal-fmt) {
    display: flex;
    align-items: center;
    gap: .35rem;
    margin-bottom: .35rem;
    font-size: 14px;
  }
  :global(.md-cal-fmt-label) {
    color: var(--text-secondary);
  }
  :global(.md-cal-fmt select) {
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 1px 4px;
    font: inherit;
  }
  :global(.md-cal-fmt-preview) {
    margin-left: auto;
    color: var(--warn-text);
    background: var(--smart-bg);
    border-radius: 3px;
    padding: 0 4px;
  }
  :global(.md-cal-head) {
    display: flex;
    align-items: center;
    gap: .15rem;
    margin-bottom: .35rem;
  }
  :global(.md-cal-head button) {
    background: none;
    border: 1px solid transparent;
    cursor: pointer;
    font-size: 15px;
    color: var(--text-secondary);
    padding: 0 .35rem;
    height: 1.4rem;
    border-radius: 3px;
  }
  :global(.md-cal-head button:hover) {
    color: var(--text);
    border-color: var(--btn-border);
  }
  :global(.md-cal-label) {
    flex: 1;
    text-align: center;
    cursor: pointer;
    color: var(--text);
    font-weight: 500;
    user-select: none;
  }
  :global(.md-cal-label:hover) { color: var(--link); }
  /* Weekday header: same column grid as the day grid below so
     letters line up under each column. Centred small caps. */
  :global(.md-cal-dow) {
    display: grid;
    grid-template-columns: repeat(7, 1.6rem);
    gap: 2px;
    margin-bottom: 2px;
    color: var(--text-secondary);
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  :global(.md-cal-dow > div) { text-align: center; }
  :global(.md-cal-grid) {
    display: grid; grid-template-columns: repeat(7, 1.6rem); gap: 2px;
  }
  :global(.md-cal-day) {
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid transparent;
    height: 1.6rem;
    cursor: pointer;
    border-radius: 3px;
    padding: 0;
    font: inherit;
  }
  :global(.md-cal-day:hover) { background: var(--hover-bg); }
  /* Today gets an accent dot via a left border; subtle but
     scannable. The keyboard cursor gets a stronger ring so the
     user can see where their next Enter will land. */
  :global(.md-cal-day.today) {
    color: var(--accent);
    font-weight: 600;
  }
  :global(.md-cal-day.cursor) {
    border-color: var(--link);
    background: var(--hover-bg);
  }
  /* Center the dow header + day grid horizontally inside the panel.
     The grid itself is fixed at 7 * 1.6rem; the wrapping flex
     centers it so the calendar reads as a deliberate block instead
     of hugging the left edge of the wider format / nav rows. */
  :global(.md-cal-gridwrap) {
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  /* Action row at the bottom: [Today] [spacer] [Cancel] [OK].
     Mirrors PromptModal's button styling (rounded, accented OK)
     so the date popover feels like a sibling of the other modals. */
  :global(.md-cal-actions) {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-top: 0.5rem;
    padding-top: 0.4rem;
    border-top: 1px solid var(--border);
  }
  :global(.md-cal-spacer) { flex: 1; }
  :global(.md-cal-action) {
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    padding: 0.3rem 0.75rem;
    font: inherit;
    cursor: pointer;
  }
  :global(.md-cal-action:hover) { border-color: var(--btn-hover); }
  :global(.md-cal-action.ok) {
    background: var(--link);
    border-color: var(--link);
    color: #fff;
  }
  :global(.md-cal-action.today) {
    color: var(--text-secondary);
  }

  :global(.md-pick) {
    background: var(--bg-elev);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,.4);
    width: 320px;
  }
  :global(.md-pick-input) {
    box-sizing: border-box; width: 100%; padding: .4rem .5rem;
    background: transparent;
    color: var(--text);
    border: 0; border-bottom: 1px solid var(--border);
    outline: none; font: inherit;
  }
  :global(.md-pick-list) { list-style: none; margin: 0; padding: 0; max-height: 220px; overflow-y: auto; }
  :global(.md-pick-list li) { padding: .3rem .5rem; cursor: pointer; }
  :global(.md-pick-list li.active),
  :global(.md-pick-list li:hover) { background: var(--hover-bg); }

  /* Image picker has a wider footprint than the wiki picker
     because it carries an upload button + a URL input below the
     search results. Keep the column compact-ish so it doesn't
     dwarf the editor. */
  :global(.md-pick-image) { width: 380px; }
  :global(.md-pick-footer) {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 6px 6px 8px;
    border-top: 1px solid var(--border);
  }
  :global(.md-pick-action) {
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 3px;
    padding: 4px 8px;
    cursor: pointer;
    font: inherit;
    font-size: 14px;
  }
  :global(.md-pick-action:hover:not(:disabled)) { border-color: var(--btn-hover); }
  :global(.md-pick-action:disabled) { opacity: 0.6; cursor: default; }
  :global(.md-pick-url),
  :global(.md-pick-alt) {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 4px 6px;
    font: inherit;
    font-size: 14px;
    outline: none;
  }
  :global(.md-pick-url:focus),
  :global(.md-pick-alt:focus) { border-color: var(--link); }

  /* Wiki-link bubble. Anchored under the caret while the user
     types between `[[ ]]`. Non-focus-stealing: no inputs, no
     tab targets, only mousedown handlers that preserve the
     editor selection. */
  :global(.md-wiki-bubble) {
    background: var(--bg-elev);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,.4);
    width: 360px;
    font-size: 13px;
    user-select: none;
  }
  :global(.md-wiki-bubble-head) {
    padding: .35rem .55rem;
    border-bottom: 1px solid var(--border);
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  :global(.md-wiki-bubble-head.is-empty) { color: var(--muted); }
  :global(.md-wiki-bubble-hint) {
    display: flex;
    flex-wrap: wrap;
    gap: .35rem .9rem;
    padding: .25rem .55rem;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 11px;
  }
  :global(.md-wiki-bubble-hint b) { color: var(--text); font-weight: 600; }
  :global(.md-wiki-bubble-results) {
    list-style: none; margin: 0; padding: 0;
    max-height: 180px; overflow-y: auto;
  }
  :global(.md-wiki-bubble-results.is-empty) { display: none; }
  :global(.md-wiki-bubble-results li) {
    padding: .3rem .55rem; cursor: pointer;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  :global(.md-wiki-bubble-results li.active),
  :global(.md-wiki-bubble-results li:hover) { background: var(--hover-bg); }
  /* Heading rows: monospace so the leading `#`s line up and the
     text is visually distinct from the file-path rows. */
  :global(.md-wiki-bubble-results li.is-heading) {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
  }
  /* Block rows: same monospace as headings but italic to suggest
     "raw text". The expanded preview below carries the full body. */
  :global(.md-wiki-bubble-results li.is-block) {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
    font-style: italic;
    color: var(--muted);
  }
  :global(.md-wiki-bubble-results li.is-block.active) { color: var(--text); }
  /* Block preview: shows the active block expanded with the typed
     query highlighted. Whitespace is preserved (multi-line blocks
     are visible) and a max height keeps long blocks scrollable. */
  :global(.md-wiki-bubble-preview) {
    padding: .35rem .55rem;
    border-top: 1px solid var(--border);
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
    color: var(--text);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 120px;
    overflow-y: auto;
  }
  :global(.md-wiki-bubble-preview.is-hidden) { display: none; }
  :global(.md-wiki-bubble-preview mark) {
    background: var(--hover-bg);
    color: var(--link);
    padding: 0 1px;
    border-radius: 2px;
  }
  /* Display-text row. Faded code-block-ish background so the user
     sees this is a transient input preview, not part of the
     persisted note. The label fades to muted when populated so the
     typed value reads as the active content. */
  :global(.md-wiki-bubble-display) {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: .35rem .55rem;
    border-top: 1px solid var(--border);
    background: color-mix(in srgb, var(--hover-bg) 60%, transparent);
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
  }
  :global(.md-wiki-bubble-display.is-hidden) { display: none; }
  :global(.md-wiki-bubble-display-label) {
    color: var(--muted);
    opacity: 0.85;
    font-style: italic;
  }
  :global(.md-wiki-bubble-display-label.is-active) { opacity: 0.45; }
  :global(.md-wiki-bubble-display-arrow) { color: var(--muted); }
  :global(.md-wiki-bubble-display-value) {
    color: var(--text);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Footer row: accept hint on the left, follow button on the
     right when in edit-existing mode. Flex layout so both share
     the row instead of overlapping. The dashed separator hugs
     the row's top edge to match the prior accept-only design. */
  :global(.md-wiki-bubble-footer) {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: .35rem .55rem;
    border-top: 1px dashed var(--border);
  }
  :global(.md-wiki-bubble-accept) {
    color: var(--muted);
    font-size: 11px;
    flex: 1;
  }
  :global(.md-wiki-bubble-accept.is-hidden) { display: none; }

  /* Follow button rendered by the wiki bubble in edit-existing
     mode. Reads as a clear primary action — accent fill, label
     plus chevron — so a user opening the bubble to navigate (the
     common case) doesn't have to hunt for it. */
  :global(.md-wiki-bubble-follow) {
    background: var(--accent);
    color: #fff;
    border: 1px solid var(--accent);
    border-radius: 4px;
    padding: 5px 12px;
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    line-height: 1.2;
    margin-left: auto;
    box-shadow: 0 1px 2px rgba(0,0,0,.25);
  }
  :global(.md-wiki-bubble-follow:hover),
  :global(.md-wiki-bubble-follow.is-active) {
    filter: brightness(1.15);
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  /* Caret-driven source-mode decorations. Set by
     `updateCursorDecorations` on every selection change. */

  /* Heading hash prefix shown only when the caret is on the
     heading line. The space after `attr()` keeps the gap between
     the hashes and the heading text. */
  :global(.md-wysiwyg :is(h1, h2, h3, h4, h5, h6)[data-cursor-in])::before {
    content: attr(data-cursor-prefix) "\00a0";
    color: var(--text-secondary);
    opacity: 0.45;
    font-weight: normal;
  }

  /* `[[` / `]]` brackets surfaced by the wiki create / edit flows
     render muted so the user's query text reads as the primary
     content. The class is applied via Decoration.inline; only the
     two-char bracket ranges are decorated, not the label between
     them. */
  :global(.md-wysiwyg .md-wiki-bracket) {
    color: var(--text-secondary);
    opacity: 0.5;
  }

  /* Inline-mark source markers (bold / italic / strike). The
     `liveSource` plugin inserts these as non-editable widget
     decorations at the mark range boundaries when the caret is
     in the mark. Visual: muted same-color text that inherits the
     surrounding font-weight / style so `**` looks bold next to
     bold text, and `*` looks italic next to italic text. */
  :global(.md-wysiwyg .md-source-marker) {
    color: var(--text-secondary);
    opacity: 0.45;
    user-select: none;
  }
  /* While editing a strike, drop the strikethrough line so the
     text stays readable. PM's Decoration.inline may land the class
     either on the `<s>` element directly or on a wrapping `<span>`,
     depending on how it merges with the mark; cover both, and use
     !important to beat the UA style on `<s>` regardless of which
     case we hit. Nested marks (e.g. bold inside strike) are
     covered by the descendant selector. */
  :global(.md-wysiwyg .md-mark-editing-strike),
  :global(.md-wysiwyg .md-mark-editing-strike s),
  :global(.md-wysiwyg .md-mark-editing-strike *),
  :global(.md-wysiwyg s.md-mark-editing-strike) {
    text-decoration: none !important;
  }

  /* Fenced code block (`CodeBlockFenced` NodeView). The wrap is
     the styled box; fences sit inside it as monospace rows. The
     language is a real `<input>` so PM treats it as opaque and we
     get native focus/blur/caret behavior. */
  :global(.md-wysiwyg .md-codeblock) {
    background: var(--bg-elev);
    border-radius: 4px;
    padding: 8px 12px;
    font-family: var(--chan-font-mono-family, monospace);
    font-size: 0.9em;
    line-height: 1.4;
    margin: 0.5em 0;
  }
  :global(.md-wysiwyg .md-codeblock-fence) {
    color: var(--text-secondary);
    opacity: 0.6;
    user-select: none;
    display: flex;
    align-items: center;
    gap: 2px;
  }
  :global(.md-wysiwyg .md-codeblock-content) {
    background: transparent;
    margin: 0;
    padding: 0;
    border: none;
    font: inherit;
    color: var(--text);
  }
  :global(.md-wysiwyg .md-codeblock-content code) {
    background: transparent;
    padding: 0;
    font: inherit;
  }
  /* The language `<input>` styled to look like inline text inside
     the fence row: no border, transparent background, inherits the
     monospace font so it doesn't visually break the fence line. */
  :global(.md-wysiwyg .md-codeblock-lang) {
    background: transparent;
    border: none;
    outline: none;
    padding: 0;
    margin: 0;
    color: var(--text);
    font: inherit;
    width: 8ch;
  }
  :global(.md-wysiwyg .md-codeblock-lang::placeholder) {
    color: var(--text-secondary);
    opacity: 0.4;
  }

  /* Wiki link click flow lives in the bubble (see
     `.md-wiki-bubble-follow`); no per-pill source span. */

  /* Plain markdown links (Link mark) wrap their text in `<a href>`.
     Marks have no node view, so the URL is shown read-only via a
     `::after` pseudo when the caret enters the mark range. The
     mark's text itself is editable in place; URL editing is not
     supported for plain links (use a wiki link for that). */
  :global(.md-wysiwyg a[data-cursor-in])::before {
    content: "[";
    color: var(--text-secondary);
    opacity: 0.55;
  }
  :global(.md-wysiwyg a[data-cursor-in])::after {
    content: "](" attr(href) ")";
    color: var(--text-secondary);
    opacity: 0.55;
  }

  /* Heading fold chevron rendered by the foldHeading plugin. The
     chevron sits inside the heading element (widget side: -1) and
     rotates between `▾` (open) and `▸` (folded). Click toggles
     the fold via the plugin's `handleClick` prop. The negative
     left margin pulls the chevron into the gutter so the heading
     text aligns with non-folded headings; with `flex` on the
     heading itself the chevron stays vertically centered.
     Inline-block + reserved width keeps long-press hit area
     reachable on touch. */
  /* Chevron sits in the left gutter, absolutely positioned so it
     never overlaps inline content (notably the `## ` source-mode
     prefix the liveSource extension reveals when the caret is on
     the heading line). Heading text starts at offset 0 — the
     chevron lives entirely in the parent's left padding. Obsidian
     and gdocs lay out their chevrons the same way. */
  :global(.md-wysiwyg .md-fold-chevron) {
    position: absolute;
    left: -1.5em;
    top: 50%;
    transform: translateY(-50%);
    display: inline-block;
    width: 1em;
    color: var(--text-secondary);
    cursor: pointer;
    user-select: none;
    font-size: 0.7em;
    line-height: 1;
    opacity: 0.5;
    transition: opacity 0.15s ease;
    font-weight: normal;
    text-align: center;
  }
  :global(.md-wysiwyg .md-fold-chevron:hover),
  :global(.md-wysiwyg .md-fold-chevron[data-folded="true"]) {
    opacity: 1;
  }
  /* Ellipsis cue at the end of a folded heading, signalling that
     there's hidden content below. */
  :global(.md-wysiwyg .md-fold-ellipsis) {
    color: var(--text-secondary);
    user-select: none;
    margin-left: 0.25em;
    opacity: 0.6;
  }
  /* Blocks under a folded heading get this class via a node
     decoration; CSS hides them entirely. The chevron + heading
     stay visible. */
  :global(.md-wysiwyg .md-fold-hidden) {
    display: none;
  }

  /* `#tag` inline pill. The decoration plugin scans the doc for
     `#word` runs; CSS turns each into a rounded chip that visually
     matches the file-info tag chips. Click handling lives in the
     plugin's `handleClick` prop. */
  :global(.md-wysiwyg .md-tag-pill) {
    background: var(--smart-bg);
    color: var(--accent);
    border-radius: 999px;
    padding: 0.05em 0.5em;
    font-size: 0.92em;
    cursor: pointer;
    text-decoration: none;
  }
  :global(.md-wysiwyg .md-tag-pill:hover) {
    filter: brightness(1.1);
  }

  /* Tag autocomplete bubble. Same anchored-under-caret pattern as
     the wiki bubble; narrower because tag names are short. */
  :global(.md-tag-bubble) {
    background: var(--bg-elev);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,.4);
    width: 220px;
    font-size: 13px;
    user-select: none;
  }
  :global(.md-tag-bubble-results) {
    list-style: none; margin: 0; padding: 0;
    max-height: 180px; overflow-y: auto;
  }
  :global(.md-tag-bubble-results li) {
    padding: .3rem .55rem; cursor: pointer;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    color: var(--link);
  }
  :global(.md-tag-bubble-results li.active),
  :global(.md-tag-bubble-results li:hover) { background: var(--hover-bg); }

  /* Contact picker bubble (@). Same anchored-under-caret pattern;
     two-line rows (display name + first email) so the user can tell
     similarly-named contacts apart without expanding the popover. */
  :global(.md-contact-bubble) {
    background: var(--bg-elev);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,.4);
    width: 280px;
    font-size: 13px;
    user-select: none;
  }
  :global(.md-contact-bubble-results) {
    list-style: none; margin: 0; padding: 0;
    max-height: 220px; overflow-y: auto;
  }
  :global(.md-contact-bubble-results li) {
    padding: .3rem .55rem; cursor: pointer;
    display: flex; flex-direction: column; gap: 1px;
    overflow: hidden;
  }
  :global(.md-contact-bubble-results li.active),
  :global(.md-contact-bubble-results li:hover) { background: var(--hover-bg); }
  :global(.md-contact-bubble-primary) {
    color: var(--link);
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  :global(.md-contact-bubble-secondary) {
    color: var(--text-secondary, var(--text));
    opacity: .7;
    font-size: 12px;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }

  /* Inline images: keep them from blowing the editor column out
     by capping max-width. The native size renders if it fits.
     The wrapper (`.md-image-wrap`) carries the resize handle in
     the bottom-right corner; visible on hover or while dragging
     to avoid clutter on a page full of images. */
  :global(.md-wysiwyg img) {
    max-width: 100%;
    height: auto;
    border-radius: 3px;
    /* Bottom-align inline images so they sit on the same baseline
       as surrounding text instead of pulling line height. Mirrors
       the way most prose engines render inline figures. */
    vertical-align: bottom;
  }
  /* ProseMirror-rendered images: same baseline-align so an image
     sharing a line with text doesn't push the cap height up. */
  :global(.md-wysiwyg .ProseMirror img) {
    vertical-align: bottom;
  }
  :global(.md-image-wrap) {
    position: relative;
    display: inline-block;
    line-height: 0;
  }
  :global(.md-image-handle) {
    position: absolute;
    right: -4px;
    bottom: -4px;
    width: 12px;
    height: 12px;
    background: var(--link);
    border: 2px solid var(--bg);
    border-radius: 2px;
    cursor: nwse-resize;
    opacity: 0;
    transition: opacity 0.15s ease;
  }
  :global(.md-image-wrap:hover .md-image-handle),
  :global(.md-image-wrap.is-resizing .md-image-handle) {
    opacity: 1;
  }
  :global(.md-image-wrap.is-resizing img) {
    /* Disable image-drag during resize so the user doesn't
       accidentally drag the image instead of grabbing the handle. */
    pointer-events: none;
    user-select: none;
  }

  /* Image bubble. Mirrors the wiki bubble (anchored under the
     caret, no focus stealing) but lays out three optional rows:
     preview, results, alt-echo. Width matches the wiki bubble so
     the visual rhythm stays consistent across triggers. */
  :global(.md-image-bubble) {
    background: var(--bg-elev);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0,0,0,.4);
    width: 360px;
    font-size: 13px;
    user-select: none;
  }
  /* Thumbnail preview of the active result. Fixed max height so a
     tall image doesn't push the result list off-screen. */
  :global(.md-image-bubble-preview) {
    padding: .4rem;
    border-bottom: 1px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg);
  }
  :global(.md-image-bubble-preview.is-hidden) { display: none; }
  :global(.md-image-bubble-preview img) {
    max-width: 100%;
    max-height: 120px;
    object-fit: contain;
    border-radius: 2px;
  }
  /* Result list. Same shape as the wiki / tag lists. */
  :global(.md-image-bubble-list) {
    list-style: none; margin: 0; padding: 0;
    max-height: 180px; overflow-y: auto;
  }
  :global(.md-image-bubble-list.is-hidden) { display: none; }
  :global(.md-image-bubble-list.is-empty) { display: none; }
  :global(.md-image-bubble-list li) {
    padding: .3rem .55rem; cursor: pointer;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
  }
  :global(.md-image-bubble-list li.active),
  :global(.md-image-bubble-list li:hover) { background: var(--hover-bg); }

  /* Alt-mode echo row. Replaces the result list when the caret
     sits inside `[alt]`. Same horizontal padding as the list rows
     so the visual column stays stable across mode flips. */
  :global(.md-image-bubble-alt) {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: .4rem .55rem;
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
  }
  :global(.md-image-bubble-alt.is-hidden) { display: none; }
  :global(.md-image-bubble-alt-label) {
    color: var(--muted);
    font-style: italic;
  }
  :global(.md-image-bubble-alt-value) {
    color: var(--text);
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  :global(.md-image-bubble-alt-value.is-empty) {
    color: var(--muted);
    font-style: italic;
  }

  /* Footer: upload button (left) + accept hint (right). Flex with
     `accept` taking the rest of the row so the two siblings stay
     spaced. Same dashed-top border the wiki bubble uses. */
  :global(.md-image-bubble-footer) {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: .35rem .55rem;
    border-top: 1px dashed var(--border);
  }
  :global(.md-image-bubble-upload) {
    background: var(--accent);
    color: #fff;
    border: 1px solid var(--accent);
    border-radius: 4px;
    padding: 4px 10px;
    cursor: pointer;
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    line-height: 1.2;
  }
  :global(.md-image-bubble-upload:hover),
  :global(.md-image-bubble-upload.is-active) {
    filter: brightness(1.15);
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  :global(.md-image-bubble-upload:disabled) {
    opacity: 0.55;
    cursor: progress;
  }
  :global(.md-image-bubble-accept) {
    color: var(--muted);
    font-size: 11px;
    margin-left: auto;
  }
  :global(.md-image-bubble-accept.is-hidden) { display: none; }

  /* Error row. Surfaced when an upload fails or exceeds the size
     cap. Sits between the alt/list region and the footer so the
     accept hint stays in place. */
  :global(.md-image-bubble-error) {
    padding: .35rem .55rem;
    border-top: 1px solid var(--border);
    color: var(--danger, #e57373);
    background: color-mix(in srgb, var(--danger, #e57373) 12%, transparent);
    font-size: 12px;
  }
  :global(.md-image-bubble-error.is-hidden) { display: none; }

  /* Image action overlay (Zoom / Edit). Floats over the clicked
     image's top-right corner; click outside or Esc dismisses. The
     buttons inherit the editor's foreground color so they read
     against either a bright or a dark image background. */
  :global(.md-image-actions) {
    position: absolute;
    z-index: 30000;
    display: inline-flex;
    gap: 2px;
    background: rgba(20, 20, 20, 0.85);
    color: #fff;
    border-radius: 6px;
    padding: 2px;
    backdrop-filter: blur(4px);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  }
  :global(.md-image-action) {
    background: transparent;
    color: inherit;
    border: 0;
    border-radius: 4px;
    padding: 4px 10px;
    font: inherit;
    font-size: 12px;
    cursor: pointer;
    line-height: 1.2;
  }
  :global(.md-image-action:hover) {
    background: rgba(255, 255, 255, 0.15);
  }

  /* Fullscreen image viewer triggered by the action overlay's
     "Zoom" button. Click anywhere on the backdrop or press Esc to
     dismiss. The image scales down to fit but never up; we don't
     want to upscale a small drawing into a pixelated mess. */
  :global(.md-image-zoom) {
    position: fixed;
    inset: 0;
    z-index: 40000;
    background: rgba(0, 0, 0, 0.92);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: zoom-out;
  }
  :global(.md-image-zoom img) {
    max-width: 92vw;
    max-height: 92vh;
    width: auto;
    height: auto;
    object-fit: contain;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }
</style>
