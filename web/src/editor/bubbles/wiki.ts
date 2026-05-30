// Wiki bubble: file picker for the `[[query` trigger, plus heading
// picker for `[[target#headingFilter`.
//
// Mode switch happens automatically inside setQuery: a `#` in the
// query splits it into target + headingFilter and switches the bubble
// to heading mode, fetching /api/headings/{target} once per target
// (in-memory filter on subsequent typing). When the user backspaces
// past the `#` the bubble drops back to file mode.
//
// File mode: /api/link-targets for the query (debounced), top
// file/heading results. File hits commit `[[target]]`; heading hits
// commit `[[target#anchor]]`.
// Heading mode: filtered headings list, commit replaces `[[query`
// with `[[target#anchor]]` where anchor is the slug returned by the
// server (so the on-disk link survives a heading rename without
// chasing the source text).
// Block mode: parses `^id` markers out of the target file body via
// wikiBlocks.parseBlocks; commit either reuses an existing anchor or
// CAS-writes a fresh one onto the picked paragraph.
//
// Errors render in the status footer; the list stays empty so nothing
// commits. Stale fetches drop via reqSeq.

import type { EditorView } from "@codemirror/view";
import { openBubbleShell } from "../bubble";
import type { BubbleHandle } from "./types";
import { createCaretAnchor } from "./anchor";
import { api } from "../../api/client";
import type { LinkTarget } from "../../api/types";
import { indexStatus } from "../../state/store.svelte";
import {
  filterBlocks,
  insertBlockAnchor,
  makeBlockId,
  parseBlocks,
  type ParsedBlock,
} from "../extensions/wikiBlocks";
import { completionEmptyState, renderBubbleEmptyState } from "./empty_state";

export interface WikiBubbleOpts {
  view: EditorView;
  triggerStart: number;
  triggerEnd: number;
  initialQuery: string;
  /// Reserved path scope from the host. File mode currently uses
  /// /api/link-targets globally so title/heading matches are visible.
  prefix: string | null;
  /// "wrap" (default): commit inserts `[[path]]`. Used when the user
  /// typed `[[` from scratch.
  /// "raw": commit inserts just `path`. Used when the caret is inside
  /// an existing `[label](path)` URL portion (the brackets stay).
  templateMode?: "wrap" | "raw";
  /// Cmd+Enter handler. Called with the currently-selected hit's
  /// target (or the trigger's parsed target if no hit is selected).
  /// Returns the navigation surface to the host (FileEditorTab calls
  /// openInActivePane). Optional; when omitted, Cmd+Enter is a no-op.
  onOpenLink?: (target: string, anchor: string | null) => void;
  onDismiss: () => void;
}

const SEARCH_LIMIT = 5;
const HEADING_LIMIT = 8;
const BLOCK_LIMIT = 8;
const FETCH_DEBOUNCE_MS = 60;

interface HeadingHit {
  level: number;
  text: string;
  anchor: string;
}

interface WikiBubbleHandle extends BubbleHandle {
  setTriggerEnd(end: number): void;
}

type Mode =
  | { kind: "file" }
  | { kind: "heading"; target: string; filter: string }
  | { kind: "block"; target: string; filter: string };

/// Split the query into target + anchor-filter. Whichever separator
/// (`#` for heading, `^` for block) appears FIRST wins. `target^id`
/// switches to block mode; `target#text` to heading mode. The user
/// can backspace past the separator to drop back to file mode.
function classifyQuery(q: string): Mode {
  const headIdx = q.indexOf("#");
  const blockIdx = q.indexOf("^");
  const idx =
    blockIdx < 0 ? headIdx : headIdx < 0 ? blockIdx : Math.min(blockIdx, headIdx);
  if (idx < 0) return { kind: "file" };
  const target = q.slice(0, idx);
  const filter = q.slice(idx + 1);
  if (q[idx] === "^") return { kind: "block", target, filter };
  return { kind: "heading", target, filter };
}

