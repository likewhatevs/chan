<script lang="ts">
  // `fullstack-a-50` (G3): FB-style inspector body for directory
  // nodes in the graph. Reads aggregated chan-report stats from
  // `systacean-15`'s /api/report/dir O(1) cache endpoint and
  // renders directory path/name + file count + by-language SLOC
  // + COCOMO summary + a "Graph from here" action.
  //
  // Drive root has path "" — the cache also covers that case
  // (returns the whole-drive roll-up). The dispatcher passes
  // `label` from the graph node so the header reads the same
  // human-readable name the canvas uses (e.g. "docs" rather than
  // the full "docs" path).

  import { onMount } from "svelte";
  import { api } from "../api/client";
  import type { ReportPrefix } from "../api/types";

  let {
    path,
    label,
    onSetAsScope,
    onClose,
  }: {
    path: string;
    label?: string;
    onSetAsScope?: () => void;
    onClose?: () => void;
  } = $props();

  /// Roll-up state. `null` while loading + when the directory
  /// has no tracked files (404 from /api/report/dir). `error`
  /// holds any non-404 fetch failure so the body can render the
  /// "stats unavailable" branch with the underlying message.
  let report = $state<ReportPrefix | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  async function load(p: string): Promise<void> {
    loading = true;
    error = null;
    report = null;
    try {
      report = await api.reportDir(p);
    } catch (e) {
      const message = (e as Error).message ?? "";
      // 404 from /api/report/dir is the "no report yet" case —
      // either chan-reports indexing hasn't run for this dir or
      // the dir has no tracked files. Treat as null + render the
      // empty branch; only show the err affordance for genuine
      // failures.
      if (/404/.test(message) || /not found/i.test(message)) {
        report = null;
      } else {
        error = message || "report fetch failed";
      }
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void load(path);
  });

  const displayName = $derived(
    label && label.length > 0 ? label : path === "" ? "Drive root" : path,
  );

  function formatNumber(n: number): string {
    return new Intl.NumberFormat(undefined).format(Math.round(n));
  }

  function formatCurrencyUSD(n: number): string {
    return new Intl.NumberFormat(undefined, {
      style: "currency",
      currency: "USD",
      maximumFractionDigits: 0,
    }).format(n);
  }

  onMount(() => {
    // Initial load handled by the $effect above; placeholder
    // so `onMount` consumers (Svelte's lifecycle hooks) can
    // chain teardown if added later.
  });
</script>

