<script lang="ts">
  // Language inspector body. Shown when a language bubble
  // (`kind: "language"`, id `language:<lang>`) is selected in the
  // graph. Phase-13 A3: language nodes previously had no inspector
  // at all (InspectorBody's dispatcher fell through to the tag body
  // with a null selection, so the panel rendered the empty
  // placeholder). This body mirrors the other inspector bodies'
  // chrome: a kind chip, the language name, a small stats grid
  // (files + lines of code), and a "Graph from here" affordance.
  //
  // The action's semantic is host-decided (the callback-agnostic
  // pattern WorkspaceInfoBody / FileInfoBody already use): the graph
  // host re-scopes the current tab to the language lens, the file
  // browser host would spawn a fresh language graph. The body just
  // calls `onSetAsScope`.

  let {
    language,
    label,
    files,
    code,
    onSetAsScope,
  }: {
    language: string;
    label: string;
    files?: number;
    code?: number;
    /// "Graph from here" handler. When unset (e.g. a host that has
    /// no graph to re-scope) the button is suppressed.
    onSetAsScope?: () => void;
  } = $props();
</script>

<div class="info">
  <header class="head">
    <span class="kind-chip language">language</span>
  </header>
  <h3 class="title" title={language}>{label}</h3>

  {#if onSetAsScope}
    <button class="open" type="button" onclick={onSetAsScope}>Graph from here</button>
  {/if}

  <div class="meta-grid">
    {#if files !== undefined}
      <span class="k">files</span>
      <span class="v">{files}</span>
    {/if}
    {#if code !== undefined}
      <span class="k">code lines</span>
      <span class="v">{code.toLocaleString()}</span>
    {/if}
  </div>
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
  /* Language kind chip: tracks the graph's language palette so the
     inspector cue matches the bubble colour on the canvas. Sits
     alongside the workspace / doc / contact / tag chips. */
  .kind-chip {
    color: #fff;
    text-transform: uppercase;
    font-size: 12px;
    font-weight: 600;
    letter-spacing: 0.05em;
    padding: 1px 6px;
    border-radius: 3px;
    flex: 1;
    text-align: center;
  }
  .kind-chip.language {
    background: var(--g-language, #7c5cff);
    color: #fff;
  }
  .title {
    margin: 0 0 0.5rem 0;
    font-size: 16px;
    font-weight: 600;
    word-break: break-word;
  }
  .meta-grid {
    display: grid;
    grid-template-columns: 6.5em 1fr;
    gap: 2px 0.5rem;
    margin: 0.4rem 0 0.6rem 0;
    font-size: 14px;
  }
  .meta-grid .k { color: var(--text-secondary); }
  .meta-grid .v {
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }
  /* Mirrors FileInfoBody / WorkspaceInfoBody `.open` so the
     "Graph from here" affordance reads consistently across bodies. */
  .open {
    width: 100%;
    background: var(--btn-bg);
    color: var(--text);
    border: 1px solid var(--btn-border);
    border-radius: 4px;
    padding: 5px 0;
    cursor: pointer;
    font: inherit;
    margin-top: 0.6rem;
  }
  .open:hover { border-color: var(--btn-hover); }
</style>
