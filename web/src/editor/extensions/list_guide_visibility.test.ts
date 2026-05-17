// @vitest-environment jsdom
//
// Coverage for the list-guide auto-hide plugin. The plugin watches
// selection updates and toggles `data-list-guides` on the editor
// DOM to drive the CSS fade. These tests mount a real EditorView
// in jsdom so we can poke the selection and observe the attribute
// transitions on the same surface the runtime uses.

import { describe, expect, test, beforeEach, afterEach, vi } from "vitest";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { listGuideVisibility } from "./list_guide_visibility";

let host: HTMLDivElement;
let view: EditorView;

function mount(doc: string, caret: number): void {
  host = document.createElement("div");
  document.body.append(host);
  view = new EditorView({
    state: EditorState.create({
      doc,
      selection: { anchor: caret },
      extensions: [listGuideVisibility()],
    }),
    parent: host,
  });
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
  view?.destroy();
  host?.remove();
});

describe("listGuideVisibility", () => {
  test("marks editor as 'on' when caret starts on a bullet line", () => {
    mount("- item", 3);
    expect(view.dom.getAttribute("data-list-guides")).toBe("on");
  });

  test("marks editor as 'off' when caret starts off a list", () => {
    mount("hello world", 5);
    expect(view.dom.getAttribute("data-list-guides")).toBe("off");
  });

  test("flips to 'on' when caret moves onto a list line", () => {
    mount("hello\n- item", 2);
    expect(view.dom.getAttribute("data-list-guides")).toBe("off");
    view.dispatch({ selection: { anchor: 9 } });
    expect(view.dom.getAttribute("data-list-guides")).toBe("on");
  });

  test("schedules fade to 'off' 1.5s after caret leaves a list line", () => {
    mount("- one\nhello", 2);
    expect(view.dom.getAttribute("data-list-guides")).toBe("on");
    view.dispatch({ selection: { anchor: 8 } });
    // Still "on" within the grace period.
    vi.advanceTimersByTime(1000);
    expect(view.dom.getAttribute("data-list-guides")).toBe("on");
    // Past the 1.5s threshold the plugin flips to "off".
    vi.advanceTimersByTime(600);
    expect(view.dom.getAttribute("data-list-guides")).toBe("off");
  });

  test("re-entering a list before fade cancels the timer", () => {
    mount("- one\nhello\n- two", 2);
    view.dispatch({ selection: { anchor: 8 } }); // off-list line
    vi.advanceTimersByTime(500);
    view.dispatch({ selection: { anchor: 14 } }); // back on a list line
    vi.advanceTimersByTime(2000);
    expect(view.dom.getAttribute("data-list-guides")).toBe("on");
  });

  test("recognises ordered and task markers as list lines", () => {
    mount("1. first\n- [ ] todo", 4);
    expect(view.dom.getAttribute("data-list-guides")).toBe("on");
    view.dispatch({ selection: { anchor: 12 } });
    expect(view.dom.getAttribute("data-list-guides")).toBe("on");
  });
});
