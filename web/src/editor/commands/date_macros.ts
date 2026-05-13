// `!/today` and `!/date` macros.
//
// Typed verbatim like markdown commands; firing them rewrites the
// trigger source as a date in the user's default format (from
// `drive.info.preferences.date_format`). The freshly-written date is
// then auto-pilled by the existing matcher (widgets/date.ts), so the
// end state is "user typed a slash command, sees a date pill".
//
// Two flavors:
//   - `!/today`: bake today's date and move on. No popover.
//   - `!/date`: same insertion, plus open the calendar / format
//     popover anchored at the freshly-written date so the user can
//     navigate to a different day or switch format without selecting
//     anything first.
//
// Commit triggers: Space and Enter. The handler returns false when
// the line doesn't end with one of the keywords, so the typed
// character falls through to normal input. Returning true consumes
// the key; the inserted space / newline does NOT make it into the
// doc — keeps the flow "type !/today, hit space, see today's date,
// keep typing".

import type { EditorView } from "@codemirror/view";
import { drive, persistDateFormat } from "../../state/store.svelte";
import {
  DATE_FORMATS,
  findDateMatches,
  formatDate,
  type DateFormatId,
} from "../dateFormats";
import { openDatePopover } from "../overlays/date_popover";

/// Resolve the active default format id. Preference field is a free
/// string; if it doesn't match a known id we fall back to ISO so the
/// macro always produces a valid match for the auto-pilled
/// re-detection.
function defaultFormatId(): DateFormatId {
  const id = drive.info?.preferences?.date_format;
  if (id && DATE_FORMATS.some((f) => f.id === id)) {
    return id as DateFormatId;
  }
  return "iso";
}

/// Recognised trigger keywords. Each entry tells us whether to open
/// the calendar popover after committing.
const TRIGGERS: { keyword: string; openPicker: boolean }[] = [
  { keyword: "!/today", openPicker: false },
  { keyword: "!/date", openPicker: true },
];

/// Find a trigger that ends exactly at the caret position. Requires
/// either start-of-line or a whitespace char before the `!` so
/// fragments inside other tokens (e.g. URLs) don't match.
function detectTrigger(view: EditorView): {
  from: number;
  keyword: string;
  openPicker: boolean;
} | null {
  const sel = view.state.selection.main;
  if (!sel.empty) return null;
  const pos = sel.head;
  const line = view.state.doc.lineAt(pos);
  const before = line.text.slice(0, pos - line.from);
  for (const t of TRIGGERS) {
    if (!before.endsWith(t.keyword)) continue;
    const start = before.length - t.keyword.length;
    // Boundary: start-of-line or preceded by whitespace.
    if (start > 0) {
      const prev = before[start - 1]!;
      if (!/\s/.test(prev)) continue;
    }
    return { from: line.from + start, keyword: t.keyword, openPicker: t.openPicker };
  }
  return null;
}

/// Mount an invisible 1×1 span at the inserted date's start so the
/// popover has something concrete to anchor against. Removed in the
/// popover's onDismiss / onCommit callbacks.
function makeAnchorAtPos(view: EditorView, pos: number): HTMLElement | null {
  const coords = view.coordsAtPos(pos);
  if (!coords) return null;
  const anchor = document.createElement("span");
  anchor.style.position = "fixed";
  anchor.style.left = `${coords.left}px`;
  anchor.style.top = `${coords.top}px`;
  anchor.style.width = "1px";
  anchor.style.height = `${coords.bottom - coords.top}px`;
  anchor.style.pointerEvents = "none";
  document.body.appendChild(anchor);
  return anchor;
}

