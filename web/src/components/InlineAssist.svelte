<script lang="ts">
  // Global assistant overlay. Bound to Cmd+P (Ctrl+H on non-Mac),
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
  // File edits land through chan-llm's standard `write_file` tool.
  // When `auto_apply_writes` is off (default), chan-llm returns
  // `Pending`, pauses with `stop_reason = ToolUse`, and we render
  // the proposed write as an edit card; Apply / Discard inject the
  // real tool result back so the next round closes the loop. The
  // model can target any path in scope (the single file in file
  // context, any of the visible files in group context, any path
  // it discovers in drive context).

  import { onDestroy, onMount } from "svelte";
  import { Check, Copy } from "lucide-svelte";

  import { api } from "../api/client";
  import { renderMarkdown } from "../api/markdown";
  import type {
    ContentHit,
    LlmCompletionResponse,
    LlmMessage,
    LlmStatus,
    LlmToolSpec,
  } from "../api/types";
  import Wysiwyg from "../editor/Wysiwyg.svelte";
  import Source from "../editor/Source.svelte";
  import StyleToolbar from "./StyleToolbar.svelte";
  import {
    assistantConversations,
    assistantOverlay,
    assistantStream,
    availableAssistantContexts,
    beginAssistantStream,
    clearFileConversation,
    clearGroupConversation,
    clearDriveConversation,
    endAssistantStream,
    loadGroupConversation,
    openAssistant,
    refreshTree,
    saveGroupConversation,
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
    if (!currentContext) {
      assistantOverlay.contextId = defaultScopeId();
    }
  });

  /// The prompt has its own mode toggle (mirrors the file
  /// editor's wysiwyg/source switch). Local state because the
  /// overlay is per-window-session and not worth persisting.
  let promptMode = $state<"wysiwyg" | "source">("wysiwyg");

  /// User-controlled prompt height (px). Mirrors the file editor's
  /// inspector resize affordance: a thin bar at the top of the
  /// prompt area lets the user grow / shrink the input. Local to
  /// the component for now; if multiple sessions want this to
  /// stick we can move it into preferences. Default mirrors the
  /// previous `30vh` cap on a typical 1080p panel so existing users
  /// don't see a jump on first load.
  const PROMPT_HEIGHT_MIN = 80;
  const PROMPT_HEIGHT_MAX = 600;
  let promptHeight = $state(220);
  let promptResizing = $state(false);

  function onPromptResizeDown(e: PointerEvent): void {
    e.preventDefault();
    const startY = e.clientY;
    const startH = promptHeight;
    const target = e.currentTarget as HTMLElement;
    target.setPointerCapture(e.pointerId);
    promptResizing = true;
    document.body.style.cursor = "row-resize";
    document.body.style.userSelect = "none";
    const onMove = (ev: PointerEvent) => {
      // Drag UP (deltaY < 0) grows the prompt; drag DOWN shrinks
      // it. The handle sits above the prompt input so this matches
      // the user's intuition.
      const next = startH - (ev.clientY - startY);
      promptHeight = Math.max(
        PROMPT_HEIGHT_MIN,
        Math.min(PROMPT_HEIGHT_MAX, next),
      );
    };
    const onUp = (ev: PointerEvent) => {
      target.releasePointerCapture(ev.pointerId);
      target.removeEventListener("pointermove", onMove);
      target.removeEventListener("pointerup", onUp);
      target.removeEventListener("pointercancel", onUp);
      promptResizing = false;
      document.body.style.removeProperty("cursor");
      document.body.style.removeProperty("user-select");
    };
    target.addEventListener("pointermove", onMove);
    target.addEventListener("pointerup", onUp);
    target.addEventListener("pointercancel", onUp);
  }

  /// Refs into the prompt editor so the floating StyleToolbar above
  /// it can call into Wysiwyg's mark/block-kind API. Source mode
  /// gets a disabled toolbar (a textarea ignores formatting).
  let wysiwygRef: Wysiwyg | undefined = $state();

  /// Bumped on every selection / doc change inside the prompt
  /// Wysiwyg so the StyleToolbar's active-mark / current-block
  /// derivations re-run. Mirrors the FileEditorTab pattern.
  let selVer = $state(0);

  // The prompt is a Wysiwyg instance now (markdown editor with
  // smart-node autocomplete). It handles its own focus on mount
  // and refocuses when its value is reset to "" after a submit,
  // so we don't keep an explicit ref here.
  let scrollEl: HTMLDivElement | undefined = $state();
  // Prompt buffer lives on `assistantOverlay.prompt` (module
  // state) so it round-trips through the URL hash. Local alias
  // keeps the binding sites compact.
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

  /// Shared formatting clause every system prompt ends with. The
  /// chat surface renders assistant replies through `renderMarkdown`
  /// (headings, lists, fenced code, inline code, blockquotes, bold,
  /// italic, links), so an explicit instruction here makes the
  /// model lean into structured output instead of wall-of-text
  /// prose. We don't enable raw HTML on purpose — `renderMarkdown`
  /// sanitizes anyway, but discouraging it keeps the model focused
  /// on the formats that actually round-trip.
  const FORMAT_CLAUSE =
    "REPLY FORMAT\n" +
    "  - Reply in GitHub-flavored markdown. Use headings, bullet/numbered lists, fenced code blocks (with a language tag when relevant), inline `code`, blockquotes, and **bold** / *italic* where they aid scanning.\n" +
    "  - Don't emit raw HTML; the renderer sanitizes it out anyway.\n" +
    "  - Keep code samples in fenced blocks even when short — never inline a multi-line snippet.";

  const PROMPT_FILE_TOOLS =
    "You are the user's writing assistant inside chan, a personal-notes editor. " +
    "This conversation is scoped to ONE file in the user's drive; the file's CURRENT content ships on every user turn under '# File', and the user's prompt under '# Instruction'. " +
    "If the user has text selected, it appears under '# Selection'.\n\n" +
    "TOOLS\n" +
    "  - read_file(path): read another markdown file.\n" +
    "  - list_files(prefix?): list files (optional path prefix).\n" +
    "  - search_content(query, limit?): hybrid keyword + semantic search.\n" +
    "  - write_file(path, content): propose a complete file replacement. When 'auto-apply writes' is off (default), the user reviews and clicks Apply or Discard before the write hits disk; when on, the write lands atomically. Always emit the FULL revised file content (no diffs, no partials).\n\n" +
    "DISCIPLINE\n" +
    "  - One category of tool call per turn (investigate OR propose). Don't mix.\n" +
    "  - Preserve frontmatter + unrelated sections in proposed edits.\n" +
    "  - Reply concisely; discuss before acting when intent is ambiguous.\n\n" +
    FORMAT_CLAUSE;

  const PROMPT_FILE_CHAT =
    "You are the user's writing assistant inside chan. This conversation is scoped to ONE file; the current content ships under '# File' on every user turn, and the prompt under '# Instruction'. Selected text (when any) appears under '# Selection'. The current model can't call tools.\n\n" +
    FORMAT_CLAUSE;

  const PROMPT_GROUP_TOOLS =
    "You are the user's writing assistant inside chan. This conversation is scoped to a GROUP of files visible in the user's layout; each file's CURRENT content ships on every user turn under its own '## <path>' heading inside the '# Files' block, with the user's prompt under '# Instruction'.\n\n" +
    "TOOLS\n" +
    "  - read_file(path): read any other markdown file.\n" +
    "  - list_files(prefix?), search_content(query, limit?): explore the wider drive.\n" +
    "  - write_file(path, content): propose a complete replacement for any path. When 'auto-apply writes' is off (default), the user reviews per-edit; when on, the write lands atomically. Target ONE file per call and emit the FULL revised content.\n\n" +
    "DISCIPLINE\n" +
    "  - When proposing edits, target ONE specific file at a time via its path.\n" +
    "  - One category of tool call per turn (investigate OR propose). Don't mix.\n" +
    "  - Preserve frontmatter + unrelated sections in each proposed edit.\n\n" +
    FORMAT_CLAUSE;

  const PROMPT_GROUP_CHAT =
    "You are the user's writing assistant inside chan. This conversation is scoped to a GROUP of files; each ships under its own '## <path>' heading inside the '# Files' block, with the prompt under '# Instruction'. The current model can't call tools.\n\n" +
    FORMAT_CLAUSE;

  const PROMPT_UNIVERSE_TOOLS =
    "You are answering questions about the user's personal-notes drive in chan. " +
    "Each user turn ships excerpts retrieved by hybrid search (BM25 + semantic) under '# Excerpts' and the user's question under '# Instruction'. " +
    "Use the excerpts as primary context; cite sources by their bracket number, like [1] or [3]. " +
    "Keep responses concise (3-6 sentences) unless more detail is clearly needed. " +
    "When the excerpts don't answer the question, say so plainly.\n\n" +
    "TOOLS\n" +
    "  - read_file(path) / list_files(prefix?) / search_content(query, limit?): refine your retrieval if the initial excerpts are insufficient.\n" +
    "  - write_file(path, content): only when the user explicitly asks for an edit. When 'auto-apply writes' is off (default), the user reviews the proposal; when on, the write lands atomically. Emit the FULL revised content.\n\n" +
    FORMAT_CLAUSE;

  const PROMPT_UNIVERSE_CHAT =
    "You are answering questions about the user's personal-notes drive in chan. Each turn ships hybrid-search excerpts under '# Excerpts' and the user's question under '# Instruction'. Use ONLY the excerpts as context; cite sources by their bracket number. If the excerpts don't answer, say so plainly. Keep responses concise (3-6 sentences). The current model can't call tools.\n\n" +
    FORMAT_CLAUSE;

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
  /// next time Cmd+P fires (no need to restart the app).
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
        assistantOverlay.prompt = "";
        error = null;
      }
      return;
    }
    const id = currentContext.id;
    if (lastOpenedContextId === id) return;
    lastOpenedContextId = id;
    const sel = captureSelection();
    error = null;
    // If the user had real text selected when they hit the assistant
    // shortcut, prefill the prompt with the selection as a markdown
    // blockquote followed by a blank line so the caret lands on a
    // fresh paragraph beneath. Making the reference explicit in the
    // prompt avoids the "what selection?" confusion of the silent
    // `# Selection` block, and the model still sees the same text
    // since the quote ships as part of the instruction. We clear
    // `savedSelection` in this branch so the user message doesn't
    // duplicate the same text under both `# Instruction` and
    // `# Selection`.
    if (sel) {
      assistantOverlay.prompt = formatQuotePrefill(sel);
      savedSelection = null;
      queueMicrotask(() => wysiwygRef?.focusEnd());
    } else {
      // Don't clobber a URL-restored prompt with "" when the
      // overlay opens. Only reset when there isn't one to preserve.
      if (!assistantOverlay.prompt) assistantOverlay.prompt = "";
      savedSelection = null;
    }
    void ensureToolsLoaded();
    void refreshLlmStatus();
    if (currentContext.kind === "file") {
      // Pull the persisted thread off disk; in-memory hits skip.
      void loadFileConversation(currentContext.path);
    } else if (currentContext.kind === "group") {
      // Group threads round-trip through the LRU manifest so the
      // last 10 are restored across reloads.
      void loadGroupConversation(currentContext.key);
    }
    queueMicrotask(scrollToBottom);
  });

  /// Format a selection as a markdown blockquote prefix for the
  /// prompt: each line gets `> `, blank inner lines become bare `>`,
  /// and we terminate with `\n\n` so the caret lands on a fresh,
  /// empty paragraph below the quote. A single trailing newline on
  /// the selection (common when the user triple-clicked a paragraph)
  /// is stripped before quoting so it doesn't become a phantom
  /// empty `>` line at the bottom of the block. CR/LF is normalised
  /// so a paste from Windows-style sources still renders one quote
  /// block instead of N short ones.
  function formatQuotePrefill(text: string): string {
    const normalised = text.replace(/\r\n?/g, "\n").replace(/\n$/, "");
    const quoted = normalised
      .split("\n")
      .map((l) => (l.length === 0 ? ">" : `> ${l}`))
      .join("\n");
    return `${quoted}\n\n`;
  }

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

  /// Debounced persistence. File contexts write a per-path blob
  /// (one file = one thread, lives until the file is renamed or
  /// the user clears it). Group contexts write through the LRU
  /// manifest so only the last `GROUP_LRU_MAX` group threads stick
  /// around; older ones drop off both disk and memory. Drive Q&A
  /// stays in-memory only (its retrieval-driven excerpts make
  /// long-term replay less useful).
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  function scheduleSave(ctx: ScopeOption): void {
    if (ctx.kind === "file") {
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
    } else if (ctx.kind === "group") {
      const key = ctx.key;
      const paths = ctx.paths;
      if (saveTimer) clearTimeout(saveTimer);
      saveTimer = setTimeout(() => {
        saveTimer = null;
        const conv = assistantConversations.byGroup[key];
        if (!conv) return;
        void saveGroupConversation(key, paths, conv);
      }, 400);
    }
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

  /// Keep the chat pinned to the bottom while deltas stream in:
  /// reading `assistantStream.text` ties this effect to every
  /// fragment, and queueMicrotask defers the scroll past the DOM
  /// update so the new content is laid out before we measure.
  $effect(() => {
    if (!loading) return;
    const _ = assistantStream.text.length;
    void _;
    queueMicrotask(scrollToBottom);
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
    assistantOverlay.prompt = "";
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

  /// chan-llm gates `write_file` calls behind `auto_apply_writes`:
  /// when off, the tool returns `Pending`, chan-llm pauses with
  /// `stop_reason = ToolUse`, and the host (us) is responsible for
  /// surfacing a confirmation UI. We render those Pending writes as
  /// edit cards in the scrollback; Apply / Discard inject the real
  /// tool result back into the next round so the model sees the
  /// outcome. There is no separate `propose_file_edit` tool — the
  /// model only sees `write_file` (the chan-llm standard schema).
  const WRITE_FILE_TOOL = "write_file";

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
    const trimmed = assistantOverlay.prompt.trim();
    if (!trimmed) return;
    // Slash commands: handled locally, no LLM round-trip.
    if (trimmed === "/clear") {
      clearCurrent();
      return;
    }
    const conv = conversationFor(ctx);
    // Auto-dismiss any still-pending edit before pushing the new
    // user turn. Anthropic and Gemini both reject a request where
    // an assistant tool_use isn't paired with a matching tool_result
    // in the very next user turn; if the user types over a pending
    // proposal without clicking Apply/Discard, the dangling tool_use
    // would 400 the next round. Treat it as a soft dismissal so the
    // model sees the user moved on.
    const lastTurn = conv.turns[conv.turns.length - 1];
    if (lastTurn && lastTurn.kind === "edit" && lastTurn.edit.status === "pending") {
      dismissEdit(lastTurn.edit, "user moved on without acting");
    }
    loading = true;
    error = null;
    pendingTurnTime = Date.now();
    // For drive context we retrieve excerpts before composing
    // the user message; for file/group the context IS the file
    // contents, no retrieval needed.
    let excerpts: ContentHit[] | null = null;
    const ctl = new AbortController();
    inflight = ctl;
    // Mint a per-request correlation id and arm the streaming buffer
    // BEFORE the HTTP request leaves so the first `llm.delta` frame
    // (which can arrive while the POST is still hanging) lands in the
    // matching session. crypto.randomUUID is available in every
    // browser chan targets; no fallback needed.
    const sessionId = crypto.randomUUID();
    beginAssistantStream(sessionId);
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
    assistantOverlay.prompt = "";
    scheduleSave(ctx);
    queueMicrotask(scrollToBottom);
    try {
      // Tool list is gated on the backend's current capability.
      // Sending a `tools` array to a non-tool-capable model causes
      // Ollama to refuse the request outright; omit the field
      // entirely so the request is plain chat.
      //
      // Server-side (chan-llm) currently uses `standard_tool_schemas`
      // unconditionally; the request's `tools` field is observed for
      // forward compatibility but not plumbed. We still ship the
      // catalog so the frontend's contract stays honest if/when the
      // server starts honoring it.
      const tools = supportsTools() ? serverTools : undefined;
      const resp = await api.llmComplete(
        {
          messages: conv.messages,
          tools,
          max_tokens: 4000,
          session_id: sessionId,
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
      // End the stream AFTER the response has been folded into
      // `conv.turns` so the live bubble's contents don't blink to
      // empty between deltas-clear and the final turn render.
      endAssistantStream();
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
    // Pair every tool call from this assistant turn with a tool
    // message in the same conversation order so Anthropic / Gemini
    // accept the next round (both reject a tool_use without a
    // matching tool_result).
    //
    //   - read_file / list_files / search_content / repo_report:
    //     chan-llm auto-executed these; the real result arrived
    //     via the `llm.tool_result` WS frame and lives in
    //     `assistantStream.toolResults`. Inject it verbatim.
    //   - write_file: chan-llm paused with a PENDING_STATUS
    //     placeholder. We do NOT push the placeholder; instead we
    //     render an edit card and let Apply / Discard inject the
    //     real result (mtime echo or dismissal note) at the moment
    //     the user acts. The dangling tool_use is safe as long as
    //     the user acts (or close/submit auto-dismisses it) before
    //     the next /api/llm/complete fires.
    const captured = assistantStream.toolResults;
    for (const call of resp.tool_calls) {
      if (call.name === WRITE_FILE_TOOL) {
        const input = (call.input ?? {}) as {
          path?: string;
          content?: string;
        };
        const edit: AssistantPendingEdit = {
          toolCallId: call.id,
          path: input.path ?? defaultEditPath(ctx),
          content: input.content ?? "",
          // write_file's schema doesn't carry a model-supplied
          // summary; leave null so the edit card hides the row.
          summary: null,
          status: "pending",
        };
        conv.turns.push({ kind: "edit", edit, created_at: Date.now() });
        continue;
      }
      // Non-write tool: ship the captured result back as a tool
      // message. Fall back to a generic "(no result)" stub when the
      // WS frame didn't land (rare; usually a backend that emits
      // `on_done` without the matching tool_result, e.g. error
      // path) so the assistant turn still has a paired result.
      const result = captured[call.id];
      const body =
        result === undefined
          ? JSON.stringify({ error: "tool result missing from stream" })
          : typeof result === "string"
            ? result
            : JSON.stringify(result);
      conv.messages.push({
        role: "tool",
        content: body,
        tool_call_id: call.id,
      });
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
    // Refuse if any open tab on this path is filesystem-locked. The
    // user can keep the readonly file in scope (the assistant can
    // still see it and answer questions) but accepting an edit
    // requires a writable target. We only check open tabs because
    // that's where fsWritable is known client-side; for paths with
    // no open tab the server's write will reject with a permission
    // error, which still surfaces in the catch below.
    let openTabUpdated = false;
    for (const node of Object.values(layout.nodes)) {
      if (node.kind !== "leaf") continue;
      for (const t of node.tabs) {
        if (t.kind !== "file" || t.path !== edit.path) continue;
        if (!t.fsWritable) {
          error = `'${edit.path}' is read-only on disk; cannot apply edit`;
          return;
        }
        t.content = edit.content;
        // Mark clean so the autosave loop doesn't re-write what
        // we're about to flush explicitly below.
        t.saved = edit.content;
        openTabUpdated = true;
      }
    }
    let writeResult: { mtime: number | null };
    try {
      writeResult = await api.write(edit.path, edit.content);
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
    // Inject a structured tool_result that mirrors chan-llm's
    // write_file success shape (status + path + size + applied_by).
    // The model already knows what it asked to write; reflecting
    // the on-disk bytes back is what closes the loop and lets the
    // next turn reason about the new state.
    appendToolResultJson(edit.toolCallId, {
      status: "ok",
      tool: WRITE_FILE_TOOL,
      path: edit.path,
      bytes: edit.content.length,
      mtime_ns: writeResult.mtime,
      applied_by: "user",
    });
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
    appendToolResultJson(edit.toolCallId, {
      status: "rejected",
      tool: WRITE_FILE_TOOL,
      path: edit.path,
      reason,
    });
    if (currentContext) scheduleSave(currentContext);
    queueMicrotask(scrollToBottom);
  }

  function appendToolResultJson(toolCallId: string, body: unknown): void {
    if (!currentContext) return;
    const conv = conversationFor(currentContext);
    conv.messages.push({
      role: "tool",
      content: JSON.stringify(body),
      tool_call_id: toolCallId,
    });
  }

  function onWindowKey(e: KeyboardEvent): void {
    if (visible && (e.metaKey || e.ctrlKey) && e.key === "Enter") {
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
  onMount(() => {
    document.addEventListener("keydown", onWindowKey);
  });
  onDestroy(() => {
    document.removeEventListener("keydown", onWindowKey);
  });

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

<OverlayShell id="assistant" open={visible} onClose={close}>
      <header>
        <span class="title">Scope</span>
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
                    <Check size={12} strokeWidth={2} aria-hidden="true" />
                  {:else}
                    <Copy size={12} strokeWidth={1.75} aria-hidden="true" />
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
                    <Check size={12} strokeWidth={2} aria-hidden="true" />
                  {:else}
                    <Copy size={12} strokeWidth={1.75} aria-hidden="true" />
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
                      <Check size={12} strokeWidth={2} aria-hidden="true" /><span>Copied</span>
                    {:else}
                      <Copy size={12} strokeWidth={1.75} aria-hidden="true" /><span>Copy</span>
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
          <!-- In-flight assistant turn. Lives outside the turns array
               so it disappears automatically when the real reply
               lands (which pushes a real assistant turn into
               `turns`). Two display modes:
                 - No deltas yet: animated "thinking…" dots so the
                   user sees the request landed.
                 - Deltas streaming: render the accumulated text
                   live with a trailing caret. We render as plain
                   text (white-space:pre-wrap) instead of markdown
                   while streaming so a half-typed code fence or
                   list marker doesn't flicker between layouts on
                   every token. The final assistant turn (in the
                   turns array) re-renders as markdown once the
                   response lands. -->
          <div class="bubble assistant pending">
            <div class="role-line">
              <span class="role">assistant</span>
              <span class="ts">{formatRelative(pendingTurnTime)}</span>
            </div>
            {#if assistantStream.text.length > 0}
              <div class="body streaming">{assistantStream.text}<span class="caret" aria-hidden="true"></span></div>
            {:else}
              <div class="body">thinking{".".repeat(thinkingDots)}</div>
            {/if}
          </div>
        {/if}
      </div>

      <!-- "Aa" StyleToolbar (same component the file editor uses)
           rendered in-flow above the prompt box so the formatting
           chrome doesn't sit on top of the text. Always mounted in
           both modes so the trailing `</>` / `¶` toggle stays
           reachable; the formatting controls grey out in source
           mode while the mode toggle stays clickable. -->
      <div class="prompt-area">
        <div class="prompt-toolbar-row">
          <StyleToolbar
            wysiwyg={wysiwygRef}
            selVer={selVer}
            disabled={!currentContext || promptMode !== "wysiwyg"}
            showImage={false}
            floating={false}
            mode={promptMode}
            onModeToggle={(next) => (promptMode = next)}
          />
        </div>
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="prompt-resize-handle"
          class:active={promptResizing}
          onpointerdown={onPromptResizeDown}
          aria-label="resize prompt"
          title="drag to resize the prompt"
        ></div>
        <div
          class="prompt-wrap"
          class:disabled={!currentContext}
          style="height: {promptHeight}px"
        >
          {#if promptMode === "wysiwyg"}
            <Wysiwyg
              bind:this={wysiwygRef}
              bind:value={assistantOverlay.prompt}
              onSelectionChange={() => (selVer = selVer + 1)}
            />
          {:else}
            <Source bind:value={assistantOverlay.prompt} />
          {/if}
        </div>
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
          >×</button>
        {:else}
          <button
            class="action-btn send"
            onclick={() => void submit()}
            disabled={!currentContext || !assistantOverlay.prompt.trim()}
            title="send (Cmd/Ctrl+Enter)"
            aria-label="send"
          >→</button>
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
    font-size: 14px;
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
    font-size: 14px;
    max-width: 320px;
  }
  header .context-select:focus { outline: none; border-color: var(--link); }
  header .sel-badge {
    background: var(--smart-bg);
    color: var(--text);
    padding: 1px 6px;
    border-radius: 3px;
    font-size: 13px;
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
  /* While streaming we override the italic + muted dots styling
     since the body is now real assistant text being built up live.
     Plain text rendering (the markdown rerender happens once the
     turn is finalized) keeps the same white-space:pre-wrap as the
     standard bubble. */
  .bubble.assistant.pending .body.streaming {
    color: var(--text);
    font-style: normal;
    font-variant-numeric: normal;
  }
  /* Trailing caret: 2px-wide vertical bar that blinks at 1Hz to
     signal the bubble is alive even when the model pauses between
     tokens. Inline-block so it sits flush against the last
     character without breaking onto its own line, and uses
     `currentColor` so it inherits the bubble's theme color. */
  .bubble .caret {
    display: inline-block;
    width: 2px;
    height: 0.95em;
    margin-left: 2px;
    vertical-align: text-bottom;
    background: currentColor;
    opacity: 0.7;
    animation: chan-caret-blink 1s steps(2, start) infinite;
  }
  @keyframes chan-caret-blink {
    to { opacity: 0; }
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
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
  }
  /* Softer than the role label; same baseline. */
  .bubble .ts {
    font-size: 12px;
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
    background: var(--assistant-bubble-bg);
    padding: 6px 10px;
    border-radius: 8px;
    font-size: 15px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .bubble.user .body { background: var(--assistant-user-bubble-bg); }
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
  .bubble .body.md :global(h4) { font-size: 16px; }
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
    font-size: 14px;
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
    font-size: 13px;
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
    font-size: 12px;
  }
  .edit-head .path { color: var(--text); font-family: ui-monospace, monospace; }
  .edit-head .size { margin-left: auto; font-variant-numeric: tabular-nums; }
  .edit-card .summary {
    color: var(--text);
    font-size: 15px;
  }
  .edit-card details summary {
    cursor: pointer;
    color: var(--text-secondary);
    font-size: 13px;
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
    font-size: 14px;
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
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .edit-card .status-tag.ok { color: var(--accent); }
  .edit-card .status-tag.muted { color: var(--text-secondary); }

  .status-line {
    padding: 4px 12px;
    font-size: 13px;
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
    /* Glyph buttons (→ / ×): scale up so the character reads at the
       same visual weight as the surrounding 14px hint text but with
       a clearer hit-area. line-height pinned so the glyph centers
       vertically inside the 22px button. */
    font-size: 16px;
    line-height: 1;
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
    font-size: 14px;
    line-height: 1.45;
    color: var(--text);
  }
  .hint-title {
    font-size: 12px;
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
  /* Prompt input surface. Distinct from the panel via a top
     border + slightly off-bg fill so the area where the user
     types is visible against the chat scrollback above and
     against the panel chrome below. The previous all-white
     light-mode look made the input invisible until the user
     started typing (and even then the cursor blended in). */
  .prompt-wrap {
    position: relative;
    background: var(--bg-card);
    min-height: 80px;
    display: flex;
    flex-direction: column;
    overflow: auto;
  }
  /* Drag-to-resize bar on top of the prompt input. Sits above the
     .prompt-wrap so a drag upward grows the input height. Same
     "thin neutral bar that thickens on hover" look as
     ResizeHandle.svelte, just rotated to the horizontal axis. */
  .prompt-resize-handle {
    height: 4px;
    flex-shrink: 0;
    background: var(--separator);
    cursor: row-resize;
    touch-action: none;
    transition: height 0.1s, background 0.1s;
  }
  .prompt-resize-handle:hover,
  .prompt-resize-handle.active {
    height: 6px;
    background: var(--separator-hover);
  }
  .prompt-wrap.disabled { opacity: 0.55; pointer-events: none; }
  /* Trim the file editor's generous default padding so the
     prompt feels compact in the chat dialog. The :global is
     required because Wysiwyg's CSS lives in its own scope. */
  .prompt-wrap :global(.md-wysiwyg) {
    padding: 8px 12px;
    line-height: 1.5;
  }

  /* Container that holds the prompt input. The StyleToolbar is
     rendered in-flow as its own row above the resize handle and
     the prompt body so the chrome doesn't overlay the user's text. */
  .prompt-area {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  /* Row hosting the StyleToolbar above the prompt. Padded to align
     with the prompt body's left edge, plus 10px below so the
     toolbar's hover-scale wobble doesn't visually crowd the
     prompt's top edge or the resize handle. */
  .prompt-toolbar-row {
    display: flex;
    align-items: flex-start;
    padding: 4px 12px 10px;
  }
</style>
