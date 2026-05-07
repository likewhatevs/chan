// @date smart node + picker UI.
//
// Typing `@date` opens an inline calendar popover with a format
// dropdown. Picking a date inserts a `date` node that displays the
// chosen date in the chosen format and serializes to plain text in
// the same format. Click an existing pill to re-open the picker.
//
// The catalog of formats lives in `../dateFormats.ts`; this file
// is kept as a thin TipTap shell so adding a new format never
// requires touching the node spec.
//
// Markdown round-trip:
//   - `format` chooses both the displayed text and the markdown
//     output (e.g. "Mon, 18 Feb" writes that exact string to disk).
//   - Document-load detection (Wysiwyg.svelte:decorateSmartNodes)
//     runs every catalog regex against text nodes, so a `02 Jan
//     2029` in the markdown re-pills with `format = medium` on the
//     next open.
//
// We use a tiny inline calendar (no extra dependency); the editor
// needs a date picker, not a fully-featured datepicker library.

import { Node, mergeAttributes } from "@tiptap/core";
import {
  DATE_FORMATS,
  dateFormat,
  dateFromIso,
  formatDate,
  isoOf,
  type DateFormatId,
} from "../dateFormats";

import { positionPopover, watchViewport } from "./popover";

export const DateNode = Node.create({
  name: "date",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,

  addAttributes() {
    return {
      // Canonical underlying date as YYYY-MM-DD. This is what the
      // pill carries internally regardless of which format renders
      // (so a year-less display "Mon, 18 Feb" still has a real
      // year on the attribute).
      date: {
        default: "",
        parseHTML: (el) => el.getAttribute("data-date") ?? "",
        renderHTML: (attrs) => ({ "data-date": attrs.date }),
      },
      // Format id from the catalog. Defaults to ISO so an entry
      // missing the attribute (legacy markdown, hand-written HTML)
      // renders predictably.
      format: {
        default: "iso",
        parseHTML: (el) => el.getAttribute("data-date-format") ?? "iso",
        renderHTML: (attrs) => ({ "data-date-format": attrs.format }),
      },
    };
  },

  parseHTML() {
    return [{ tag: "span[data-md-date]" }];
  },

  renderHTML({ HTMLAttributes, node }) {
    const iso = (node.attrs.date as string) || "";
    const fmtId = (node.attrs.format as string) || "iso";
    const d = dateFromIso(iso);
    const text = d ? formatDate(d, fmtId) : iso || "(date)";
    return [
      "span",
      mergeAttributes(HTMLAttributes, {
        "data-md-date": "true",
        class: "md-smart md-smart-date",
        title: "click to change",
      }),
      text,
    ];
  },

  addStorage() {
    return {
      markdown: {
        // On serialize: write the formatted string to disk. The
        // round-trip path is regex-driven (decorateSmartNodes), so
        // the disk text has to match one of the catalog patterns
        // for re-pilling on the next open.
        serialize(
          state: unknown,
          node: { attrs: { date: string; format: string } },
        ) {
          const iso = node.attrs.date || "";
          const d = dateFromIso(iso);
          const out = d ? formatDate(d, node.attrs.format || "iso") : iso;
          (state as { write(s: string): void }).write(out);
        },
        parse: { setup() {} },
      },
    };
  },
});

/// Build a small flyout calendar attached to `host`. Calls `pick` with
/// the chosen date plus the format id, or `null` to dismiss.
///
/// `initialFormat` seeds the format dropdown so @date picks up the
/// user's default and click-to-edit preserves the existing pill's
/// format.
///
/// Keyboard model:
///   ←/→     move cursor by a day
///   ↑/↓     move cursor by a week
///   PgUp/Dn move cursor by a month (Shift = year)
///   Home/End jump to first / last of the current month
///   T       jump cursor to today
///   Enter   pick the cursor's date
///   Esc     dismiss
///
/// The cursor is a separate Date from the displayed month so that
/// arrow-key motion across a month boundary scrolls the view too.
export type DatePick = { iso: string; format: DateFormatId };

