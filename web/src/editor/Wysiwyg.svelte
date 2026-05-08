<script lang="ts">
  // TipTap-based WYSIWYG editor with smart-node extensions for
  // @date, [[wiki]], and ![image]. Two-way bound to the parent's
  // `value` (markdown text). Round-trips through tiptap-markdown.
  //
  // Trigger handling: we listen for input events. When the buffer
  // gains `@today`, `@date`, `[[`, or `![`, we insert the
  // corresponding node and clean up the trigger text. (`@today`
  // is just a shortcut: it inserts a `date` node prefilled with
  // today's date so the styling and round-trip semantics match
  // `@date`.)

  import { onDestroy, onMount } from "svelte";
  import { Editor } from "@tiptap/core";
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
  import {
    ImageNode,
    isImagePath,
    showImagePicker,
    uploadImageFile,
  } from "./extensions/image";
  import {
    createWikiLinkNode,
    handleWikiClick,
    openWikiBubble,
    type WikiBubble,
  } from "./extensions/wikiLink";
  import { api } from "../api/client";
  import { resolveRelativePath } from "../api/wasm";
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
  $effect(() => {
    if (!editor) return;
    editor.setEditable(!readonly);
    if (readonly) dismissWikiBubble();
  });

  /// Wiki-link bubble. Open while the caret sits between an
  /// auto-paired `[[ ]]` in the editor. The bubble is informational
  /// (no focus); the caret stays inside the brackets and the user's
  /// typing IS the search query. Wysiwyg owns the keyboard and
  /// selection-tracking; the bubble owns its own DOM and result
  /// list. Cleared when the caret leaves the bracket range, on
  /// Escape, on accept, or on editor destroy.
  let wikiBubble: WikiBubble | undefined;

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
        StarterKit,
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
        ImageNode,
      ],
      content: value,
      // Cmd/Ctrl+Enter -> parent's onSubmit (assistant prompt
      // case). Drop / paste hooks funnel image files and image
      // URLs through `handleImageInsert` so the picker, drag-drop,
      // and clipboard paste flows all share one upload + node-
      // insert path.
      editorProps: {
        handleKeyDown: (_view, event) => {
          // Wiki bubble owns Enter / Escape / Arrow keys while the
          // caret is inside `[[ ]]`. We check bubble state instead
          // of just findBracketRange() so that an empty `[[ ]]`
          // typed by the user without going through the trigger
          // path doesn't accidentally swallow Enter.
          if (wikiBubble) {
            if (event.key === "Enter") {
              event.preventDefault();
              acceptWikiBubble();
              return true;
            }
            if (event.key === "Escape") {
              event.preventDefault();
              dismissWikiBubble();
              return true;
            }
            if (event.key === "ArrowDown") {
              event.preventDefault();
              wikiBubble.moveActive(1);
              return true;
            }
            if (event.key === "ArrowUp") {
              event.preventDefault();
              wikiBubble.moveActive(-1);
              return true;
            }
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
          return false;
        },
        handleDrop: (view, event, _slice, moved) => {
          // `moved` is true for in-editor drag-rearrange; we let
          // ProseMirror's default handler take that case.
          if (moved) return false;
          const dt = (event as DragEvent).dataTransfer;
          if (!dt) return false;
          const imageFiles = Array.from(dt.files).filter((f) =>
            f.type.startsWith("image/"),
          );
          if (imageFiles.length === 0) return false;
          event.preventDefault();
          const coords = view.posAtCoords({
            left: (event as DragEvent).clientX,
            top: (event as DragEvent).clientY,
          });
          const at = coords?.pos ?? view.state.selection.from;
          void insertImageFilesAt(at, imageFiles);
          return true;
        },
        handlePaste: (view, event) => {
          const cd = (event as ClipboardEvent).clipboardData;
          if (!cd) return false;
          // First: any image files in the clipboard items? Both
          // direct file paste and clipboard images (Cmd+V from a
          // screenshot tool) land here as `kind: 'file'`.
          const files = Array.from(cd.items)
            .filter((it) => it.kind === "file")
            .map((it) => it.getAsFile())
            .filter((f): f is File => !!f && f.type.startsWith("image/"));
          if (files.length > 0) {
            event.preventDefault();
            const at = view.state.selection.from;
            void insertImageFilesAt(at, files);
            return true;
          }
          // Second: pasted text that looks like an image URL.
          // Accept http(s) URLs whose path component ends in a
          // known image extension. Anything else falls through
          // to the default text-paste handler.
          const text = cd.getData("text/plain").trim();
          if (text && /^https?:\/\//i.test(text)) {
            try {
              const u = new URL(text);
              if (isImagePath(u.pathname)) {
                event.preventDefault();
                const last = u.pathname.split("/").pop() ?? "";
                const alt = last.replace(/\.[^./]+$/, "");
                editor!
                  .chain()
                  .focus()
                  .insertContent({ type: "image", attrs: { src: text, alt } })
                  .insertContent(" ")
                  .run();
                return true;
              }
            } catch {
              // malformed URL: fall through to plain paste.
            }
          }
          return false;
        },
      },
      onUpdate: ({ editor }) => {
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
        onSelectionChange?.();
      },
      onSelectionUpdate: () => {
        syncWikiBubble();
        onSelectionChange?.();
      },
    });
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
      // External content change = tab switch or fresh load. Refocus
      // so the user can keep typing without clicking.
      editor.commands.focus("start");
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
  /// `link` mark whose href is internal-looking (no scheme; passes
  /// `isInternalHref`), we replace the marked range with a fresh
  /// `wikiLink` atom node carrying the decoded target + the
  /// rendered text as the label. External http(s)/mailto links are
  /// left as Link marks. Idempotent: a doc with only existing
  /// wikiLink nodes (no Link marks) walks to no replacements.
  function decorateWikiLinks(): void {
    if (!editor) return;
    const wikiType = editor.schema.nodes.wikiLink;
    const linkMarkType = editor.schema.marks.link;
    if (!wikiType || !linkMarkType) return;

    type Range = { from: number; to: number; target: string; label: string };
    const ranges: Range[] = [];

    editor.state.doc.descendants((node, pos) => {
      if (!node.isText || !node.text) return;
      const linkMark = node.marks.find((m) => m.type === linkMarkType);
      if (!linkMark) return;
      const href = (linkMark.attrs.href as string | null) ?? "";
      if (!href || !isInternalHref(href)) return;
      // Decode the href once: chan-shared encodes spaces / parens
      // when serializing, so the on-disk form looks like
      // `my%20note.md`; the wikiLink attr expects the human-
      // readable path. After decoding, hrefs that look relative
      // (`./foo.md` / `../docs/foo.md`) get resolved against the
      // current file's directory so the atom's `target` is always
      // the canonical drive-rooted path the click handler expects.
      let target: string;
      try {
        target = decodeURIComponent(href);
      } catch {
        target = href;
      }
      if (currentPath) {
        target = resolveRelativePath(target, currentPath);
      }
      ranges.push({
        from: pos,
        to: pos + node.text.length,
        target,
        label: node.text,
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
        wikiType.create({ target: r.target, label: r.label }),
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

  function onInput(_e: Event): void {
    if (!editor) return;
    // Look at text immediately before the cursor (up to 16 chars). This is
    // more reliable than matching the serialized markdown, which may have
    // trailing newlines or surrounding content that defeat end-anchors.
    const { from } = editor.state.selection;
    const before = editor.state.doc.textBetween(Math.max(0, from - 16), from, "\n", "\n");

    if (before.endsWith("@today")) {
      replaceTrailingTrigger("@today", () => {
        // @today and @date both produce dates; we use the same
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
    if (before.endsWith("@date")) {
      replaceTrailingTrigger("@date", () => {
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
    if (before.endsWith("![")) {
      // The trigger eats both characters; the picker resolves
      // with either a drive-relative path, a remote URL, or
      // null (cancel). Alt text comes from the filename so the
      // markdown round-trip carries something readable.
      replaceTrailingTrigger("![", () => {
        const anchor = caretAnchorHost();
        showImagePicker(anchor, (src) => {
          if (!src || !editor) return;
          const last = src.split("/").pop() ?? src;
          const alt = last.replace(/\.[^./]+$/, "");
          editor
            .chain()
            .focus()
            .insertContent({ type: "image", attrs: { src, alt } })
            .insertContent(" ")
            .run();
        });
      });
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

  /// Upload a sequence of image files and insert each as an
  /// `image` node at `pos` (in document order). Used by both
  /// drop and paste paths so the failure modes stay consistent.
  /// We don't show an inline placeholder while the upload is in
  /// flight: typical image uploads run in well under a second on
  /// localhost, and a placeholder that races the cursor is more
  /// disruptive than a brief delay.
  async function insertImageFilesAt(pos: number, files: File[]): Promise<void> {
    if (!editor) return;
    let cursor = pos;
    for (const file of files) {
      try {
        const path = await uploadImageFile(file);
        const last = path.split("/").pop() ?? path;
        const alt = last.replace(/\.[^./]+$/, "");
        editor
          .chain()
          .focus()
          .insertContentAt(cursor, [
            { type: "image", attrs: { src: path, alt } },
            { type: "text", text: " " },
          ])
          .run();
        // Advance the cursor for subsequent inserts in the same
        // batch so we don't stack everything at the original pos.
        // +2 accounts for the atomic image node + the trailing
        // space we appended.
        cursor += 2;
      } catch (e) {
        // eslint-disable-next-line no-console
        console.error("image upload failed:", e);
      }
    }
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
    wikiBubble = openWikiBubble({
      host: caretAnchorHost(),
      prefix: wikiPickerPrefix,
      onClickAccept: () => acceptWikiBubble(),
    });
    wikiBubble.setQuery("");
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
    if (t.matches("[data-md-wiki]")) {
      e.preventDefault();
      const target = t.getAttribute("data-target") ?? "";
      if (target) handleWikiClick(target);
      return;
    }
    if (t.matches("[data-md-date]")) {
      e.preventDefault();
      // Preserve the originating format so click-to-edit doesn't
      // jump the pill to the user's default; if the user wants a
      // different format they pick it explicitly in the dropdown.
      const existingFormat = (t.getAttribute("data-date-format") ?? "iso") as DateFormatId;
      const existingIso = t.getAttribute("data-date") ?? "";
      showCalendar(
        t,
        (picked) => {
          if (!picked || !editor) return;
          // Find the date node by attribute scan. We use the DOM's
          // data-date as the key; ambiguity (two pills with the
          // same date AND same format) resolves to the first match,
          // which is good enough for a click-anchored edit.
          const dateType = editor.schema.nodes.date;
          let from = -1;
          editor.state.doc.descendants((n, p) => {
            if (from >= 0) return false;
            if (
              n.type === dateType &&
              n.attrs.date === existingIso &&
              n.attrs.format === existingFormat
            ) {
              from = p;
              return false;
            }
          });
          if (from < 0) return;
          const tr = editor.state.tr.replaceWith(
            from,
            from + 1,
            dateType.create({ date: picked.iso, format: picked.format }),
          );
          editor.view.dispatch(tr);
        },
        existingFormat,
      );
      return;
    }
    // Standard markdown links saved as <a href>. If the href looks
    // internal (no scheme, ends with .md or has no extension), treat it
    // like a wiki click. Hold Cmd/Ctrl to fall through to default
    // browser behavior (open externally). Relative hrefs (`./foo.md`,
    // `../docs/foo.md`) are resolved against the current file's
    // directory before opening, so the click works regardless of
    // whether the link round-tripped through `decorateWikiLinks`.
    const a = t.closest("a") as HTMLAnchorElement | null;
    if (a && !e.metaKey && !e.ctrlKey) {
      const href = a.getAttribute("href") ?? "";
      if (href && isInternalHref(href)) {
        e.preventDefault();
        let resolved = decodeURIComponent(href);
        if (currentPath) {
          resolved = resolveRelativePath(resolved, currentPath);
        }
        handleWikiClick(resolved);
      }
    }
  }

  function isInternalHref(href: string): boolean {
    // External if it has a scheme (`https:`, `mailto:`, etc.).
    return !/^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(href);
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
    padding: 1rem 1.25rem;
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
  :global(.md-wysiwyg .md-smart-date)  { color: var(--warn-text); }
  :global(.md-wysiwyg .md-smart-wiki)  { color: var(--link); text-decoration: underline; }
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
  :global(.md-pick-url) {
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 4px 6px;
    font: inherit;
    font-size: 14px;
    outline: none;
  }
  :global(.md-pick-url:focus) { border-color: var(--link); }

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
  /* Faded separator + accept hint. Hidden when there are no
     results to commit so an empty bubble doesn't claim Enter
     does something it cannot. */
  :global(.md-wiki-bubble-accept) {
    padding: .3rem .55rem;
    border-top: 1px dashed var(--border);
    color: var(--muted);
    font-size: 11px;
    text-align: right;
  }
  :global(.md-wiki-bubble-accept.is-hidden) { display: none; }

  /* Inline images: keep them from blowing the editor column out
     by capping max-width. The native size renders if it fits.
     The wrapper (`.md-image-wrap`) carries the resize handle in
     the bottom-right corner; visible on hover or while dragging
     to avoid clutter on a page full of images. */
  :global(.md-wysiwyg img) {
    max-width: 100%;
    height: auto;
    border-radius: 3px;
    vertical-align: middle;
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
  :global(.md-image-wrap.resizing .md-image-handle) {
    opacity: 1;
  }
  :global(.md-image-wrap.resizing img) {
    /* Disable image-drag during resize so the user doesn't
       accidentally drag the image instead of grabbing the handle. */
    pointer-events: none;
    user-select: none;
  }
</style>
