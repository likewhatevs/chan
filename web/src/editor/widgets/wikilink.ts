// Wikilink atom widget for `[[note|alias#anchor]]` AND internal
// `[label](path)` markdown links.
//
// Per design.md spec #5, both forms are atomic widgets when the path
// is internal (workspace-relative or workspace-rooted). External markdown
// links (http://, https://, mailto:, etc.) stay handled by
// decorations/marks.ts handleLink.
//
// Body parsing differs:
//   - WikiLink: body text inside `[[...]]` is `target|alias` with
//     optional `#anchor` / `^block`.
//   - Link: label is between `[`/`]`, URL is between `(`/`)`. The
//     URL may carry an `#anchor` fragment.
//
// In both cases we end up with a ParsedWikiLink: { target, label,
// anchor, wasAbs }. Click fires onWikiClick.
//
// Kind cache: module-scoped Map<canonical-target, kind>. `kind` is
// "file" | "contact" | "image" | "broken". Image kind is detected
// synchronously via isImagePath (file extension). The other three
// resolve via an async GET /api/resolve-link; while in-flight the
// pill renders without a kind (default file styling). On resolve we
// dispatch a kindResolvedEffect on every active editor view so the
// decoration walker re-runs and the pill re-renders with the right
// data-refkind.
//
// Cache lives at module scope, so multiple files / multiple editor
// mounts share resolved kinds. Targets are canonicalized via
// normalizeHref against the editing file's directory, so
// `[label](./foo.md)` from `notes/a.md` and `[label](notes/foo.md)`
// from anywhere both hit the same `notes/foo.md` cache key.

import {
  Decoration,
  type DecorationSet,
  EditorView,
  ViewPlugin,
  type ViewUpdate,
  WidgetType,
} from "@codemirror/view";
import { syntaxTree } from "@codemirror/language";
import { type Extension, StateEffect } from "@codemirror/state";
import { selectionInRange } from "../decorations/selection";
import { normalizeHref } from "../links";
import { isImagePath, resolveImageSrc } from "../extensions/image";
import { api } from "../../api/client";
import { openPreviewPopover } from "../overlays/preview_popover";

export type LinkKind = "file" | "contact" | "image" | "broken";

/// Build a small lucide `user` icon as a stand-alone SVG node.
/// Inline because the wikilink widget builds DOM directly (no svelte
/// runtime); we'd lose the per-pill class and a11y attrs by mounting
/// a Svelte component just for the glyph. Stroke = currentColor so
/// the icon picks up the pill's text colour automatically.
const SVG_NS = "http://www.w3.org/2000/svg";
function makeUserIcon(): SVGElement {
  const svg = document.createElementNS(SVG_NS, "svg");
  svg.setAttribute("class", "cm-md-wiki-pill-icon");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("fill", "none");
  svg.setAttribute("stroke", "currentColor");
  svg.setAttribute("stroke-width", "2");
  svg.setAttribute("stroke-linecap", "round");
  svg.setAttribute("stroke-linejoin", "round");
  svg.setAttribute("aria-hidden", "true");
  const path = document.createElementNS(SVG_NS, "path");
  path.setAttribute("d", "M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2");
  const circle = document.createElementNS(SVG_NS, "circle");
  circle.setAttribute("cx", "12");
  circle.setAttribute("cy", "7");
  circle.setAttribute("r", "4");
  svg.append(path, circle);
  return svg;
}

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
  /// Read the editing file's workspace-rooted path. Used to canonicalize
  /// internal `[label](path)` URLs so the kind cache key is stable
  /// across files that reference the same target via different
  /// relative paths.
  getCurrentPath?: () => string | null;
}

/// Parse the raw inner text of a `[[wikilink|alias#anchor]]` body.
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
    anchor = body.slice(anchorIdx + (body[anchorIdx] === "#" ? 1 : 0));
    body = body.slice(0, anchorIdx);
  }
  const target = body.trim();
  const wasAbs = target.startsWith("/");
  const displayLabel =
    label ?? (target.split("/").pop() ?? target).replace(/\.md$/, "");
  return { target, label: displayLabel, anchor, wasAbs };
}

