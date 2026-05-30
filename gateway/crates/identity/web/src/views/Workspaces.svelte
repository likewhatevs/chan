<script lang="ts">
  import {
    api,
    HttpError,
    type Workspace,
    type WorkspaceGrant,
    type WorkspaceGrantRole,
    type IncomingShare,
    type OwnedWorkspaceSummary,
  } from "../lib/api";
  import { meStore } from "../state/me.svelte";

  let { username, workspaces }: { username: string; workspaces: Workspace[] } = $props();

  // Pull the configured (potentially-offline) workspace list and the
  // shared-with-me list once on mount. /api/me already gives us live
  // tunnels; the two profile-backed lists are loaded here so the
  // Workspaces view doesn't bloat the /api/me payload for users who
  // never open this tab.
  let owned = $state<OwnedWorkspaceSummary[]>([]);
  let incoming = $state<IncomingShare[]>([]);
  let loadingLists = $state(true);
  let listsError = $state<string | null>(null);

  // Per-workspace grant cache. Keys are workspace_name (lowercase, matching
  // server normalisation). Only populated on first expand of a row;
  // subsequent updates mutate this object in place.
  let grants = $state<Record<string, WorkspaceGrant[]>>({});
  let grantsLoading = $state<Record<string, boolean>>({});
  let grantsError = $state<Record<string, string | null>>({});

  // Tracks which workspace's share panel is open. Single-open keeps the
  // UI compact; multi-open would also be fine, but visually noisy.
  let expanded = $state<string | null>(null);

  // Add-grant form state, keyed by workspace_name. Reset on submit.
  let addEmail = $state<Record<string, string>>({});
  let addRole = $state<Record<string, WorkspaceGrantRole>>({});
  let addBusy = $state<Record<string, boolean>>({});
  let addError = $state<Record<string, string | null>>({});

  // New-workspace form state. Submits to POST /api/workspaces, which
  // persists the workspace in profile-service so it survives a reload
  // even with no grants and no live tunnel.
  let newWorkspace = $state("");
  let newWorkspaceOpen = $state(false);
  let newBusy = $state(false);
  let newError = $state<string | null>(null);

  // Refresh button: pulls /api/me (live tunnels) and the two new
  // lists. Errors surface inline; we don't bounce to the error view.
  let refreshing = $state(false);

  // Toast-style copied feedback. Keyed by workspace_name so the button
  // shows "Copied" briefly without losing focus on the row.
  let copied = $state<Record<string, boolean>>({});

  function unifyWorkspaces() {
    // My workspaces = live tunnels (status: online) UNION owned workspaces
    // from profile (status: offline when no live tunnel matches).
    // The owned list is the authoritative roster; live tunnels just
    // flip status to online when present.
    const liveBy = new Map(workspaces.map((d) => [d.workspace, d]));
    const seen = new Set<string>();
    const rows: { name: string; label: string; online: boolean; public: boolean }[] = [];
    for (const o of owned) {
      const live = liveBy.get(o.workspace_name);
      rows.push({
        name: o.workspace_name,
        label: o.workspace_name,
        online: !!live,
        public: live?.public ?? false,
      });
      seen.add(o.workspace_name);
    }
    // Live tunnels that aren't in owned yet (registry-only, e.g. a
    // `chan serve` started before the user opened this tab and the
    // owned list hasn't refreshed). Show them too so nothing
    // disappears between renders.
    for (const d of workspaces) {
      if (!seen.has(d.workspace)) {
        rows.push({ name: d.workspace, label: d.label, online: true, public: d.public });
        seen.add(d.workspace);
      }
    }
    rows.sort((a, b) => a.name.localeCompare(b.name));
    return rows;
  }

  // Re-derived whenever workspaces / owned change.
  let myWorkspaces = $derived(unifyWorkspaces());

  async function loadLists() {
    loadingLists = true;
    listsError = null;
    try {
      const [o, i] = await Promise.all([
        api.listOwnedWorkspaces(),
        api.listIncomingShares(),
      ]);
      owned = o;
      incoming = i;
    } catch (e) {
      listsError = e instanceof Error ? e.message : String(e);
    } finally {
      loadingLists = false;
    }
  }

  async function refresh() {
    refreshing = true;
    try {
      await Promise.all([meStore.refresh(), loadLists()]);
    } finally {
      refreshing = false;
    }
  }

  // Lazy-load grants for one workspace. Subsequent expansions hit the
  // cache; explicit refresh-after-mutate paths re-call this.
  async function loadGrants(workspace: string, force = false) {
    if (grants[workspace] && !force) return;
    grantsLoading[workspace] = true;
    grantsError[workspace] = null;
    try {
      grants[workspace] = await api.listWorkspaceGrants(workspace);
    } catch (e) {
      grantsError[workspace] = e instanceof Error ? e.message : String(e);
    } finally {
      grantsLoading[workspace] = false;
    }
  }

  function toggle(workspace: string) {
    if (expanded === workspace) {
      expanded = null;
      return;
    }
    expanded = workspace;
    if (!addRole[workspace]) addRole[workspace] = "viewer";
    void loadGrants(workspace);
  }

  // Client-side email shape check. Stricter than backend on
  // purpose: backend keeps `valid_email` lax so OAuth flows whose
  // providers return surprising (but valid) addresses still work,
  // but the dashboard knows the grant only resolves when an OAuth
  // sign-in surfaces the same address, so demanding a `local@host.tld`
  // shape catches the typo case before a row that will never claim
  // lands in the DB.
  function isLikelyEmail(s: string): boolean {
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(s);
  }

  async function addGrant(workspace: string) {
    const email = (addEmail[workspace] ?? "").trim();
    const role = addRole[workspace] ?? "viewer";
    if (!isLikelyEmail(email)) {
      addError[workspace] = "Enter a valid email (local@host.tld)";
      return;
    }
    addBusy[workspace] = true;
    addError[workspace] = null;
    try {
      const row = await api.addWorkspaceGrant(workspace, email, role);
      const list = grants[workspace] ?? [];
      // POST is create-or-promote: if a row with this email already
      // existed, replace it; otherwise prepend so the newest is
      // visible.
      const idx = list.findIndex((g) => g.id === row.id);
      grants[workspace] = idx >= 0
        ? [...list.slice(0, idx), row, ...list.slice(idx + 1)]
        : [row, ...list];
      addEmail[workspace] = "";
      // Owned-list grant_count may have ticked up if this was a brand
      // new (workspace, email) pair. Refresh in the background.
      void loadLists();
    } catch (e) {
      if (e instanceof HttpError) {
        addError[workspace] = e.message;
      } else {
        addError[workspace] = e instanceof Error ? e.message : String(e);
      }
    } finally {
      addBusy[workspace] = false;
    }
  }

  async function removeGrant(workspace: string, id: string) {
    try {
      await api.deleteWorkspaceGrant(id);
      grants[workspace] = (grants[workspace] ?? []).filter((g) => g.id !== id);
      void loadLists();
    } catch (e) {
      grantsError[workspace] = e instanceof Error ? e.message : String(e);
    }
  }

  async function copyShareLink(owner: string, workspace: string) {
    const url = api.shareUrl(owner, workspace);
    try {
      await navigator.clipboard.writeText(url);
    } catch {
      // Clipboard can be blocked in non-secure contexts (file://,
      // some embedded webviews). Fall back to a prompt so the user
      // can copy manually.
      window.prompt("Copy this share link:", url);
      return;
    }
    copied[workspace] = true;
    setTimeout(() => {
      copied[workspace] = false;
    }, 1500);
  }

  function open(owner: string, workspace: string) {
    location.assign(api.workspaceOpenUrl(owner, workspace));
  }

  function isValidWorkspaceName(s: string): boolean {
    if (s.length < 1 || s.length > 64) return false;
    // Lowercase ascii alnum + . _ - (matches backend's validator
    // so we don't surface a server 400 for a cheap client check).
    return /^[a-z0-9._-]+$/.test(s);
  }

  async function startNewWorkspace() {
    const d = newWorkspace.trim().toLowerCase();
    if (!isValidWorkspaceName(d)) return;
    newBusy = true;
    newError = null;
    try {
      await api.createWorkspace(d);
      newWorkspaceOpen = false;
      newWorkspace = "";
      // Pull owned again so the new row enters myWorkspaces with the
      // (zero) grant_count; no flicker because the create succeeded
      // before we expand.
      await loadLists();
      expanded = d;
      if (!addRole[d]) addRole[d] = "viewer";
      if (!grants[d]) grants[d] = [];
    } catch (e) {
      newError = e instanceof Error ? e.message : String(e);
    } finally {
      newBusy = false;
    }
  }

  async function removeWorkspace(workspace: string) {
    if (!confirm(`Delete workspace "${workspace}" and all its grants?`)) return;
    try {
      await api.deleteWorkspace(workspace);
      // Collapse the panel if we were viewing this one.
      if (expanded === workspace) expanded = null;
      delete grants[workspace];
      await loadLists();
    } catch (e) {
      listsError = e instanceof Error ? e.message : String(e);
    }
  }

  $effect(() => {
    void loadLists();
  });
