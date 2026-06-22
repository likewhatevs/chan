<script lang="ts">
  // The New / Edit dialog. Two choices: a local directory or a devserver
  // (the old "Remote" choice is gone). The devserver body doubles as the
  // edit form, prefilled from the registry entry; one form does Add (POST)
  // and Save changes (PUT). It carries the token field (a proxied/gateway
  // devserver connects without scraping) masked as a password, write-only:
  // on edit, a blank token keeps the stored one. The component mounts fresh
  // each time the dialog opens, so its fields seed from the edit target and
  // need no manual reset.
  import Modal from "./Modal.svelte";
  import { closeDialog, dialog, selectChoice, type DialogChoice } from "../state/dialog.svelte";
  import { addLocalWorkspace, saveDevserver } from "../state/library.svelte";

  const editing = dialog.editing;

  let error = $state<string | null>(null);
  let submitting = $state(false);

  // Local-directory form. In a plain browser this is a path text input; the
  // desktop embed swaps in a native folder picker (both POST the same path).
  let localPath = $state("");

  // Devserver form, seeded from the edit target.
  let url = $state(editing?.url ?? "");
  let name = $state(editing?.label ?? "");
  let script = $state(editing?.script ?? "");
  let token = $state("");

  const title = $derived(editing ? "Edit devserver" : "New workspace");
  const showChoices = $derived(!editing);
  const showLocal = $derived(dialog.choice === "local" && !editing);

  function choose(c: DialogChoice): void {
    selectChoice(c);
    error = null;
  }

  function msg(e: unknown): string {
    return e instanceof Error ? e.message : String(e);
  }

  function onFieldKey(e: KeyboardEvent, submit: () => void): void {
    if (e.key === "Enter") {
      e.preventDefault();
      submit();
    }
  }

  async function submitLocal(): Promise<void> {
    const path = localPath.trim();
    if (!path) {
      error = "A folder path is required.";
      return;
    }
    submitting = true;
    try {
      await addLocalWorkspace(path);
      closeDialog();
    } catch (e) {
      error = msg(e);
    } finally {
      submitting = false;
    }
  }

  // A devserver URL must carry an explicit scheme://host form: a bare
  // `host:port` is rejected as ambiguous (`new URL` would read the host as the
  // scheme), and the desktop derives the port from the scheme (https→443,
  // http→80) when one is omitted, so the scheme is required.
  function isDevserverUrl(raw: string): boolean {
    try {
      const parsed = new URL(raw);
      return (parsed.protocol === "http:" || parsed.protocol === "https:") && parsed.hostname !== "";
    } catch {
      return false;
    }
  }

  async function submitDevserver(): Promise<void> {
    const u = url.trim();
    if (!isDevserverUrl(u)) {
      error = "Enter a full URL with a scheme, e.g. https://box.example.com:8787.";
      return;
    }
    const t = token.trim();
    submitting = true;
    try {
      await saveDevserver(
        {
          url: u,
          label: name.trim() || undefined,
          script: script.trim() || undefined,
          token: t || undefined,
        },
        editing?.id,
      );
      closeDialog();
    } catch (e) {
      error = msg(e);
    } finally {
      submitting = false;
    }
  }
</script>

<Modal {title} onclose={closeDialog}>
  {#if showChoices}
    <div class="choices" role="radiogroup" aria-label="New workspace type">
      <button
        class="choice"
        class:on={dialog.choice === "local"}
        role="radio"
        aria-checked={dialog.choice === "local"}
        type="button"
        onclick={() => choose("local")}>Local directory</button>
      <button
        class="choice"
        class:on={dialog.choice === "devserver"}
        role="radio"
        aria-checked={dialog.choice === "devserver"}
        type="button"
        onclick={() => choose("devserver")}>Devserver</button>
    </div>
  {/if}

  {#if showLocal}
    <p class="intro">A local folder with your markdown files (a git repository, or any directory).</p>
    <label class="field">
      Folder path
      <input
        type="text"
        bind:value={localPath}
        placeholder="/Users/you/notes"
        autocomplete="off"
        spellcheck="false"
        onkeydown={(e) => onFieldKey(e, submitLocal)} />
    </label>
  {:else}
    <p class="intro">
      Connect to a chan devserver, a headless box serving many workspaces. The desktop dials
      the URL; its workspaces appear in their own group.
    </p>
    <div class="row2">
      <label class="field">
        Devserver URL
        <input
          type="text"
          bind:value={url}
          placeholder="https://box.example.com:8787"
          autocomplete="off"
          spellcheck="false"
          onkeydown={(e) => onFieldKey(e, submitDevserver)} />
      </label>
      <label class="field">
        Name
        <input
          type="text"
          bind:value={name}
          placeholder="optional"
          autocomplete="off"
          onkeydown={(e) => onFieldKey(e, submitDevserver)} />
      </label>
    </div>
    <label class="field">
      Token <span class="muted">(connect to a proxied or gateway devserver without scraping)</span>
      <input
        type="password"
        bind:value={token}
        placeholder={editing?.has_token ? "stored; leave blank to keep" : "optional"}
        autocomplete="off"
        onkeydown={(e) => onFieldKey(e, submitDevserver)} />
    </label>
    <label class="field">
      Connect command <span class="muted">(optional; runs in a control terminal)</span>
      <textarea
        rows="2"
        bind:value={script}
        placeholder="ssh box -L 8787:localhost:8787 chan devserver --bind 127.0.0.1 --port 8787"
        autocomplete="off"
        spellcheck="false"></textarea>
    </label>
  {/if}

  {#if error}
    <div class="error" role="alert">{error}</div>
  {/if}

  <div class="dialog-footer">
    {#if showLocal}
      <button class="btn primary" type="button" disabled={submitting} onclick={submitLocal}>Add</button>
    {:else}
      <button class="btn primary" type="button" disabled={submitting} onclick={submitDevserver}>
        {editing ? "Save changes" : "Add devserver"}
      </button>
    {/if}
  </div>
</Modal>

<style>
  .choices {
    display: flex;
    gap: 0.5rem;
    margin-bottom: 1rem;
  }

  .choice {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--btn-border);
    border-radius: 7px;
    background: var(--btn-bg);
    color: var(--text-secondary);
    font-size: 0.9rem;
    cursor: pointer;
  }

  .choice.on {
    border-color: var(--brand);
    color: var(--text);
    background: color-mix(in srgb, var(--brand) 14%, transparent);
  }

  .intro {
    margin: 0 0 1rem;
    color: var(--text-secondary);
    font-size: 0.9rem;
    line-height: 1.45;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin-bottom: 0.85rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .field input,
  .field textarea {
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--bg);
    color: var(--text);
    font-size: 0.9rem;
    font-family: inherit;
  }

  .field textarea {
    resize: vertical;
  }

  .field input:focus,
  .field textarea:focus {
    outline: none;
    border-color: var(--brand);
  }

  .muted {
    color: var(--text-secondary);
    font-weight: 400;
    opacity: 0.8;
  }

  .row2 {
    display: grid;
    grid-template-columns: 1.8fr 1fr;
    gap: 0.6rem;
  }

  .error {
    margin: 0.25rem 0 0;
    padding: 0.5rem 0.65rem;
    border-radius: 7px;
    background: color-mix(in srgb, var(--danger) 16%, transparent);
    color: var(--danger);
    font-size: 0.85rem;
  }

  /* The corrected action row: a clear top margin so the submit button never
     overlaps the last field, the bug the old launcher's dialog had. */
  .dialog-footer {
    display: flex;
    justify-content: flex-end;
    margin-top: 1.5rem;
  }
</style>
