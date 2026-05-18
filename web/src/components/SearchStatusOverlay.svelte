<script lang="ts">
  // Search-index dashboard. Lives as its own overlay so index state
  // is reachable from Search without burying it in the file-browser
  // drive inspector. Data is intentionally read-only except for the
  // explicit rebuild action.

  import { untrack } from "svelte";
  import { Maximize2, Minimize2, Network, RefreshCw, X } from "lucide-svelte";
  import { api } from "../api/client";
  import type { ReportPrefix } from "../api/types";
  import {
    indexStatus,
    openLanguageGraphForDrive,
    searchStatusOverlay,
  } from "../state/store.svelte";
  import {
    overlayMaximized,
    setOverlayMaximized,
  } from "../state/pageWidth.svelte";
  import OverlayShell from "./OverlayShell.svelte";

  let report = $state<ReportPrefix | null>(null);
  let reportLoading = $state(false);
  let reportError = $state<string | null>(null);
  let indexResetting = $state(false);
  let indexResetError = $state<string | null>(null);
  let statusPollTimer: ReturnType<typeof setTimeout> | null = null;

  const visible = $derived(searchStatusOverlay.open);
  const topLanguages = $derived(report?.by_language.slice(0, 12) ?? []);

  $effect(() => {
    if (!visible) return;
    untrack(() => {
      void refreshIndexStatus();
      void loadReport();
      scheduleStatusPoll(0);
    });
    return () => stopStatusPoll();
  });

  function close(): void {
    searchStatusOverlay.open = false;
  }

  function doToggleOverlayMaximized(): void {
    setOverlayMaximized(!overlayMaximized.on);
  }

  function graphCodeReport(): void {
    close();
    openLanguageGraphForDrive();
  }

  async function loadReport(): Promise<void> {
    reportLoading = true;
    reportError = null;
    try {
      report = await api.reportPrefix("");
    } catch (e) {
      report = null;
      reportError = (e as Error).message;
    } finally {
      reportLoading = false;
    }
  }

  function stopStatusPoll(): void {
    if (statusPollTimer) {
      clearTimeout(statusPollTimer);
      statusPollTimer = null;
    }
  }

  function scheduleStatusPoll(delayMs = 1500): void {
    stopStatusPoll();
    statusPollTimer = setTimeout(() => {
      statusPollTimer = null;
      void refreshIndexStatus();
    }, delayMs);
  }

  async function refreshIndexStatus(): Promise<void> {
    if (!visible) return;
    try {
      const s = await api.indexStatus();
      indexStatus.value = s;
      scheduleStatusPoll(s.state === "idle" ? 5000 : 1000);
    } catch {
      indexStatus.value = null;
      scheduleStatusPoll(5000);
    }
  }

  async function rebuildIndex(): Promise<void> {
    indexResetting = true;
    indexResetError = null;
    try {
      await api.indexRebuild();
      indexStatus.value = { state: "reindexing", file: "" };
      scheduleStatusPoll(250);
    } catch (e) {
      indexResetError = (e as Error).message;
    } finally {
      indexResetting = false;
    }
  }

  function fmt(n: number): string {
    return n.toLocaleString();
  }
</script>