export function openWikiBubble(opts: WikiBubbleOpts): WikiBubbleHandle {
  // Anchor at the opening `[[` (triggerStart) so the bubble lines up
  // with where the user started the link. Visually this reads better
  // than anchoring at the live caret, which would slide the bubble
  // sideways as the query grows and leave it floating off-axis from
  // the link being authored. positionPopover already clamps the
  // bubble inside the viewport when the trigger sits near the right
  // edge, so the only cost is the long-query wrap case (the bubble
  // can overlap the wrapped text); short queries are the common path.
  const anchorPos = (): number => opts.triggerStart;
  const anchor = createCaretAnchor(opts.view, anchorPos());
  const shell = openBubbleShell({
    host: anchor.el,
    className: "md-wiki-bubble cm-bubble",
  });

  let query = opts.initialQuery;
  let triggerEnd = opts.triggerEnd;
  let mode: Mode = classifyQuery(query);
  // File-mode results.
  let fileHits: LinkTarget[] = [];
  // Heading-mode all-headings cache (keyed by target so target switch
  // re-fetches only when needed) + filtered display list.
  let headingTarget: string | null = null;
  let headingAll: HeadingHit[] = [];
  let headingHits: HeadingHit[] = [];
  // Block-mode cache: parsed blocks + the file's mtime (needed for the
  // CAS write when committing a block that doesn't yet have an `^id`).
  let blockTarget: string | null = null;
  let blockAll: ParsedBlock[] = [];
  let blockOriginalText = "";
  let blockMtime: number | null = null;
  let blockMtimeNs: string | null = null;
  let blockHits: ParsedBlock[] = [];
  let selectedIndex = 0;
  let reqSeq = 0;
  let debounceTimer: number | undefined;
  let alive = true;

  const list = document.createElement("div");
  list.className = "md-bubble-list";
  shell.wrap.appendChild(list);
  const status = document.createElement("div");
  status.className = "md-bubble-status";
  shell.wrap.appendChild(status);

  function activeHits(): Array<LinkTarget | HeadingHit | ParsedBlock> {
    if (mode.kind === "heading") return headingHits;
    if (mode.kind === "block") return blockHits;
    return fileHits;
  }

  function render(): void {
    list.innerHTML = "";
    list.classList.remove("md-bubble-empty-state");
    const hits = activeHits();
    if (hits.length === 0) {
      if (mode.kind === "file") {
        renderBubbleEmptyState(list, completionEmptyState(query, indexStatus.value));
        status.textContent = "";
        status.classList.add("md-bubble-status-empty");
      } else if (mode.kind === "heading") {
        status.textContent = headingTarget === mode.target
          ? "No matching headings"
          : "Loading headings...";
        status.classList.remove("md-bubble-status-empty");
      } else {
        status.textContent = blockTarget === mode.target
          ? "No matching blocks"
          : "Loading blocks...";
        status.classList.remove("md-bubble-status-empty");
      }
      shell.reposition();
      return;
    }
    status.classList.remove("md-bubble-status-empty");
    const openHint = opts.onOpenLink ? " - ⌘↵ open" : "";
    if (mode.kind === "heading") {
      status.textContent = `${hits.length} heading${hits.length === 1 ? "" : "s"} in ${mode.target} - ↵ insert${openHint}`;
    } else if (mode.kind === "block") {
      status.textContent = `${hits.length} block${hits.length === 1 ? "" : "s"} in ${mode.target} - ↵ insert${openHint}`;
    } else {
      status.textContent = `${hits.length} result${hits.length === 1 ? "" : "s"} - ↵ insert${openHint}`;
    }
    for (let i = 0; i < hits.length; i++) {
      const hit = hits[i]!;
      const row = document.createElement("div");
      row.className = "md-bubble-row";
      if (i === selectedIndex) row.classList.add("md-bubble-row-selected");
      if (mode.kind === "heading") {
        const h = hit as HeadingHit;
        const level = document.createElement("span");
        level.className = "md-bubble-row-level";
        level.textContent = `H${h.level}`;
        const text = document.createElement("span");
        text.textContent = h.text;
        row.appendChild(level);
        row.appendChild(text);
      } else if (mode.kind === "block") {
        const b = hit as ParsedBlock;
        const tag = document.createElement("span");
        tag.className = "md-bubble-row-level";
        tag.textContent = b.existingAnchor ? "ID" : "BLK";
        const text = document.createElement("span");
        // First line of the block, ellipsised if very long. Anchor
        // suffix is stripped from the visible text since the row tag
        // already signals it.
        const firstLine = b.text.split("\n")[0] ?? "";
        text.textContent = firstLine.replace(/\s*\^[A-Za-z0-9-]{4,}\s*$/, "");
        row.appendChild(tag);
        row.appendChild(text);
      } else {
        const t = hit as LinkTarget;
        if (t.kind === "Heading") {
          const level = document.createElement("span");
          level.className = "md-bubble-row-level";
          level.textContent = t.level ? `H${t.level}` : "H";
          const text = document.createElement("span");
          text.textContent = t.heading ?? t.path;
          const path = document.createElement("span");
          path.className = "md-bubble-row-sub";
          path.textContent = ` ${t.path}`;
          row.appendChild(level);
          row.appendChild(text);
          row.appendChild(path);
        } else {
          const title = t.title?.trim();
          row.textContent = title && title !== t.path ? `${title} - ${t.path}` : t.path;
        }
      }
      row.addEventListener("mousedown", (e) => {
        e.preventDefault();
        e.stopPropagation();
        commit(hit);
      });
      list.appendChild(row);
    }
    shell.reposition();
  }

  function fetchFile(): void {
    if (debounceTimer !== undefined) clearTimeout(debounceTimer);
    const seq = ++reqSeq;
    debounceTimer = window.setTimeout(() => {
      api
        .linkTargets(query, SEARCH_LIMIT)
        .then((results) => {
          if (!alive || seq !== reqSeq || mode.kind !== "file") return;
          fileHits = results;
          if (selectedIndex >= fileHits.length) selectedIndex = 0;
          render();
        })
        .catch((err) => {
          if (!alive || seq !== reqSeq) return;
          fileHits = [];
          status.textContent = `Search failed: ${err.message ?? err}`;
        });
    }, FETCH_DEBOUNCE_MS);
  }

  function fetchHeadings(target: string): void {
    if (headingTarget === target) {
      // Cache hit; just re-filter.
      filterHeadings();
      return;
    }
    if (debounceTimer !== undefined) clearTimeout(debounceTimer);
    const seq = ++reqSeq;
    headingAll = [];
    headingHits = [];
    headingTarget = null;
    render();
    api
      .headings(target)
      .then((results) => {
        if (!alive || seq !== reqSeq) return;
        if (mode.kind !== "heading" || mode.target !== target) return;
        headingTarget = target;
        headingAll = results as HeadingHit[];
        filterHeadings();
      })
      .catch((err) => {
        if (!alive || seq !== reqSeq) return;
        headingAll = [];
        headingHits = [];
        status.textContent = `Headings failed: ${err.message ?? err}`;
      });
  }

  function filterHeadings(): void {
    if (mode.kind !== "heading") return;
    const f = mode.filter.toLowerCase();
    headingHits = (f.length === 0
      ? headingAll
      : headingAll.filter((h) => h.text.toLowerCase().includes(f))
    ).slice(0, HEADING_LIMIT);
    if (selectedIndex >= headingHits.length) selectedIndex = 0;
    render();
  }

  function fetchBlocks(target: string): void {
    if (blockTarget === target) {
      filterBlocksLocal();
      return;
    }
    if (debounceTimer !== undefined) clearTimeout(debounceTimer);
    const seq = ++reqSeq;
    blockAll = [];
    blockHits = [];
    blockTarget = null;
    blockOriginalText = "";
    blockMtime = null;
    blockMtimeNs = null;
    render();
    api
      .read(target)
      .then((res) => {
        if (!alive || seq !== reqSeq) return;
        if (mode.kind !== "block" || mode.target !== target) return;
        blockTarget = target;
        blockOriginalText = res.content;
        blockMtime = res.mtime;
        blockMtimeNs = res.mtime_ns ?? null;
        blockAll = parseBlocks(res.content);
        filterBlocksLocal();
      })
      .catch((err) => {
        if (!alive || seq !== reqSeq) return;
        blockAll = [];
        blockHits = [];
        status.textContent = `Blocks failed: ${err.message ?? err}`;
      });
  }

  function filterBlocksLocal(): void {
    if (mode.kind !== "block") return;
    blockHits = filterBlocks(blockAll, mode.filter, BLOCK_LIMIT);
    if (selectedIndex >= blockHits.length) selectedIndex = 0;
    render();
  }

  function refetchForMode(): void {
    if (mode.kind === "file") {
      fetchFile();
    } else if (mode.kind === "heading") {
      fetchHeadings(mode.target);
    } else {
      fetchBlocks(mode.target);
    }
  }

  function commit(hit: LinkTarget | HeadingHit | ParsedBlock): void {
    const raw = opts.templateMode === "raw";
    if (mode.kind === "block") {
      // Block commit may need a fresh `^id` written to the target file.
      // Run async, then dispatch the link insert. The bubble stays
      // open during the write; show a transient status.
      const block = hit as ParsedBlock;
      void commitBlock(block, raw);
      return;
    }
    let insert: string;
    if (mode.kind === "heading") {
      const h = hit as HeadingHit;
      const ref = `${mode.target}#${h.anchor}`;
      insert = raw ? ref : `[[${ref}]]`;
    } else {
      const ref = linkTargetRef(hit as LinkTarget);
      insert = raw ? ref : `[[${ref}]]`;
    }
    opts.view.dispatch({
      changes: { from: opts.triggerStart, to: triggerEnd, insert },
      selection: { anchor: opts.triggerStart + insert.length },
    });
    dismiss();
  }

  function linkTargetRef(hit: LinkTarget): string {
    if (hit.kind === "Heading" && hit.anchor) {
      return `${hit.path}#${hit.anchor}`;
    }
    return hit.path;
  }

  async function commitBlock(
    block: ParsedBlock,
    raw: boolean,
  ): Promise<void> {
    if (mode.kind !== "block" || !blockTarget) return;
    const target = mode.target;
    let anchorId: string;
    if (block.existingAnchor) {
      // Strip leading `^`: we store the anchor as `^id` in
      // ParsedBlock but the link form is `target^id` (no double ^).
      anchorId = block.existingAnchor.replace(/^\^/, "");
    } else {
      // Generate a fresh id, write to the target file via CAS, then
      // insert the link. If the write fails (mtime conflict, network),
      // surface the error and leave the bubble open.
      anchorId = makeBlockId();
      const newContent = insertBlockAnchor(blockOriginalText, block, anchorId);
      status.textContent = "Adding anchor...";
      try {
        const res = await api.write(target, newContent, blockMtimeNs, blockMtime);
        if (!alive) return;
        blockOriginalText = newContent;
        blockMtime = res.mtime;
        blockMtimeNs = res.mtime_ns ?? null;
      } catch (err: unknown) {
        const msg =
          err instanceof Error ? err.message : String(err);
        status.textContent = `Anchor write failed: ${msg}`;
        return;
      }
    }
    const ref = `${target}^${anchorId}`;
    const insert = raw ? ref : `[[${ref}]]`;
    opts.view.dispatch({
      changes: { from: opts.triggerStart, to: triggerEnd, insert },
      selection: { anchor: opts.triggerStart + insert.length },
    });
    dismiss();
  }

  function openSelected(): void {
    if (!opts.onOpenLink) return;
    const hits = activeHits();
    const hit = hits[selectedIndex];
    let target: string;
    let anchor: string | null = null;
    if (mode.kind === "heading") {
      target = mode.target;
      if (hit) anchor = (hit as HeadingHit).anchor;
    } else if (mode.kind === "block") {
      target = mode.target;
      if (hit) {
        const b = hit as ParsedBlock;
        if (b.existingAnchor) anchor = b.existingAnchor; // includes leading ^
        // No anchor write happens on Cmd+Enter; opening doesn't
        // mutate the target file. The host just navigates to the
        // file (anchor scroll TBD).
      }
    } else if (hit) {
      const linkTarget = hit as LinkTarget;
      target = linkTarget.path;
      if (linkTarget.kind === "Heading") anchor = linkTarget.anchor ?? null;
    } else {
      target = query;
    }
    opts.onOpenLink(target, anchor);
    dismiss();
  }

  function dismiss(): void {
    if (!alive) return;
    alive = false;
    if (debounceTimer !== undefined) clearTimeout(debounceTimer);
    shell.dismiss();
    anchor.dismiss();
    opts.onDismiss();
  }

  // Initial fetch for whichever mode the opening query parses to.
  refetchForMode();
  render();

  return {
    handleKey(event: KeyboardEvent): boolean {
      if (event.key === "Escape") {
        dismiss();
        return true;
      }
      const hits = activeHits();
      if (event.key === "Enter") {
        // Cmd/Ctrl+Enter -> open the selected hit (or trigger target)
        // instead of committing a replace. The host wires this to
        // openInActivePane via onOpenLink.
        if (event.metaKey || event.ctrlKey) {
          if (opts.onOpenLink) {
            openSelected();
            return true;
          }
          return false;
        }
        const hit = hits[selectedIndex];
        if (hit) {
          commit(hit);
          return true;
        }
        return false;
      }
      if (event.key === "ArrowDown") {
        if (hits.length === 0) return false;
        selectedIndex = (selectedIndex + 1) % hits.length;
        render();
        return true;
      }
      if (event.key === "ArrowUp") {
        if (hits.length === 0) return false;
        selectedIndex = (selectedIndex - 1 + hits.length) % hits.length;
        render();
        return true;
      }
      return false;
    },
    setQuery(q: string): void {
      // Re-anchor on every spec update. Anchor pos is the trigger's
      // opening `[[`, which only moves when an upstream edit shifts
      // the whole line; recomputing keeps the bubble glued to its
      // marker through those shifts.
      anchor.update(opts.view, anchorPos());
      shell.reposition();
      if (q === query) return;
      query = q;
      const newMode = classifyQuery(q);
      const modeChanged = newMode.kind !== mode.kind;
      const targetChanged =
        newMode.kind === "heading" &&
        mode.kind === "heading" &&
        newMode.target !== mode.target;
      const blockTargetChanged =
        newMode.kind === "block" &&
        mode.kind === "block" &&
        newMode.target !== mode.target;
      mode = newMode;
      selectedIndex = 0;
      if (modeChanged || targetChanged || blockTargetChanged) {
        refetchForMode();
      } else if (mode.kind === "heading") {
        // Same target, filter changed: local re-filter only.
        filterHeadings();
      } else if (mode.kind === "block") {
        filterBlocksLocal();
      } else {
        fetchFile();
      }
    },
    setTriggerEnd(end: number): void {
      triggerEnd = end;
    },
    reposition(): void {
      anchor.update(opts.view, anchorPos());
      shell.reposition();
    },
    dismiss,
  };
}
