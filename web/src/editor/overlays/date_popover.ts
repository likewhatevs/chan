// Date popover. Click a date pill -> month-grid calendar + format
// dropdown + commit. Replaces the date range in source on commit;
// dismiss without commit is a no-op (the source stays).
//
// UI: a small body-attached popover positioned under the pill DOM.
//   - Header: <prev> Month YYYY <next> + format dropdown.
//   - 7-column grid: Mo Tu We Th Fr Sa Su day-of-week labels (start
//     of week is Monday — adjust if a future preference asks).
//   - Day cells highlight selected and today.
//   - Click a day to commit that date in the chosen format.
//   - Esc / outside click / scroll dismisses without commit.
//
// Keep this surface intentionally small: no time-of-day, no range
// selection, no recurrence — we're a notes app.

import {
  DATE_FORMATS,
  formatDate,
  type DateFormatId,
} from "../dateFormats";

export interface DatePopoverOpts {
  /// The DOM element to anchor the popover under. Typically the date
  /// pill's wrap span.
  anchor: HTMLElement;
  initialDate: Date;
  initialFormatId: DateFormatId;
  /// Called with the formatted text the caller should write to the
  /// source. Caller owns the doc dispatch.
  onCommit: (formatted: string, formatId: DateFormatId) => void;
  /// Called when the user dismisses without committing.
  onDismiss: () => void;
}

const DOW_LABELS = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
const MONTH_LABELS = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

