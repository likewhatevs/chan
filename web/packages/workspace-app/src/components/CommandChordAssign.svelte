<script lang="ts">
  // The launcher row's chord cell as an assign affordance. Shows the
  // command's resolved chord (user override or the SHORTCUTS baseline) or an
  // "Assign" prompt when it has none, and on click captures a new chord for
  // the current client's OS slot, checks it against the resolved keymap, and
  // either assigns it or reports the conflict. A cleared override falls back
  // to the built-in chord. Every command in the catalog is assignable,
  // chorded or not.
  //
  // Capture runs on a dedicated focusable element that swallows its keydowns,
  // so it never reaches the launcher's arrow/enter navigation or types into
  // the search input. Escape or a click away cancels.

  import { tick } from "svelte";
  import { X } from "lucide-svelte";
  import { SHORTCUTS } from "../state/shortcuts";
  import { captureChord, keymapConflicts } from "../state/keymapAssign";
  import { allCommands, type Command } from "../state/commands";
  import {
    assignOverride,
    clearOverride,
    currentSlot,
    formattedChordForSlot,
    overrideChordForSlot,
    resolvedKeymapEntriesForSlot,
    type OverrideSlot,
  } from "../state/keymapOverrides.svelte";

  let {
    cmd,
    slot = currentSlot(),
    onCaptureEnd,
  }: {
    cmd: Command;
    // Which OS slot to assign. Defaults to the current client's slot (the
    // launcher's quick-assign); the per-OS grid passes an explicit slot.
    slot?: OverrideSlot;
    // Called after capture commits or is cancelled by Escape, so the caller
    // can return focus. Not called on a click-away blur.
    onCaptureEnd?: () => void;
  } = $props();

  let capturing = $state(false);
  let conflictLabel = $state<string | null>(null);
  let captureEl = $state<HTMLButtonElement>();

  const chord = $derived(formattedChordForSlot(cmd.id, slot));
  const hasOverride = $derived(overrideChordForSlot(cmd.id, slot) !== undefined);

  // A conflicting binding's display name: catalog title, else the SHORTCUTS
  // label (editor / terminal chords are registry-only), else the raw id.
  function labelForId(id: string): string {
    const command = allCommands().find((c) => c.id === id);
    if (command) return command.title;
    return SHORTCUTS.find((s) => s.id === id)?.label ?? id;
  }

  async function startCapture(e: MouseEvent): Promise<void> {
    e.stopPropagation();
    conflictLabel = null;
    capturing = true;
    await tick();
    captureEl?.focus();
  }

  function endCapture(refocus: boolean): void {
    capturing = false;
    conflictLabel = null;
    if (refocus) onCaptureEnd?.();
  }

  function onCaptureKeydown(e: KeyboardEvent): void {
    e.stopPropagation();
    if (e.key === "Escape") {
      e.preventDefault();
      endCapture(true);
      return;
    }
    e.preventDefault();
    const candidate = captureChord(e);
    if (!candidate) return; // modifier-only or bare key: keep composing
    const conflicts = keymapConflicts(
      candidate,
      resolvedKeymapEntriesForSlot(allCommands(), slot),
      cmd.id,
    );
    if (conflicts.length > 0) {
      conflictLabel = labelForId(conflicts[0].id);
      return; // hold capture open so the user can pick a free chord
    }
    assignOverride(cmd.id, candidate, slot);
    endCapture(true);
  }

  function reset(e: MouseEvent): void {
    e.stopPropagation();
    clearOverride(cmd.id, slot);
  }
</script>

{#if capturing}
  <button
    class="capture"
    class:conflict={conflictLabel !== null}
    bind:this={captureEl}
    type="button"
    aria-label={`Press a shortcut for ${cmd.title}, or Escape to cancel`}
    onkeydown={onCaptureKeydown}
    onblur={() => endCapture(false)}
    onclick={(e) => e.stopPropagation()}
  >
    {conflictLabel !== null ? `In use by ${conflictLabel}` : "Press keys"}
  </button>
{:else}
  <span class="assign">
    <button
      class="chord-btn"
      class:empty={!chord}
      type="button"
      aria-label={chord
        ? `Change shortcut for ${cmd.title}`
        : `Assign a shortcut to ${cmd.title}`}
      onclick={startCapture}
    >
      {chord ?? "Assign"}
    </button>
    {#if hasOverride}
      <button
        class="reset"
        type="button"
        aria-label={`Reset ${cmd.title} to its default shortcut`}
        onclick={reset}
      >
        <X size={13} strokeWidth={2} aria-hidden="true" />
      </button>
    {/if}
  </span>
{/if}

<style>
  .assign {
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .chord-btn {
    padding: 1px 7px;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--code-bg);
    color: var(--text-secondary);
    font-size: 11px;
    font-family: inherit;
    white-space: nowrap;
    cursor: pointer;
  }
  .chord-btn:hover {
    border-color: color-mix(in srgb, var(--text) 34%, transparent);
    color: var(--text);
  }
  .chord-btn.empty {
    background: transparent;
    border-style: dashed;
    opacity: 0.75;
  }
  .reset {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 2px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .reset:hover {
    background: color-mix(in srgb, var(--text) 12%, transparent);
    color: var(--text);
  }
  .capture {
    padding: 1px 8px;
    border: 1px solid color-mix(in srgb, var(--accent, var(--text)) 60%, transparent);
    border-radius: 5px;
    background: color-mix(in srgb, var(--accent, var(--text)) 12%, transparent);
    color: var(--text);
    font-size: 11px;
    font-family: inherit;
    white-space: nowrap;
    cursor: pointer;
  }
  .capture.conflict {
    border-color: color-mix(in srgb, var(--danger, #d9534f) 70%, transparent);
    background: color-mix(in srgb, var(--danger, #d9534f) 14%, transparent);
    color: var(--text);
  }
</style>
