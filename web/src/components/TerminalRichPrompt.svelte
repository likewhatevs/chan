<script lang="ts">
  import { FilePlus, GripHorizontal, Send, X } from "lucide-svelte";
  import Source from "../editor/Source.svelte";
  import Wysiwyg from "../editor/Wysiwyg.svelte";
  import StyleToolbar from "./StyleToolbar.svelte";
  import type { TerminalRichPromptState } from "../state/tabs.svelte";
  import { api } from "../api/client";
  import { appendDefaultMd } from "../state/pathValidate";
  import { isEditableText } from "../state/fileTypes";
  import {
    refreshTree,
    ui,
    uiPathPrompt,
  } from "../state/store.svelte";
  import { openInActivePane } from "../state/tabs.svelte";

  let {
    prompt,
    onSubmit,
    onClose,
  }: {
    prompt: TerminalRichPromptState;
    onSubmit: (source: string) => void;
    onClose: () => void;
  } = $props();

  const MIN_HEIGHT = 150;
  const TOP_GAP = 36;
  let rootEl: HTMLDivElement | undefined = $state();
  let wysiwygRef: Wysiwyg | undefined = $state();
  let selVer = $state(0);
  let menu = $state<{ x: number; y: number } | null>(null);
  let dragging = false;

  function mode(): "wysiwyg" | "source" {
    return prompt.mode ?? "wysiwyg";
  }

  function setMode(next: "wysiwyg" | "source"): void {
    prompt.mode = next;
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
    const target = e.target as HTMLElement | null;
    e.stopPropagation();
    if (target?.closest(".composer-editor")) return;
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
    <StyleToolbar
      wysiwyg={wysiwygRef}
      {selVer}
      disabled={mode() === "source"}
      floating={false}
    />
    <div class="spacer"></div>
    <button type="button" class="icon-btn" onclick={newFileFromHere} title="New file from here" aria-label="New file from here">
      <FilePlus size={16} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <button type="button" class="icon-btn" onclick={submit} title="Send prompt" aria-label="Send prompt">
      <Send size={16} strokeWidth={1.75} aria-hidden="true" />
    </button>
    <button type="button" class="icon-btn" onclick={onClose} title="Close" aria-label="Close">
      <X size={16} strokeWidth={1.75} aria-hidden="true" />
    </button>
  </header>
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
      <button type="button" onclick={newFileFromHere}>
        <FilePlus size={15} strokeWidth={1.75} aria-hidden="true" />
        <span>New File from here</span>
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