export function openDatePopover(opts: DatePopoverOpts): { dismiss: () => void } {
  let selected = new Date(opts.initialDate);
  selected.setHours(0, 0, 0, 0);
  let viewMonth = new Date(selected.getFullYear(), selected.getMonth(), 1);
  let formatId = opts.initialFormatId;
  let alive = true;

  const wrap = document.createElement("div");
  wrap.className = "md-date-popover";
  wrap.style.position = "absolute";
  wrap.style.zIndex = "30000";
  document.body.appendChild(wrap);
  positionUnderAnchor();

  function positionUnderAnchor(): void {
    const rect = opts.anchor.getBoundingClientRect();
    const popH = wrap.offsetHeight; // 0 on first call (before render)
    const popW = wrap.offsetWidth;
    const viewH = window.innerHeight;
    const viewW = window.innerWidth;
    const GAP = 4;
    // Vertical: prefer below; flip above when below would overflow
    // and there's more room above.
    const spaceBelow = viewH - rect.bottom;
    const spaceAbove = rect.top;
    let top: number;
    if (popH > 0 && spaceBelow < popH + GAP && spaceAbove > spaceBelow) {
      // Flip above: anchor the popover's bottom to the pill's top.
      top = rect.top + window.scrollY - popH - GAP;
    } else {
      top = rect.bottom + window.scrollY + GAP;
    }
    // Horizontal: keep left-aligned with the pill, but clamp so the
    // popover stays inside the viewport.
    let left = rect.left + window.scrollX;
    if (popW > 0) {
      const maxLeft = window.scrollX + viewW - popW - GAP;
      if (left > maxLeft) left = Math.max(window.scrollX + GAP, maxLeft);
    }
    wrap.style.left = `${Math.round(left)}px`;
    wrap.style.top = `${Math.round(top)}px`;
  }

  function render(): void {
    wrap.innerHTML = "";
    // Header.
    const header = document.createElement("div");
    header.className = "md-date-header";
    const prev = makeIconBtn("‹", () => {
      viewMonth = new Date(viewMonth.getFullYear(), viewMonth.getMonth() - 1, 1);
      render();
    });
    const next = makeIconBtn("›", () => {
      viewMonth = new Date(viewMonth.getFullYear(), viewMonth.getMonth() + 1, 1);
      render();
    });
    const title = document.createElement("span");
    title.className = "md-date-title";
    title.textContent = `${MONTH_LABELS[viewMonth.getMonth()]} ${viewMonth.getFullYear()}`;
    header.appendChild(prev);
    header.appendChild(title);
    header.appendChild(next);
    wrap.appendChild(header);

    // Grid.
    const grid = document.createElement("div");
    grid.className = "md-date-grid";
    for (const dow of DOW_LABELS) {
      const cell = document.createElement("div");
      cell.className = "md-date-dow";
      cell.textContent = dow;
      grid.appendChild(cell);
    }
    // Compute leading blanks: getDay() returns 0=Sun..6=Sat. We start
    // weeks on Monday, so shift: (jsDay + 6) % 7.
    const firstDow = (viewMonth.getDay() + 6) % 7;
    for (let i = 0; i < firstDow; i++) {
      const blank = document.createElement("div");
      blank.className = "md-date-blank";
      grid.appendChild(blank);
    }
    const daysInMonth = new Date(
      viewMonth.getFullYear(),
      viewMonth.getMonth() + 1,
      0,
    ).getDate();
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    for (let d = 1; d <= daysInMonth; d++) {
      const cell = document.createElement("button");
      cell.type = "button";
      cell.className = "md-date-day";
      cell.textContent = String(d);
      const cellDate = new Date(viewMonth.getFullYear(), viewMonth.getMonth(), d);
      if (cellDate.getTime() === today.getTime()) {
        cell.classList.add("md-date-day-today");
      }
      if (cellDate.getTime() === selected.getTime()) {
        cell.classList.add("md-date-day-selected");
      }
      cell.addEventListener("mousedown", (e) => {
        e.preventDefault();
        e.stopPropagation();
        selected = cellDate;
        commit();
      });
      grid.appendChild(cell);
    }
    wrap.appendChild(grid);

    // Format dropdown.
    const formatRow = document.createElement("div");
    formatRow.className = "md-date-format-row";
    const label = document.createElement("span");
    label.className = "md-date-format-label";
    label.textContent = "Format";
    formatRow.appendChild(label);
    const select = document.createElement("select");
    select.className = "md-date-format-select";
    for (const f of DATE_FORMATS) {
      const opt = document.createElement("option");
      opt.value = f.id;
      // Show the actual rendering for the currently-selected date
      // so users see exactly what they'd commit.
      opt.textContent = formatDate(selected, f.id);
      if (f.id === formatId) opt.selected = true;
      select.appendChild(opt);
    }
    select.addEventListener("change", () => {
      formatId = select.value as DateFormatId;
    });
    formatRow.appendChild(select);
    wrap.appendChild(formatRow);
    positionUnderAnchor();
  }

  function commit(): void {
    if (!alive) return;
    const formatted = formatDate(selected, formatId);
    dismiss();
    opts.onCommit(formatted, formatId);
  }

  function dismiss(): void {
    if (!alive) return;
    alive = false;
    document.removeEventListener("mousedown", outsideClick, true);
    document.removeEventListener("keydown", escListener, true);
    window.removeEventListener("scroll", dismiss, true);
    wrap.remove();
    opts.onDismiss();
  }

  function outsideClick(e: MouseEvent): void {
    if (wrap.contains(e.target as Node)) return;
    if (opts.anchor.contains(e.target as Node)) return; // re-clicking the pill is a no-op
    dismiss();
  }

  function escListener(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      dismiss();
    }
  }

  // Defer outside-click wiring so the click that opened the popover
  // doesn't immediately count as outside.
  window.setTimeout(() => {
    if (!alive) return;
    document.addEventListener("mousedown", outsideClick, true);
    document.addEventListener("keydown", escListener, true);
    window.addEventListener("scroll", dismiss, true);
  }, 0);

  render();
  return { dismiss };
}

function makeIconBtn(label: string, onClick: () => void): HTMLElement {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = "md-date-nav";
  btn.textContent = label;
  btn.addEventListener("mousedown", (e) => {
    e.preventDefault();
    e.stopPropagation();
    onClick();
  });
  return btn;
}
