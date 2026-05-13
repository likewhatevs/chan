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
    caretLine = null,
    onSelect,
  }: {
    content: string;
    /// 0-indexed source line the caret sits on. When provided, the
    /// outline highlights the most recent heading at or above that
    /// line as "active" (Google-Docs-style current-position marker).
    /// Null disables the marker.
    caretLine?: number | null;
    onSelect: (h: Heading) => void;
  } = $props();

  const headings = $derived(parseHeadings(content));
  const activeIndex = $derived(computeActiveIndex(headings, caretLine));

  function computeActiveIndex(hs: Heading[], line: number | null): number | null {
    if (line == null || hs.length === 0) return null;
    let best: number | null = null;
    for (const h of hs) {
      if (h.line <= line) best = h.index;
      else break;
    }
    return best;
  }

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
      <li class:active={h.index === activeIndex}>
        <button
          class="row"
          style="padding-right: {(h.depth - 1) * 14 + 14}px"
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
  /* Google-Docs-style outline, mirrored for chan's right-side
     inspector: every heading hangs off a single vertical guide
     line at the RIGHT (toward the editor edge), with items
     extending leftward. The line is the same colour as the panel
     border so it reads as structure, not decoration. Depth is
     encoded purely in right-padding; the line itself does not
     branch per level. The 10px gap between the line and the
     panel edge gives the tree breathing room from the inspector
     chrome. */
  ul {
    list-style: none;
    margin: 0 10px 0 0;
    padding: 0.35rem 0;
    border-right: 1px solid var(--border);
  }
  li { position: relative; }
  /* Active-heading marker. The 2px accent bar sits on top of the
     ul's 1px guide line on the right edge (right: -1px overlaps by
     1px outward, the bar is 2px wide, so it fully covers the 1px
     guide and extends 1px further into the gutter). top/bottom =
     2px crop the bar to the row's vertical center. */
  li.active::before {
    content: "";
    position: absolute;
    right: -1px;
    top: 2px;
    bottom: 2px;
    width: 2px;
    background: var(--link);
    border-radius: 1px;
  }
  li.active .row { color: var(--link); font-weight: 500; }
  .row {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: 0;
    cursor: pointer;
    padding: 0.22rem 0.6rem;
    color: inherit;
    /* Use the editor theme's body family so the outline matches
       the document's typography. Size and weight are inherited
       from the inspector chrome so the outline reads as chrome,
       not as body text spilled into the side panel. */
    font-family: var(--chan-editor-body-family, inherit);
    font-size: inherit;
    font-weight: inherit;
    line-height: inherit;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    border-radius: 3px;
  }
  .row:hover { background: var(--hover-bg); }
</style>
