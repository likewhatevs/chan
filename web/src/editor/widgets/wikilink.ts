// Wikilink atom widget for `[[note|alias#anchor]]` syntax.
//
// Per design.md spec #5, wikilinks (the `[[...]]` form) are atomic
// widgets. Rendered as a pill with the label (or basename of target);
// click opens the target via callback. Selection-intersect rule per
// spec #4 — caret touching the source range suppresses the widget so
// the user can edit literally.
//
// `[label](path)` where path is internal also belongs to the atomic
// wikilink class per the spec, but v1 ships without the internal-
// detection branch — those render as external links via decorations/
// marks.ts handleLink. v1.1 polish: split internal `[label](path)`
// off into the wikilink atom path.
//
// Body parsing: `target|label#anchor` / `target|label^block`. Anchor
// can use `#` (heading slug) or `^` (block id). Label after `|` is
// optional; default = basename of target with `.md` stripped.
//
// kind-cache (v1.1): the legacy editor maintains a Map<target, "file" |
// "contact" | "image" | "broken"> populated via async /api/resolve-link
// calls so the pill can color-code. v1 omits the cache and renders all
// pills uniformly; the wikilink bubble (step 7) will handle async
// resolution at edit time.

import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
  WidgetType,
} from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";
import { type Extension } from "@codemirror/state";
import { selectionInRange } from "../decorations/selection";

export interface ParsedWikiLink {
  target: string;
  label: string;
  anchor: string;
  wasAbs: boolean;
}

export interface WikiLinkClickArgs extends ParsedWikiLink {
  openInNewPane: boolean;
}

export interface WikiLinkOptions {
  onWikiClick: (args: WikiLinkClickArgs) => void;
}

export function parseWikiBody(body: string): ParsedWikiLink {
  let label: string | null = null;
  let anchor = "";
  const pipeIdx = body.indexOf("|");
  if (pipeIdx !== -1) {
    label = body.slice(pipeIdx + 1).trim();
    body = body.slice(0, pipeIdx);
  }
  const blockIdx = body.indexOf("^");
  const headIdx = body.indexOf("#");
  const anchorIdx =
    blockIdx === -1
      ? headIdx
      : headIdx === -1
        ? blockIdx
        : Math.min(blockIdx, headIdx);
  if (anchorIdx !== -1) {
    anchor = body.slice(
      anchorIdx + (body[anchorIdx] === "#" ? 1 : 0),
    );
    body = body.slice(0, anchorIdx);
  }
  const target = body.trim();
  const wasAbs = target.startsWith("/");
  const displayLabel =
    label ?? (target.split("/").pop() ?? target).replace(/\.md$/, "");
  return { target, label: displayLabel, anchor, wasAbs };
}

class WikiLinkWidget extends WidgetType {
  constructor(
    readonly parsed: ParsedWikiLink,
    readonly onClick: (args: WikiLinkClickArgs) => void,
  ) {
    super();
  }

  eq(other: WikiLinkWidget): boolean {
    return (
      this.parsed.target === other.parsed.target &&
      this.parsed.label === other.parsed.label &&
      this.parsed.anchor === other.parsed.anchor
    );
  }

  toDOM(): HTMLElement {
    const el = document.createElement("span");
    el.className = "cm-md-wiki-pill";
    el.dataset.target = this.parsed.target;
    if (this.parsed.anchor) el.dataset.anchor = this.parsed.anchor;
    el.textContent = this.parsed.label;
    el.addEventListener("mousedown", (e) => {
      // Only handle plain / Cmd / Ctrl clicks. Right-click and other
      // modifiers stay with the editor's default behavior.
      if (e.button !== 0) return;
      e.preventDefault();
      e.stopPropagation();
      this.onClick({
        ...this.parsed,
        openInNewPane: e.metaKey || e.ctrlKey,
      });
    });
    return el;
  }

  ignoreEvent(): boolean {
    // Click is owned by our listener above; CM6 should not place the
    // caret as a side effect.
    return true;
  }
}

export function wikiLinkDecorations(opts: WikiLinkOptions): Extension {
  const plugin = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;

      constructor(view: EditorView) {
        this.decorations = scanWikiLinks(view, opts);
      }

      update(u: ViewUpdate): void {
        if (u.docChanged || u.viewportChanged || u.selectionSet) {
          this.decorations = scanWikiLinks(u.view, opts);
        }
      }
    },
    {
      decorations: (v) => v.decorations,
    },
  );
  return [
    plugin,
    EditorView.atomicRanges.of(
      (view) => view.plugin(plugin)?.decorations ?? Decoration.none,
    ),
  ];
}

function scanWikiLinks(
  view: EditorView,
  opts: WikiLinkOptions,
): DecorationSet {
  const { state } = view;
  const sel = state.selection;
  const { from, to } = view.viewport;
  const decos: Array<{ from: number; to: number; deco: Decoration }> = [];
  syntaxTree(state).iterate({
    from,
    to,
    enter(node) {
      if (node.name !== "WikiLink") return;
      const outerFrom = node.from;
      const outerTo = node.to;
      if (selectionInRange(sel, outerFrom, outerTo)) return; // reveal source
      // Find the WikiLinkBody child to read the body text.
      const cursor = node.node.cursor();
      if (!cursor.firstChild()) return;
      let bodyFrom = -1;
      let bodyTo = -1;
      do {
        if (cursor.name === "WikiLinkBody") {
          bodyFrom = cursor.from;
          bodyTo = cursor.to;
          break;
        }
      } while (cursor.nextSibling());
      if (bodyFrom < 0 || bodyTo <= bodyFrom) return;
      const body = state.doc.sliceString(bodyFrom, bodyTo);
      const parsed = parseWikiBody(body);
      const widget = new WikiLinkWidget(parsed, opts.onWikiClick);
      decos.push({
        from: outerFrom,
        to: outerTo,
        deco: Decoration.replace({ widget }),
      });
    },
  });
  decos.sort((a, b) => a.from - b.from);
  return Decoration.set(
    decos.map((d) => d.deco.range(d.from, d.to)),
    true,
  );
}