</script>

<section class="workspaces">
  <header>
    <h2>Workspaces</h2>
    <button
      class="ghost"
      type="button"
      disabled={refreshing}
      onclick={refresh}
    >
      {refreshing ? "Refreshing..." : "Refresh"}
    </button>
  </header>

  <div class="block">
    <div class="block-head">
      <h3>My workspaces</h3>
      <button
        type="button"
        class="ghost small-btn"
        onclick={() => (newWorkspaceOpen = !newWorkspaceOpen)}
      >
        {newWorkspaceOpen ? "Cancel" : "+ Share a new workspace"}
      </button>
    </div>

    {#if newWorkspaceOpen}
      <form
        class="newworkspace"
        onsubmit={(e) => {
          e.preventDefault();
          void startNewWorkspace();
        }}
      >
        <div class="row">
          <input
            id="new-workspace-name"
            type="text"
            bind:value={newWorkspace}
            placeholder="workspace name (e.g. photos)"
            maxlength="64"
            autocomplete="off"
            spellcheck="false"
            disabled={newBusy}
          />
          <button
            type="submit"
            disabled={newBusy || !isValidWorkspaceName(newWorkspace.trim().toLowerCase())}
          >
            {newBusy ? "..." : "Create"}
          </button>
        </div>
        {#if newError}
          <p class="err small">{newError}</p>
        {/if}
        <p class="muted small">
          Lowercase letters, digits, and <code>._-</code> only. Add grants
          and copy the share link in the next step; status flips to
          online once you run
          <code>chan serve --tunnel-workspace-name={newWorkspace.trim().toLowerCase() || "<name>"}</code>.
        </p>
      </form>
    {/if}

    {#if myWorkspaces.length === 0 && !loadingLists}
      <div class="empty">
        <p>No workspaces connected or configured.</p>
        <p class="muted small">
          Run <code>chan serve &lt;path&gt;</code> on the machine that holds the
          workspace, with a personal access token set in the
          <code>CHAN_TUNNEL_TOKEN</code> environment variable. Generate a token
          under the Tokens tab.
        </p>
      </div>
    {:else}
      <ul class="list">
        {#each myWorkspaces as d (d.name)}
          <li class="card" class:offline={!d.online}>
            <div class="row">
              <div class="meta">
                <div class="label">{d.label}</div>
                <div class="muted small">
                  {#if d.online}
                    {#if d.public}
                      Public - anyone with the link can read
                    {:else}
                      Online - only you and grantees can open
                    {/if}
                  {:else}
                    Offline - start <code>chan serve --tunnel-workspace-name={d.name}</code>
                  {/if}
                </div>
              </div>
              <div class="actions">
                <span class="status" data-status={d.online ? "online" : "offline"} aria-hidden="true">
                  {d.online ? "online" : "offline"}
                </span>
                <button
                  type="button"
                  class="ghost"
                  onclick={() => toggle(d.name)}
                  aria-expanded={expanded === d.name}
                >
                  {expanded === d.name ? "Hide" : "Share"}
                </button>
                <button
                  type="button"
                  disabled={!d.online}
                  onclick={() => open(username, d.name)}
                >
                  Open
                </button>
              </div>
            </div>

            {#if expanded === d.name}
              <div class="panel">
                <div class="panel-head">
                  <strong>Share access</strong>
                  <div class="panel-actions">
                    <button
                      type="button"
                      class="ghost small-btn"
                      onclick={() => copyShareLink(username, d.name)}
                    >
                      {copied[d.name] ? "Copied" : "Copy share link"}
                    </button>
                    <button
                      type="button"
                      class="ghost small-btn danger"
                      onclick={() => removeWorkspace(d.name)}
                      aria-label="Delete workspace"
                    >
                      Delete workspace
                    </button>
                  </div>
                </div>

                <form
                  class="addgrant"
                  onsubmit={(e) => {
                    e.preventDefault();
                    void addGrant(d.name);
                  }}
                >
                  <input
                    type="email"
                    placeholder="grantee@example.com"
                    bind:value={addEmail[d.name]}
                    disabled={addBusy[d.name]}
                    autocomplete="off"
                    spellcheck="false"
                  />
                  <select
                    bind:value={addRole[d.name]}
                    disabled={addBusy[d.name]}
                    aria-label="Role"
                  >
                    <option value="viewer">Viewer</option>
                    <option value="editor">Editor</option>
                  </select>
                  <button
                    type="submit"
                    disabled={addBusy[d.name] || !isLikelyEmail((addEmail[d.name] ?? "").trim())}
                  >
                    {addBusy[d.name] ? "..." : "Add"}
                  </button>
                </form>
                {#if addError[d.name]}
                  <p class="err small">{addError[d.name]}</p>
                {/if}

                {#if grantsLoading[d.name]}
                  <p class="muted small">Loading grants...</p>
                {:else if grantsError[d.name]}
                  <p class="err small">{grantsError[d.name]}</p>
                {:else if (grants[d.name]?.length ?? 0) === 0}
                  <p class="muted small">No grants yet. The workspace stays
                    private until you add at least one.</p>
                {:else}
                  <ul class="grantlist">
                    {#each grants[d.name] ?? [] as g (g.id)}
                      <li>
                        <span class="grant-email">{g.grantee_email}</span>
                        <span class="grant-role">{g.role}</span>
                        <span class="grant-status muted small">
                          {g.accepted_at ? "active" : "pending sign-in"}
                        </span>
                        <button
                          type="button"
                          class="ghost small-btn"
                          onclick={() => removeGrant(d.name, g.id)}
                          aria-label="Revoke"
                        >
                          Revoke
                        </button>
                      </li>
                    {/each}
                  </ul>
                {/if}
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <div class="block">
    <h3>Shared with me</h3>
    {#if loadingLists}
      <p class="muted small">Loading...</p>
    {:else if listsError}
      <p class="err small">{listsError}</p>
    {:else if incoming.length === 0}
      <p class="muted small">Nothing has been shared with you yet.</p>
    {:else}
      <ul class="list">
        {#each incoming as s (s.grant_id)}
          <li class="card">
            <div class="row">
              <div class="meta">
                <div class="label">
                  {s.workspace_name}
                  <span class="muted small"> - from @{s.owner_username}</span>
                </div>
                <div class="muted small">
                  {s.role === "editor" ? "Editor" : "Viewer"} access
                </div>
              </div>
              <div class="actions">
                <button type="button" onclick={() => open(s.owner_username, s.workspace_name)}>
                  Open
                </button>
              </div>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</section>

<style>
  .workspaces {
    max-width: 720px;
    width: 100%;
    margin: 0 auto;
    padding: 1rem;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
  }
  h3 {
    margin: 0 0 .5rem 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-secondary);
  }
  .block {
    display: flex;
    flex-direction: column;
    gap: .5rem;
  }
  .block-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .newworkspace {
    border: 1px dashed var(--border);
    border-radius: 8px;
    padding: .75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: .5rem;
  }
  .newworkspace .row {
    display: flex;
    gap: .5rem;
    align-items: center;
  }
  .newworkspace input {
    flex: 1;
  }
  .empty {
    border: 1px dashed var(--border);
    border-radius: 8px;
    padding: 1.25rem;
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: .5rem;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: .5rem;
  }
  .card {
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: .75rem 1rem;
    background: var(--card-bg, transparent);
  }
  .card.offline {
    opacity: .85;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
  }
  .meta { min-width: 0; }
  .label {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .muted { color: var(--text-secondary); }
  .small { font-size: 13px; }
  .err { color: var(--warn-text, #b14a3a); }
  .actions {
    display: flex;
    align-items: center;
    gap: .5rem;
    flex-shrink: 0;
  }
  .status {
    text-transform: uppercase;
    font-size: 11px;
    letter-spacing: .04em;
    color: var(--text-secondary);
  }
  .status[data-status="online"] {
    color: var(--ok-text, #2a8c4a);
  }
  .panel {
    margin-top: .75rem;
    padding-top: .75rem;
    border-top: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: .5rem;
  }
  .panel-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .panel-actions {
    display: flex;
    gap: .5rem;
  }
  button.danger {
    color: var(--warn-text, #b14a3a);
  }
  button.danger:hover:not(:disabled) {
    color: var(--warn-text, #b14a3a);
    border-color: var(--warn-text, #b14a3a);
  }
  .addgrant {
    display: flex;
    gap: .5rem;
    align-items: center;
    flex-wrap: wrap;
  }
  .addgrant input[type="email"] {
    flex: 1;
    min-width: 12rem;
  }
  .grantlist {
    list-style: none;
    margin: .25rem 0 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: .25rem;
  }
  .grantlist li {
    display: grid;
    grid-template-columns: 1fr auto auto auto;
    align-items: center;
    gap: .5rem;
    padding: .25rem .5rem;
    border-radius: 4px;
  }
  .grantlist li:hover {
    background: var(--card-bg-hover, rgba(127, 127, 127, .06));
  }
  .grant-email {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .grant-role {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: .04em;
    color: var(--text-secondary);
  }
  input, select {
    font: inherit;
    padding: .35rem .5rem;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--input-bg, transparent);
    color: inherit;
  }
  button {
    font: inherit;
    padding: .35rem .75rem;
    border-radius: 6px;
    cursor: pointer;
  }
  button.ghost {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }
  button.ghost:hover:not(:disabled) {
    color: var(--text);
  }
  button.small-btn {
    padding: .2rem .5rem;
    font-size: 12px;
  }
  button:disabled {
    opacity: .5;
    cursor: not-allowed;
  }
  code {
    background: var(--code-bg, rgba(127, 127, 127, .12));
    padding: .1em .35em;
    border-radius: 3px;
    font-size: .9em;
  }
</style>
