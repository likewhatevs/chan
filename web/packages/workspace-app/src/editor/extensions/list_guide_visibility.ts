// Auto-hide list guide bars after the caret leaves a list.
//
// The vertical guide bars on `.cm-md-list-line` (drawn by the
// editor CSS) are useful while the user navigates a nested list,
// but they read as visual noise on screens that aren't actively
// being edited. Per request.md, they should fade out 1.5s after
// the caret last sat on a list line and reappear immediately when
// the caret returns. Selection changes that REMAIN inside the list
// reset the visible state; the fade only schedules when the caret
// is no longer on a list line.
//
// Implementation: a ViewPlugin watches selection updates and
// toggles a `data-list-guides` attribute on `.cm-editor`. CSS in
// Wysiwyg.svelte fades the `::before` opacity off that attribute.

import type { Extension } from "@codemirror/state";
import {
  EditorView,
  ViewPlugin,
  type PluginValue,
  type ViewUpdate,
} from "@codemirror/view";

const LIST_LINE_RE = /^[ \t]*([-*+]|\d+[.)])[ \t]+/;
const FADE_DELAY_MS = 1500;
const ATTR = "data-list-guides";

function caretOnListLine(view: EditorView): boolean {
  const sel = view.state.selection.main;
  const line = view.state.doc.lineAt(sel.head);
  return LIST_LINE_RE.test(line.text);
}

class GuideVisibility implements PluginValue {
  private timer: ReturnType<typeof setTimeout> | undefined;
  private state: "on" | "off" | null = null;

  constructor(private readonly view: EditorView) {
    this.apply(caretOnListLine(view) ? "on" : "off");
  }

  update(u: ViewUpdate): void {
    if (!u.selectionSet && !u.docChanged) return;
    if (caretOnListLine(u.view)) {
      this.cancelTimer();
      this.apply("on");
    } else if (this.state === "on") {
      this.scheduleHide();
    }
  }

  destroy(): void {
    this.cancelTimer();
  }

  private apply(s: "on" | "off"): void {
    if (this.state === s) return;
    this.state = s;
    this.view.dom.setAttribute(ATTR, s);
  }

  private scheduleHide(): void {
    this.cancelTimer();
    this.timer = setTimeout(() => {
      this.timer = undefined;
      this.apply("off");
    }, FADE_DELAY_MS);
  }

  private cancelTimer(): void {
    if (this.timer === undefined) return;
    clearTimeout(this.timer);
    this.timer = undefined;
  }
}

export function listGuideVisibility(): Extension {
  return ViewPlugin.fromClass(GuideVisibility);
}
