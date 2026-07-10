// The remoteCursors StateField is exercised without an EditorView:
// effects and doc changes go through EditorState.update and assertions
// read the field + its DecorationSet directly. Widget DOM (toDOM) runs
// under jsdom. What this cannot cover: real paint, the CSS fade
// animation, and WKWebView text-layer stacking - those are host-smoke
// rows.

import { afterEach, describe, expect, test, vi } from "vitest";
import {
  EditorState,
  type StateEffect,
  type TransactionSpec,
} from "@codemirror/state";
import { sessionWindowId } from "../../api/client";
import { sessionState, type SessionParticipant } from "../../state/session.svelte";
import {
  clearPeersEffect,
  cursorFrameEffects,
  peerColorIdx,
  peersIn,
  remoteCursorsField,
  removePeerEffect,
  rosterRestampEffects,
  type SetPeer,
} from "./remoteCursors";

// Structural view of PeerCaretWidget (the class is private to the
// module; decorations expose instances via spec.widget).
interface PeerWidgetLike {
  peer: number;
  name: string;
  colorIdx: number;
  fresh: boolean;
  eq(other: PeerWidgetLike): boolean;
  toDOM(): HTMLElement;
}

function stateWith(doc: string): EditorState {
  return EditorState.create({ doc, extensions: [remoteCursorsField] });
}

function apply(state: EditorState, spec: TransactionSpec): EditorState {
  return state.update(spec).state;
}

function join(
  state: EditorState,
  id: number,
  w: string,
  anchor: number,
  head: number,
  opts?: { fresh?: boolean },
): EditorState {
  const effects = cursorFrameEffects({ id, w, anchor, head }, opts);
  return apply(state, { effects });
}

function widgets(state: EditorState): Array<{ pos: number; widget: PeerWidgetLike }> {
  const out: Array<{ pos: number; widget: PeerWidgetLike }> = [];
  const it = state.field(remoteCursorsField).decos.iter();
  while (it.value) {
    const spec = it.value.spec as { widget?: PeerWidgetLike };
    if (spec.widget) out.push({ pos: it.from, widget: spec.widget });
    it.next();
  }
  return out;
}

function marks(state: EditorState): Array<{ from: number; to: number; cls: string }> {
  const out: Array<{ from: number; to: number; cls: string }> = [];
  const it = state.field(remoteCursorsField).decos.iter();
  while (it.value) {
    const spec = it.value.spec as { class?: string };
    if (spec.class) out.push({ from: it.from, to: it.to, cls: spec.class });
    it.next();
  }
  return out;
}

function participant(windowId: string, name: string | null): SessionParticipant {
  return { window_id: windowId, name, role: "follower", status: "live" };
}

afterEach(() => {
  sessionState.participants = [];
  sessionState.leader = null;
  vi.useRealTimers();
});

describe("remoteCursors field", () => {
  test("a cursor frame joins a peer: caret widget plus selection mark", () => {
    const st = join(stateWith("hello world"), 1, "w-alpha", 2, 7);
    expect(peersIn(st).get(1)).toMatchObject({
      anchor: 2,
      head: 7,
      windowId: "w-alpha",
    });
    const ms = marks(st);
    expect(ms).toHaveLength(1);
    expect(ms[0]).toMatchObject({ from: 2, to: 7 });
    expect(ms[0]!.cls).toContain("cm-peer-sel");
    const ws = widgets(st);
    expect(ws).toHaveLength(1);
    expect(ws[0]!.pos).toBe(7);
  });

  test("a caret (empty selection) renders a widget and no mark", () => {
    const st = join(stateWith("hello"), 1, "w-alpha", 3, 3);
    expect(marks(st)).toHaveLength(0);
    expect(widgets(st)).toHaveLength(1);
  });

  test("cursor-gone removes the peer; unknown ids keep the field value", () => {
    let st = join(stateWith("hello"), 1, "w-alpha", 1, 1);
    const before = st.field(remoteCursorsField);
    st = apply(st, { effects: removePeerEffect.of({ id: 99 }) });
    expect(st.field(remoteCursorsField)).toBe(before);
    st = apply(st, { effects: removePeerEffect.of({ id: 1 }) });
    expect(peersIn(st).size).toBe(0);
    expect(widgets(st)).toHaveLength(0);
  });

  test("clearPeers drops everyone at once", () => {
    let st = join(stateWith("hello"), 1, "w-alpha", 1, 1);
    st = join(st, 2, "w-beta", 2, 4);
    expect(peersIn(st).size).toBe(2);
    st = apply(st, { effects: clearPeersEffect.of(null) });
    expect(peersIn(st).size).toBe(0);
    expect(widgets(st)).toHaveLength(0);
  });

  test("out-of-range frame positions clamp to the document", () => {
    const st = join(stateWith("hi"), 1, "w-alpha", 50, 60);
    expect(peersIn(st).get(1)).toMatchObject({ anchor: 2, head: 2 });
  });
});

