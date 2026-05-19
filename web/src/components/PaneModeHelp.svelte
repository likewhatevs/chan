<script lang="ts">
  // Pane Mode cheatsheet. Shown while `h` is toggled inside Pane Mode
  // (`Cmd+K`). Read-only overlay: lists every key + its action,
  // grouped (Move / Spawn / Split / Close / Resize / Commit). Esc /
  // pressing `h` again hides it without committing the draft.
  //
  // Visual style: small, dense, TUI-density per `fullstack-42`. Not
  // an OverlayShell to keep the focus / Escape semantics inside
  // App.svelte; this is purely a passive informational panel.

  type Row = { keys: string; action: string };
  type Group = { title: string; rows: Row[] };

  const groups: Group[] = [
    {
      title: "Move",
      rows: [
        { keys: "↑ ← ↓ →", action: "Move focus" },
        { keys: "W A S D", action: "Swap tile with neighbour" },
      ],
    },
    {
      title: "Spawn",
      rows: [
        { keys: "1", action: "Terminal" },
        { keys: "2", action: "File Browser" },
        { keys: "3", action: "Graph" },
        { keys: "4", action: "New file" },
        { keys: "s", action: "Search overlay" },
      ],
    },
    {
      title: "Split",
      rows: [
        { keys: "/", action: "Split right" },
        { keys: "\\", action: "Split down" },
      ],
    },
    {
      title: "Close",
      rows: [
        { keys: "x", action: "Close all tabs in pane" },
        { keys: "k", action: "Kill pane" },
      ],
    },
    {
      title: "Resize",
      rows: [
        { keys: "[ ]", action: "Shrink / grow horizontally" },
        { keys: "- =", action: "Shrink / grow vertically" },
        { keys: "Shift + [ ] - =", action: "Larger nudge" },
        { keys: "0", action: "Equalize siblings" },
      ],
    },
    {
      title: "Commit",
      rows: [
        { keys: "Enter", action: "Commit draft" },
        { keys: "Esc", action: "Discard draft" },
        { keys: "h", action: "Toggle this help" },
      ],
    },
  ];
</script>

<div class="pane-mode-help" aria-label="Pane Mode help" role="dialog">
  <div class="title">Pane Mode</div>
  <div class="grid">
    {#each groups as g (g.title)}
      <section class="group">
        <h4>{g.title}</h4>
        <dl>
          {#each g.rows as row (row.keys)}
            <dt><kbd>{row.keys}</kbd></dt>
            <dd>{row.action}</dd>
          {/each}
        </dl>
      </section>
    {/each}
  </div>
  <div class="hint">Press <kbd>h</kbd> or <kbd>Esc</kbd> to dismiss</div>
</div>

<style>
  .pane-mode-help {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 26010;
    max-width: min(960px, 92vw);
    max-height: 80vh;
    overflow: auto;
    padding: 14px 18px 12px;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 18px 60px rgba(0, 0, 0, 0.45);
    font-size: 13px;
    pointer-events: auto;
  }
  .title {
    font-size: 14px;
    font-weight: 600;
    margin-bottom: 10px;
    letter-spacing: 0.02em;
    color: var(--text);
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 12px 24px;
  }
  .group h4 {
    margin: 0 0 4px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-secondary);
  }
  dl {
    margin: 0;
    display: grid;
    grid-template-columns: max-content 1fr;
    column-gap: 10px;
    row-gap: 2px;
    align-items: baseline;
  }
  dt {
    margin: 0;
  }
  dd {
    margin: 0;
    color: var(--text);
  }
  kbd {
    display: inline-block;
    padding: 1px 6px;
    font: 11px/1.5 var(--chan-editor-code-family, ui-monospace, monospace);
    color: var(--text);
    background: var(--bg-card, var(--bg));
    border: 1px solid var(--border);
    border-radius: 3px;
    white-space: nowrap;
  }
  .hint {
    margin-top: 10px;
    font-size: 11px;
    color: var(--text-secondary);
  }
</style>
