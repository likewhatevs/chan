// SPA-global file-drop guard: the no-takeover guarantee.
//
// The guard must act ONLY on OS file drags (dataTransfer.types
// includes "Files") — in-page HTML5 drags (tab moves, tree moves,
// image-atom moves) must pass through with their dropEffect semantics
// untouched — and inside allowlisted zones it must leave the drop
// for the zone's own handlers while the bubble-phase net still
// cancels anything left unhandled.
import { afterEach, beforeEach, describe, expect, test } from "vitest";

import {
  escapePosixPath,
  inFileDropZone,
  installFileDropGuard,
  isOsFileDrag,
  shellEscapePaths,
} from "./fileDropGuard";

/// jsdom has no DragEvent constructor; a plain Event with a stubbed
/// dataTransfer mirrors what the guard reads (types, dropEffect).
function dragEvent(
  type: "dragover" | "drop",
  types: string[],
): Event & { dataTransfer: { types: string[]; dropEffect: string } } {
  const e = new Event(type, { bubbles: true, cancelable: true });
  Object.defineProperty(e, "dataTransfer", {
    value: { types, dropEffect: "" },
  });
  return e as Event & { dataTransfer: { types: string[]; dropEffect: string } };
}

let dispose: () => void;
let zone: HTMLDivElement;
let cmZone: HTMLDivElement;
let outside: HTMLDivElement;

beforeEach(() => {
  dispose = installFileDropGuard(window);
  zone = document.createElement("div");
  zone.setAttribute("data-file-drop-zone", "");
  cmZone = document.createElement("div");
  cmZone.className = "cm-editor";
  outside = document.createElement("div");
  document.body.append(zone, cmZone, outside);
});

afterEach(() => {
  dispose();
  zone.remove();
  cmZone.remove();
  outside.remove();
});

describe("Files-type discriminator", () => {
  test("dragover with types ['text/plain'] is NOT prevented (in-page DnD untouched)", () => {
    const e = dragEvent("dragover", ["text/plain"]);
    outside.dispatchEvent(e);
    expect(e.defaultPrevented).toBe(false);
    expect(e.dataTransfer.dropEffect).toBe("");
  });

  test("drop with types ['text/plain'] is NOT prevented", () => {
    const e = dragEvent("drop", ["text/plain"]);
    outside.dispatchEvent(e);
    expect(e.defaultPrevented).toBe(false);
  });

  test("isOsFileDrag reads the Files type, missing dataTransfer is false", () => {
    expect(isOsFileDrag(dragEvent("drop", ["Files"]) as unknown as DragEvent)).toBe(true);
    expect(
      isOsFileDrag(dragEvent("drop", ["text/uri-list"]) as unknown as DragEvent),
    ).toBe(false);
    expect(isOsFileDrag(new Event("drop") as DragEvent)).toBe(false);
  });
});

describe("outside allowlisted zones", () => {
  test("Files dragover is prevented and the cursor reads not-allowed", () => {
    const e = dragEvent("dragover", ["Files"]);
    outside.dispatchEvent(e);
    expect(e.defaultPrevented).toBe(true);
    expect(e.dataTransfer.dropEffect).toBe("none");
  });

  test("Files drop is prevented (no webview navigation)", () => {
    const e = dragEvent("drop", ["Files"]);
    outside.dispatchEvent(e);
    expect(e.defaultPrevented).toBe(true);
  });
});

describe("inside allowlisted zones", () => {
  test("Files dragover is still cancelled but keeps the browser drop cursor", () => {
    const e = dragEvent("dragover", ["Files"]);
    zone.dispatchEvent(e);
    // Cancelling dragover is what allows the drop to fire at all;
    // the zone's effect is left to the browser / zone handler.
    expect(e.defaultPrevented).toBe(true);
    expect(e.dataTransfer.dropEffect).toBe("");
  });

  test("the guard's capture phase leaves the drop to the zone handler", () => {
    let preventedWhenZoneSawIt: boolean | null = null;
    const zoneHandler = (e: Event) => {
      preventedWhenZoneSawIt = e.defaultPrevented;
    };
    zone.addEventListener("drop", zoneHandler);
    const e = dragEvent("drop", ["Files"]);
    zone.dispatchEvent(e);
    zone.removeEventListener("drop", zoneHandler);
    // The zone handler saw an uncancelled event (it owns the drop)...
    expect(preventedWhenZoneSawIt).toBe(false);
    // ...and the bubble-phase net cancelled whatever it left.
    expect(e.defaultPrevented).toBe(true);
  });

  test(".cm-editor counts as a zone (editable CodeMirror owns its drops)", () => {
    let preventedWhenSeen: boolean | null = null;
    const handler = (e: Event) => {
      preventedWhenSeen = e.defaultPrevented;
    };
    cmZone.addEventListener("drop", handler);
    const e = dragEvent("drop", ["Files"]);
    cmZone.dispatchEvent(e);
    cmZone.removeEventListener("drop", handler);
    expect(preventedWhenSeen).toBe(false);
    expect(e.defaultPrevented).toBe(true);
  });

  test("inFileDropZone matches nested targets", () => {
    const inner = document.createElement("span");
    zone.append(inner);
    expect(inFileDropZone(inner)).toBe(true);
    expect(inFileDropZone(outside)).toBe(false);
    expect(inFileDropZone(null)).toBe(false);
  });
});

describe("terminal path-print escaping", () => {
  test("plain path single-quoted", () => {
    expect(escapePosixPath("/tmp/a.png")).toBe("'/tmp/a.png'");
  });

  test("embedded single quote escapes as close-escape-reopen", () => {
    expect(escapePosixPath("/tmp/it's here.png")).toBe("'/tmp/it'\\''s here.png'");
  });

  test("multiple paths space-separated with a single trailing space", () => {
    expect(shellEscapePaths(["/a b.txt", "/c.md"])).toBe("'/a b.txt' '/c.md' ");
  });

  test("no paths produce an empty string (silent no-op)", () => {
    expect(shellEscapePaths([])).toBe("");
  });
});
