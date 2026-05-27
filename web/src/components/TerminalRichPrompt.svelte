<script lang="ts">
  import { tick } from "svelte";
  import { Bot, ChevronDown, ChevronUp, Code2, Copy, GripHorizontal, Pilcrow, Plus, Send, Type } from "lucide-svelte";
  import {
    PAGE_WIDTH_MAX_PCT,
    PAGE_WIDTH_MIN_PCT,
    PAGE_WIDTH_STEP_PCT,
  } from "../state/pageWidth.svelte";
  import Source from "../editor/Source.svelte";
  import Wysiwyg from "../editor/Wysiwyg.svelte";
  import StyleToolbar from "./StyleToolbar.svelte";
  import type { TerminalRichPromptState } from "../state/tabs.svelte";
  import { api } from "../api/client";
  import {
    workspace,
    setTransientStatus,
    ui,
  } from "../state/store.svelte";
  import type { TerminalSpawnResponse } from "../api/types";
  import { openSpawnDialog as openGlobalSpawnDialog } from "../state/spawnDialog.svelte";
  import {
    defaultTeamConfig,
    exportTeamDialogConfig,
    openTeamDialog as openGlobalTeamDialog,
  } from "../state/teamDialog.svelte";
  import { runTeamBootstrap } from "../state/teamOrchestrator.svelte";

  let {
    prompt,
    onSubmit,
    terminalSessionId,
    watcherPath,
    onSpawned,
    bubbleCount = 0,
  }: {
    prompt: TerminalRichPromptState;
    onSubmit: (source: string) => void;
    terminalSessionId?: string;
    watcherPath?: string | null;
    onSpawned?: (response: TerminalSpawnResponse, name: string) => void;
    /// `fullstack-a-4`: number of unanswered survey bubbles
    /// currently rendered above the prompt. When > 0, the prompt
    /// does NOT auto-focus its input so numbered keystrokes flow
    /// to the BubbleOverlay's window keydown handler. When the
    /// count drops to 0, focus returns to the prompt input.
    bubbleCount?: number;
  } = $props();

  const MIN_HEIGHT = 150;
  const TOP_GAP = 36;
  /// `fullstack-a-89`: empty-state hint copy. Threaded as
  /// `placeholderText` to both the Wysiwyg + Source editors
  /// so CM6's `placeholder` extension renders it inside the
  /// first line at the cursor position.
  // `fullstack-a-89b`: leading space per @@Alex's literal spec
  // `{cursor}{space}{default-text}`. The space gives a visible
  // gap between the blinking cursor and the placeholder text so
  // the cursor reads as a starting position rather than overlapping
  // the first glyph. Paired with the `.cm-placeholder` CSS rule
  // in TerminalRichPrompt's style block below.
  const PROMPT_PLACEHOLDER_TEXT = " Write a multi-line command and Cmd+Enter";
  let rootEl: HTMLDivElement | undefined = $state();
  let wysiwygRef: Wysiwyg | undefined = $state();
  let sourceRef: Source | undefined = $state();
  let selVer = $state(0);
  let menu = $state<{ x: number; y: number } | null>(null);
  let agentModeBusy = $state(false);
  let dragging = false;
  const workspaceRow = $derived(workspaceStatus());
  const eventCount = $derived(Math.max(0, bubbleCount));

  // `fullstack-79`: auto-focus the input on every `openActiveTerminalRichPrompt`
  // call. The `focusNonce` is bumped by the open helper even when the prompt
  // is already open, so re-show via Cmd+K p / Cmd+P steals focus back
  // even if the user had clicked away. `tick()` waits for the editor child's
  // `bind:this` to settle on first mount, and for the `{#key mode()}` block
  // to remount when the user toggles between wysiwyg and source.
  //
  // `fullstack-a-4`: when survey bubbles are present we leave focus
  // alone so the BubbleOverlay's window keydown handler receives
  // the numbered-reply keystrokes (its `editableTarget` guard
  // would otherwise swallow them if the editor stole focus first).
  // Once `bubbleCount` drops to 0, the effect re-runs and snaps
  // focus back to the prompt input.
  $effect(() => {
    void prompt.focusNonce;
    if (bubbleCount > 0) return;
    const inSource = mode() === "source";
    void tick().then(() => {
      if (inSource) {
        sourceRef?.focusAt(prompt.buffer.length);
      } else {
        wysiwygRef?.focusEnd();
      }
    });
  });

  // `fullstack-a-29` + `fullstack-a-30`: track the prompt's actual
  // rendered height AND width so two downstream reactors stay in
  // sync with whatever the browser painted:
  //   - height feeds the terminal-host reserved-space reactor in
  //     TerminalTab.svelte (covers the `fullstack-a-24` collapse
  //     transition where `heightPx` is stale).
  //   - width feeds the per-prompt page-width clamp in
  //     `richPromptPageWidthPx` below, needed so the cap is
  //     relative to THIS prompt's painted width, not the pane's
  //     editor wrapper.
  // ResizeObserver is the natural source of truth here; both
  // values land on non-persisted state fields and repopulate on
  // every mount.
  $effect(() => {
    const el = rootEl;
    if (!el || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      prompt.measuredHeightPx = Math.round(entry.contentRect.height);
      prompt.measuredWidthPx = Math.round(entry.contentRect.width);
    });
    observer.observe(el);
    return () => observer.disconnect();
  });

  // `fullstack-a-30`: per-prompt page-width override. Computes a
  // pixel cap relative to the prompt's current painted width and
  // OVERRIDES the inherited `--chan-page-max-width` set by
  // Pane.svelte on the editor wrapper. Result: narrowing the
  // editor's page-width slider in pane A no longer cascades onto
  // pane B's rich prompt, each prompt carries its own.
  //
  // Absent / undefined ratio reads as "no cap" (the prompt's
  // composer uses 100% of the painted width). Explicit ratio in
  // [0.25, 1.0); 1.0 / 100% rounds to absent on serialize so the
  // no-cap default keeps the persisted shape short.
  function richPromptPageWidthPx(): string {
    // `none` is the documented sentinel both Wysiwyg.svelte and
    // Source.svelte recognise via `max-width: var(--chan-page-max-width, none);`
    // Overriding the pane-level value with `none` is what
    // achieves the cross-tile decoupling. Default = `none`
    // (composer fills the prompt's painted width) so the prompt
    // does not inherit the editor's narrow cap. The explicit
    // ratio branch swaps in a pixel cap relative to THIS
    // prompt's painted width.
    const ratio = prompt.pageWidthRatio;
    if (!ratio || ratio >= 1) return "none";
    const width = prompt.measuredWidthPx;
    if (!width || width <= 0) return "none";
    return `${Math.max(240, Math.round(width * ratio))}px`;
  }

  function onRichPromptPageWidthSlider(e: Event): void {
    const pct = Number((e.currentTarget as HTMLInputElement).value);
    if (!Number.isFinite(pct)) return;
    const clamped = Math.min(PAGE_WIDTH_MAX_PCT, Math.max(PAGE_WIDTH_MIN_PCT, pct));
    const ratio = clamped / 100;
    prompt.pageWidthRatio = ratio >= 1 ? undefined : ratio;
  }

  function richPromptPageWidthPct(): number {
    const ratio = prompt.pageWidthRatio;
    if (!ratio || ratio >= 1) return PAGE_WIDTH_MAX_PCT;
    return Math.round(ratio * 100);
  }

  function mode(): "wysiwyg" | "source" {
    return prompt.mode ?? "wysiwyg";
  }

  function setMode(next: "wysiwyg" | "source"): void {
    prompt.mode = next;
  }

  function toolbarOpen(): boolean {
    // `fullstack-a-24`: default-off. The toolbar is opt-in now and
    // mounts INSIDE the prompt bubble (above the editor body) when
    // toggled on. Previously it was default-on and sat outside the
    // bubble. `undefined` reads as off; explicit `true` opens it.
    return prompt.styleToolbarOpen === true;
  }

  function collapsed(): boolean {
    return prompt.collapsed === true;
  }

  function toggleCollapsed(): void {
    prompt.collapsed = !collapsed();
  }

  function toggleMode(): void {
    prompt.mode = mode() === "source" ? "wysiwyg" : "source";
    menu = null;
  }

  function toggleToolbar(): void {
    prompt.styleToolbarOpen = !toolbarOpen();
    menu = null;
  }

  type AgentTarget = "none" | "claude" | "codex" | "gemini";

  function agentTarget(): AgentTarget {
    return prompt.agentTarget ?? (prompt.submitMode === "agent" ? "claude" : "none");
  }

  async function setAgentTarget(next: AgentTarget): Promise<void> {
    const previousTarget = agentTarget();
    const previousMode = prompt.submitMode;
    prompt.agentTarget = next;
    prompt.submitMode = next === "none" ? "shell" : "agent";
    if (!terminalSessionId) return;
    agentModeBusy = true;
    try {
      await api.setTerminalSubmitMode(terminalSessionId, prompt.submitMode);
    } catch (err) {
      prompt.agentTarget = previousTarget;
      prompt.submitMode = previousMode;
      ui.status = `submit-mode flip failed: ${(err as Error).message}`;
    } finally {
      agentModeBusy = false;
    }
  }

  function workspaceStatus(): { text: string; error: boolean } | null {
    if (prompt.workspaceError) return { text: prompt.workspaceError, error: true };
    if (prompt.workspaceBusy) return { text: "preparing workspace", error: false };
    if (prompt.phase === "broken") return { text: "workspace broken", error: true };
    if (prompt.phase === "submitted") return { text: "prompt archived", error: false };
    if (prompt.workspacePath) return { text: `workspace ${prompt.workspacePath}`, error: false };
    return null;
  }

  async function copyWorkspacePath(): Promise<void> {
    menu = null;
    const path = prompt.workspaceAbs || prompt.workspacePath;
    if (!path) return;
    const clipboard = navigator.clipboard;
    if (!clipboard?.writeText) {
      ui.status = "clipboard write is unavailable";
      return;
    }
    try {
      await clipboard.writeText(path);
      setTransientStatus("copied metadata dir");
    } catch (err) {
      ui.status = `copy failed: ${(err as Error).message}`;
    }
  }

  function submit(): void {
    onSubmit(prompt.buffer);
  }

  function onKeydown(e: KeyboardEvent): void {
    // `fullstack-a-20`: respect children that already handled the
    // chord. Wysiwyg's CM6 keymap has its own Mod-Enter binding
    // (`fullstack-a-18` threaded `onSubmit` to it), and CM's keymap
    // runner calls `preventDefault()` when its handler returns true.
    // Without this guard the chord triggers submit twice: once from
    // the CM keymap, once from the wrapper after the event bubbles,
    // `pwd` arrives in the PTY as `pwdpwd`. Source mode has no
    // Mod-Enter binding so it still reaches this wrapper unhandled
    // and dispatches once.
    if (e.defaultPrevented) return;
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      menu = null;
      return;
    }
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey) && !e.altKey && !e.shiftKey) {
      e.preventDefault();
      e.stopPropagation();
      submit();
    }
  }

  function onResizePointerDown(e: PointerEvent): void {
    if (!rootEl) return;
    dragging = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault();
  }

  function onResizePointerMove(e: PointerEvent): void {
    if (!dragging || !rootEl?.parentElement) return;
    const bounds = rootEl.parentElement.getBoundingClientRect();
    const next = bounds.bottom - e.clientY;
    const max = Math.max(MIN_HEIGHT, bounds.height - TOP_GAP);
    prompt.heightPx = Math.round(Math.min(Math.max(next, MIN_HEIGHT), max));
  }

  function onResizePointerUp(e: PointerEvent): void {
    dragging = false;
    try {
      (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
    } catch {
      // Pointer capture can already be gone if the pointer left the window.
    }
  }

  function onContextMenu(e: MouseEvent): void {
    e.stopPropagation();
    e.preventDefault();
    menu = { x: e.clientX, y: e.clientY };
  }

  function openMenuFromButton(e: MouseEvent): void {
    e.stopPropagation();
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    menu = { x: Math.round(rect.left), y: Math.round(rect.bottom + 6) };
  }

  function onWindowPointerDown(e: PointerEvent): void {
    if (!menu) return;
    const target = e.target as HTMLElement | null;
    if (target?.closest(".ctx")) return;
    menu = null;
  }

  /// Phase 9: Spawn agents moved into the prompt action menu. The
  /// dialog's Bootstrap button hands off to the orchestrator, which
  /// persists the team config, loads the per-team watcher, spawns
  /// terminals, and waits for preflight confirmation before prompt
  /// staging.
  function openNewTeamDialog(): void {
    menu = null;
    openGlobalTeamDialog({
      hostSessionId: terminalSessionId,
      onBootstrap: async (config) => {
        await runTeamBootstrap(config, terminalSessionId);
      },
      onSpawned: (response, agentName) => onSpawned?.(response, agentName),
    });
  }

  function openSpawnDialog(): void {
    menu = null;
    openGlobalSpawnDialog({
      orchestratorSessionId: terminalSessionId,
      onSpawned: (response, agentName) => onSpawned?.(response, agentName),
    });
  }

  async function copySpawnAgentsConfig(): Promise<void> {
    menu = null;
    const clipboard = navigator.clipboard;
    if (!clipboard?.writeText) {
      ui.status = "clipboard write is unavailable";
      return;
    }
    try {
      await clipboard.writeText(exportTeamDialogConfig(defaultTeamConfig()));
      setTransientStatus("copied spawn agents config");
    } catch (err) {
      ui.status = `copy failed: ${(err as Error).message}`;
    }
  }

  async function setBubbleMode(mode: "stack" | "tray"): Promise<void> {
    menu = null;
    if (workspace.info) {
      workspace.info = {
        ...workspace.info,
        preferences: { ...workspace.info.preferences, bubble_overlay_mode: mode },
      };
    }
    try {
      await api.setBubbleOverlayMode(mode);
    } catch (err) {
      ui.status = `bubble mode failed: ${(err as Error).message}`;
    }
  }
</script>

<svelte:window onpointerdown={onWindowPointerDown} />

<div
  class="rich-prompt"
  class:collapsed={collapsed()}
  bind:this={rootEl}
  style:height={collapsed() ? null : `${prompt.heightPx ?? 320}px`}
  style:--chan-page-max-width={richPromptPageWidthPx()}
  role="dialog"
  tabindex="-1"
  aria-label="rich terminal prompt"
  onkeydown={onKeydown}
  oncontextmenu={onContextMenu}
>
  <button
    type="button"
    class="resize-handle"
    aria-label="resize prompt"
    title="Resize"
    onpointerdown={onResizePointerDown}
    onpointermove={onResizePointerMove}
    onpointerup={onResizePointerUp}
    onpointercancel={onResizePointerUp}
  >
    <GripHorizontal size={16} strokeWidth={1.75} aria-hidden="true" />
  </button>
  <header>
    {#if toolbarOpen()}
      <StyleToolbar
        wysiwyg={wysiwygRef}
        {selVer}
        disabled={mode() === "source"}
        floating={false}
        mode={mode()}
        onModeToggle={setMode}
      />
    {/if}
    <div class="spacer"></div>
    <button
      type="button"
      class="icon-btn"
      onclick={openMenuFromButton}
      title="Rich Prompt actions"
      aria-label="Rich Prompt actions"
      aria-haspopup="menu"
      aria-expanded={menu ? "true" : "false"}
    >
      <Plus size={16} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <span
      class="event-pill"
      title={watcherPath ? `Watching ${watcherPath}` : "No event watcher"}
      aria-label={`${eventCount} visible events`}
    >
      {eventCount}
    </span>
    <select
      class="agent-picker"
      value={agentTarget()}
      disabled={agentModeBusy}
      aria-label="Agent picker"
      title="Agent picker"
      onchange={(e) => void setAgentTarget((e.currentTarget as HTMLSelectElement).value as AgentTarget)}
    >
      <option value="none">none</option>
      <option value="claude">claude</option>
      <option value="codex">codex</option>
      <option value="gemini">gemini</option>
    </select>
    <button type="button" class="icon-btn" onclick={submit} title="Send prompt" aria-label="Send prompt">
      <Send size={16} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <!-- `fullstack-a-24`: collapse/expand the prompt to a minimal
         bar so chat / survey bubbles above get more vertical room.
         Distinct from Close (dismiss); collapse keeps the prompt
         attached, just smaller. Chevron orientation flips with the
         state, down to collapse-toward-bottom, up to expand. -->
    <button
      type="button"
      class="icon-btn"
      onclick={toggleCollapsed}
      title={collapsed() ? "Expand prompt" : "Collapse prompt"}
      aria-label={collapsed() ? "Expand prompt" : "Collapse prompt"}
      aria-pressed={collapsed()}
    >
      {#if collapsed()}
        <ChevronUp size={16} strokeWidth={1.75} aria-hidden="true" />
      {:else}
        <ChevronDown size={16} strokeWidth={1.75} aria-hidden="true" />
      {/if}
    </button>
  </header>
  {#if workspaceRow}
    <div class="workspace-row" class:error={workspaceRow.error}>
      <span>{workspaceRow.text}</span>
      {#if prompt.workspaceAbs || prompt.workspacePath}
        <button type="button" onclick={copyWorkspacePath}>Copy path</button>
      {/if}
    </div>
  {/if}
  <div class="composer-editor">
    {#key mode()}
      {#if mode() === "wysiwyg"}
        <Wysiwyg
          bind:this={wysiwygRef}
          bind:value={prompt.buffer}
          currentPath={null}
          autoFocus={bubbleCount === 0}
          placeholderText={PROMPT_PLACEHOLDER_TEXT}
          onSubmit={submit}
          onSelectionChange={() => (selVer += 1)}
        />
      {:else}
        <Source
          bind:this={sourceRef}
          bind:value={prompt.buffer}
          path="prompt.md"
          syntaxHighlight
          autoFocus={bubbleCount === 0}
          placeholderText={PROMPT_PLACEHOLDER_TEXT}
        />
      {/if}
    {/key}
    <!-- `fullstack-a-89`: placeholder moved from the
         pre-fix CSS overlay (`-a-24`) into CM6's built-in
         `placeholder` extension threaded as
         `placeholderText` on both Wysiwyg + Source. The
         extension renders the hint at the cursor position
         INSIDE the editor's first line, so cursor and
         placeholder share the exact same x/y instead of
         living in parallel positioning systems. `-a-84`
         (10px X-offset) + `-a-87` (line-height 1.8 match)
         were CSS patches that couldn't fully close the
         empirical gap because the architecture itself
         (overlay vs in-editor) was misaligned. -->
  </div>
  {#if menu}
    <div class="ctx" style:left={`${menu.x}px`} style:top={`${menu.y}px`} role="menu">
      <!-- `fullstack-a-30`: per-prompt page-width slider, mirrors
           the editor's tab-menu slider. Sets `prompt.pageWidthRatio`
           directly, does not touch the global `pageWidth.ratio`,
           so narrowing this prompt does not cascade onto the
           editor's wrap or onto sibling tiles. 100 % unsets the
           per-prompt ratio (rounds to absent on serialize). -->
      <div class="page-width-row">
        <span class="page-width-label">Page width</span>
        <input
          class="page-width-slider"
          type="range"
          min={PAGE_WIDTH_MIN_PCT}
          max={PAGE_WIDTH_MAX_PCT}
          step={PAGE_WIDTH_STEP_PCT}
          value={richPromptPageWidthPct()}
          oninput={onRichPromptPageWidthSlider}
          onmousedown={(e) => e.stopPropagation()}
          aria-label="rich prompt page width"
        />
        <span class="page-width-value">{richPromptPageWidthPct()}%</span>
      </div>
      <button type="button" onclick={toggleMode}>
        {#if mode() === "source"}
          <Pilcrow size={15} strokeWidth={1.75} aria-hidden="true" />
          <span>Show rendered</span>
        {:else}
          <Code2 size={15} strokeWidth={1.75} aria-hidden="true" />
          <span>Show source code</span>
        {/if}
      </button>
      <button type="button" onclick={toggleToolbar}>
        <Type size={15} strokeWidth={1.75} aria-hidden="true" />
        <span>{toolbarOpen() ? "Hide style toolbar" : "Show style toolbar"}</span>
      </button>
      <button type="button" onclick={openSpawnDialog}>
        <Bot size={15} strokeWidth={1.75} aria-hidden="true" />
        <span>Spawn agent</span>
      </button>
      <button type="button" onclick={openNewTeamDialog}>
        <Bot size={15} strokeWidth={1.75} aria-hidden="true" />
        <span>Spawn agents</span>
      </button>
      {#if prompt.workspaceAbs || prompt.workspacePath}
        <button type="button" onclick={copyWorkspacePath}>
          <Copy size={15} strokeWidth={1.75} aria-hidden="true" />
          <span>Copy metadata dir</span>
        </button>
      {/if}
      <button type="button" onclick={copySpawnAgentsConfig}>
        <Copy size={15} strokeWidth={1.75} aria-hidden="true" />
        <span>Copy Spawn agents config</span>
      </button>
      <button type="button" onclick={toggleCollapsed}>
        {#if collapsed()}
          <ChevronUp size={15} strokeWidth={1.75} aria-hidden="true" />
          <span>Expand prompt</span>
        {:else}
          <ChevronDown size={15} strokeWidth={1.75} aria-hidden="true" />
          <span>Collapse prompt</span>
        {/if}
      </button>
      <button type="button" onclick={() => void setBubbleMode("stack")}>
        <Type size={15} strokeWidth={1.75} aria-hidden="true" />
        <span>Bubble stack</span>
      </button>
      <button type="button" onclick={() => void setBubbleMode("tray")}>
        <Type size={15} strokeWidth={1.75} aria-hidden="true" />
        <span>Bubble tray</span>
      </button>
    </div>
  {/if}
</div>

<style>
  /* `fullstack-a-24`: floating-pill redesign. Previously the prompt
     was a rectangle flush against the bottom edge (full-bleed
     left/right/bottom, square corners, border on the top edge only).
     @@Alex's spec: rounded corners on all sides, visible terminal
     underneath, inset margins on every edge. The border-radius reads
     as a chip rather than a header bar; the floating-shadow on all
     sides replaces the prior single top-edge shadow that hinted
     attached-to-bottom.

     Collapsed state (`.rich-prompt.collapsed`): clamps the prompt to
     min-height = the header row only; the composer-editor + watcher
     row + spawn-dialog block all hide. The user keeps the prompt
     attached (chevron expands) but reclaims vertical room for the
     bubbles above. */
  .rich-prompt {
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 12px;
    min-height: 150px;
    max-height: calc(100% - 48px);
    z-index: 20;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 14px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.32);
    overflow: hidden;
  }
  .rich-prompt.collapsed {
    min-height: 0;
    height: auto;
  }
  .rich-prompt.collapsed .workspace-row,
  .rich-prompt.collapsed .composer-editor {
    display: none;
  }
  .rich-prompt.collapsed header {
    border-bottom: 0;
  }
  .rich-prompt.collapsed .resize-handle {
    display: none;
  }
  .resize-handle {
    position: absolute;
    top: -13px;
    left: 50%;
    transform: translateX(-50%);
    width: 58px;
    height: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    border-radius: 4px 4px 0 0;
    background: var(--bg-card);
    color: var(--text-secondary);
    cursor: ns-resize;
  }
  header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 8px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-card);
    flex-shrink: 0;
    min-width: 0;
  }
  .spacer {
    flex: 1;
    min-width: 8px;
  }
  .icon-btn {
    width: 28px;
    height: 26px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--btn-bg);
    color: var(--text-secondary);
    cursor: pointer;
    flex-shrink: 0;
  }
  .icon-btn:hover {
    color: var(--text);
    border-color: var(--btn-hover);
  }
  .icon-btn:disabled {
    cursor: default;
    opacity: .55;
  }
  .event-pill {
    min-width: 28px;
    height: 26px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--text-secondary);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }
  .agent-picker {
    height: 26px;
    max-width: 92px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--btn-bg);
    color: var(--text);
    font: inherit;
    font-size: 12px;
    padding: 0 6px;
    flex-shrink: 0;
  }
  .agent-picker:disabled {
    opacity: .55;
  }
  .workspace-row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 28px;
    padding: 0 10px 7px;
    font-size: 12px;
    color: var(--text-secondary);
    background: var(--bg-card);
    border-bottom: 1px solid var(--border);
  }
  .workspace-row.error {
    color: var(--danger-text);
  }
  .workspace-row span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .workspace-row button {
    flex-shrink: 0;
    min-height: 24px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--btn-bg);
    color: var(--text);
  }
  .composer-editor {
    flex: 1;
    min-height: 0;
    display: flex;
    position: relative;
    --editor-top-pad: 16px;
  }
  /* `fullstack-a-89`: removed `.prompt-placeholder` CSS
     overlay (-a-24 / -a-84 / -a-87). CM6's `placeholder`
     extension threaded via `placeholderText` on Wysiwyg +
     Source now handles the empty-state hint. */
  /* `fullstack-a-89b`: empirical fix for the cursor/placeholder
     Y-misalignment that survived `-a-89`. Measured in browser
     devtools (cursor: top 717.5, bottom 736.5, height 19;
     placeholder default: top 713, bottom 741.8, height 28.8 →
     ~4.5px cursor-above-text-top delta).
     Root cause: CM6 sizes the cursor from the font's natural
     line-box (~1.2 × font-size = 19.2px for 16px font), but the
     placeholder span inherits `.cm-line`'s `line-height: 1.8`
     (28.8px) and vertically aligns to `top`, so its text
     glyphs sit lower than the cursor's bounding box.
     Fix: collapse the placeholder's box to match the cursor's
     natural line-box (`line-height: 1.2`) + center-align vertically
     so the placeholder text top aligns with the cursor top. The
     leading space character in `PROMPT_PLACEHOLDER_TEXT` adds the
     `{cursor}{space}{default-text}` gap @@Alex's literal spec
     calls for.
     Scope is the prompt area only; the `:global` chain pins to
     `.rich-prompt` (the root of this component) so the rule
     doesn't leak to other CM6 editors in the app. */
  :global(.rich-prompt .cm-placeholder) {
    line-height: 1.2;
    vertical-align: middle;
  }
  /* `fullstack-a-8`: easeOutBack bubble-pop matching every other
     right-click surface (HamburgerMenu, TerminalTab / GraphPanel
     tab-menu bubbles). Origin sits at top-left because the
     bubble is anchored at the cursor position via `style:left`
     / `style:top`. */
  .ctx {
    position: fixed;
    z-index: 26000;
    display: flex;
    flex-direction: column;
    min-width: 190px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
    transform-origin: top left;
    animation: ctx-pop 260ms cubic-bezier(0.34, 1.56, 0.64, 1);
  }
  @keyframes ctx-pop {
    0%   { opacity: 0; transform: scale(0.92); }
    100% { opacity: 1; transform: scale(1); }
  }
  @media (prefers-reduced-motion: reduce) {
    .ctx { animation: none; }
  }
  .ctx button {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: .4rem .6rem;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    cursor: pointer;
    text-align: left;
  }
  .ctx button:hover {
    background: var(--hover-bg);
  }
  /* `fullstack-a-30`: per-prompt page-width slider in the
     context menu. Same shape as the editor's tab-menu slider in
     FileEditorTab.svelte so both surfaces read alike. */
  .page-width-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--separator, var(--border));
  }
  .page-width-label {
    color: var(--text-secondary);
    font-size: 12px;
    min-width: 64px;
  }
  .page-width-slider {
    flex: 1;
    accent-color: var(--btn-hover);
  }
  .page-width-value {
    min-width: 40px;
    text-align: right;
    color: var(--text-secondary);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
</style>
