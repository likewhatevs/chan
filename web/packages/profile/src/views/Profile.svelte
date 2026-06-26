<script lang="ts">
  import { initial } from "@chan/web-shared/initial";
  import type { Me } from "../lib/api";
  import { meStore, MAX_USERNAME_EDITS } from "../state/me.svelte";

  let { me }: { me: Me } = $props();

  let formError = $state<string | null>(null);

  let editingUsername = $state(false);
  let usernameDraft = $state("");
  let usernameError = $state<string | null>(null);
  let usernameBusy = $state(false);

  let editsRemaining = $derived(
    Math.max(0, MAX_USERNAME_EDITS - me.user.username_edits),
  );

  function startEditUsername() {
    usernameDraft = me.user.username;
    usernameError = null;
    editingUsername = true;
  }

  async function saveUsername(e: Event) {
    e.preventDefault();
    const next = usernameDraft.trim().toLowerCase();
    if (!next || next === me.user.username) {
      editingUsername = false;
      return;
    }
    usernameBusy = true;
    usernameError = null;
    try {
      await meStore.updateUsername(next);
      editingUsername = false;
    } catch (err) {
      usernameError = err instanceof Error ? err.message : String(err);
    } finally {
      usernameBusy = false;
    }
  }

  let confirmDeleteText = $state("");
  let deletingAccount = $state(false);

  async function deleteAccount() {
    if (confirmDeleteText !== "delete my account") {
      formError = 'Type "delete my account" to confirm.';
      return;
    }
    deletingAccount = true;
    formError = null;
    try {
      await meStore.deleteAccount();
    } catch (err) {
      formError = err instanceof Error ? err.message : String(err);
    } finally {
      deletingAccount = false;
    }
  }
</script>

<section class="page">
  <header class="who">
    {#if me.user.avatar_url}
      <img
        class="avatar avatar-img"
        src={me.user.avatar_url}
        alt=""
        referrerpolicy="no-referrer"
      />
    {:else}
      <span class="avatar" aria-hidden="true">{initial(me.user)}</span>
    {/if}
    <div class="who-text">
      <h1>{me.user.display_name ?? me.user.email}</h1>
      {#if me.user.display_name}
        <p class="muted small">{me.user.email}</p>
      {/if}
    </div>
  </header>

  <section class="username">
    <h2>Username</h2>
    <p class="muted small">
      {#if editsRemaining > 0}
        {editsRemaining} rename{editsRemaining === 1 ? "" : "s"} left.
      {:else}
        No renames left.
      {/if}
    </p>
    {#if editingUsername}
      <form onsubmit={saveUsername} class="username-edit">
        <input
          type="text"
          bind:value={usernameDraft}
          minlength="3"
          maxlength="32"
          pattern="[a-z0-9][a-z0-9-]{'{1,30}'}[a-z0-9]"
          required
        />
        <button type="submit" disabled={usernameBusy}>
          {usernameBusy ? "Saving..." : "Save"}
        </button>
        <button type="button" onclick={() => (editingUsername = false)}>
          Cancel
        </button>
      </form>
      {#if usernameError}
        <p class="error small">{usernameError}</p>
      {/if}
    {:else}
      <div class="username-row">
        <code>{me.user.username}</code>
        <button
          onclick={startEditUsername}
          disabled={editsRemaining === 0}
        >
          Rename
        </button>
      </div>
    {/if}
  </section>

  {#if formError}
    <p class="error small">{formError}</p>
  {/if}

  <details class="danger-disclosure">
    <summary>Delete account</summary>
    <section class="danger">
      <h2>Danger zone</h2>
      <p class="muted small">
        Deleting your account removes the user record, all linked
        OAuth identities, and all personal access tokens. Active
        tunnels keep running until their <code>chan devserver</code>
        process exits or the token they use is rotated. This cannot
        be undone.
      </p>
      <p class="muted small">
        Type <code>delete my account</code> to enable the button.
      </p>
      <div class="delete-row">
        <input
          type="text"
          placeholder="delete my account"
          bind:value={confirmDeleteText}
        />
        <button
          class="destructive"
          disabled={deletingAccount || confirmDeleteText !== "delete my account"}
          onclick={deleteAccount}
        >
          {deletingAccount ? "Deleting..." : "Delete account"}
        </button>
      </div>
    </section>
  </details>
</section>

<style>
  .page {
    max-width: 720px;
    width: 100%;
    margin: 0 auto;
    padding: 1.5rem 1rem 4rem;
    box-sizing: border-box;
  }
  h1 {
    color: var(--text-heading);
    margin: .5rem 0 .25rem;
    font-size: 22px;
  }
  h2 {
    color: var(--text-heading);
    margin: 1.5rem 0 .5rem;
    font-size: 14px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: .05em;
    color: var(--text-secondary);
  }
  code {
    background: var(--bg-code);
    padding: .05em .35em;
    border-radius: 3px;
    font-size: 0.9em;
  }
  .muted { color: var(--text-secondary); }
  .error { color: var(--warn-text); }
  .small { font-size: 13px; }
  header.who {
    display: flex;
    align-items: center;
    gap: .9rem;
    margin: .5rem 0 1rem;
  }
  .avatar {
    width: 56px;
    height: 56px;
    border-radius: 50%;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    color: var(--text);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 22px;
    font-weight: 600;
    flex: 0 0 auto;
  }
  img.avatar-img {
    object-fit: cover;
    background: var(--bg-elev);
  }
  .who-text { display: flex; flex-direction: column; }
  .who-text h1 { margin: 0; }
  .who-text p { margin: 2px 0 0; }
  details.danger-disclosure {
    margin-top: 2.5rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-card);
  }
  details.danger-disclosure[open] {
    border-color: var(--warn-text);
  }
  details.danger-disclosure > summary {
    cursor: pointer;
    list-style: none;
    padding: .75rem 1rem;
    color: var(--warn-text);
    font-size: 14px;
    font-weight: 600;
    user-select: none;
  }
  details.danger-disclosure > summary::-webkit-details-marker { display: none; }
  details.danger-disclosure > summary::before {
    content: '\25B8';
    display: inline-block;
    width: 1em;
    transition: transform 120ms ease;
    color: var(--warn-text);
  }
  details.danger-disclosure[open] > summary::before {
    transform: rotate(90deg);
  }
  section.danger {
    padding: 0 1.25rem 1rem;
  }
  section.danger h2 {
    margin-top: .25rem;
    color: var(--warn-text);
  }
  .delete-row {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: .5rem;
    margin-top: .5rem;
  }
  button.destructive {
    border-color: var(--warn-text);
    color: var(--warn-text);
  }
  button.destructive:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  section.username {
    margin-bottom: 1rem;
  }
  .username-row {
    display: flex;
    align-items: center;
    gap: .75rem;
  }
  .username-row code {
    font-size: 14px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: .25rem .5rem;
  }
  form.username-edit {
    display: flex;
    gap: .5rem;
    align-items: center;
  }
  form.username-edit input { flex: 1; }
</style>