<OverlayShell id="search-status" open={visible} onClose={close}>
  <div class="status">
    <header>
      <button
        type="button"
        class="chrome-btn"
        onclick={doToggleOverlayMaximized}
        title={overlayMaximized.on ? "Restore size" : "Maximize"}
        aria-label={overlayMaximized.on ? "Restore size" : "Maximize"}
      >
        {#if overlayMaximized.on}
          <Minimize2 size={14} strokeWidth={1.75} aria-hidden="true" />
        {:else}
          <Maximize2 size={14} strokeWidth={1.75} aria-hidden="true" />
        {/if}
      </button>
      <span class="title">Search Status</span>
      <button
        type="button"
        class="chrome-btn"
        onclick={() => void loadReport()}
        disabled={reportLoading}
        title="Refresh report"
        aria-label="Refresh report"
      >
        <RefreshCw size={14} strokeWidth={1.75} aria-hidden="true" />
      </button>
      <button type="button" class="chrome-btn close" onclick={close} title="Close" aria-label="Close">
        <X size={14} strokeWidth={1.75} aria-hidden="true" />
      </button>
    </header>

    <main>
      <section>
        <h3>Index</h3>
        <div class="grid">
          <span class="k">state</span>
          <span class="v">{indexStatus.value?.state ?? "n/a"}</span>
          {#if indexStatus.value?.state === "idle"}
            <span class="k">chunks</span>
            <span class="v">{fmt(indexStatus.value.indexed_docs)}</span>
            <span class="k">vectors</span>
            <span class="v">{fmt(indexStatus.value.indexed_vectors)}</span>
            <span class="k">model</span>
            <span class="v mono">{indexStatus.value.model}</span>
          {:else if indexStatus.value?.state === "building"}
            <span class="k">progress</span>
            <span class="v">{fmt(indexStatus.value.current)} / {fmt(indexStatus.value.total)}</span>
            <span class="k">file</span>
            <span class="v mono path">{indexStatus.value.file}</span>
          {:else if indexStatus.value?.state === "reindexing"}
            <span class="k">file</span>
            <span class="v mono path">{indexStatus.value.file}</span>
          {:else if indexStatus.value?.state === "error"}
            <span class="k">error</span>
            <span class="v err">{indexStatus.value.message}</span>
          {/if}
        </div>
        <button class="action" onclick={() => void rebuildIndex()} disabled={indexResetting}>
          {indexResetting ? "Rebuilding..." : "Rebuild index"}
        </button>
        {#if indexResetError}
          <div class="err-line">{indexResetError}</div>
        {/if}
      </section>

      <section>
        <div class="section-title">
          <h3>Code Report</h3>
          {#if report}
            <button
              type="button"
              class="inline-action"
              onclick={graphCodeReport}
              title="Graph from here code report"
              aria-label="Graph from here code report"
            >
              <Network size={14} strokeWidth={1.75} aria-hidden="true" />
              <span>Graph from here</span>
            </button>
          {/if}
        </div>
        {#if reportLoading}
          <div class="placeholder">loading report...</div>
        {:else if reportError}
          <div class="err-line">report unavailable: {reportError}</div>
        {:else if report}
          <div class="grid">
            <span class="k">files</span>
            <span class="v">{fmt(report.totals.files)}</span>
            <span class="k">SLOC</span>
            <span class="v">{fmt(report.totals.code)}</span>
            <span class="k">comments</span>
            <span class="v">{fmt(report.totals.comments)}</span>
            <span class="k">complexity</span>
            <span class="v">{fmt(report.totals.complexity)}</span>
          </div>
          {#if topLanguages.length > 0}
            <ul class="langs">
              {#each topLanguages as lang (lang.name)}
                <li>
                  <span class="lang">{lang.name}</span>
                  <span class="bar"><span style:width={`${Math.max(4, (lang.code / Math.max(1, report.totals.code)) * 100)}%`}></span></span>
                  <span class="sloc">{fmt(lang.code)} SLOC</span>
                  <span class="files">{fmt(lang.files)} files</span>
                </li>
              {/each}
            </ul>
          {/if}
        {:else}
          <div class="placeholder">no report data</div>
        {/if}
      </section>
    </main>
  </div>
</OverlayShell>

<style>
  .status {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-card);
  }
  .title {
    flex: 1;
    min-width: 0;
    color: var(--text);
    font-weight: 600;
  }
  .chrome-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 24px;
    padding: 0;
    background: var(--bg);
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
  }
  .chrome-btn:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--btn-hover);
  }
  .chrome-btn:disabled { opacity: 0.45; cursor: default; }
  main {
    flex: 1;
    min-height: 0;
    overflow: auto;
    padding: 14px;
    display: grid;
    gap: 14px;
    align-content: start;
  }
  section {
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-card);
    padding: 12px;
  }
  h3 {
    margin: 0 0 10px 0;
    font-size: 13px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
  }
  .section-title {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 10px;
  }
  .section-title h3 {
    margin: 0;
    flex: 1;
    min-width: 0;
  }
  .inline-action {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 4px 8px;
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    background: var(--btn-bg);
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
    white-space: nowrap;
  }
  .inline-action:hover { border-color: var(--btn-hover); }
  .grid {
    display: grid;
    grid-template-columns: 8em minmax(0, 1fr);
    gap: 4px 10px;
    font-size: 14px;
  }
  .k { color: var(--text-secondary); }
  .v { color: var(--text); min-width: 0; }
  .mono { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
  .path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .err, .err-line { color: var(--warn-text); }
  .err-line { margin-top: 8px; font-size: 13px; }
  .placeholder { color: var(--text-secondary); font-size: 14px; }
  .action {
    margin-top: 12px;
    padding: 5px 10px;
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    background: var(--btn-bg);
    color: var(--text);
    cursor: pointer;
    font: inherit;
    font-size: 13px;
  }
  .action:hover:not(:disabled) { border-color: var(--btn-hover); }
  .action:disabled { opacity: 0.55; cursor: default; }
  .langs {
    list-style: none;
    margin: 12px 0 0 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  .langs li {
    display: grid;
    grid-template-columns: minmax(8em, 1fr) minmax(90px, 2fr) auto auto;
    align-items: center;
    gap: 10px;
    font-size: 13px;
  }
  .lang { color: var(--text); min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .bar {
    height: 7px;
    border-radius: 999px;
    background: var(--bg);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--link);
  }
  .sloc, .files {
    color: var(--text-secondary);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  @media (max-width: 720px) {
    .langs li {
      grid-template-columns: minmax(0, 1fr) auto;
    }
    .bar, .files { display: none; }
  }
</style>
