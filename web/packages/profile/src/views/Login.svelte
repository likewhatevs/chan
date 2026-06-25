<script lang="ts">
  // Provider buttons are driven by GET /api/providers; only those
  // the operator has wired up appear here. Each anchor points at
  // /auth/<name>, the backend handles the redirect dance.
  import { meStore } from "../state/me.svelte";

  const labels: Record<string, string> = {
    github: "GitHub",
    google: "Google",
    gitlab: "GitLab",
  };

  function label(p: string): string {
    return labels[p] ?? p;
  }

  // OAuth callback can 303 back with ?denied=<flag> when the user
  // resolved successfully but a gate flag is off (today: only
  // `oauth_login`). The query param is set by identity-service; the
  // session is not granted, so the user lands back on Login. We
  // surface a friendlier message than a bare 403.
  function deniedFlag(): string | null {
    if (typeof location === "undefined") return null;
    const p = new URLSearchParams(location.search);
    return p.get("denied");
  }
  const denied = $state(deniedFlag());
</script>

<section class="card">
  <span class="mark" aria-hidden="true"></span>
  <h1>chan id</h1>

  {#if denied === "oauth_login"}
    <p class="warn small">
      Sign-in is closed for your account right now. If you expect
      access, ask an operator to enrol you.
    </p>
  {:else}
    <p class="muted">Sign in to manage your account.</p>
  {/if}

  {#if meStore.providers.length === 0}
    <p class="error small">No providers configured.</p>
  {:else}
    <div class="providers">
      {#each meStore.providers as p (p)}
        <a class="provider" href={`/auth/${p}`}>
          Continue with {label(p)}
        </a>
      {/each}
    </div>
  {/if}
</section>

<style>
  section.card {
    margin: auto;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 2rem 2.25rem;
    width: min(360px, 92vw);
    text-align: center;
  }
  .mark {
    display: block;
    margin: 0 auto .75rem;
    width: 72px; height: 72px;
    background-color: var(--brand);
    -webkit-mask: url('/chan-mark.png') center / contain no-repeat;
            mask: url('/chan-mark.png') center / contain no-repeat;
  }
  h1 {
    color: var(--text-heading);
    margin: .25rem 0 .25rem;
    font-size: 18px;
    font-weight: 600;
    letter-spacing: .01em;
  }
  .muted {
    color: var(--text-secondary);
    margin: 0 0 1.25rem;
    font-size: 14px;
  }
  .providers {
    display: flex;
    flex-direction: column;
    gap: .5rem;
    align-items: stretch;
  }
  .provider {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    text-decoration: none;
    color: var(--text);
    background: var(--btn-bg);
    border: 1px solid var(--btn-border);
    border-radius: 8px;
    padding: .55rem 1rem;
    font-weight: 500;
  }
  .provider:hover { border-color: var(--btn-hover); }
  .error { color: var(--warn-text); }
  .warn {
    color: var(--warn-text);
    margin: 0 0 1.25rem;
  }
  .small { font-size: 13px; }
</style>
