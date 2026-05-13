// Wikilink action popover. Click a wiki pill -> small floating panel
// anchored under the pill with:
//   - Target path (truncated, with optional #anchor)
//   - "Open" button (current pane)
//   - "Open in new pane" button
//   - "Edit source" button (places caret in the pill's source range
//     so the selection-intersect rule reveals `[[...]]` for in-place
//     edit)
//
// Replaces the previous "click -> immediately navigate" behavior. The
// extra round trip is cheap and gives the user a chance to copy the
// path or open in a side pane without keyboard modifiers.
//
// Lifetime: dismissed on outside click / Esc / scroll. Same shape as
// overlays/image_action.ts.

import type { EditorView } from "@codemirror/view";
import type { ParsedWikiLink, WikiLinkClickArgs } from "../widgets/wikilink";

export interface LinkActionOpts {
  view: EditorView;
  /// The pill DOM element (used as the position anchor).
  anchor: HTMLElement;
  parsed: ParsedWikiLink;
  /// Live document position of the pill's source range start. Used by
  /// "Edit source" to drop the caret inside.
  pillFrom: number;
  /// Live position of the pill's end (just after the closing `]]` or
  /// `)`). Used by "Edit source" to extend the selection so the
  /// selection-intersect rule reveals the whole source range.
  pillTo: number;
  onOpen: (args: WikiLinkClickArgs) => void;
}

export function openLinkAction(opts: LinkActionOpts): { dismiss: () => void } {
  const wrap = document.createElement("div");
  wrap.className = "md-link-action";
  wrap.style.position = "absolute";
  wrap.style.zIndex = "30000";
  document.body.appendChild(wrap);
  positionUnderAnchor();

  function positionUnderAnchor(): void {
    const rect = opts.anchor.getBoundingClientRect();
    const popH = wrap.offsetHeight;
    const viewH = window.innerHeight;
    const spaceBelow = viewH - rect.bottom;
    const spaceAbove = rect.top;
    const GAP = 4;
    let top: number;
    if (popH > 0 && spaceBelow < popH + GAP && spaceAbove > spaceBelow) {
      top = rect.top + window.scrollY - popH - GAP;
    } else {
      top = rect.bottom + window.scrollY + GAP;
    }
    let left = rect.left + window.scrollX;
    const popW = wrap.offsetWidth;
    if (popW > 0) {
      const maxLeft = window.scrollX + window.innerWidth - popW - GAP;
      if (left > maxLeft) left = Math.max(window.scrollX + GAP, maxLeft);
    }
    wrap.style.left = `${Math.round(left)}px`;
    wrap.style.top = `${Math.round(top)}px`;
  }

  let alive = true;

  function render(): void {
    wrap.innerHTML = "";
    const targetRow = document.createElement("div");
    targetRow.className = "md-link-action-target";
    targetRow.textContent = opts.parsed.anchor
      ? `${opts.parsed.target}#${opts.parsed.anchor}`
      : opts.parsed.target;
    wrap.appendChild(targetRow);
    const buttons = document.createElement("div");
    buttons.className = "md-link-action-buttons";
    buttons.appendChild(makeBtn("Open", () => {
      opts.onOpen({ ...opts.parsed, openInNewPane: false });
      dismiss();
    }));
    buttons.appendChild(makeBtn("Open in new pane", () => {
      opts.onOpen({ ...opts.parsed, openInNewPane: true });
      dismiss();
    }));
    buttons.appendChild(makeBtn("Edit source", () => {
      // Select the pill's range so the selection-intersect rule
      // reveals source. Pill range comes from the closure (captured
      // at click time); positions may have shifted by intervening
      // edits but for a fresh click the values are current.
      opts.view.dispatch({
        selection: { anchor: opts.pillFrom, head: opts.pillTo },
      });
      opts.view.focus();
      dismiss();
    }));
    wrap.appendChild(buttons);
    positionUnderAnchor();
  }

  function dismiss(): void {
    if (!alive) return;
    alive = false;
    document.removeEventListener("mousedown", outsideClick, true);
    document.removeEventListener("keydown", escListener, true);
    window.removeEventListener("scroll", dismiss, true);
    wrap.remove();
  }

  function outsideClick(e: MouseEvent): void {
    if (wrap.contains(e.target as Node)) return;
    if (opts.anchor.contains(e.target as Node)) return;
    dismiss();
  }

  function escListener(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      dismiss();
    }
  }

  window.setTimeout(() => {
    if (!alive) return;
    document.addEventListener("mousedown", outsideClick, true);
    document.addEventListener("keydown", escListener, true);
    window.addEventListener("scroll", dismiss, true);
  }, 0);

  render();
  return { dismiss };
}

function makeBtn(label: string, onClick: () => void): HTMLElement {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = "md-link-action-btn";
  btn.textContent = label;
  btn.addEventListener("mousedown", (e) => {
    e.preventDefault();
    e.stopPropagation();
    onClick();
  });
  return btn;
}