/// Percent-decode a URL path, mirroring pulldown-cmark's destination
/// decoding on the backend. Returns the input unchanged when it carries
/// no escapes or when an escape is malformed (decodeURIComponent throws
/// on a stray `%`), so a literal path is never corrupted.
function decodePercent(s: string): string {
  if (!s.includes("%")) return s;
  try {
    return decodeURIComponent(s);
  } catch {
    return s;
  }
}

/// Detect whether a markdown link URL is internal (workspace-relative or
/// workspace-rooted, no scheme prefix). Returns the parsed parts on
/// success, null otherwise.
export function parseInternalLink(
  url: string,
  label: string,
  fromPath: string | null,
): ParsedWikiLink | null {
  if (!url) return null;
  // Bail on scheme-prefixed URLs (http://, https://, mailto:, etc.)
  // and intra-doc fragments (`#section` alone).
  if (/^[a-z][a-z0-9+.-]*:/i.test(url)) return null;
  if (url.startsWith("#")) return null;
  // Split anchor (everything after the first `#` in the URL portion).
  const hashIdx = url.indexOf("#");
  const rawPath = hashIdx >= 0 ? url.slice(0, hashIdx) : url;
  const anchor = hashIdx >= 0 ? url.slice(hashIdx + 1) : "";
  if (!rawPath) return null;
  // Percent-decode the destination before resolving. On disk we write
  // relative-markdown URLs with the path percent-encoded (a filename
  // with a space becomes `Brazilian%20Rice.md`), and the backend graph
  // scanner (pulldown-cmark) decodes the destination before resolving.
  // The editor must decode too or the pill resolves `Brazilian%20Rice`
  // (no such file) and renders as a broken link even though the on-disk
  // edge is valid. Malformed escapes fall back to the raw path.
  const path = decodePercent(rawPath);
  const sourceDir = fromPath ? fromPath.split("/").slice(0, -1).join("/") : "";
  const target = normalizeHref(path, sourceDir);
  if (target === null) return null;
  const wasAbs = path.startsWith("/");
  const displayLabel =
    label.trim() || (target.split("/").pop() ?? target).replace(/\.md$/, "");
  return { target, label: displayLabel, anchor, wasAbs };
}

// ---- kind cache ----------------------------------------------------------

const kindCache = new Map<string, LinkKind>();
const inflight = new Set<string>();
const watchedViews = new Set<EditorView>();

/// State effect dispatched after a kind resolves so each registered
/// view's decoration walker re-runs and picks up the new kind. Module-
/// scoped so cross-mount resolves still propagate.
const kindResolvedEffect = StateEffect.define<void>();

function registerView(view: EditorView): void {
  watchedViews.add(view);
}

function unregisterView(view: EditorView): void {
  watchedViews.delete(view);
}

function broadcastKindResolved(): void {
  for (const v of watchedViews) {
    if (!v.dom.isConnected) {
      watchedViews.delete(v);
      continue;
    }
    v.dispatch({ effects: kindResolvedEffect.of(undefined) });
  }
}

/// Look up a target's kind. Returns the cached kind synchronously, or
/// undefined while an async resolve is in flight (the pill renders
/// uncolored until the resolve lands and broadcasts a re-render).
function getKind(target: string): LinkKind | undefined {
  const cached = kindCache.get(target);
  if (cached !== undefined) return cached;
  // Synchronous image detection by file extension.
  if (isImagePath(target)) {
    kindCache.set(target, "image");
    return "image";
  }
  // Async resolve - only one in-flight request per target.
  if (inflight.has(target)) return undefined;
  inflight.add(target);
  api
    .resolveLink(target)
    .then((res) => {
      kindCache.set(target, res.kind);
    })
    .catch(() => {
      kindCache.set(target, "broken");
    })
    .finally(() => {
      inflight.delete(target);
      broadcastKindResolved();
    });
  return undefined;
}

// ---- widget --------------------------------------------------------------

