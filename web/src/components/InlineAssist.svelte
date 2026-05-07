<script lang="ts">
  // Global assistant overlay. Bound to Cmd+H (Ctrl+H on non-Mac),
  // matching VSCode / Cursor's inline-chat convention.
  //
  // v3 contract: one overlay, three contexts, picked via a
  // dropdown at the top of the panel:
  //
  //   (a) a single visible file (default when any file is on
  //       screen). The per-file thread persists to
  //       `.chan/assistant/<sha256(path)>.json` so the
  //       conversation survives across runs.
  //   (b) the group of all files visible across the layout (only
  //       available when 2+ files are on screen). The model
  //       sees every file as context. In-memory only; when the
  //       visible set changes the group conversation is dropped
  //       so we don't accumulate orphan threads.
  //   (c) Drive Q&A: hybrid-search-retrieval Q&A over the
  //       entire drive. In-memory only; replaces the "ask"
  //       tab that used to live in the search palette.
  //
  // File edits land through the existing `propose_file_edit`
  // tool; the model can target any path in scope (the single
  // file in file context, any of the visible files in group
  // context, any path it discovers in drive context).

  import { onDestroy, onMount } from "svelte";

  import { api } from "../api/client";
  import { renderMarkdown } from "../api/markdown";
  import type {
    ContentHit,
    LlmCompletionResponse,
    LlmMessage,
    LlmStatus,
    LlmToolSpec,
  } from "../api/types";
  import Wysiwyg, { type BlockKind } from "../editor/Wysiwyg.svelte";
  import Source from "../editor/Source.svelte";
  import {
    assistantConversations,
    assistantOverlay,
    availableAssistantContexts,
    clearFileConversation,
    clearGroupConversation,
    clearDriveConversation,
    openAssistant,
    pruneStaleAssistantGroups,
    refreshTree,
    drive,
    type AssistantConversation,
    type AssistantPendingEdit,
    type AssistantTurn,
  } from "../state/store.svelte";
  import { defaultScopeId, type ScopeOption } from "../state/scope.svelte";
  import { layout, openInActivePane } from "../state/tabs.svelte";
  import OverlayShell from "./OverlayShell.svelte";

  /// Dropdown options derived from the live layout. Updated
  /// reactively as panes split, tabs switch, files close.
  const contextOptions = $derived<ScopeOption[]>(availableAssistantContexts());
  /// Current context object (null when the saved contextId points
  /// at a file or group that's no longer visible). Reading this
  /// is how the rest of the component asks "what's the active
  /// scope right now?".
  const currentContext = $derived<ScopeOption | null>(
    contextOptions.find((o) => o.id === assistantOverlay.contextId) ?? null,
  );
  const visible = $derived(assistantOverlay.open);

  /// Snap to a sensible context when the overlay opens with an
  /// invalid contextId (saved file path closed, group key no
  /// longer in the option list). We deliberately do NOT snap
  /// while the overlay is closed: the layout flips through
  /// transient states the user shouldn't be reacting to from a
  /// hidden panel.
  $effect(() => {
    if (!visible) return;
    pruneStaleAssistantGroups();
    if (!currentContext) {
      assistantOverlay.contextId = defaultScopeId();
    }
  });

  /// The prompt has its own mode toggle (mirrors the file
  /// editor's wysiwyg/source switch). Local state because the
  /// overlay is per-window-session and not worth persisting.
  let promptMode = $state<"wysiwyg" | "source">("wysiwyg");

  /// Refs into the prompt editor so the formatting toolbar above
  /// it can call into Wysiwyg's mark/block-kind API. Source mode
  /// has no formatting toolbar (a textarea ignores them).
  let wysiwygRef: Wysiwyg | undefined = $state();

  /// Bumped on every selection / doc change inside the prompt
  /// Wysiwyg so the toolbar's active-mark / current-block
  /// derivations re-run. Mirrors the FileEditorTab pattern.
  let selVer = $state(0);

  // Reactive accessors. Reading `selVer` ties them to the
  // editor's selection updates so the toolbar buttons reflect
  // cursor moves; the void cast keeps lint quiet.
  const isBold = $derived.by(() => {
    void selVer;
    return wysiwygRef?.isActive("bold") ?? false;
  });
  const isItalic = $derived.by(() => {
    void selVer;
    return wysiwygRef?.isActive("italic") ?? false;
  });
  const isStrike = $derived.by(() => {
    void selVer;
    return wysiwygRef?.isActive("strike") ?? false;
  });
  const isInlineCode = $derived.by(() => {
    void selVer;
    return wysiwygRef?.isActive("code") ?? false;
  });
  const isBulletList = $derived.by(() => {
    void selVer;
    return wysiwygRef?.isActive("bulletList") ?? false;
  });
  const isOrderedList = $derived.by(() => {
    void selVer;
    return wysiwygRef?.isActive("orderedList") ?? false;
  });
  const blockKind = $derived.by<BlockKind>(() => {
    void selVer;
    return wysiwygRef?.currentBlockKind() ?? "normal";
  });

  function onBlockKindChange(e: Event): void {
    const v = (e.currentTarget as HTMLSelectElement).value as BlockKind;
    wysiwygRef?.setBlockKind(v);
  }

  // The prompt is a Wysiwyg instance now (markdown editor with
  // smart-node autocomplete). It handles its own focus on mount
  // and refocuses when its value is reset to "" after a submit,
  // so we don't keep an explicit ref here.
  let scrollEl: HTMLDivElement | undefined = $state();
  let prompt = $state("");
  let loading = $state(false);
  let error = $state<string | null>(null);
  let savedSelection = $state<string | null>(null);

  /// Animated "thinking" indicator: dots cycle 0..3 every 400ms
  /// while a request is in flight. Bare timeline animation, no
  /// new dependencies. Stops + resets when loading flips off so
  /// the next request starts cleanly.
  let thinkingDots = $state(0);
  $effect(() => {
    if (!loading) {
      thinkingDots = 0;
      return;
    }
    const id = setInterval(() => {
      thinkingDots = (thinkingDots + 1) % 4;
    }, 400);
    return () => clearInterval(id);
  });

  /// `now` ticks every 30 s while the panel is open so relative
  /// timestamps next to each turn (e.g. "3m ago") refresh without
  /// depending on any other reactive write. Reading `now` inside
  /// `formatRelative` is what makes those spans re-render.
  let now = $state(Date.now());
  $effect(() => {
    if (!visible) return;
    const id = setInterval(() => {
      now = Date.now();
    }, 30_000);
    return () => clearInterval(id);
  });

  function formatRelative(ts: number | undefined): string {
    if (!ts) return "";
    const diffSec = Math.max(0, Math.floor((now - ts) / 1000));
    if (diffSec < 60) return "just now";
    if (diffSec < 3600) return `${Math.floor(diffSec / 60)}m ago`;
    if (diffSec < 86400) return `${Math.floor(diffSec / 3600)}h ago`;
    if (diffSec < 7 * 86400) return `${Math.floor(diffSec / 86400)}d ago`;
    return new Date(ts).toISOString().slice(0, 10);
  }

  // Octicon-style 16x16 SVGs lifted from sdme's site/static/js/copy.js
  // for the per-turn copy button (with check confirmation), plus
  // the prompt-bar's send + stop affordances. Kept as inline
  // strings rendered via {@html} so we can swap them through a
  // plain {#if} without a child component.
  const ICON_COPY =
    '<svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor" aria-hidden="true">' +
    '<path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 0 1 0 1.5h-1.5a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-1.5a.75.75 0 0 1 1.5 0v1.5A1.75 1.75 0 0 1 9.25 16h-7.5A1.75 1.75 0 0 1 0 14.25Z"></path>' +
    '<path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0 1 14.25 11h-7.5A1.75 1.75 0 0 1 5 9.25Zm1.75-.25a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-7.5a.25.25 0 0 0-.25-.25Z"></path>' +
    "</svg>";
  const ICON_CHECK =
    '<svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor" aria-hidden="true">' +
    '<path d="M13.78 4.22a.75.75 0 0 1 0 1.06l-7.25 7.25a.75.75 0 0 1-1.06 0L2.22 9.28a.75.75 0 0 1 1.06-1.06L6 10.94l6.72-6.72a.75.75 0 0 1 1.06 0Z"></path>' +
    "</svg>";
  /// Octicon `paper-airplane`: send icon for the in-overlay
  /// primary action. 14px so it sits a touch larger than the
  /// 12px copy/check icons in the chat bubbles.
  const ICON_SEND =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="currentColor" aria-hidden="true">' +
    '<path d="M.989 8 .064 2.68a1.342 1.342 0 0 1 1.85-1.462l13.402 5.744a1.13 1.13 0 0 1 0 2.076L1.913 14.782a1.343 1.343 0 0 1-1.85-1.463L.99 8Zm.603-5.288L2.38 7.25h4.87a.75.75 0 0 1 0 1.5H2.38l-.788 4.538L13.929 8Z"></path>' +
    "</svg>";
  /// Octicon `square-fill`: stop icon, universally read as "halt".
  const ICON_STOP =
    '<svg viewBox="0 0 16 16" width="14" height="14" fill="currentColor" aria-hidden="true">' +
    '<path d="M0 1.75C0 .784.784 0 1.75 0h12.5C15.216 0 16 .784 16 1.75v12.5A1.75 1.75 0 0 1 14.25 16H1.75A1.75 1.75 0 0 1 0 14.25Z"></path>' +
    "</svg>";

  /// Index of the most recently copied turn; the matching button
  /// briefly says "copied" so the user gets feedback. Reset to
  /// null after 1.2 s. Storing one index (not a set) is enough
  /// because rapid copies just shift the indicator forward.
  let copiedTurn = $state<number | null>(null);
  let copiedTimer: ReturnType<typeof setTimeout> | null = null;
  async function copyTurn(index: number, text: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(text);
      copiedTurn = index;
      if (copiedTimer) clearTimeout(copiedTimer);
      copiedTimer = setTimeout(() => {
        copiedTurn = null;
        copiedTimer = null;
      }, 1200);
    } catch {
      // Clipboard write can fail in non-secure contexts (rare in
      // chan since we're on localhost); swallow the error and
      // leave the button label unchanged.
    }
  }

  function captureSelection(): string | null {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0) return null;
    const text = sel.toString();
    return text.trim().length === 0 ? null : text;
  }

  /// System prompts per context kind. Each gets a focused
  /// version of the contract so the model knows what it's looking
  /// at before the first user turn lands.
  ///
  /// We don't swap the system prompt retroactively when the
  /// backend's tool capability changes: the seeded message only
  /// governs initial behavior, and subsequent tool availability
  /// is enforced at the request level by sending `tools = []`.

  const PROMPT_FILE_TOOLS =
    "You are the user's writing assistant inside chan, a personal-notes editor. " +
    "This conversation is scoped to ONE file in the user's drive; the file's CURRENT content ships on every user turn under '# File', and the user's prompt under '# Instruction'. " +
    "If the user has text selected, it appears under '# Selection'.\n\n" +
    "TOOLS\n" +
    "  - read_file(path): read another markdown file.\n" +
    "  - list_files(prefix?): list files (optional path prefix).\n" +
    "  - search_content(query, limit?): hybrid keyword + semantic search.\n" +
    "  - propose_file_edit(path, content, summary): propose a complete file replacement; the user reviews and clicks Apply or Discard.\n" +
    "  - write_file(path, content): direct atomic write, only succeeds when 'auto-apply writes' is on in Settings. Prefer propose_file_edit.\n\n" +
    "DISCIPLINE\n" +
    "  - One category of tool call per turn (investigate OR propose). Don't mix.\n" +
    "  - Preserve frontmatter + unrelated sections in proposed edits.\n" +
    "  - Reply concisely; discuss before acting when intent is ambiguous.";

  const PROMPT_FILE_CHAT =
    "You are the user's writing assistant inside chan. This conversation is scoped to ONE file; the current content ships under '# File' on every user turn, and the prompt under '# Instruction'. Selected text (when any) appears under '# Selection'. Reply in plain markdown; the current model can't call tools.";

  const PROMPT_GROUP_TOOLS =
    "You are the user's writing assistant inside chan. This conversation is scoped to a GROUP of files visible in the user's layout; each file's CURRENT content ships on every user turn under its own '## <path>' heading inside the '# Files' block, with the user's prompt under '# Instruction'.\n\n" +
    "TOOLS\n" +
    "  - read_file(path): read any other markdown file.\n" +
    "  - list_files(prefix?), search_content(query, limit?): explore the wider drive.\n" +
    "  - propose_file_edit(path, content, summary): propose a complete replacement for any path; the user reviews per-edit.\n" +
    "  - write_file(path, content): gated by 'auto-apply writes'; prefer propose_file_edit.\n\n" +
    "DISCIPLINE\n" +
    "  - When proposing edits, target ONE specific file at a time via its path.\n" +
    "  - One category of tool call per turn (investigate OR propose). Don't mix.\n" +
    "  - Preserve frontmatter + unrelated sections in each proposed edit.";

  const PROMPT_GROUP_CHAT =
    "You are the user's writing assistant inside chan. This conversation is scoped to a GROUP of files; each ships under its own '## <path>' heading inside the '# Files' block, with the prompt under '# Instruction'. Reply in plain markdown; the current model can't call tools.";

  const PROMPT_UNIVERSE_TOOLS =
    "You are answering questions about the user's personal-notes drive in chan. " +
    "Each user turn ships excerpts retrieved by hybrid search (BM25 + semantic) under '# Excerpts' and the user's question under '# Instruction'. " +
    "Use the excerpts as primary context; cite sources by their bracket number, like [1] or [3]. " +
    "Keep responses concise (3-6 sentences) unless more detail is clearly needed. " +
    "When the excerpts don't answer the question, say so plainly.\n\n" +
    "TOOLS\n" +
    "  - read_file(path) / list_files(prefix?) / search_content(query, limit?): refine your retrieval if the initial excerpts are insufficient.\n" +
    "  - propose_file_edit(path, content, summary): only when the user explicitly asks for an edit; the user reviews the proposal.\n" +
    "  - write_file(path, content): gated by 'auto-apply writes'; prefer propose_file_edit.";

  const PROMPT_UNIVERSE_CHAT =
    "You are answering questions about the user's personal-notes drive in chan. Each turn ships hybrid-search excerpts under '# Excerpts' and the user's question under '# Instruction'. Use ONLY the excerpts as context; cite sources by their bracket number. If the excerpts don't answer, say so plainly. Keep responses concise (3-6 sentences). The current model can't call tools.";

  function systemPromptFor(
    kind: ScopeOption["kind"],
    tools: boolean,
  ): string {
    if (kind === "file") return tools ? PROMPT_FILE_TOOLS : PROMPT_FILE_CHAT;
    if (kind === "group") return tools ? PROMPT_GROUP_TOOLS : PROMPT_GROUP_CHAT;
    return tools ? PROMPT_UNIVERSE_TOOLS : PROMPT_UNIVERSE_CHAT;
  }

  /// Resolve (lazy-initializing) the conversation for `ctx`.
  /// Always re-reads through the proxied store so callers mutate
  /// the deep proxy rather than the unproxied seed object.
  function conversationFor(ctx: ScopeOption): AssistantConversation {
    const seed: AssistantConversation = {
      messages: [
        { role: "system", content: systemPromptFor(ctx.kind, supportsTools()) },
      ],
      turns: [],
    };
    if (ctx.kind === "file") {
      if (!assistantConversations.byFile[ctx.path]) {
        assistantConversations.byFile[ctx.path] = seed;
      }
      return assistantConversations.byFile[ctx.path]!;
    }
    if (ctx.kind === "group") {
      if (!assistantConversations.byGroup[ctx.key]) {
        assistantConversations.byGroup[ctx.key] = seed;
      }
      return assistantConversations.byGroup[ctx.key]!;
    }
    if (!assistantConversations.drive) {
      assistantConversations.drive = seed;
    }
    return assistantConversations.drive;
  }

  /// Whether the configured backend can use tools right now.
  /// `null` (status not loaded yet) is treated as "no tools" so a
  /// race doesn't accidentally send a tool list to a non-tool
  /// model on first paint.
  function supportsTools(): boolean {
    return llmStatus?.supports_tools === true;
  }

  /// Tools shipped to the model on every llmComplete call. Lazy-
  /// loaded once from the server and cached for the rest of the
  /// session; the propose-edit tool is added on top because that
  /// one is client-handled (the server doesn't execute it).
  let serverTools: LlmToolSpec[] = $state([]);
  /// Backend status, refreshed on every overlay open so a user
  /// who flips backends in Settings sees the new capability the
  /// next time Cmd+H fires (no need to restart the app).
  let llmStatus = $state<LlmStatus | null>(null);

  async function ensureToolsLoaded(): Promise<void> {
    if (serverTools.length > 0) return;
    try {
      serverTools = await api.llmTools();
    } catch {
      // Tool catalog is optional; without it the model can still
      // chat, just can't call read/list/search. Surface as a
      // status hint rather than blocking the UI.
      serverTools = [];
    }
  }

  async function refreshLlmStatus(): Promise<void> {
    try {
      llmStatus = await api.llmStatus();
    } catch {
      llmStatus = null;
    }
  }

  /// Track the path of the file tab the overlay was last opened
  /// for. When the user switches tabs (and the overlay implicitly
  /// hides), we don't want to re-fire the open hooks if they
  /// switch back to the same file. Re-fire only when the visible
  /// path changes (open on a different file, or first-time open).
  /// Track the most-recently-entered context so we only fire the
  /// open hooks (selection capture, status refresh, conversation
  /// load, scroll-to-bottom) when the visible context truly
  /// changes. Without this, every re-render of the dropdown's
  /// derived options would refire the hooks.
  let lastOpenedContextId = $state<string | null>(null);
  $effect(() => {
    if (!visible || !currentContext) {
      // Reset transient view state on hide so re-opening on a
      // different context doesn't carry the old prompt forward.
      if (lastOpenedContextId !== null) {
        lastOpenedContextId = null;
        prompt = "";
        error = null;
      }
      return;
    }
    const id = currentContext.id;
    if (lastOpenedContextId === id) return;
    lastOpenedContextId = id;
    savedSelection = captureSelection();
    error = null;
    prompt = "";
    void ensureToolsLoaded();
    void refreshLlmStatus();
    if (currentContext.kind === "file") {
      // Pull the persisted thread off disk; in-memory hits skip.
      void loadFileConversation(currentContext.path);
    }
    queueMicrotask(scrollToBottom);
  });

  /// Read the persisted file conversation back from
  /// `.chan/assistant/<sha256>.json`. Only called for file
  /// context; group + drive live in memory only.
  async function loadFileConversation(path: string): Promise<void> {
    if (assistantConversations.byFile[path]) return; // already in memory
    try {
      const remote = await api.getConversation(path);
      if (!remote) return;
      const parsed = remote as {
        messages?: LlmMessage[];
        turns?: AssistantTurn[];
      };
      // Migration is forgiving: missing fields fall back to a
      // fresh seed so an old / partial file doesn't break the UI.
      assistantConversations.byFile[path] = {
        messages:
          parsed.messages && parsed.messages.length > 0
            ? parsed.messages
            : [{ role: "system", content: systemPromptFor("file", supportsTools()) }],
        turns: parsed.turns ?? [],
      };
      queueMicrotask(scrollToBottom);
    } catch {
      // Server unreachable / invalid JSON: leave the bucket empty
      // so the next submit creates a fresh conversation.
    }
  }

  /// Debounced persistence for file contexts. Group and drive
  /// are intentionally not persisted: their lifecycle is bound to
  /// the current layout / app session, so writing to disk would
  /// just leak orphan blobs into `.chan/assistant/`.
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  function scheduleSave(ctx: ScopeOption): void {
    if (ctx.kind !== "file") return;
    const path = ctx.path;
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      saveTimer = null;
      const conv = assistantConversations.byFile[path];
      if (!conv) return;
      void api.putConversation(path, {
        schema_version: 1,
        path,
        messages: conv.messages,
        turns: conv.turns,
      });
    }, 400);
  }

  function scrollToBottom(): void {
    if (scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight;
  }

  /// Auto-scroll the chat to the bottom whenever the loading
  /// flag flips on (the in-flight placeholder bubble just
  /// appeared) so the user doesn't have to scroll manually after
  /// hitting Send. Reading `loading` here ties the effect to it.
  $effect(() => {
    if (loading) queueMicrotask(scrollToBottom);
  });

  function close(): void {
    // If a proposal is dangling unanswered, treat the close as a
    // dismiss so the next round has a valid tool_result.
    if (currentContext) {
      const conv = conversationFor(currentContext);
      const last = conv.turns[conv.turns.length - 1];
      if (last && last.kind === "edit" && last.edit.status === "pending") {
        dismissEdit(last.edit, "user closed the dialog");
      }
    }
    assistantOverlay.open = false;
  }

  function clearCurrent(): void {
    const ctx = currentContext;
    if (!ctx) return;
    if (ctx.kind === "file") {
      clearFileConversation(ctx.path);
      // Idempotent server-side; safe even when never persisted.
      void api.deleteConversation(ctx.path);
    } else if (ctx.kind === "group") {
      clearGroupConversation(ctx.key);
    } else {
      clearDriveConversation();
    }
    error = null;
    prompt = "";
  }

  /// Build the per-turn user message. Each context kind gets its
  /// own framing so the model knows whether it's looking at one
  /// file, several, or retrieved excerpts.
  ///
  /// File / group ship the latest buffer content on every turn so
  /// the model sees current state (a manual edit or an applied
  /// previous proposal could have moved the file since the last
  /// round). Drive re-runs retrieval per turn.
  function buildUserMessage(
    ctx: ScopeOption,
    userPrompt: string,
    selection: string | null,
    excerpts: ContentHit[] | null,
  ): string {
    if (ctx.kind === "file") {
      const content = currentFileContent(ctx.path);
      const selBlock = selection ? `\n\n# Selection\n\n${selection}` : "";
      return (
        `# File\n\nPath: ${ctx.path}\n\n${content}` +
        selBlock +
        `\n\n# Instruction\n\n${userPrompt}`
      );
    }
    if (ctx.kind === "group") {
      const sections = ctx.paths
        .map((p) => `## ${p}\n\n${currentFileContent(p)}`)
        .join("\n\n");
      return `# Files\n\n${sections}\n\n# Instruction\n\n${userPrompt}`;
    }
    const block =
      excerpts && excerpts.length > 0
        ? excerpts
            .map(
              (h, i) =>
                `[${i + 1}] ${h.path}` +
                (h.heading ? ` # ${h.heading}` : "") +
                `\n${stripHighlight(h.snippet)}`,
            )
            .join("\n\n")
        : "(no relevant notes found)";
    return `# Excerpts\n\n${block}\n\n# Instruction\n\n${userPrompt}`;
  }

  /// Resolve the latest in-memory buffer for `path` from the
  /// layout. Falls back to empty when the path isn't open in any
  /// pane (the model still gets a meaningful prompt; an empty
  /// "# File" block reads as "no content yet").
  function currentFileContent(path: string): string {
    for (const node of Object.values(layout.nodes)) {
      if (node.kind !== "leaf") continue;
      for (const t of node.tabs) {
        if (t.kind === "file" && t.path === path) return t.content;
      }
    }
    return "";
  }

  /// BM25 highlights are useful in the search UI but noise inside
  /// an LLM prompt; strip the marker tags before shipping.
  function stripHighlight(s: string): string {
    return s.replace(/<\/?b>/g, "");
  }

  const PROPOSE_TOOL: LlmToolSpec = {
    name: "propose_file_edit",
    description:
      "Propose a complete replacement for a markdown file. Use when the user asks for an edit and you want to make a concrete change. The user reviews and chooses to apply or discard. Output the FULL revised file content (no fences, no commentary).",
    input_schema: {
      type: "object",
      required: ["path", "content"],
      properties: {
        path: { type: "string" },
        content: { type: "string" },
        summary: { type: "string" },
      },
    },
  };

  /// Timestamp captured when the user submits a prompt; drives
  /// the relative timestamp on the in-flight ASSISTANT placeholder
  /// bubble shown in the chat scrollback. Cleared when the
  /// request completes (the real assistant turn replaces the
  /// placeholder via the turns array).
  let pendingTurnTime = $state<number | null>(null);

  /// AbortController for the in-flight llmComplete request, if
  /// any. Cleared in the `finally` block so a stale controller
  /// doesn't haunt the next request. The Stop button calls
  /// `cancel()` which aborts the fetch; the caught AbortError
  /// flows through the existing error path with a friendlier
  /// message.
  let inflight: AbortController | null = null;

  function cancel(): void {
    inflight?.abort();
  }

  async function submit(): Promise<void> {
    const ctx = currentContext;
    if (!ctx || loading) return;
    const trimmed = prompt.trim();
    if (!trimmed) return;
    // Slash commands: handled locally, no LLM round-trip.
    if (trimmed === "/clear") {
      clearCurrent();
      return;
    }
    loading = true;
    error = null;
    pendingTurnTime = Date.now();
    const conv = conversationFor(ctx);
    // For drive context we retrieve excerpts before composing
    // the user message; for file/group the context IS the file
    // contents, no retrieval needed.
    let excerpts: ContentHit[] | null = null;
    const ctl = new AbortController();
    inflight = ctl;
    try {
      if (ctx.kind === "drive") {
        const r = await api.searchContent(trimmed, { limit: 8 });
        excerpts = r.hits;
      }
    } catch (e) {
      // Retrieval failure isn't fatal: ship the prompt without
      // excerpts so the model still answers what it can.
      tracingWarn(`retrieval failed: ${(e as Error).message}`);
    }
    const userBody = buildUserMessage(ctx, trimmed, savedSelection, excerpts);
    conv.messages.push({ role: "user", content: userBody });
    conv.turns.push({ kind: "user", content: trimmed, created_at: Date.now() });
    prompt = "";
    scheduleSave(ctx);
    queueMicrotask(scrollToBottom);
    try {
      // Tool list is gated on the backend's current capability.
      // Sending a `tools` array to a non-tool-capable model causes
      // Ollama to refuse the request outright; omit the field
      // entirely so the request is plain chat. propose_file_edit
      // is client-handled but still ships as a tool spec, so it's
      // gated alongside the rest.
      const tools = supportsTools()
        ? [PROPOSE_TOOL, ...serverTools]
        : undefined;
      const resp = await api.llmComplete(
        {
          messages: conv.messages,
          tools,
          max_tokens: 4000,
          // Temperature intentionally omitted: every backend has
          // a sensible default, and reasoning / extended-thinking
          // models reject any explicit value. Letting the model
          // pick avoids both a hard-coded preference and a HTTP-
          // 400 retry dance for the few models that do not
          // accept the parameter.
        },
        ctl.signal,
      );
      handleResponse(ctx, conv, resp, excerpts);
      scheduleSave(ctx);
    } catch (e) {
      // AbortError surfaces as DOMException("...","AbortError") on
      // most runtimes; treat it as a soft cancellation rather than
      // an error so the chat doesn't show a scary red message.
      if ((e as Error).name === "AbortError") {
        error = "stopped";
      } else {
        error = (e as Error).message;
      }
      // Roll back the optimistic user message? No: keep it visible
      // so the user can retry without retyping. Their prompt stays
      // in `conv.messages`; the next submit just appends another.
    } finally {
      loading = false;
      inflight = null;
      pendingTurnTime = null;
      queueMicrotask(scrollToBottom);
    }
  }

  function tracingWarn(message: string): void {
    // eslint-disable-next-line no-console
    console.warn(`[chan/assistant] ${message}`);
  }

  /// Open the citation's source file in the active pane and
  /// dismiss the overlay. Mirrors the search-palette flow that
  /// used to live in SearchPanel before drive Q&A moved here.
  function openCitation(c: ContentHit): void {
    void openInActivePane(c.path);
    assistantOverlay.open = false;
  }

  /// Default path for proposed edits when the model omits the
  /// `path` attr: file context uses its single path, group falls
  /// back to the first visible path, drive leaves it empty
  /// (the model is expected to specify in drive context).
  function defaultEditPath(ctx: ScopeOption): string {
    if (ctx.kind === "file") return ctx.path;
    if (ctx.kind === "group") return ctx.paths[0] ?? "";
    return "";
  }

  function handleResponse(
    ctx: ScopeOption,
    conv: AssistantConversation,
    resp: LlmCompletionResponse,
    excerpts: ContentHit[] | null,
  ): void {
    conv.messages.push({
      role: "assistant",
      content: resp.content,
      tool_calls: resp.tool_calls,
    });
    if (resp.content.trim()) {
      conv.turns.push({
        kind: "assistant",
        content: resp.content,
        created_at: Date.now(),
        ...(excerpts && excerpts.length > 0 ? { citations: excerpts } : {}),
      });
    }
    for (const call of resp.tool_calls) {
      if (call.name !== "propose_file_edit") continue;
      const input = (call.input ?? {}) as {
        path?: string;
        content?: string;
        summary?: string;
      };
      const edit: AssistantPendingEdit = {
        toolCallId: call.id,
        path: input.path ?? defaultEditPath(ctx),
        content: input.content ?? "",
        summary: input.summary ?? null,
        status: "pending",
      };
      conv.turns.push({ kind: "edit", edit, created_at: Date.now() });
    }
  }

  /// Apply a proposed edit: locate the target file in the current
  /// layout, push the new content into its in-memory buffer (the
  /// regular autosave loop carries it to disk), and append a
  /// tool_result so the next round sees the user's decision.
  /// Edits without a corresponding open tab still record the
  /// decision; the user is expected to open the file separately
  /// to inspect, but the conversation stays consistent.
  /// Persist a proposed edit. Writes the file to disk via the API
  /// AND updates any open tabs at the same path. The on-disk write
  /// is the load-bearing step: previously this only mutated open
  /// tabs and counted on autosave to flush, which silently no-op'd
  /// for proposals targeting a path with no tab open (the user saw
  /// "applied" in green but the file never reached disk).
  async function applyEdit(edit: AssistantPendingEdit): Promise<void> {
    if (edit.status !== "pending") return;
    let openTabUpdated = false;
    for (const node of Object.values(layout.nodes)) {
      if (node.kind !== "leaf") continue;
      for (const t of node.tabs) {
        if (t.kind === "file" && t.path === edit.path) {
          t.content = edit.content;
          // Mark clean so the autosave loop doesn't re-write what
          // we're about to flush explicitly below.
          t.saved = edit.content;
          openTabUpdated = true;
        }
      }
    }
    try {
      await api.write(edit.path, edit.content);
    } catch (e) {
      // Surface the failure in the chat error line and keep the
      // proposal as pending so the user can retry; flipping it to
      // "applied" with no file on disk is exactly the bug we are
      // fixing.
      error = `apply failed: ${(e as Error).message}`;
      return;
    }
    edit.status = "applied";
    // Refresh the file tree so a brand-new path shows up in the
    // browser without waiting for the watcher's debounce. Skipped
    // when an open tab matched, since the watcher event for the
    // existing path already fans out a refresh.
    if (!openTabUpdated) {
      void refreshTree();
    }
    appendToolResult(
      edit.toolCallId,
      openTabUpdated
        ? "user applied the proposed edit (open buffer + disk updated)"
        : "user applied the proposed edit (file written to disk)",
    );
    if (currentContext) scheduleSave(currentContext);
    queueMicrotask(scrollToBottom);
  }

  /// Track which proposal was just copied so the matching button
  /// briefly switches to a check icon. Keyed by toolCallId because
  /// turn indexes shift as the conversation grows.
  let copiedEditId = $state<string | null>(null);
  let copiedEditTimer: ReturnType<typeof setTimeout> | null = null;
  async function copyEdit(edit: AssistantPendingEdit): Promise<void> {
    try {
      await navigator.clipboard.writeText(edit.content);
      copiedEditId = edit.toolCallId;
      if (copiedEditTimer) clearTimeout(copiedEditTimer);
      copiedEditTimer = setTimeout(() => {
        copiedEditId = null;
        copiedEditTimer = null;
      }, 1200);
    } catch {
      // Same fallback as copyTurn: clipboard write can fail in
      // restricted contexts; silently leave the icon as-is.
    }
  }

  function dismissEdit(edit: AssistantPendingEdit, reason: string): void {
    if (edit.status !== "pending") return;
    edit.status = "dismissed";
    appendToolResult(edit.toolCallId, `user dismissed: ${reason}`);
    if (currentContext) scheduleSave(currentContext);
    queueMicrotask(scrollToBottom);
  }

  function appendToolResult(toolCallId: string, message: string): void {
    if (!currentContext) return;
    const conv = conversationFor(currentContext);
    conv.messages.push({
      role: "tool",
      content: message,
      tool_call_id: toolCallId,
    });
  }

  function onWindowKey(e: KeyboardEvent): void {
    if (
      (e.metaKey || e.ctrlKey) &&
      !e.shiftKey &&
      !e.altKey &&
      e.key.toLowerCase() === "h"
    ) {
      // Cmd/Ctrl+H opens / toggles the assistant overlay. Picked
      // for the "Help" mnemonic and to leave Cmd+I for the
      // editor's native italic. Note: on macOS Cmd+H is also the
      // system "Hide app" shortcut; the Tauri WKWebView's keydown
      // handler runs before the OS forwards the chord, so this
      // intercept fires reliably inside chan, while desktop apps
      // outside chan keep the OS Hide.
      // Master switch. When the assistant is disabled in Settings
      // we let the keypress fall through (browsers don't bind
      // Cmd+H globally; nothing to clobber). The user gets no
      // visible response, matching the hidden toolbar button.
      const enabled = drive.info?.preferences.assistant.enabled ?? true;
      if (!enabled) return;
      e.preventDefault();
      if (visible) {
        // Toggle off: same as clicking the toolbar button while
        // the overlay is already open.
        assistantOverlay.open = false;
      } else {
        // Snap to a sensible context whenever we open via the
        // shared helper so the toolbar button + Cmd+H behave
        // identically.
        openAssistant();
      }
    } else if (visible && (e.metaKey || e.ctrlKey) && e.key === "Enter") {
      // Cmd/Ctrl+Enter sends from anywhere in the overlay
      // (prompt editor, source view, even chat scrollback). The
      // window-level handler covers both prompt modes; the
      // Wysiwyg's own onSubmit prop used to handle this but
      // wouldn't fire in source mode.
      e.preventDefault();
      if (!loading && currentContext) void submit();
    } else if (e.key === "Escape" && visible) {
      // Wysiwyg / ProseMirror doesn't intercept Escape, so a
      // window-level handler reaches it cleanly even with the
      // editor focused. If a request is in flight, Escape cancels
      // it instead of closing the panel; the user expects the
      // visible Stop button to also be reachable from the keyboard.
      e.preventDefault();
      if (loading) {
        cancel();
      } else {
        close();
      }
    }
  }
  onMount(() => document.addEventListener("keydown", onWindowKey));
  onDestroy(() => document.removeEventListener("keydown", onWindowKey));

  // Reactive accessor for the currently-rendered scrollback.
  // Reading through the proxied map / object tracks both the
  // entry's existence and the array's content, so Svelte
  // re-renders the bubbles as the conversation grows.
  const turns = $derived<AssistantTurn[]>(
    !currentContext
      ? []
      : currentContext.kind === "file"
        ? (assistantConversations.byFile[currentContext.path]?.turns ?? [])
        : currentContext.kind === "group"
          ? (assistantConversations.byGroup[currentContext.key]?.turns ?? [])
          : (assistantConversations.drive?.turns ?? []),
  );
</script>

<OverlayShell open={visible} onClose={close}>
      <header>
        <span class="title">assistant</span>
        <select
          class="context-select"
          value={assistantOverlay.contextId}
          onchange={(e) => (assistantOverlay.contextId = (e.currentTarget as HTMLSelectElement).value)}
          title="conversation context"
        >
          {#each contextOptions as opt (opt.id)}
            <option value={opt.id} disabled={opt.enabled === false}>
              {opt.label}
            </option>
          {/each}
        </select>
        {#if currentContext?.kind === "file" && savedSelection}
          <span class="sel-badge" title={savedSelection}>
            selection: {savedSelection.length} chars
          </span>
        {/if}
      </header>

      <div class="scroll" bind:this={scrollEl}>
        {#if llmStatus && !llmStatus.supports_tools}
          <!-- One-shot hint when the configured model can't call
               tools. Non-dismissible because it's load-bearing
               context, not noise: the model genuinely cannot do
               anything beyond chat in this state, and the user's
               first prompt would otherwise expect more. -->
          <div class="hint">
            <div class="hint-title">chat-only model</div>
            <div class="hint-body">
              The current model ({llmStatus.model ?? llmStatus.backend}) cannot
              call tools, so the assistant cannot read other files, search the
              drive, or propose file edits. Discuss freely; copy any
              suggestions you want to apply manually. Pick a tool-capable model
              in Settings to enable those.
            </div>
          </div>
        {/if}
        {#if turns.length === 0}
          <div class="empty">
            <div class="empty-title">No conversation yet</div>
          </div>
        {/if}
        {#each turns as turn, i (i)}
          {#if turn.kind === "user"}
            <div class="bubble user">
              <div class="role-line">
                <!-- Copy comes first in source order so the user-
                     side flex-row-reverse on the role-line floats
                     it leftmost (visually opposite the role label). -->
                <button
                  class="copy-btn"
                  title="copy this prompt"
                  aria-label="copy this prompt"
                  onclick={() => void copyTurn(i, turn.content)}
                >
                  {#if copiedTurn === i}
                    {@html ICON_CHECK}
                  {:else}
                    {@html ICON_COPY}
                  {/if}
                </button>
                <span class="role">you</span>
                <span class="ts">{formatRelative(turn.created_at)}</span>
              </div>
              <div class="body">{turn.content}</div>
            </div>
          {:else if turn.kind === "assistant"}
            <div class="bubble assistant">
              <div class="role-line">
                <span class="role">assistant</span>
                <span class="ts">{formatRelative(turn.created_at)}</span>
                <!-- Copy hands the user the raw markdown the model
                     emitted (not the sanitized HTML), so pasting
                     into another markdown buffer keeps formatting. -->
                <button
                  class="copy-btn"
                  title="copy this reply (markdown)"
                  aria-label="copy this reply"
                  onclick={() => void copyTurn(i, turn.content)}
                >
                  {#if copiedTurn === i}
                    {@html ICON_CHECK}
                  {:else}
                    {@html ICON_COPY}
                  {/if}
                </button>
              </div>
              <!-- Assistant output is markdown; render it (sanitized)
                   so headers / lists / code blocks / inline code /
                   links read like the rest of the editor. User
                   bubbles stay plain so the user's own typing
                   doesn't get reinterpreted. -->
              <div class="body md">{@html renderMarkdown(turn.content)}</div>
              {#if turn.citations && turn.citations.length > 0}
                <h4 class="cites-title">Sources</h4>
                <ul class="cites">
                  {#each turn.citations as c, j (c.path + c.chunk_id)}
                    <!-- svelte-ignore a11y_click_events_have_key_events -->
                    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
                    <li onmousedown={(e) => { e.preventDefault(); openCitation(c); }}>
                      <div class="row1">
                        <span class="cite-num">[{j + 1}]</span>
                        <span class="path">{c.path}</span>
                        {#if c.heading}<span class="heading">· {c.heading}</span>{/if}
                      </div>
                      <div class="snippet">{stripHighlight(c.snippet)}</div>
                    </li>
                  {/each}
                </ul>
              {/if}
            </div>
          {:else}
            <div class="edit-card" class:applied={turn.edit.status === "applied"} class:dismissed={turn.edit.status === "dismissed"}>
              <div class="edit-head">
                <span class="kind-chip">propose edit</span>
                <span class="path mono">{turn.edit.path}</span>
                <span class="size">{turn.edit.content.length} chars</span>
              </div>
              {#if turn.edit.summary}
                <div class="summary">{turn.edit.summary}</div>
              {/if}
              <details>
                <summary>show full proposal</summary>
                <pre class="proposal">{turn.edit.content}</pre>
              </details>
              {#if turn.edit.status === "pending"}
                <div class="actions">
                  <button class="primary" onclick={() => void applyEdit(turn.edit)}>Apply</button>
                  <button
                    class="copy"
                    title="copy proposal to clipboard"
                    aria-label="copy proposal"
                    onclick={() => void copyEdit(turn.edit)}
                  >
                    {#if copiedEditId === turn.edit.toolCallId}
                      {@html ICON_CHECK}<span>Copied</span>
                    {:else}
                      {@html ICON_COPY}<span>Copy</span>
                    {/if}
                  </button>
                  <button onclick={() => dismissEdit(turn.edit, "manual")}>Discard</button>
                </div>
              {:else if turn.edit.status === "applied"}
                <div class="status-tag ok">applied</div>
              {:else}
                <div class="status-tag muted">dismissed</div>
              {/if}
            </div>
          {/if}
        {/each}
        {#if loading && pendingTurnTime}
          <!-- In-flight assistant turn placeholder. Lives outside
               the turns array so it disappears automatically when
               the real reply lands (which pushes a real assistant
               turn into `turns`). The animated dots make it
               obvious that work is happening. -->
          <div class="bubble assistant pending">
            <div class="role-line">
              <span class="role">assistant</span>
              <span class="ts">{formatRelative(pendingTurnTime)}</span>
            </div>
            <div class="body">thinking{".".repeat(thinkingDots)}</div>
          </div>
        {/if}
      </div>

      <!-- Formatting toolbar above the prompt input. Mirrors the
           file editor's tab-bar (heading dropdown + B/I/S/Code on
           the left, mode toggle on the right) so the prompt feels
           like a first-class markdown editor and not a special
           "chat input". Source mode renders a textarea instead of
           the WYSIWYG editor; the formatting buttons hide in
           source mode because they can't act on a textarea. -->
      <div class="prompt-bar">
        <span class="left fmt">
          {#if promptMode === "wysiwyg"}
            <select
              class="block-kind"
              value={blockKind}
              onchange={onBlockKindChange}
              title="block style"
            >
              <option value="h1">h1</option>
              <option value="h2">h2</option>
              <option value="h3">h3</option>
              <option value="normal">text</option>
              <option value="code">code</option>
              <option value="quote">quote</option>
            </select>
            <button
              class="hbtn fmtbtn"
              class:on={isBold}
              title="bold (Cmd/Ctrl+B)"
              onmousedown={(e) => e.preventDefault()}
              onclick={() => wysiwygRef?.toggleBold()}
            ><b>B</b></button>
            <button
              class="hbtn fmtbtn"
              class:on={isItalic}
              title="italic (Cmd/Ctrl+I)"
              onmousedown={(e) => e.preventDefault()}
              onclick={() => wysiwygRef?.toggleItalic()}
            ><i>I</i></button>
            <button
              class="hbtn fmtbtn"
              class:on={isStrike}
              title="strikethrough"
              onmousedown={(e) => e.preventDefault()}
              onclick={() => wysiwygRef?.toggleStrike()}
            ><s>S</s></button>
            <button
              class="hbtn fmtbtn"
              class:on={isInlineCode}
              title="inline code"
              onmousedown={(e) => e.preventDefault()}
              onclick={() => wysiwygRef?.toggleInlineCode()}
            ><code>{`<>`}</code></button>
            <button
              class="hbtn fmtbtn"
              class:on={isBulletList}
              title="bullet list"
              aria-label="bullet list"
              onmousedown={(e) => e.preventDefault()}
              onclick={() => wysiwygRef?.toggleBulletList()}
            >•</button>
            <button
              class="hbtn fmtbtn"
              class:on={isOrderedList}
              title="ordered list"
              aria-label="ordered list"
              onmousedown={(e) => e.preventDefault()}
              onclick={() => wysiwygRef?.toggleOrderedList()}
            >1.</button>
          {/if}
        </span>
        <span class="actions">
          <button
            class="hbtn"
            title={promptMode === "wysiwyg" ? "view source" : "view rendered"}
            onclick={() => (promptMode = promptMode === "wysiwyg" ? "source" : "wysiwyg")}
          >{promptMode === "wysiwyg" ? "</>" : "¶"}</button>
        </span>
      </div>

      <div class="prompt-wrap" class:disabled={!currentContext}>
        {#if promptMode === "wysiwyg"}
          <Wysiwyg
            bind:this={wysiwygRef}
            bind:value={prompt}
            onSelectionChange={() => (selVer = selVer + 1)}
          />
        {:else}
          <Source bind:value={prompt} />
        {/if}
      </div>

      <!-- Status line moved below the prompt so the status text
           and the send/stop button sit beneath what the user is
           typing rather than between the chat history and the
           input (which read like a divider rather than a hint). -->
      <div class="status-line">
        <span class="status-msg">
          {#if loading}
            <span class="muted">press Esc to interrupt the assistant</span>
          {:else if error}
            <span class="err">{error}</span>
          {:else}
            <span class="muted">Cmd+Enter to send  ·  /clear to reset</span>
          {/if}
        </span>
        {#if loading}
          <button
            class="action-btn stop"
            onclick={cancel}
            title="stop the in-flight request (Esc also cancels)"
            aria-label="stop"
          >{@html ICON_STOP}</button>
        {:else}
          <button
            class="action-btn send"
            onclick={() => void submit()}
            disabled={!currentContext || !prompt.trim()}
            title="send (Cmd/Ctrl+Enter)"
            aria-label="send"
          >{@html ICON_SEND}</button>
        {/if}
      </div>
</OverlayShell>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    color: var(--text-secondary);
  }
  header .title {
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 600;
    color: var(--text);
  }
  /* Context dropdown next to the title. The select shrinks
     gracefully when the picker label is long ("all 4 visible
     files") so the close button still reaches the right edge. */
  header .context-select {
    flex: 1;
    min-width: 0;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 2px 6px;
    font: inherit;
    font-size: 12px;
    max-width: 320px;
  }
  header .context-select:focus { outline: none; border-color: var(--link); }
  header .sel-badge {
    background: var(--smart-bg);
    color: var(--text);
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 11px;
  }
  /* Scrollable chat history; takes the remaining vertical space. */
  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .empty {
    color: var(--text-secondary);
    text-align: center;
    padding-top: 1.5rem;
  }
  .empty-title { color: var(--text); font-weight: 600; }

  /* Chat bubbles. User aligns right with a tinted background; the
     assistant aligns left and uses the bg color. Both grow as
     wide as needed but cap at ~85% so the column stays readable. */
  .bubble {
    max-width: 85%;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .bubble.user { align-self: flex-end; align-items: flex-end; }
  .bubble.assistant { align-self: flex-start; align-items: flex-start; }
  /* The in-flight placeholder reads as a real bubble (same role
     line, same column) but with a slightly muted body so the
     user doesn't mistake the dots for actual assistant output. */
  .bubble.assistant.pending .body {
    color: var(--text-secondary);
    font-style: italic;
    font-variant-numeric: tabular-nums;
  }
  /* Role + timestamp on one line above each bubble. The user-side
     bubble aligns to the right, so the row also right-aligns to
     keep the role label closest to the bubble corner; the
     assistant-side row stays left-aligned. */
  .bubble .role-line {
    display: flex;
    align-items: baseline;
    gap: 6px;
  }
  .bubble.user .role-line { flex-direction: row-reverse; }
  .bubble .role {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
  }
  /* Softer than the role label; same baseline. */
  .bubble .ts {
    font-size: 10px;
    color: var(--text-secondary);
    opacity: 0.65;
    font-variant-numeric: tabular-nums;
  }
  /* Copy button on each turn. Always visible (was hover-only and
     hard to discover); icon-only with the role label nearby
     keeping the row compact. The icon itself flips to a check
     for ~1s after a successful copy so the user gets feedback
     without changing the row width. */
  .bubble .copy-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 1px solid var(--btn-border);
    color: var(--text-secondary);
    border-radius: 3px;
    cursor: pointer;
    padding: 0;
    width: 18px;
    height: 16px;
    transition: color 0.15s ease, border-color 0.15s ease;
  }
  .bubble .copy-btn:hover { color: var(--text); border-color: var(--btn-hover); }
  .bubble .body {
    background: var(--bg-card);
    padding: 6px 10px;
    border-radius: 8px;
    font-size: 13px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .bubble.user .body { background: var(--smart-bg); }
  /* Rendered markdown inside an assistant bubble. The body
     itself drops white-space:pre-wrap because the rendered HTML
     handles its own paragraph breaks; tighten margins so multi-
     paragraph replies don't push the bubble open. */
  .bubble .body.md { white-space: normal; }
  .bubble .body.md :global(p) { margin: 0 0 0.4em 0; }
  .bubble .body.md :global(p:last-child) { margin-bottom: 0; }
  .bubble .body.md :global(h1),
  .bubble .body.md :global(h2),
  .bubble .body.md :global(h3),
  .bubble .body.md :global(h4) {
    margin: 0.4em 0 0.2em 0;
    font-weight: 600;
  }
  .bubble .body.md :global(h1) { font-size: 16px; }
  .bubble .body.md :global(h2) { font-size: 15px; }
  .bubble .body.md :global(h3),
  .bubble .body.md :global(h4) { font-size: 14px; }
  .bubble .body.md :global(ul),
  .bubble .body.md :global(ol) {
    margin: 0.2em 0;
    padding-left: 1.4em;
  }
  .bubble .body.md :global(li) { margin: 0.1em 0; }
  .bubble .body.md :global(code) {
    background: var(--bg);
    padding: 0 4px;
    border-radius: 3px;
    font-family: ui-monospace, monospace;
    font-size: 0.92em;
  }
  .bubble .body.md :global(pre) {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 6px 8px;
    overflow-x: auto;
    margin: 0.4em 0;
  }
  .bubble .body.md :global(pre code) {
    background: transparent;
    padding: 0;
    border-radius: 0;
    font-size: 12px;
  }
  .bubble .body.md :global(a) {
    color: var(--link);
    text-decoration: underline;
  }
  .bubble .body.md :global(blockquote) {
    margin: 0.3em 0;
    padding: 0.1em 0.6em;
    border-left: 3px solid var(--border);
    color: var(--text-secondary);
  }

  /* Pending edit cards: full-width, distinct from chat bubbles
     so the user reads them as actionable artifacts rather than
     conversation. The collapsed details/summary keeps the
     scrollback compact when the proposal is large. */
  .edit-card {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 10px;
    background: var(--bg-card);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .edit-card.applied { border-color: var(--accent); }
  .edit-card.dismissed { opacity: 0.6; }
  .edit-head {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-secondary);
  }
  .edit-head .kind-chip {
    background: var(--link);
    color: #fff;
    padding: 1px 6px;
    border-radius: 3px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-weight: 600;
    font-size: 10px;
  }
  .edit-head .path { color: var(--text); font-family: ui-monospace, monospace; }
  .edit-head .size { margin-left: auto; font-variant-numeric: tabular-nums; }
  .edit-card .summary {
    color: var(--text);
    font-size: 13px;
  }
  .edit-card details summary {
    cursor: pointer;
    color: var(--text-secondary);
    font-size: 11px;
  }
  .edit-card .proposal {
    margin: 6px 0 0 0;
    padding: 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-family: ui-monospace, monospace;
    font-size: 11.5px;
    line-height: 1.45;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 40vh;
    overflow: auto;
  }
  .edit-card .actions {
    display: flex;
    gap: 6px;
  }
  .edit-card .actions button {
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    padding: 4px 12px;
    cursor: pointer;
    font: inherit;
    font-size: 12px;
  }
  .edit-card .actions button:hover { border-color: var(--btn-hover); }
  .edit-card .actions button.primary {
    background: var(--link);
    color: #fff;
    border-color: var(--link);
  }
  /* Copy button keeps icon + label inline so the action row reads
     left-to-right cleanly: Apply | Copy | Discard. The icon SVG
     is rendered via {@html} so Svelte's scoped CSS can't reach it
     directly; vertical alignment is fine via parent flex. */
  .edit-card .actions button.copy {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .edit-card .status-tag {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .edit-card .status-tag.ok { color: var(--accent); }
  .edit-card .status-tag.muted { color: var(--text-secondary); }

  .status-line {
    padding: 4px 12px;
    font-size: 11px;
    color: var(--text-secondary);
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .status-line .status-msg { flex: 1; min-width: 0; }
  .status-line .err { color: #d33; }
  .status-line .muted { opacity: 0.7; }
  /* Send / Stop primary action. Icon-only square button so the
     glyph (paper plane / filled square) reads at a glance and
     doesn't fight the rest of the chrome for horizontal space.
     Color carries semantic meaning: blue link for send, red for
     stop. */
  .action-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 22px;
    padding: 0;
    border-radius: 3px;
    cursor: pointer;
    flex: 0 0 auto;
    font: inherit;
    border: 1px solid var(--btn-border);
    background: transparent;
    color: var(--text);
  }
  .action-btn.send {
    border-color: var(--link);
    color: var(--link);
  }
  .action-btn.send:hover:not(:disabled) {
    background: var(--link);
    color: #fff;
  }
  .action-btn.send:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .action-btn.stop {
    border-color: #d33;
    color: #d33;
  }
  .action-btn.stop:hover {
    background: #d33;
    color: #fff;
  }

  /* One-shot info banner shown at the top of the chat when the
     configured model can't use tools. Distinct from chat bubbles
     and edit cards so it reads as meta-context, not part of the
     conversation. */
  .hint {
    border: 1px solid var(--border);
    border-left: 3px solid var(--link);
    border-radius: 4px;
    padding: 6px 10px;
    background: var(--bg-card);
    font-size: 12px;
    line-height: 1.45;
    color: var(--text);
  }
  .hint-title {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    margin-bottom: 2px;
  }
  .hint-body { color: var(--text-secondary); }

  /* Input row anchored at the bottom; top border separates it
     from the (scrollable) chat above. The prompt is a Wysiwyg
     instance, so we style its host wrapper here and let the
     editor's own CSS (md-wysiwyg) take care of the inner
     ProseMirror chrome. Cap the height so a long prompt doesn't
     push the chat history out of view; user can drag the divider
     in the future if we add one. */
  .prompt-wrap {
    background: var(--bg);
    max-height: 30vh;
    min-height: 80px;
    display: flex;
    flex-direction: column;
    overflow: auto;
  }
  .prompt-wrap.disabled { opacity: 0.55; pointer-events: none; }
  /* Trim the file editor's generous default padding so the
     prompt feels compact in the chat dialog. The :global is
     required because Wysiwyg's CSS lives in its own scope. */
  .prompt-wrap :global(.md-wysiwyg) {
    padding: 8px 12px;
    line-height: 1.5;
  }

  /* Toolbar above the prompt input. Same dimensions and treatment
     as the file editor's tab-bar so the visual language stays
     consistent across editors. The bar sits between the chat
     scrollback and the input area; both edges get a border so
     it reads as its own strip. */
  .prompt-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0.5rem;
    background: var(--bg-card);
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    font-size: 12px;
    color: var(--text-secondary);
    flex-shrink: 0;
    min-height: 28px;
  }
  .prompt-bar .left { flex: 1; }
  .prompt-bar .left.fmt { display: flex; gap: 4px; align-items: center; }
  .prompt-bar .actions { display: flex; gap: 2px; }
  .prompt-bar .hbtn {
    background: none;
    border: 1px solid transparent;
    border-radius: 3px;
    cursor: pointer;
    color: var(--text-secondary);
    font: inherit;
    padding: 0 5px;
    line-height: 18px;
    height: 20px;
  }
  .prompt-bar .hbtn:hover { color: var(--text); border-color: var(--btn-border); }
  .prompt-bar .hbtn.on {
    color: var(--text);
    border-color: var(--btn-hover);
    background: var(--hover-bg);
  }
  .prompt-bar .block-kind {
    background: var(--bg-card);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 3px;
    padding: 1px 4px;
    font: inherit;
    font-size: 11px;
    height: 20px;
  }
  .prompt-bar .fmtbtn { min-width: 22px; text-align: center; }
  .prompt-bar .fmtbtn b,
  .prompt-bar .fmtbtn i,
  .prompt-bar .fmtbtn s,
  .prompt-bar .fmtbtn code { font-size: 12px; }
  .prompt-bar .fmtbtn code { font-family: ui-monospace, monospace; }
</style>