export function showCalendar(
  host: HTMLElement,
  pick: (picked: DatePick | null) => void,
  initialFormat: DateFormatId = "iso",
): void {
  const wrap = document.createElement("div");
  wrap.className = "md-cal";
  wrap.style.position = "absolute";
  // Above any overlay (InlineAssist + SearchPanel sit at 25000).
  wrap.style.zIndex = "30000";
  // Append first so positionPopover can measure the rendered size
  // and decide whether to flip above the host. We re-position
  // again after each render() below to track size changes (the
  // grid swaps between 4 and 6 rows depending on the month).
  document.body.appendChild(wrap);

  const today = new Date();
  let cursor = new Date(today.getFullYear(), today.getMonth(), today.getDate());
  let activeFormat: DateFormatId = initialFormat;
  // The displayed month: derived from the cursor on every render so
  // we don't drift if a keyboard motion crosses a boundary.

  const sameDay = (a: Date, b: Date): boolean =>
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate();

  const render = () => {
    const view = new Date(cursor.getFullYear(), cursor.getMonth(), 1);
    wrap.innerHTML = "";

    // Format dropdown row. Sits at the top so it's the first thing
    // the user sees; the chosen format previews the cursor's date
    // inline as a hint of what the pill will look like.
    const fmtRow = document.createElement("div");
    fmtRow.className = "md-cal-fmt";
    const fmtLabel = document.createElement("span");
    fmtLabel.className = "md-cal-fmt-label";
    fmtLabel.textContent = "Format:";
    const fmtSel = document.createElement("select");
    for (const f of DATE_FORMATS) {
      const opt = document.createElement("option");
      opt.value = f.id;
      opt.textContent = f.label;
      if (f.id === activeFormat) opt.selected = true;
      fmtSel.appendChild(opt);
    }
    fmtSel.onchange = () => {
      activeFormat = fmtSel.value as DateFormatId;
      render();
    };
    const fmtPreview = document.createElement("span");
    fmtPreview.className = "md-cal-fmt-preview";
    fmtPreview.textContent = formatDate(cursor, activeFormat);
    fmtRow.append(fmtLabel, fmtSel, fmtPreview);
    wrap.appendChild(fmtRow);

    const head = document.createElement("div");
    head.className = "md-cal-head";
    const yPrev = document.createElement("button");
    yPrev.textContent = "‹‹";
    yPrev.title = "previous year (Shift+PageUp)";
    yPrev.onclick = () => {
      cursor = new Date(cursor.getFullYear() - 1, cursor.getMonth(), cursor.getDate());
      render();
    };
    const mPrev = document.createElement("button");
    mPrev.textContent = "‹";
    mPrev.title = "previous month (PageUp)";
    mPrev.onclick = () => {
      cursor = new Date(cursor.getFullYear(), cursor.getMonth() - 1, cursor.getDate());
      render();
    };
    const label = document.createElement("span");
    label.className = "md-cal-label";
    label.textContent = `${view.toLocaleString(undefined, { month: "long" })} ${view.getFullYear()}`;
    label.title = "today (T)";
    label.onclick = () => {
      cursor = new Date(today.getFullYear(), today.getMonth(), today.getDate());
      render();
    };
    const mNext = document.createElement("button");
    mNext.textContent = "›";
    mNext.title = "next month (PageDown)";
    mNext.onclick = () => {
      cursor = new Date(cursor.getFullYear(), cursor.getMonth() + 1, cursor.getDate());
      render();
    };
    const yNext = document.createElement("button");
    yNext.textContent = "››";
    yNext.title = "next year (Shift+PageDown)";
    yNext.onclick = () => {
      cursor = new Date(cursor.getFullYear() + 1, cursor.getMonth(), cursor.getDate());
      render();
    };
    head.append(yPrev, mPrev, label, mNext, yNext);
    wrap.appendChild(head);

    // Weekday header. Locale-aware short names; rendered Sun..Sat
    // because the day grid below also uses Sunday=0 (matches
    // JavaScript's getDay()). Switching to Mon-first would also
    // require offsetting `startDow` below. Goes into the centered
    // gridWrap below alongside the day grid.
    const dowRow = document.createElement("div");
    dowRow.className = "md-cal-dow";
    for (let i = 0; i < 7; i++) {
      const d = new Date(2026, 1, 1 + i); // 2026-02-01 was a Sunday
      const el = document.createElement("div");
      el.textContent = d.toLocaleString(undefined, { weekday: "short" }).slice(0, 2);
      dowRow.appendChild(el);
    }

    const grid = document.createElement("div");
    grid.className = "md-cal-grid";
    const startDow = view.getDay();
    const daysInMonth = new Date(view.getFullYear(), view.getMonth() + 1, 0).getDate();
    for (let i = 0; i < startDow; i++) {
      const blank = document.createElement("div");
      blank.className = "md-cal-blank";
      grid.appendChild(blank);
    }
    for (let d = 1; d <= daysInMonth; d++) {
      const cell = document.createElement("button");
      cell.className = "md-cal-day";
      const cellDate = new Date(view.getFullYear(), view.getMonth(), d);
      if (sameDay(cellDate, today)) cell.classList.add("today");
      if (sameDay(cellDate, cursor)) cell.classList.add("cursor");
      cell.textContent = String(d);
      cell.onclick = () => {
        cleanup();
        pick({ iso: isoOf(cellDate), format: activeFormat });
      };
      grid.appendChild(cell);
    }
    // Wrap the dow header + day grid in a centered container so the
    // narrow seven-column grid (~11rem) sits middle-aligned inside
    // the wider popover (~16rem to fit the format-row preview pill)
    // instead of hugging the left edge.
    const gridWrap = document.createElement("div");
    gridWrap.className = "md-cal-gridwrap";
    gridWrap.append(dowRow, grid);
    wrap.appendChild(gridWrap);

    // Action row: Today on the left for quick reset, Cancel + OK
    // on the right. OK is the primary action (mirrors PromptModal /
    // ConflictModal). Provides an explicit click-to-confirm path
    // for users who don't know to press Enter, and gives the
    // popover a deliberate dismiss button instead of relying on
    // click-outside.
    const actions = document.createElement("div");
    actions.className = "md-cal-actions";
    const todayBtn = document.createElement("button");
    todayBtn.className = "md-cal-action today";
    todayBtn.textContent = "Today";
    todayBtn.title = "jump cursor to today (T)";
    todayBtn.onclick = () => {
      cursor = new Date(today.getFullYear(), today.getMonth(), today.getDate());
      render();
    };
    const cancelBtn = document.createElement("button");
    cancelBtn.className = "md-cal-action cancel";
    cancelBtn.textContent = "Cancel";
    cancelBtn.title = "dismiss without inserting (Esc)";
    cancelBtn.onclick = () => {
      cleanup();
      pick(null);
    };
    const okBtn = document.createElement("button");
    okBtn.className = "md-cal-action ok";
    okBtn.textContent = "OK";
    okBtn.title = "insert the highlighted date (Enter)";
    okBtn.onclick = () => {
      cleanup();
      pick({ iso: isoOf(cursor), format: activeFormat });
    };
    const spacer = document.createElement("span");
    spacer.className = "md-cal-spacer";
    actions.append(todayBtn, spacer, cancelBtn, okBtn);
    wrap.appendChild(actions);

    // Re-position after every render: the grid height shifts
    // between 5 and 6 rows when navigating months, so what fit
    // below the host on January might no longer fit on February.
    positionPopover(host, wrap);
  };
  render();

  const move = (days: number) => {
    cursor = new Date(cursor.getFullYear(), cursor.getMonth(), cursor.getDate() + days);
    render();
  };
  const moveMonth = (months: number) => {
    cursor = new Date(cursor.getFullYear(), cursor.getMonth() + months, cursor.getDate());
    render();
  };

  const onKey = (e: KeyboardEvent) => {
    // The format dropdown's native arrow-key navigation runs on the
    // browser's bubble phase. We listen in capture phase (so the
    // editor doesn't see arrows first), so we'd otherwise swallow
    // the key before the select sees it. Bail on input-like targets.
    const tgt = e.target as HTMLElement | null;
    if (tgt && (tgt.tagName === "SELECT" || tgt.tagName === "INPUT")) return;
    let handled = true;
    switch (e.key) {
      case "ArrowLeft": move(-1); break;
      case "ArrowRight": move(1); break;
      case "ArrowUp": move(-7); break;
      case "ArrowDown": move(7); break;
      case "PageUp": moveMonth(e.shiftKey ? -12 : -1); break;
      case "PageDown": moveMonth(e.shiftKey ? 12 : 1); break;
      case "Home":
        cursor = new Date(cursor.getFullYear(), cursor.getMonth(), 1);
        render();
        break;
      case "End":
        cursor = new Date(cursor.getFullYear(), cursor.getMonth() + 1, 0);
        render();
        break;
      case "t":
      case "T":
        cursor = new Date(today.getFullYear(), today.getMonth(), today.getDate());
        render();
        break;
      case "Enter":
        cleanup();
        pick({ iso: isoOf(cursor), format: activeFormat });
        break;
      case "Escape":
        cleanup();
        pick(null);
        // Don't bubble: a parent overlay (InlineAssist) listens
        // for window-level Escape and would close the dialog.
        e.stopPropagation();
        break;
      default:
        handled = false;
    }
    if (handled) e.preventDefault();
  };

  const onAway = (e: MouseEvent) => {
    // `Node` is shadowed in this file by tiptap's `Node` import.
    // Qualify with `globalThis.Node` so we get the DOM type that
    // `Element.contains` actually expects.
    if (!wrap.contains(e.target as globalThis.Node)) {
      cleanup();
      pick(null);
    }
  };
  const stopWatch = watchViewport(host, wrap);
  const cleanup = () => {
    document.removeEventListener("mousedown", onAway);
    document.removeEventListener("keydown", onKey, true);
    stopWatch();
    wrap.remove();
  };
  document.addEventListener("mousedown", onAway);
  // Capture phase so the calendar handles arrows + Enter before
  // the editor (ProseMirror) sees them. Same trick used by the
  // wiki picker for its own arrow / Enter handling.
  document.addEventListener("keydown", onKey, true);
}