<div class="info">
  <header class="head">
    <span class="kind-chip" class:drafts={path === "Drafts"}>
      {path === "Drafts" ? "DRAFTS" : "DIR"}
    </span>
  </header>
  <h3 class="title">{displayName}</h3>
  {#if path !== ""}
    <div class="path-row" title={path}>{path}</div>
  {/if}
  {#if path === "Drafts"}
    <!-- `fullstack-a-66` slice c: Drafts lives in chan-drive's
         metadata folder (drafts_dir handle), NOT under the
         drive root. The synthetic FB row + the unified
         `Drive::list` make it appear in the wire keyspace as
         `Drafts/...`, but every Drafts/ path routes through
         the drafts cap-std handle (-26 read/write + -29 list).
         The notice tells users why their `crates/` or `docs/`
         aren't sibling to Drafts on disk. -->
    <div class="drafts-notice" role="note">
      <strong>Drafts lives outside the drive's root.</strong>
      Files here are stored in chan's metadata folder so they
      survive drive moves + don't clutter your tree. Cmd+N
      creates a fresh draft under <code>Drafts/untitled-N/</code>;
      Rich Prompts persist as <code>Drafts/rich-prompt-N/</code>
      in a follow-up slice.
    </div>
  {/if}

  {#if onSetAsScope}
    <div class="actions">
      <button class="set-as-scope" onclick={onSetAsScope} type="button">
        Graph from here
      </button>
    </div>
  {/if}

  {#if loading}
    <div class="muted">loading directory stats…</div>
  {:else if error}
    <div class="err">stats unavailable: {error}</div>
  {:else if !report}
    <!-- No tracked files in this directory, OR chan-reports
         hasn't indexed it yet. Either way, render the empty
         affordance + a hint that the toggle gates the data
         per the `-a-48` chan-reports toggle. -->
    <div class="muted">
      No chan-report data for this directory. Make sure
      <strong>chan-reports</strong> is enabled in the Hybrid File
      Browser back-side settings.
    </div>
  {:else}
    <section class="stats">
      <h4>Totals</h4>
      <div class="meta-grid">
        <span class="k">files</span>
        <span class="v">{formatNumber(report.totals.files)}</span>
        <span class="k">code (SLOC)</span>
        <span class="v">{formatNumber(report.totals.code)}</span>
        <span class="k">comments</span>
        <span class="v">{formatNumber(report.totals.comments)}</span>
        <span class="k">blanks</span>
        <span class="v">{formatNumber(report.totals.blanks)}</span>
      </div>
    </section>

    {#if report.by_language.length > 0}
      <section class="stats">
        <h4>By language</h4>
        <table class="lang-table">
          <thead>
            <tr>
              <th class="lang-name">Language</th>
              <th class="lang-num">Files</th>
              <th class="lang-num">SLOC</th>
            </tr>
          </thead>
          <tbody>
            {#each report.by_language as lang (lang.name)}
              <tr>
                <td class="lang-name">{lang.name}</td>
                <td class="lang-num">{formatNumber(lang.files)}</td>
                <td class="lang-num">{formatNumber(lang.code)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>
    {/if}

    <section class="stats">
      <h4>COCOMO ({report.cocomo.model})</h4>
      <div class="meta-grid">
        <span class="k">effort</span>
        <span class="v">{report.cocomo.effort_person_months.toFixed(1)} pmo</span>
        <span class="k">schedule</span>
        <span class="v">{report.cocomo.schedule_months.toFixed(1)} mo</span>
        <span class="k">developers</span>
        <span class="v">{report.cocomo.developers.toFixed(1)}</span>
        <span class="k">cost (est)</span>
        <span class="v">{formatCurrencyUSD(report.cocomo.estimated_cost_usd)}</span>
      </div>
    </section>
  {/if}
</div>

<style>
  .info {
    padding: 0.6rem 0.7rem 0.8rem 0.7rem;
    font-size: 12.5px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.4rem;
  }
  .kind-chip {
    display: inline-block;
    padding: 2px 7px;
    border-radius: 3px;
    background: var(--g-folder);
    color: var(--text-on-accent, white);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
  }
  /* `fullstack-a-66` slice c: Drafts chip picks up the same
     yellow tone the FB row uses (`-a-66b`). Cross-surface
     consistency matters here — the chip is the only inspector
     header element that visually distinguishes Drafts from a
     regular directory. */
  .kind-chip.drafts {
    background: var(--fb-drafts-fg);
  }
  .drafts-notice {
    margin: 0.5rem 0;
    padding: 0.5rem 0.6rem;
    border-radius: 4px;
    background: var(--fb-drafts-bg);
    border-left: 3px solid var(--fb-drafts-fg);
    font-size: 12.5px;
    color: var(--text);
    line-height: 1.45;
  }
  .drafts-notice strong {
    display: block;
    margin-bottom: 0.25rem;
  }
  .drafts-notice code {
    background: var(--bg);
    padding: 1px 4px;
    border-radius: 3px;
    font-family: ui-monospace, monospace;
    font-size: 11.5px;
  }
  .title {
    margin: 0 0 0.25rem 0;
    font-size: 16px;
    font-weight: 600;
    word-break: break-word;
  }
  .path-row {
    font-family: ui-monospace, monospace;
    font-size: 11.5px;
    color: var(--text-secondary);
    margin: 0 0 0.5rem 0;
    word-break: break-all;
  }
  .actions {
    margin: 0.5rem 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .set-as-scope {
    background: transparent;
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    padding: 4px 8px;
    width: 100%;
  }
  .set-as-scope:hover {
    background: var(--hover-bg);
  }
  .stats {
    margin: 0.7rem 0 0 0;
  }
  .stats h4 {
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    margin: 0 0 0.25rem 0;
  }
  .meta-grid {
    display: grid;
    grid-template-columns: 6.5em 1fr;
    gap: 2px 0.5rem;
    font-size: 14px;
  }
  .meta-grid .k { color: var(--text-secondary); }
  .meta-grid .v { color: var(--text); font-variant-numeric: tabular-nums; }
  .lang-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  .lang-table th,
  .lang-table td {
    padding: 2px 6px;
    text-align: left;
  }
  .lang-table th {
    color: var(--text-secondary);
    font-weight: 600;
    text-transform: uppercase;
    font-size: 11px;
    letter-spacing: 0.04em;
    border-bottom: 1px solid var(--border);
  }
  .lang-table td.lang-num,
  .lang-table th.lang-num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .lang-table td.lang-name {
    color: var(--text);
  }
  .muted {
    color: var(--text-secondary);
    font-size: 13px;
    margin-top: 0.4rem;
    font-style: italic;
  }
  .err {
    color: var(--warn-text);
    font-size: 13px;
    margin-top: 0.4rem;
  }
</style>
