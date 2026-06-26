<script lang="ts">
  // Full-app overlay surfaced when the watcher channel drops. The
  // watcher is the only push channel between the server and the UI;
  // without it, file changes from outside the editor stop
  // propagating, so silently letting the user keep typing risks
  // divergence between the buffer and disk.
  //
  // Replaces the previous toolbar WS pill, which named the problem
  // but offered nothing actionable. The overlay greys out the entire
  // UI and gives the user one button: retry now (skip the auto-
  // reconnect backoff). The auto-reconnect still runs underneath, so
  // doing nothing eventually heals on its own.

  import { reconnectWatcher, ui } from "../state/store.svelte";
  import { windowLibraryId } from "../api/client";
  import { isTauriDesktop, abandonDevserverForWindow } from "../api/desktop";

  // Abandon is offered only on a devserver-backed desktop window: a stuck remote
  // connection the user can give up on. windowLibraryId() is "local" for the
  // local library; isTauriDesktop() gates out the plain browser (which has no IPC
  // and whose tab the user closes themselves). Both read once -- not reactive.
  const canAbandon = isTauriDesktop() && windowLibraryId() !== "local";
  let abandonBtn: HTMLButtonElement | null = $state(null);

  // Abandon: ask the desktop to disconnect this window's devserver. The window
  // closes async via the watcher; best-effort, so a failed/inert IPC just leaves
  // the overlay (the user can still Retry or wait for auto-reconnect).
  function abandon(): void {
    void abandonDevserverForWindow();
  }

  /// Show the overlay only AFTER the watcher channel has been open
  /// at least once during this session. The "connecting" state at
  /// app boot is unbounded in length: on slow networks it can take
  /// several seconds. Blocking the UI during cold boot would make
  /// the app appear unresponsive ("nothing clicks") with no useful
  /// signal to the user.
  ///
  /// Once we've seen "open" once, any later transition to a
  /// non-open state is a real disconnect that's worth surfacing -
  /// file changes won't propagate, autosave can't reach the server,
  /// etc. A 600 ms grace still hides the overlay through brief
  /// reconnects.
  ///
  /// Done with an $effect that owns the timer rather than a $derived
  /// computing on Date.now: $derived must be pure, and recording
  /// state transitions is a side effect.
  import { onDestroy } from "svelte";

  const STARTUP_GRACE_MS = 600;
  let visible = $state(false);
  let hasBeenOpen = $state(false);
  let retryBtn: HTMLButtonElement | null = $state(null);

  // Mirror `visible` into the shared store so App.svelte's document-level
  // key handlers can suppress pane/tab shortcuts while the overlay blocks
  // the UI (the backdrop stops clicks, but not keystrokes). Reset on teardown
  // so a stale `true` can't outlive the overlay.
  $effect(() => {
    ui.disconnectBlocking = visible;
  });
  onDestroy(() => {
    ui.disconnectBlocking = false;
  });

  $effect(() => {
    if (ui.ws === "open") {
      hasBeenOpen = true;
      visible = false;
      return;
    }
    if (!hasBeenOpen) {
      // Cold boot still in progress. Stay invisible so the user
      // can interact with the app while the watcher catches up.
      visible = false;
      return;
    }
    const t = setTimeout(() => {
      visible = true;
    }, STARTUP_GRACE_MS);
    return () => clearTimeout(t);
  });

  /// Steal focus when the overlay appears. The backdrop's
  /// pointer-events stop *clicks* from reaching the editor, but
  /// keystrokes still flow to whatever was focused before the
  /// disconnect (typically the WYSIWYG / CodeMirror surface), so a
  /// user mid-edit could keep typing into a buffer the watcher
  /// can't observe. Moving focus to the Retry button parks input
  /// somewhere harmless until the channel comes back. Paired with
  /// the keydown trap below, Tab can't leak focus back to the
  /// background.
  $effect(() => {
    if (!visible) return;
    const active = document.activeElement as HTMLElement | null;
    active?.blur();
    queueMicrotask(() => retryBtn?.focus());
  });

  function trapTab(e: KeyboardEvent): void {
    // Keep focus on the dialog's buttons (Retry, plus Abandon when offered):
    // Tab/Shift+Tab cycles between them and never leaks to the blocked UI behind.
    if (e.key !== "Tab") return;
    e.preventDefault();
    const focusables = [retryBtn, abandonBtn].filter(
      (b): b is HTMLButtonElement => b !== null,
    );
    if (focusables.length === 0) return;
    const here = focusables.indexOf(document.activeElement as HTMLButtonElement);
    const step = e.shiftKey ? -1 : 1;
    const next = (here + step + focusables.length) % focusables.length;
    focusables[next < 0 ? 0 : next]!.focus();
  }

  const message = $derived.by(() => {
    switch (ui.ws) {
      case "connecting":
        return "connecting to the chan server";
      case "reconnecting":
        return "reconnecting to the chan server";
      case "closed":
        return "disconnected from the chan server";
      default:
        return "";
    }
  });

  const subline = $derived.by(() => {
    if (ui.ws === "closed") {
      return "the server may have stopped; check the terminal where you ran `chan open`";
    }
    return "this usually clears on its own; press Retry to skip the wait";
  });
</script>

{#if visible}
  <div
    class="overlay"
    role="alertdialog"
    aria-modal="true"
    aria-live="assertive"
    aria-label={message}
    tabindex="-1"
    onkeydown={trapTab}
  >
    <div class="card">
      <div class="title">{message}</div>
      <div class="subline">{subline}</div>
      <div class="actions">
        <button class="retry" bind:this={retryBtn} onclick={reconnectWatcher}>
          Retry now
        </button>
        {#if canAbandon}
          <button class="abandon" bind:this={abandonBtn} onclick={abandon}>
            Abandon
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  /* Cover the entire viewport with a semi-opaque backdrop so the
     UI underneath visibly greys out. Pointer-events on so clicks
     don't reach controls behind. */
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 30000;
    backdrop-filter: blur(2px);
  }
  .card {
    background: var(--bg-elev);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 14px 44px rgba(0, 0, 0, 0.5);
    padding: 18px 22px;
    max-width: 420px;
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .title {
    font-size: 16px;
    font-weight: 600;
  }
  .subline {
    font-size: 14px;
    color: var(--text-secondary);
    line-height: 1.4;
  }
  .actions {
    display: flex;
    gap: 10px;
    justify-content: center;
  }
  .retry {
    background: var(--link);
    color: #fff;
    border: 1px solid var(--link);
    border-radius: 4px;
    padding: 6px 14px;
    font: inherit;
    cursor: pointer;
  }
  .retry:hover { filter: brightness(1.1); }
  /* Abandon is the destructive escape hatch: muted until hover, then danger. */
  .abandon {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 6px 14px;
    font: inherit;
    cursor: pointer;
  }
  .abandon:hover {
    border-color: var(--danger);
    color: var(--danger);
  }
</style>
