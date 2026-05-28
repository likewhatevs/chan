<script lang="ts">
  import {
    api,
    HttpError,
    type Drive,
    type DriveGrant,
    type DriveGrantRole,
    type IncomingShare,
    type OwnedDriveSummary,
  } from "../lib/api";
  import { meStore } from "../state/me.svelte";

  let { username, drives }: { username: string; drives: Drive[] } = $props();

  // Pull the configured (potentially-offline) drive list and the
  // shared-with-me list once on mount. /api/me already gives us live
  // tunnels; the two profile-backed lists are loaded here so the
  // Drives view doesn't bloat the /api/me payload for users who
  // never open this tab.
  let owned = $state<OwnedDriveSummary[]>([]);
  let incoming = $state<IncomingShare[]>([]);
  let loadingLists = $state(true);
  let listsError = $state<string | null>(null);

  // Per-drive grant cache. Keys are drive_name (lowercase, matching
  // server normalisation). Only populated on first expand of a row;
  // subsequent updates mutate this object in place.
  let grants = $state<Record<string, DriveGrant[]>>({});
  let grantsLoading = $state<Record<string, boolean>>({});
  let grantsError = $state<Record<string, string | null>>({});

  // Tracks which drive's share panel is open. Single-open keeps the
  // UI compact; multi-open would also be fine, but visually noisy.
  let expanded = $state<string | null>(null);

  // Add-grant form state, keyed by drive_name. Reset on submit.
  let addEmail = $state<Record<string, string>>({});
  let addRole = $state<Record<string, DriveGrantRole>>({});
  let addBusy = $state<Record<string, boolean>>({});
  let addError = $state<Record<string, string | null>>({});

  // New-drive form state. Submits to POST /api/drives, which
  // persists the drive in profile-service so it survives a reload
  // even with no grants and no live tunnel.
  let newDrive = $state("");
  let newDriveOpen = $state(false);
  let newBusy = $state(false);
  let newError = $state<string | null>(null);

  // Refresh button: pulls /api/me (live tunnels) and the two new
  // lists. Errors surface inline; we don't bounce to the error view.
  let refreshing = $state(false);

  // Toast-style copied feedback. Keyed by drive_name so the button
  // shows "Copied" briefly without losing focus on the row.
  let copied = $state<Record<string, boolean>>({});

  function unifyDrives() {
    // My drives = live tunnels (status: online) UNION owned drives
    // from profile (status: offline when no live tunnel matches).
    // The owned list is the authoritative roster; live tunnels just
    // flip status to online when present.
    const liveBy = new Map(drives.map((d) => [d.drive, d]));
    const seen = new Set<string>();
    const rows: { name: string; label: string; online: boolean; public: boolean }[] = [];
    for (const o of owned) {
      const live = liveBy.get(o.drive_name);
      rows.push({
        name: o.drive_name,
        label: o.drive_name,
        online: !!live,
        public: live?.public ?? false,
      });
      seen.add(o.drive_name);
    }
    // Live tunnels that aren't in owned yet (registry-only, e.g. a
    // `chan serve` started before the user opened this tab and the
    // owned list hasn't refreshed). Show them too so nothing
    // disappears between renders.
    for (const d of drives) {
      if (!seen.has(d.drive)) {
        rows.push({ name: d.drive, label: d.label, online: true, public: d.public });
        seen.add(d.drive);
      }
    }
    rows.sort((a, b) => a.name.localeCompare(b.name));
    return rows;
  }

  // Re-derived whenever drives / owned change.
  let myDrives = $derived(unifyDrives());

  async function loadLists() {
    loadingLists = true;
    listsError = null;
    try {
      const [o, i] = await Promise.all([
        api.listOwnedDrives(),
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

  // Lazy-load grants for one drive. Subsequent expansions hit the
  // cache; explicit refresh-after-mutate paths re-call this.
  async function loadGrants(drive: string, force = false) {
    if (grants[drive] && !force) return;
    grantsLoading[drive] = true;
    grantsError[drive] = null;
    try {
      grants[drive] = await api.listDriveGrants(drive);
    } catch (e) {
      grantsError[drive] = e instanceof Error ? e.message : String(e);
    } finally {
      grantsLoading[drive] = false;
    }
  }

  function toggle(drive: string) {
    if (expanded === drive) {
      expanded = null;
      return;
    }
    expanded = drive;
    if (!addRole[drive]) addRole[drive] = "viewer";
    void loadGrants(drive);
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

  async function addGrant(drive: string) {
    const email = (addEmail[drive] ?? "").trim();
    const role = addRole[drive] ?? "viewer";
    if (!isLikelyEmail(email)) {
      addError[drive] = "Enter a valid email (local@host.tld)";
      return;
    }
    addBusy[drive] = true;
    addError[drive] = null;
    try {
      const row = await api.addDriveGrant(drive, email, role);
      const list = grants[drive] ?? [];
      // POST is create-or-promote: if a row with this email already
      // existed, replace it; otherwise prepend so the newest is
      // visible.
      const idx = list.findIndex((g) => g.id === row.id);
      grants[drive] = idx >= 0
        ? [...list.slice(0, idx), row, ...list.slice(idx + 1)]
        : [row, ...list];
      addEmail[drive] = "";
      // Owned-list grant_count may have ticked up if this was a brand
      // new (drive, email) pair. Refresh in the background.
      void loadLists();
    } catch (e) {
      if (e instanceof HttpError) {
        addError[drive] = e.message;
      } else {
        addError[drive] = e instanceof Error ? e.message : String(e);
      }
    } finally {
      addBusy[drive] = false;
    }
  }

  async function removeGrant(drive: string, id: string) {
    try {
      await api.deleteDriveGrant(id);
      grants[drive] = (grants[drive] ?? []).filter((g) => g.id !== id);
      void loadLists();
    } catch (e) {
      grantsError[drive] = e instanceof Error ? e.message : String(e);
    }
  }

  async function copyShareLink(owner: string, drive: string) {
    const url = api.shareUrl(owner, drive);
    try {
      await navigator.clipboard.writeText(url);
    } catch {
      // Clipboard can be blocked in non-secure contexts (file://,
      // some embedded webviews). Fall back to a prompt so the user
      // can copy manually.
      window.prompt("Copy this share link:", url);
      return;
    }
    copied[drive] = true;
    setTimeout(() => {
      copied[drive] = false;
    }, 1500);
  }

  function open(owner: string, drive: string) {
    location.assign(api.driveOpenUrl(owner, drive));
  }

  function isValidDriveName(s: string): boolean {
    if (s.length < 1 || s.length > 64) return false;
    // Lowercase ascii alnum + . _ - (matches backend's validator
    // so we don't surface a server 400 for a cheap client check).
    return /^[a-z0-9._-]+$/.test(s);
  }

  async function startNewDrive() {
    const d = newDrive.trim().toLowerCase();
    if (!isValidDriveName(d)) return;
    newBusy = true;
    newError = null;
    try {
      await api.createDrive(d);
      newDriveOpen = false;
      newDrive = "";
      // Pull owned again so the new row enters myDrives with the
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

  async function removeDrive(drive: string) {
    if (!confirm(`Delete drive "${drive}" and all its grants?`)) return;
    try {
      await api.deleteDrive(drive);
      // Collapse the panel if we were viewing this one.
      if (expanded === drive) expanded = null;
      delete grants[drive];
      await loadLists();
    } catch (e) {
      listsError = e instanceof Error ? e.message : String(e);
    }
  }

  $effect(() => {
    void loadLists();
  });
</script>

<section class="drives">
  <header>
    <h2>Drives</h2>
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
      <h3>My drives</h3>
      <button
        type="button"
        class="ghost small-btn"
        onclick={() => (newDriveOpen = !newDriveOpen)}
      >
        {newDriveOpen ? "Cancel" : "+ Share a new drive"}
      </button>
    </div>

    {#if newDriveOpen}
      <form
        class="newdrive"
        onsubmit={(e) => {
          e.preventDefault();
          void startNewDrive();
        }}
      >
        <div class="row">
          <input
            id="new-drive-name"
            type="text"
            bind:value={newDrive}
            placeholder="drive name (e.g. photos)"
            maxlength="64"
            autocomplete="off"
            spellcheck="false"
            disabled={newBusy}
          />
          <button
            type="submit"
            disabled={newBusy || !isValidDriveName(newDrive.trim().toLowerCase())}
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
          <code>chan serve --tunnel-drive={newDrive.trim().toLowerCase() || "<name>"}</code>.
        </p>
      </form>
    {/if}

    {#if myDrives.length === 0 && !loadingLists}
      <div class="empty">
        <p>No drives connected or configured.</p>
        <p class="muted small">
          Run <code>chan serve &lt;path&gt;</code> on the machine that holds the
          drive, with a personal access token set in the
          <code>CHAN_TUNNEL_TOKEN</code> environment variable. Generate a token
          under the Tokens tab.
        </p>
      </div>
    {:else}
      <ul class="list">
        {#each myDrives as d (d.name)}
          <li class="card" class:offline={!d.online}>
            <div class="row">
              <div class="meta">
                <div class="label">{d.label}</div>
                <div class="muted small">
                  {#if d.online}
                    {#if d.public}
                      Public &middot; anyone with the link can read
                    {:else}
                      Online &middot; only you and grantees can open
                    {/if}
                  {:else}
                    Offline &middot; start <code>chan serve --tunnel-drive={d.name}</code>
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
                      onclick={() => removeDrive(d.name)}
                      aria-label="Delete drive"
                    >
                      Delete drive
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
                  <p class="muted small">No grants yet. The drive stays
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
                  {s.drive_name}
                  <span class="muted small">&middot; from @{s.owner_username}</span>
                </div>
                <div class="muted small">
                  {s.role === "editor" ? "Editor" : "Viewer"} access
                </div>
              </div>
              <div class="actions">
                <button type="button" onclick={() => open(s.owner_username, s.drive_name)}>
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
  .drives {
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
  .newdrive {
    border: 1px dashed var(--border);
    border-radius: 8px;
    padding: .75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: .5rem;
  }
  .newdrive .row {
    display: flex;
    gap: .5rem;
    align-items: center;
  }
  .newdrive input {
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
