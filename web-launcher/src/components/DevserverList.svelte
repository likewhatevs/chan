<script lang="ts">
  // The devserver registry: each remote library the desktop dials out to,
  // with connect, edit, and remove. A stored token shows as a lock chip (the
  // value is never returned). Connect is a desktop action — enabled on the
  // mutable (desktop loopback) surface and disabled on the read-only
  // devserver/gateway surface; the registry CRUD here is fully browser-testable.
  import { library, connectDevserver, removeDevserver } from "../state/library.svelte";
  import { openEditDevserver } from "../state/dialog.svelte";
  import { readOnly } from "../state/capabilities";
  import type { DevserverEntry } from "../api/library";

  function endpoint(ds: DevserverEntry): string {
    return ds.url;
  }
  function displayName(ds: DevserverEntry): string {
    return ds.label || endpoint(ds);
  }
</script>

{#if library.devservers.length}
  <section class="group">
    <h2 class="group-title">↗ Devservers</h2>
    <ul class="rows">
      {#each library.devservers as ds (ds.id)}
        <li class="row">
          <div class="row-main">
            <span class="row-name">
              {displayName(ds)}
              {#if ds.has_token}<span class="chip" title="A connect token is stored">🔒 token</span>{/if}
            </span>
            <span class="row-sub" title={endpoint(ds)}>{endpoint(ds)}</span>
          </div>
          <div class="row-actions">
            <button
              class="btn-ghost"
              type="button"
              disabled={readOnly}
              title={readOnly ? "Connect is a desktop action" : undefined}
              aria-label={`Connect ${displayName(ds)}`}
              onclick={() => connectDevserver(ds.id)}>Connect</button>
            <button
              class="btn-ghost"
              type="button"
              aria-label={`Edit ${displayName(ds)}`}
              onclick={() => openEditDevserver(ds)}>Edit</button>
            <button
              class="btn-ghost"
              type="button"
              aria-label={`Remove ${displayName(ds)}`}
              onclick={() => removeDevserver(ds.id)}>Remove</button>
          </div>
        </li>
      {/each}
    </ul>
  </section>
{/if}
