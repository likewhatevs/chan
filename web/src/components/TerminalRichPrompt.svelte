<script lang="ts">
  import { Code2, FilePlus, FolderSearch, GripHorizontal, Pilcrow, Send, Type, X } from "lucide-svelte";
  import Source from "../editor/Source.svelte";
  import Wysiwyg from "../editor/Wysiwyg.svelte";
  import StyleToolbar from "./StyleToolbar.svelte";
  import type { TerminalRichPromptState } from "../state/tabs.svelte";
  import { api } from "../api/client";
  import { appendDefaultMd } from "../state/pathValidate";
  import { isEditableText } from "../state/fileTypes";
  import {
    drive,
    refreshTree,
    ui,
    uiPathPrompt,
  } from "../state/store.svelte";
  import { openInActivePane } from "../state/tabs.svelte";

  let {
    prompt,
    onSubmit,
    onClose,
    terminalSessionId,
    watcherPath,
    onWatcherStarted,
    onWatcherStopped,
  }: {
    prompt: TerminalRichPromptState;
    onSubmit: (source: string) => void;
    onClose: () => void;
    terminalSessionId?: string;
    watcherPath?: string | null;
    onWatcherStarted?: (path: string) => void;
    onWatcherStopped?: () => void;
  } = $props();

  const MIN_HEIGHT = 150;
  const TOP_GAP = 36;
  let rootEl: HTMLDivElement | undefined = $state();
  let wysiwygRef: Wysiwyg | undefined = $state();
  let selVer = $state(0);
  let menu = $state<{ x: number; y: number } | null>(null);
  let watcherError = $state("");
  let watcherBusy = $state(false);
  let dragging = false;

  function mode(): "wysiwyg" | "source" {
    return prompt.mode ?? "wysiwyg";
  }

  function setMode(next: "wysiwyg" | "source"): void {
    prompt.mode = next;
  }

  function toolbarOpen(): boolean {
    return prompt.styleToolbarOpen !== false;
  }

  function toggleMode(): void {
    prompt.mode = mode() === "source" ? "wysiwyg" : "source";
    menu = null;
  }

  function toggleToolbar(): void {
    prompt.styleToolbarOpen = !toolbarOpen();
    menu = null;
  }

  function submit(): void {
    onSubmit(prompt.buffer);
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
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

  function onWindowPointerDown(e: PointerEvent): void {
    if (!menu) return;
    const target = e.target as HTMLElement | null;
    if (target?.closest(".ctx")) return;
    menu = null;
  }

  async function newFileFromHere(): Promise<void> {
    menu = null;
    const path = await uiPathPrompt({
      title: "new file from prompt",
      defaultValue: "prompt.md",
      kind: "file",
      mode: "create",
      validate: (value) =>
        isEditableText(value)
          ? null
          : `'${value}' is not an editable text file (only .md and .txt)`,
    });
    if (!path) return;
    const target = appendDefaultMd(path);
    try {
      await api.create(target, false, prompt.buffer);
      await refreshTree();
      await openInActivePane(target);
      ui.status = `Created ${target}`;
    } catch (err) {
      ui.status = `create failed: ${(err as Error).message}`;
    }
  }

  async function watchDirectory(): Promise<void> {
    menu = null;
    watcherError = "";
    if (!terminalSessionId) {
      watcherError = "terminal session is not ready";
      return;
    }
    const path = await uiPathPrompt({
      title: "watch directory",
      defaultValue: watcherPath ?? "",
      kind: "folder",
      mode: "move",
      allowAbsolute: true,
    });
    if (!path) return;
    watcherBusy = true;
    try {
      await api.setTerminalWatcher(terminalSessionId, path);
      onWatcherStarted?.(path);
    } catch (err) {
      watcherError = `watch failed: ${(err as Error).message}`;
    } finally {
      watcherBusy = false;
    }
  }

  async function stopWatching(): Promise<void> {
    menu = null;
    watcherError = "";
    if (!terminalSessionId) {
      onWatcherStopped?.();
      return;
    }
    watcherBusy = true;
    try {
      await api.clearTerminalWatcher(terminalSessionId);
      onWatcherStopped?.();
    } catch (err) {
      if (/409|404|watcher|not found|not attached|conflict/i.test((err as Error).message || "")) {
        onWatcherStopped?.();
        ui.status = "watcher detached on reload";
        return;
      }
      watcherError = `stop failed: ${(err as Error).message}`;
    } finally {
      watcherBusy = false;
    }
  }

  async function setBubbleMode(mode: "stack" | "tray"): Promise<void> {
    menu = null;
    if (drive.info) {
      drive.info = {
        ...drive.info,
        preferences: { ...drive.info.preferences, bubble_overlay_mode: mode },
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
  bind:this={rootEl}
  style:height={`${prompt.heightPx ?? 320}px`}
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
    <button type="button" class="icon-btn" onclick={newFileFromHere} title="New file from here" aria-label="New file from here">
      <FilePlus size={16} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <button
      type="button"
      class="icon-btn"
      class:on={Boolean(watcherPath)}
      onclick={watchDirectory}
      disabled={watcherBusy}
      title="Watch directory"
      aria-label="Watch directory"
    >
      <FolderSearch size={16} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <button type="button" class="icon-btn" onclick={submit} title="Send prompt" aria-label="Send prompt">
      <Send size={16} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <button type="button" class="icon-btn" onclick={onClose} title="Close" aria-label="Close">
      <X size={16} strokeWidth={1.75} aria-hidden="true" />
    </button>
  </header>
  {#if watcherPath || watcherError}
    <div class="watcher-row" class:error={Boolean(watcherError)}>
      <span>{watcherError || `watching ${watcherPath}`}</span>
      {#if watcherPath}
        <button type="button" onclick={stopWatching} disabled={watcherBusy}>Stop watching</button>
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
          onSelectionChange={() => (selVer += 1)}
        />
      {:else}
        <Source
          bind:value={prompt.buffer}
          path="prompt.md"
          syntaxHighlight
        />
      {/if}
    {/key}
  </div>
  {#if menu}
    <div class="ctx" style:left={`${menu.x}px`} style:top={`${menu.y}px`}>
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
      <button type="button" onclick={newFileFromHere}>
        <FilePlus size={15} strokeWidth={1.75} aria-hidden="true" />
        <span>New File from here</span>
      </button>
      <button type="button" onclick={watchDirectory}>
        <FolderSearch size={15} strokeWidth={1.75} aria-hidden="true" />
        <span>Watch directory</span>
      </button>
      {#if watcherPath}
        <button type="button" onclick={stopWatching}>
          <FolderSearch size={15} strokeWidth={1.75} aria-hidden="true" />
          <span>Stop watching</span>
        </button>
      {/if}
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
  .rich-prompt {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    min-height: 150px;
    max-height: calc(100% - 36px);
    z-index: 20;
    display: flex;
    flex-direction: column;
    background: var(--bg);
    color: var(--text);
    border-top: 1px solid var(--border);
    box-shadow: 0 -10px 24px rgba(0, 0, 0, 0.32);
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
  .icon-btn.on {
    color: var(--link);
    border-color: var(--link);
  }
  .icon-btn:disabled {
    cursor: default;
    opacity: .55;
  }
  .watcher-row {
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
  .watcher-row.error {
    color: var(--danger-text);
  }
  .watcher-row span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .watcher-row button {
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
    --editor-top-pad: 16px;
  }
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
</style>
