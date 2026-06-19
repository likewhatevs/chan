<script lang="ts">
  // The local workspace registry: each registered folder with an on/off
  // toggle (serve / stop the tenant) and a remove action.
  import { library, toggleWorkspace, removeWorkspace } from "../state/library.svelte";
  import { basename } from "../lib/windowLabel";
  import type { WorkspaceEntry } from "../api/library";

  function displayName(ws: WorkspaceEntry): string {
    return ws.label || basename(ws.path) || ws.path;
  }
</script>

{#if library.workspaces.length}
  <section class="group">
    <h2 class="group-title">🏠 Local</h2>
    <ul class="rows">
      {#each library.workspaces as ws (ws.workspace_id)}
        <li class="row">
          <div class="row-main">
            <span class="row-name">{displayName(ws)}</span>
            <span class="row-sub" title={ws.path}>{ws.path}</span>
          </div>
          <div class="row-actions">
            <button
              class="pill"
              class:on={ws.on}
              type="button"
              aria-pressed={ws.on}
              onclick={() => toggleWorkspace(ws.workspace_id, !ws.on)}>{ws.on ? "On" : "Off"}</button>
            <button
              class="btn-ghost"
              type="button"
              aria-label={`Remove ${displayName(ws)}`}
              onclick={() => removeWorkspace(ws.workspace_id)}>Remove</button>
          </div>
        </li>
      {/each}
    </ul>
  </section>
{/if}
