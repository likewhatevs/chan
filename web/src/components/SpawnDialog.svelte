<script lang="ts">
  import { Bot, Loader2, X } from "lucide-svelte";
  import { api } from "../api/client";
  import { closeSpawnDialog, spawnDialogState } from "../state/spawnDialog.svelte";

  // `fullstack-a-4`: SpawnDialog is now state-driven. It mounts
  // once at the App root and renders whenever
  // `spawnDialogState.request` is non-null. Callers (rich
  // prompt's "Spawn agent" button, etc.) open it via
  // `openSpawnDialog({ ...props })`. This keeps the dialog out
  // of every ancestor stacking context that used to clip its
  // `position: fixed` backdrop in practice.
  const request = $derived(spawnDialogState.request);
  const open = $derived(request !== null);
  const defaultName = $derived(request?.defaultName ?? "@@Agent");
  const defaultCommand = $derived(request?.defaultCommand ?? "");
  const orchestratorSessionId = $derived(request?.orchestratorSessionId);

  let name = $state("");
  let command = $state("");
  let envText = $state("");
  let error = $state("");
  let busy = $state(false);
  let nameInput: HTMLInputElement | undefined = $state();

  $effect(() => {
    if (!open) return;
    name = defaultName;
    command = defaultCommand;
    envText = "";
    error = "";
    busy = false;
    queueMicrotask(() => nameInput?.focus());
  });

  const canSubmit = $derived(name.trim() !== "" && command.trim() !== "" && !busy);

  function parseEnv(): Record<string, string> | undefined {
    const env: Record<string, string> = {};
    for (const [idx, raw] of envText.split(/\r?\n/).entries()) {
      const line = raw.trim();
      if (!line) continue;
      const eq = line.indexOf("=");
      if (eq <= 0) {
        throw new Error(`env line ${idx + 1} must be KEY=value`);
      }
      const key = line.slice(0, eq).trim();
      if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(key)) {
        throw new Error(`env line ${idx + 1} has an invalid key`);
      }
      env[key] = line.slice(eq + 1);
    }
    return Object.keys(env).length ? env : undefined;
  }

  async function submit(): Promise<void> {
    if (!canSubmit || !request) return;
    error = "";
    let env: Record<string, string> | undefined;
    try {
      env = parseEnv();
    } catch (err) {
      error = (err as Error).message;
      return;
    }
    busy = true;
    try {
      const response = await api.spawnTerminal({
        name: name.trim(),
        command: command.trim(),
        ...(env ? { env } : {}),
        ...(orchestratorSessionId ? { orchestrator_session: orchestratorSessionId } : {}),
      });
      request.onSpawned(response, name.trim());
      closeSpawnDialog();
    } catch (err) {
      error = `spawn failed: ${(err as Error).message}`;
    } finally {
      busy = false;
    }
  }

  function close(): void {
    if (busy) return;
    closeSpawnDialog();
  }

  function onKey(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    } else if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      void submit();
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="spawn-backdrop" onclick={close}>
    <!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role -->
    <section
      class="spawn-dialog"
      role="dialog"
      aria-label="spawn agent"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={onKey}
    >
      <header>
        <span class="title">
          <Bot size={16} strokeWidth={1.8} aria-hidden="true" />
          <span>Spawn agent</span>
        </span>
        <button type="button" class="icon" onclick={close} aria-label="Close spawn dialog" title="Close" disabled={busy}>
          <X size={15} strokeWidth={1.8} aria-hidden="true" />
        </button>
      </header>

      <label>
        <span>Tab name</span>
        <input bind:this={nameInput} bind:value={name} spellcheck="false" autocomplete="off" />
      </label>
      <label>
        <span>Command</span>
        <textarea bind:value={command} rows="3" spellcheck="false"></textarea>
      </label>
      <label>
        <span>Env</span>
        <textarea bind:value={envText} rows="4" spellcheck="false" placeholder="KEY=value"></textarea>
      </label>

      {#if error}
        <div class="error">{error}</div>
      {/if}

      <footer>
        <button type="button" onclick={close} disabled={busy}>Cancel</button>
        <button type="button" class="primary" onclick={() => void submit()} disabled={!canSubmit}>
          {#if busy}
            <span class="spin">
              <Loader2 size={14} strokeWidth={1.8} aria-hidden="true" />
            </span>
          {/if}
          <span>Spawn</span>
        </button>
      </footer>
    </section>
  </div>
{/if}

<style>
  .spawn-backdrop {
    position: fixed;
    inset: 0;
    z-index: 26100;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
    background: rgba(0, 0, 0, 0.38);
  }
  .spawn-dialog {
    width: min(520px, 100%);
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-elev);
    color: var(--text);
    box-shadow: 0 16px 36px rgba(0, 0, 0, 0.36);
  }
  header,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  .title {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 14px;
    color: var(--text);
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    color: var(--text-secondary);
  }
  input,
  textarea {
    width: 100%;
    box-sizing: border-box;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--text);
    font: 13px/1.35 ui-monospace, SFMono-Regular, Menlo, monospace;
    padding: 7px 8px;
    resize: vertical;
  }
  input:focus,
  textarea:focus {
    outline: none;
    border-color: var(--link);
  }
  .icon,
  footer button {
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--btn-bg);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }
  .icon {
    width: 26px;
    height: 24px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }
  footer {
    justify-content: flex-end;
  }
  footer button {
    min-height: 28px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 0 10px;
  }
  button:disabled {
    cursor: default;
    opacity: 0.55;
  }
  .primary {
    background: var(--link);
    border-color: var(--link);
    color: #fff;
  }
  .error {
    color: var(--danger-text);
    font-size: 12px;
  }
  .spin {
    animation: spin 900ms linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  @media (prefers-reduced-motion: reduce) {
    .spin { animation: none; }
  }
</style>
