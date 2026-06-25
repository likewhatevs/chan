<script lang="ts">
  // The devserver control-terminal-closed survey modal. A connected devserver's
  // control terminal (the terminal running its connect command) exited, so the
  // connection is dead: offer Re-run (reconnect), Edit (change the connect
  // command first), or Abandon (drop it). Built on the shared in-SPA Modal
  // (WKWebView blocks native dialogs); mounted in App.svelte while open.
  // Desktop-only — the driving `devserver-control-closed` event never fires in a
  // plain browser, so this never appears on the served surfaces.
  import Modal from "./Modal.svelte";
  import {
    controlClosed,
    rerunControlClosed,
    editControlClosed,
    abandonControlClosed,
    dismissControlClosed,
  } from "../state/controlClosed.svelte";
</script>

<Modal title={`${controlClosed.name} disconnected`} onclose={dismissControlClosed}>
  <p class="message">
    The control terminal's connect command stopped, so this devserver is no longer
    reachable. Re-run the command to reconnect, edit it first, or abandon the devserver.
  </p>
  <div class="dialog-footer">
    <button
      class="btn danger"
      type="button"
      disabled={controlClosed.busy}
      onclick={abandonControlClosed}>Abandon</button>
    <button
      class="btn"
      type="button"
      disabled={controlClosed.busy}
      onclick={editControlClosed}>Edit</button>
    <button
      class="btn primary"
      type="button"
      disabled={controlClosed.busy}
      onclick={rerunControlClosed}>Re-run</button>
  </div>
</Modal>

<style>
  .message {
    margin: 0;
    color: var(--text);
    font-size: 0.95rem;
    line-height: 1.5;
  }

  .dialog-footer {
    display: flex;
    justify-content: flex-end;
    gap: 0.6rem;
    margin-top: 1.5rem;
  }

  /* Abandon is the destructive choice, reddened away from the default Re-run on
     the far side of the row (the launcher has no `.btn.danger` base; only the
     icon buttons carry `.danger`). */
  .btn.danger {
    border-color: var(--danger);
    color: var(--danger);
  }

  .btn.danger:hover:not(:disabled) {
    background: color-mix(in srgb, var(--danger) 14%, transparent);
  }
</style>