describe("mapping through local edits", () => {
  test("an insert before a peer caret shifts it", () => {
    let st = join(stateWith("hello world"), 1, "w-alpha", 5, 5);
    st = apply(st, { changes: { from: 0, insert: "abc" } });
    expect(peersIn(st).get(1)).toMatchObject({ anchor: 8, head: 8 });
  });

  test("an insert exactly at a peer caret leaves the caret before it", () => {
    let st = join(stateWith("hello world"), 1, "w-alpha", 5, 5);
    st = apply(st, { changes: { from: 5, insert: "abc" } });
    expect(peersIn(st).get(1)).toMatchObject({ anchor: 5, head: 5 });
  });

  test("a selection shrinks at its boundaries and keeps orientation", () => {
    // Forward selection: an insert at the leading edge stays outside.
    let fwd = join(stateWith("hello world"), 1, "w-alpha", 2, 6);
    fwd = apply(fwd, { changes: { from: 2, insert: "xyz" } });
    expect(peersIn(fwd).get(1)).toMatchObject({ anchor: 5, head: 9 });
    // Backward selection: same range, head stays the low end.
    let bwd = join(stateWith("hello world"), 1, "w-alpha", 6, 2);
    bwd = apply(bwd, { changes: { from: 2, insert: "xyz" } });
    expect(peersIn(bwd).get(1)).toMatchObject({ anchor: 9, head: 5 });
    expect(widgets(bwd)[0]!.pos).toBe(5);
  });

  test("a deletion spanning the selection collapses it to a caret", () => {
    let st = join(stateWith("hello world"), 1, "w-alpha", 2, 6);
    st = apply(st, { changes: { from: 1, to: 8 } });
    const p = peersIn(st).get(1)!;
    expect(p.anchor).toBe(p.head);
    expect(marks(st)).toHaveLength(0);
    expect(widgets(st)).toHaveLength(1);
  });
});

describe("colors", () => {
  test("color derives from the window id, stable across attach ids", () => {
    const idx = peerColorIdx("w-alpha");
    expect(idx).toBeGreaterThanOrEqual(0);
    expect(idx).toBeLessThan(8);
    expect(peerColorIdx("w-alpha")).toBe(idx);
    let st = join(stateWith("hello"), 1, "w-alpha", 1, 1);
    st = join(st, 2, "w-alpha", 3, 3);
    const ws = widgets(st);
    expect(ws).toHaveLength(2);
    expect(ws[0]!.widget.colorIdx).toBe(idx);
    expect(ws[1]!.widget.colorIdx).toBe(idx);
  });

  test("the color class rides both the mark and the caret DOM", () => {
    const idx = peerColorIdx("w-alpha");
    const st = join(stateWith("hello"), 1, "w-alpha", 0, 4);
    expect(marks(st)[0]!.cls).toContain(`cm-peer-c${idx}`);
    const dom = widgets(st)[0]!.widget.toDOM();
    expect(dom.className).toContain(`cm-peer-c${idx}`);
  });
});

