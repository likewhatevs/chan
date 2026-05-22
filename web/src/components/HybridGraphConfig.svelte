<script lang="ts">
  // `fullstack-a-51` Task D (bundled with G6): legend grid for the
  // Hybrid Graph back-side. Renders the G6 colour palette as
  // `[label] [colour swatch]` rows. Each swatch reads its colour
  // from the central CSS palette so light / dark mode + per-Hybrid
  // theme override (`-a-53`) all cascade through automatically.
  //
  // Source of truth for the palette is `App.svelte`'s `:root` (and
  // `[data-theme="light"]` override). The CSS variables consumed
  // here mirror what `GraphCanvas.svelte`'s theme reader picks up,
  // so the swatch hue + the actual node colour stay in lockstep.

  /// Node-class rows in render order. Top-level groups (Files,
  /// Containers, Graph relations) are visual organizers; each
  /// `kind` row hosts the label + swatch. The palette token is the
  /// CSS variable name the swatch reads (`background: var(--g-X)`).
  type LegendRow = {
    label: string;
    cssVar: string;
    description?: string;
  };
  type LegendGroup = {
    title: string;
    rows: LegendRow[];
  };

  const groups: LegendGroup[] = [
    {
      title: "Files",
      rows: [
        {
          label: "Markdown",
          cssVar: "--g-doc",
          description: ".md / .txt",
        },
        {
          label: "Source code",
          cssVar: "--g-source",
          description: ".rs / .py / .ts / config",
        },
        {
          label: "Binary",
          cssVar: "--g-binary",
          description: "archives / executables / other",
        },
        {
          label: "Media",
          cssVar: "--g-img",
          description: "images / PDFs",
        },
        {
          label: "Contact",
          cssVar: "--warn-text",
          description: "chan.kind: contact",
        },
      ],
    },
    {
      title: "Containers",
      rows: [
        {
          label: "Directory",
          cssVar: "--g-folder",
          description: "filesystem dir + drive root",
        },
      ],
    },
    {
      title: "Graph relations",
      rows: [
        {
          label: "Hashtag",
          cssVar: "--g-tag",
          description: "#tag",
        },
        {
          label: "Mention",
          cssVar: "--warn-text",
          description: "@@mention",
        },
        {
          label: "Language",
          cssVar: "--g-language",
          description: "tokei language nodes",
        },
      ],
    },
  ];
</script>

<section class="hybrid-config" aria-label="Hybrid Graph configuration">
  <header class="config-header">
    <h2 class="config-title">Hybrid Graph</h2>
  </header>
  <div class="config-body">
    <p class="hint">
      Colour scheme for graph nodes. Same palette renders on the
      graph canvas + here; per-Hybrid Appearance overrides cascade
      through automatically.
    </p>

    {#each groups as group (group.title)}
      <section class="legend-group">
        <h3>{group.title}</h3>
        <ul class="legend-grid">
          {#each group.rows as row (row.label)}
            <li class="legend-row">
              <span class="legend-label">
                <span class="legend-name">{row.label}</span>
                {#if row.description}
                  <span class="legend-desc">{row.description}</span>
                {/if}
              </span>
              <span
                class="legend-swatch"
                style="background: var({row.cssVar})"
                aria-hidden="true"
              ></span>
            </li>
          {/each}
        </ul>
      </section>
    {/each}
  </div>
</section>

<style>
  .hybrid-config {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  .config-header {
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }
  .config-title {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--text);
  }
  .config-body {
    flex: 1;
    overflow: auto;
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .hint {
    margin: 0;
    color: var(--text-secondary);
    font-size: 13px;
  }
  .legend-group h3 {
    margin: 0 0 0.4rem 0;
    font-size: 12px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
  }
  .legend-grid {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .legend-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    padding: 4px 8px;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: 4px;
  }
  .legend-label {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }
  .legend-name {
    font-size: 14px;
    color: var(--text);
  }
  .legend-desc {
    font-size: 12px;
    color: var(--text-secondary);
  }
  .legend-swatch {
    flex-shrink: 0;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 1px solid var(--border);
  }
</style>