class WikiLinkWidget extends WidgetType {
  constructor(
    readonly parsed: ParsedWikiLink,
    readonly kind: LinkKind | undefined,
    readonly sourceLen: number,
    readonly onClick: (args: WikiLinkClickArgs) => void,
  ) {
    super();
  }

  eq(other: WikiLinkWidget): boolean {
    return (
      this.parsed.target === other.parsed.target &&
      this.parsed.label === other.parsed.label &&
      this.parsed.anchor === other.parsed.anchor &&
      this.kind === other.kind &&
      this.sourceLen === other.sourceLen
    );
  }

  toDOM(view: EditorView): HTMLElement {
    const el = document.createElement("span");
    el.className = "cm-md-wiki-pill";
    el.dataset.target = this.parsed.target;
    if (this.parsed.anchor) el.dataset.anchor = this.parsed.anchor;
    if (this.kind) el.dataset.refkind = this.kind;
    if (this.kind === "image") {
      // Image-kind wikilinks render the actual file as an inline
      // thumbnail rather than a text pill - a `[[Recipes/photo.jpg]]`
      // link to a media asset is more useful when you can see what
      // it points to. The pill stays a wrapping span so click +
      // selection-intersect behavior still applies; the only change
      // is the inner content.
      el.classList.add("cm-md-wiki-pill-image");
      const img = document.createElement("img");
      img.alt = this.parsed.label;
      const resolved = resolveImageSrc(this.parsed.target, null);
      if (resolved) img.src = resolved;
      img.draggable = false;
      el.replaceChildren(img);
    } else if (this.kind === "contact") {
      // Contact-kind wikilinks lead with a lucide `user` icon so the
      // pill reads as "a person" at a glance, matching the file-tab
      // icon and the file-browser inspector chip. Plain text label
      // follows the icon.
      el.classList.add("cm-md-wiki-pill-contact");
      el.replaceChildren(makeUserIcon(), document.createTextNode(this.parsed.label));
    } else {
      el.textContent = this.parsed.label;
    }
    el.addEventListener("mousedown", (e) => {
      if (e.button !== 0) return;
      e.preventDefault();
      e.stopPropagation();
      // Read-only mode (chat replies, user-toggled read mode, an
      // fs-locked file) replaces the source-reveal-and-edit path
      // with a non-destructive preview: click pops a popover with
      // the target file's content (markdown rendered) or, for
      // image targets, the image inline. Cmd/Ctrl+Enter inside
      // the popover (or the Open button) commits to fully opening
      // the file. Same widget covers chat / read toggle / fs-lock
      // via the live editable facet.
      const editable = view.state.facet(EditorView.editable);
      if (!editable) {
        // Cmd/Ctrl-click skips the preview and opens directly -         // power-user shortcut that matches the write-mode
        // Cmd-click semantics.
        if (e.metaKey || e.ctrlKey) {
          this.onClick({ ...this.parsed, openInNewPane: true });
          return;
        }
        const parsed = this.parsed;
        const onClick = this.onClick;
        openPreviewPopover({
          anchor: el,
          path: parsed.target,
          onOpen: (openInNewPane) =>
            onClick({ ...parsed, openInNewPane }),
        });
        return;
      }
      // Cmd/Ctrl-click navigates immediately (open-in-new-pane).
      // Plain click reveals the source AND lands the caret in a
      // position the trigger detector recognizes - the wiki bubble
      // pops up with the existing target as the query, and the user
      // can pick a new target with Enter or open via Cmd+Enter.
      if (e.metaKey || e.ctrlKey) {
        this.onClick({ ...this.parsed, openInNewPane: true });
        return;
      }
      const pillFrom = view.posAtDOM(el);
      if (pillFrom < 0) {
        this.onClick({ ...this.parsed, openInNewPane: false });
        return;
      }
      // Caret position depends on form:
      //   [[...]]      -> just before the closing ]] (sourceLen-2)
      //   [label](url) -> middle of the URL portion (sourceLen-1
      //                   lands just before the closing `)`)
      // Both positions are inside an existing source range that the
      // trigger detector recognizes (matchBracket for [[, the new
      // internalLinkUrlAtCaret detector for [..](..)).
      const pillTo = pillFrom + this.sourceLen;
      const isWikiForm =
        view.state.doc.sliceString(pillFrom, pillFrom + 2) === "[[";
      const caret = isWikiForm ? pillTo - 2 : pillTo - 1;
      view.dispatch({ selection: { anchor: caret } });
      view.focus();
    });
    return el;
  }