describe("name flag freshness", () => {
  test("a live frame is fresh; it goes stale after the fade window", () => {
    vi.useFakeTimers();
    vi.setSystemTime(1_000_000);
    let st = join(stateWith("hello world"), 1, "w-alpha", 2, 2);
    expect(widgets(st)[0]!.widget.fresh).toBe(true);
    expect(widgets(st)[0]!.widget.toDOM().dataset.fresh).toBe("true");
    // Rebuild within the window keeps freshness...
    vi.setSystemTime(1_001_000);
    st = apply(st, { changes: { from: 0, insert: "x" } });
    expect(widgets(st)[0]!.widget.fresh).toBe(true);
    // ...and a rebuild past it goes stale (no data-fresh on the DOM).
    vi.setSystemTime(1_003_500);
    st = apply(st, { changes: { from: 0, insert: "y" } });
    expect(widgets(st)[0]!.widget.fresh).toBe(false);
    expect(widgets(st)[0]!.widget.toDOM().dataset.fresh).toBeUndefined();
  });

  test("snapshot-seeded cursors never flash a label", () => {
    const st = join(stateWith("hello"), 1, "w-alpha", 1, 1, { fresh: false });
    expect(widgets(st)[0]!.widget.fresh).toBe(false);
  });
});

describe("widget identity (DOM churn guard)", () => {
  test("eq holds across rebuilds that change nothing visible", () => {
    let st = join(stateWith("hello world"), 1, "w-alpha", 2, 2, {
      fresh: false,
    });
    const before = widgets(st)[0]!.widget;
    st = apply(st, { changes: { from: 10, insert: "x" } });
    const after = widgets(st)[0]!.widget;
    expect(before.eq(after)).toBe(true);
  });

  test("eq breaks on name, color, freshness, or peer identity", () => {
    const base = join(stateWith("hello"), 1, "w-alpha", 1, 1, { fresh: false });
    const w = widgets(base)[0]!.widget;
    const renamed = widgets(
      apply(base, {
        effects: rosterEffectsFor(base, "w-alpha", "Alexei"),
      }),
    )[0]!.widget;
    expect(w.eq(renamed)).toBe(false);
    const freshened = widgets(join(base, 1, "w-alpha", 1, 1))[0]!.widget;
    expect(w.eq(freshened)).toBe(false);
    const otherPeer = widgets(
      join(stateWith("hello"), 2, "w-alpha", 1, 1, { fresh: false }),
    )[0]!.widget;
    expect(w.eq(otherPeer)).toBe(false);
  });
});

// Roster helper: stamp the roster, then produce the restamp effects.
function rosterEffectsFor(
  state: EditorState,
  windowId: string,
  name: string,
): StateEffect<SetPeer>[] {
  sessionState.participants = [participant(windowId, name)];
  return rosterRestampEffects(state);
}

describe("names and the roster", () => {
  test("frames from this window are not peers", () => {
    expect(
      cursorFrameEffects({ id: 9, w: sessionWindowId(), anchor: 0, head: 0 }),
    ).toHaveLength(0);
  });

  test("a frame beating the roster falls back to a window-id prefix", () => {
    const st = join(stateWith("hello"), 1, "w-alpha-123456789", 1, 1);
    expect(widgets(st)[0]!.widget.name).toBe("w-alpha-".slice(0, 8));
  });

  test("a roster restamp renames silently and is idempotent", () => {
    let st = join(stateWith("hello"), 1, "w-alpha", 1, 1, { fresh: false });
    const effects = rosterEffectsFor(st, "w-alpha", "Alexei");
    expect(effects).toHaveLength(1);
    st = apply(st, { effects });
    const w = widgets(st)[0]!.widget;
    expect(w.name).toBe("Alexei");
    // Freshness is untouched: a rename must not flash the flag.
    expect(w.fresh).toBe(false);
    expect(rosterRestampEffects(st)).toHaveLength(0);
  });

  test("a frame arriving after the roster resolves the live name", () => {
    sessionState.participants = [participant("w-beta", "Kim")];
    const st = join(stateWith("hello"), 3, "w-beta", 2, 2);
    expect(widgets(st)[0]!.widget.name).toBe("Kim");
  });
});

describe("caret DOM", () => {
  test("the widget is decorative, flagged, and carries the name", () => {
    sessionState.participants = [participant("w-alpha", "Alexei")];
    const st = join(stateWith("hello"), 1, "w-alpha", 1, 1);
    const dom = widgets(st)[0]!.widget.toDOM();
    expect(dom.getAttribute("aria-hidden")).toBe("true");
    expect(dom.className).toContain("cm-peer-caret");
    const flag = dom.querySelector(".cm-peer-flag");
    expect(flag?.textContent).toBe("Alexei");
  });
});