/// Detect whether the caret currently sits inside a recognised date
/// match. Returns the match (range + parsed date + format) or null.
/// Reuses the same matcher widgets/date.ts uses so behaviour stays
/// consistent across "click a pill" and "Cmd+Enter at a pill".
export function dateAtCaret(view: EditorView): {
  from: number;
  to: number;
  date: Date;
  formatId: DateFormatId;
  text: string;
} | null {
  const sel = view.state.selection.main;
  if (!sel.empty) return null;
  const pos = sel.head;
  const line = view.state.doc.lineAt(pos);
  const matches = findDateMatches(line.text);
  for (const m of matches) {
    const from = line.from + m.start;
    const to = line.from + m.end;
    if (pos >= from && pos <= to) {
      return { from, to, date: m.date, formatId: m.formatId, text: m.text };
    }
  }
  return null;
}

/// Cmd+Enter at a date pill: open the popover so the user can
/// re-pick day / format with the keyboard. Returns true (consumed)
/// when a pill matches; false to fall through to the next keymap
/// entry (which today is Mod-Enter → assistant submit).
export function openDateAtCaret(view: EditorView): boolean {
  const hit = dateAtCaret(view);
  if (!hit) return false;
  const anchor = makeAnchorAtPos(view, hit.from);
  if (!anchor) return false;
  openDatePopover({
    anchor,
    initialDate: hit.date,
    initialFormatId: hit.formatId,
    onCommit: (replacement, pickedFormatId) => {
      // Same trailing-space behaviour as the !/date macro and the
      // click-a-pill paths: drop a space + park the caret past
      // the date so the user can keep typing. Skip when the next
      // char is already a space.
      const after = view.state.doc.sliceString(hit.to, hit.to + 1);
      const trailing = after === " " ? "" : " ";
      const insert = replacement + trailing;
      view.dispatch({
        changes: { from: hit.from, to: hit.to, insert },
        selection: { anchor: hit.from + insert.length },
      });
      if (pickedFormatId !== hit.formatId) persistDateFormat(pickedFormatId);
      anchor.remove();
      view.focus();
    },
    onDismiss: () => anchor.remove(),
  });
  return true;
}

/// Run the macro expansion if a trigger sits at the caret. Returns
/// true when a macro fired (caller should consume the keypress);
/// false otherwise so the key falls through to normal input.
export function expandDateMacro(view: EditorView): boolean {
  const hit = detectTrigger(view);
  if (!hit) return false;
  const formatId = defaultFormatId();
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const formatted = formatDate(today, formatId);
  const sel = view.state.selection.main;
  view.dispatch({
    changes: { from: hit.from, to: sel.head, insert: formatted },
    selection: { anchor: hit.from + formatted.length },
  });
  if (hit.openPicker) {
    // Defer to the next animation frame so the pill widget mounts
    // before we try to position the popover; otherwise the anchor
    // sits at the bare caret position which is fine but feels
    // disconnected from the rendered pill.
    requestAnimationFrame(() => {
      const anchor = makeAnchorAtPos(view, hit.from);
      if (!anchor) return;
      openDatePopover({
        anchor,
        initialDate: today,
        initialFormatId: formatId,
        onCommit: (replacement, pickedFormatId) => {
          // Macro flow: rewrite the bake-as-today placeholder with
          // the popover's pick, then drop a trailing space + park
          // the caret past it so the user can keep typing. Skip the
          // extra space when there's already one immediately after
          // the date (avoids the double-space "alice  is here").
          const end = hit.from + formatted.length;
          const after = view.state.doc.sliceString(end, end + 1);
          const trailing = after === " " ? "" : " ";
          const insert = replacement + trailing;
          view.dispatch({
            changes: {
              from: hit.from,
              to: end,
              insert,
            },
            selection: { anchor: hit.from + insert.length },
          });
          if (pickedFormatId !== formatId) persistDateFormat(pickedFormatId);
          anchor.remove();
          // Focus the editor so the user can keep typing without
          // having to click back into the doc — the popover stole
          // focus while open.
          view.focus();
        },
        onDismiss: () => anchor.remove(),
      });
    });
  }
  return true;
}
