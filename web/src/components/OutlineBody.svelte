<script lang="ts" module>
  /// One ATX heading recognized in the buffer. Stable index across a
  /// single parse so the editor's WYSIWYG side can stamp matching
  /// `data-heading-id="h-<index>"` attributes.
  export type Heading = {
    /// 0-indexed position in document order.
    index: number;
    /// 0-indexed source line number.
    line: number;
    depth: number; // 1..6
    text: string;
  };
</script>

<script lang="ts">
  // Outline body for the inspector: parses ATX headings from the
  // buffer and renders them as an indented clickable list. Used by
  // file editor tabs; the host (Inspector) provides the chrome.
  //
  // Heading detection is a single regex pass with state for fenced
  // code blocks (so `# foo` inside a ``` block doesn't pollute the
  // outline). Cheap; no debounce needed.

  let {
    content,
    onSelect,
  }: {
    content: string;
    onSelect: (h: Heading) => void;
  } = $props();

  const headings = $derived(parseHeadings(content));

  function parseHeadings(src: string): Heading[] {
    const out: Heading[] = [];
    let inFence = false;
    let fenceMarker = "";
    const lines = src.split("\n");
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i] ?? "";
      const fence = line.match(/^(```+|~~~+)/);
      if (fence) {
        if (!inFence) {
          inFence = true;
          fenceMarker = fence[1] ?? "";
        } else if (line.startsWith(fenceMarker)) {
          inFence = false;
          fenceMarker = "";
        }
        continue;
      }
      if (inFence) continue;
      const m = line.match(/^(#{1,6})\s+(.+?)\s*#*\s*$/);
      if (!m) continue;
      out.push({
        index: out.length,
        line: i,
        depth: m[1]!.length,
        text: stripInlineMarkdown(m[2]!.trim()),
      });
    }
    return out;
  }

  /// Strip the common inline markdown markers from a heading's text
  /// so the outline renders as readable plain text. Order matters:
  /// `**bold**` runs first so `*x*` doesn't eat the inner `*` of a
  /// double-asterisk pair.
  function stripInlineMarkdown(text: string): string {
    return text
      // Bold ** ** and __ __
      .replace(/\*\*([^*\n]+?)\*\*/g, "$1")
      .replace(/__([^_\n]+?)__/g, "$1")
      // Italic * * and _ _
      .replace(/(?<![*_])\*([^*\n]+?)\*(?![*_])/g, "$1")
      .replace(/(?<![_*])_([^_\n]+?)_(?![_*])/g, "$1")
      // Strikethrough ~~ ~~
      .replace(/~~([^~\n]+?)~~/g, "$1")
      // Inline code ` `
      .replace(/`([^`\n]+?)`/g, "$1")
      // Wikilinks [[target|label]] / [[target]] -> label or target
      .replace(/\[\[([^\[\]\n|]+?)\|([^\[\]\n]+?)\]\]/g, "$2")
      .replace(/\[\[([^\[\]\n]+?)\]\]/g, (_, body: string) => {
        const last = body.split("/").pop() ?? body;
        return last.replace(/\.md$/, "");
      })
      // Markdown links [label](url) -> label
      .replace(/\[([^\[\]\n]+?)\]\([^)\n]+?\)/g, "$1");
  }
</script>

{#if headings.length === 0}
  <div class="empty">No headings yet</div>
{:else}
  <ul>
    {#each headings as h (h.index)}
      <li>
        <button
          class="row"
          style="padding-left: {(h.depth - 1) * 12 + 8}px"
          title={h.text}
          onclick={() => onSelect(h)}
        >{h.text}</button>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .empty {
    padding: 0.5rem 0.6rem;
    color: var(--text-secondary);
    font-style: italic;
  }
  ul { list-style: none; margin: 0; padding: 0.25rem 0; }
  .row {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    cursor: pointer;
    padding: 0.2rem 0.6rem;
    color: inherit;
    font: inherit;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    border-radius: 3px;
  }
  .row:hover { background: var(--hover-bg); }
</style>