  ignoreEvent(): boolean {
    return true;
  }
}

// ---- ViewPlugin ----------------------------------------------------------

export function wikiLinkDecorations(opts: WikiLinkOptions): Extension {
  const plugin = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;

      constructor(view: EditorView) {
        registerView(view);
        this.decorations = scanWikiLinks(view, opts);
      }

      update(u: ViewUpdate): void {
        const kindResolved = u.transactions.some((tr) =>
          tr.effects.some((e) => e.is(kindResolvedEffect)),
        );
        if (
          u.docChanged ||
          u.viewportChanged ||
          u.selectionSet ||
          kindResolved
        ) {
          this.decorations = scanWikiLinks(u.view, opts);
        }
      }

      destroy(): void {
        // Best-effort: the constructor closure doesn't carry the view
        // ref, so we just sweep stale entries (DOM disconnected) on
        // next broadcast.
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
  const fromPath = opts.getCurrentPath?.() ?? null;
  const decos: Array<{ from: number; to: number; deco: Decoration }> = [];
  syntaxTree(state).iterate({
    from,
    to,
    enter(node) {
      if (node.name === "WikiLink") {
        const outerFrom = node.from;
        const outerTo = node.to;
        if (selectionInRange(sel, outerFrom, outerTo)) return;
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
        const kind = getKind(parsed.target);
        decos.push({
          from: outerFrom,
          to: outerTo,
          deco: Decoration.replace({
            widget: new WikiLinkWidget(
              parsed,
              kind,
              outerTo - outerFrom,
              opts.onWikiClick,
            ),
          }),
        });
        return;
      }
      if (node.name === "Link") {
        // Internal-link branch: skip Image (it's a separate node) and
        // any Link without a URL child (reference-style).
        const outerFrom = node.from;
        const outerTo = node.to;
        if (selectionInRange(sel, outerFrom, outerTo)) return;
        const cursor = node.node.cursor();
        if (!cursor.firstChild()) return;
        type Range = { from: number; to: number };
        const linkMarks: Range[] = [];
        let urlFrom = -1;
        let urlTo = -1;
        do {
          if (cursor.name === "LinkMark") {
            linkMarks.push({ from: cursor.from, to: cursor.to });
          } else if (cursor.name === "URL") {
            urlFrom = cursor.from;
            urlTo = cursor.to;
          }
        } while (cursor.nextSibling());
        if (linkMarks.length < 4 || urlFrom < 0) return;
        const labelFrom = linkMarks[0]!.to;
        const labelTo = linkMarks[1]!.from;
        const label = state.doc.sliceString(labelFrom, labelTo);
        const url = state.doc.sliceString(urlFrom, urlTo);
        const parsed = parseInternalLink(url, label, fromPath);
        if (!parsed) return; // external - handled by decorations/marks.ts
        const kind = getKind(parsed.target);
        decos.push({
          from: outerFrom,
          to: outerTo,
          deco: Decoration.replace({
            widget: new WikiLinkWidget(
              parsed,
              kind,
              outerTo - outerFrom,
              opts.onWikiClick,
            ),
          }),
        });
        return;
      }
    },
  });
  decos.sort((a, b) => a.from - b.from);
  return Decoration.set(
    decos.map((d) => d.deco.range(d.from, d.to)),
    true,
  );
}

// Cleanup: prune disconnected views. Called periodically by the
// broadcaster anyway, but exported for tests.
export function _pruneWatchedViews(): void {
  for (const v of watchedViews) {
    if (!v.dom.isConnected) watchedViews.delete(v);
  }
}

// Expose unregister for symmetry, though current call sites rely on
// the prune-on-broadcast path.
export const _internal = { unregisterView };
