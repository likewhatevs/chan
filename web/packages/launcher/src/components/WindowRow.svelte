<script lang="ts">
  // One window/terminal row in the machine tree (control terminals, standalone
  // terminals, and the windows nested in a workspace card). The mutable surface
  // carries two icon actions: [FOCUS] (openWindow: focus a live window / un-hide
  // a buried one) and [SHOW/HIDE] (toggleWindow: Eye when visible, EyeOff when
  // hidden, keyed on the server-persisted `hidden`). A control terminal whose
  // inner process exited slow-flashes its eye yellow for attention; the user
  // acting on the window clears it. The read-only surface (gateway/devserver, no
  // desktop bridge) renders the row static with the connection dot and no actions.
  //
  // `icon` adds a leading kind glyph (accent for the control terminal) and, for a
  // control terminal awaiting attention, an amber "disconnected..." pill; the
  // machine tree passes it for every row.
  import { AppWindow, Eye, EyeOff, Focus, SquareTerminal } from "lucide-svelte";
  import { focusWindow, toggleWindow, reportError, clearError } from "../state/library.svelte";
  import { windowRowLabel } from "../lib/windowLabel";
  import { hasControlAttention, clearControlAttention } from "../state/controlAttention.svelte";
  import { readOnly } from "../state/capabilities";
  import type { WindowRecord } from "../api/library";

  interface Props {
    w: WindowRecord;
    icon?: boolean;
  }
  let { w, icon = false }: Props = $props();

  // A control terminal whose inner process exited flags its devserver's library
  // for attention; its eye slow-flashes yellow until the user acts on the window.
  function needsAttention(rec: WindowRecord): boolean {
    return rec.control && hasControlAttention(rec.library_id);
  }

  // The user acting on the window (focus or show/hide) acknowledges the
  // attention -- clear the flash.
  function acknowledge(rec: WindowRecord): void {
    if (rec.control) clearControlAttention(rec.library_id);
  }

  // The FOCUS / SHOW-HIDE bridge ops reject on a surface with no desktop and on a
  // stale/reaped window. Catch here and surface the failure in the banner instead
  // of letting a floating promise reject into the console.
  async function run(rec: WindowRecord, action: Promise<void>): Promise<void> {
    clearError();
    try {
      await action;
      acknowledge(rec);
    } catch (e) {
      reportError(e);
    }
  }
</script>

{#if readOnly}
  <!-- Read-only surface (gateway/devserver): the dot shows the connection state
       but can't drive a native window. -->
  <div class="row">
    {#if icon}
      <span class="row-glyph" class:control={w.control} aria-hidden="true">
        {#if w.kind === "workspace"}<AppWindow size={15} />{:else}<SquareTerminal size={15} />{/if}
      </span>
    {/if}
    <div class="row-main">
      <span class="row-name">{windowRowLabel(w)}</span>
    </div>
    <span class="dot" class:live={w.connected} title={w.connected ? "Connected" : "Detached"}></span>
  </div>
{:else}
  <div class="row">
    {#if icon}
      <span class="row-glyph" class:control={w.control} aria-hidden="true">
        {#if w.kind === "workspace"}<AppWindow size={15} />{:else}<SquareTerminal size={15} />{/if}
      </span>
    {/if}
    <div class="row-main">
      <span class="row-name">
        {windowRowLabel(w)}
        {#if icon && needsAttention(w)}
          <span class="disconnected">disconnected...</span>
        {/if}
      </span>
    </div>
    <div class="row-actions">
      <button
        class="icon-btn"
        type="button"
        title="Focus window"
        aria-label="Focus window"
        onclick={() => {
          run(w, focusWindow(w));
        }}>
        <Focus size={16} />
      </button>
      <button
        class="icon-btn"
        class:on={!w.hidden}
        class:attention={needsAttention(w)}
        type="button"
        title={needsAttention(w)
          ? "Control terminal exited -- show window"
          : w.hidden
            ? "Show window"
            : "Hide window"}
        aria-label={needsAttention(w)
          ? `${w.hidden ? "Show window" : "Hide window"} (needs attention)`
          : w.hidden
            ? "Show window"
            : "Hide window"}
        onclick={() => {
          run(w, toggleWindow(w));
        }}>
        {#if w.hidden}<EyeOff size={16} />{:else}<Eye size={16} />{/if}
      </button>
    </div>
  </div>
{/if}

<style>
  /* The leading kind glyph (nested tree); the control terminal reads accent. */
  .row-glyph {
    display: inline-flex;
    align-items: center;
    flex-shrink: 0;
    color: var(--text-secondary);
  }

  .row-glyph.control {
    color: var(--accent);
  }

  /* The control terminal's "disconnected..." cue: amber, flashing beside the name
     while the devserver awaits attention (additive to the eye flash). */
  .disconnected {
    display: inline-flex;
    align-items: center;
    font-size: 0.72rem;
    font-weight: 500;
    color: #e3b341;
    animation: control-disconnected 1.6s ease-in-out infinite;
  }

  @keyframes control-disconnected {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.35;
    }
  }

  /* A control terminal whose inner process exited: its eye slow-flashes yellow to
     request attention. The pulse overrides the .on accent tint while it runs; the
     user acting on the window clears it. */
  .icon-btn.attention {
    animation: control-attention 1.6s ease-in-out infinite;
  }

  @keyframes control-attention {
    0%,
    100% {
      border-color: var(--btn-border);
      color: var(--text-secondary);
      background: var(--btn-bg);
    }
    50% {
      border-color: #e3b341;
      color: #e3b341;
      background: color-mix(in srgb, #e3b341 18%, transparent);
    }
  }

  /* Respect reduced-motion: hold a steady yellow instead of pulsing. */
  @media (prefers-reduced-motion: reduce) {
    .disconnected {
      animation: none;
    }
    .icon-btn.attention {
      animation: none;
      border-color: #e3b341;
      color: #e3b341;
      background: color-mix(in srgb, #e3b341 18%, transparent);
    }
  }
</style>
