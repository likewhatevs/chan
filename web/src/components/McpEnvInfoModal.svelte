<script lang="ts">
  // `fullstack-a-67d` slice 2: MCP env info dialog. Replaces the
  // inline toggle popover that the slice-1 menu inherited from
  // pre-`-a-67d` per addendum-a's "the info button should bring
  // up a dialog like the New File one" framing. Same width as
  // PathPromptModal (min-width 420px); modal-with-backdrop
  // pattern matches PathPromptModal / ConfirmModal.
  //
  // The dialog hosts the explanation + a single CTA: "Show MCP
  // env in terminal". The CTA fires `onShowInTerminal` and
  // closes the dialog; the disabled state piggybacks on the
  // outer `disabled` prop so the parent (TerminalTab.svelte)
  // can gate it on `sessionMcpEnv === false` (the existing
  // `showMcpEnvDisabled` derived).

  import { X } from "lucide-svelte";

  let {
    open,
    onClose,
    onShowInTerminal,
    showInTerminalDisabled,
  }: {
    open: boolean;
    onClose: () => void;
    onShowInTerminal: () => void;
    showInTerminalDisabled: boolean;
  } = $props();

  function onBackdropClick(e: MouseEvent): void {
    if (e.target === e.currentTarget) onClose();
  }

  function onKey(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }

  function commitShow(): void {
    onShowInTerminal();
    onClose();
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="mcp-modal-overlay"
    onclick={onBackdropClick}
    onkeydown={onKey}
    role="presentation"
  >
    <div class="mcp-modal" role="dialog" aria-modal="true" aria-label="MCP env vars">
      <div class="mcp-modal-head">
        <h2 class="mcp-modal-title">MCP env vars</h2>
        <button
          type="button"
          class="mcp-modal-close"
          onclick={onClose}
          aria-label="Close"
        >
          <X size={16} strokeWidth={1.75} aria-hidden="true" />
        </button>
      </div>
      <p class="mcp-modal-body">
        When on, chan sets <code>CHAN_MCP_SOCKET</code>,
        <code>CHAN_MCP_SERVER_JSON</code>, and friends in the PTY
        env so external agent CLIs can discover the chan MCP
        server automatically. Turn this off to launch a vanilla
        shell. Applies to new sessions only.
      </p>
      <div class="mcp-modal-actions">
        <button
          type="button"
          class="mcp-modal-cta"
          disabled={showInTerminalDisabled}
          onclick={commitShow}
        >
          Show MCP env in terminal
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .mcp-modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 26000;
  }
  /* `fullstack-a-67d` slice 2: width-match with PathPromptModal
     ("dialog like the New File one"). */
  .mcp-modal {
    background: var(--bg-elev);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4);
    padding: 1rem;
    min-width: 420px;
    max-width: 80vw;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .mcp-modal-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .mcp-modal-title {
    margin: 0;
    font-size: 15px;
    color: var(--text);
  }
  .mcp-modal-close {
    background: none;
    border: 0;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 4px;
    border-radius: 4px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
  .mcp-modal-close:hover {
    color: var(--text);
    background: var(--hover-bg);
  }
  .mcp-modal-body {
    margin: 0;
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.55;
  }
  .mcp-modal-body code {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 12px;
    background: var(--bg);
    padding: 1px 4px;
    border-radius: 3px;
    color: var(--text);
  }
  .mcp-modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
  .mcp-modal-cta {
    background: var(--accent);
    color: var(--bg);
    border: 1px solid var(--accent);
    border-radius: 4px;
    padding: 6px 14px;
    font: inherit;
    cursor: pointer;
  }
  .mcp-modal-cta:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
