import { describe, expect, test } from "vitest";
import canvas from "./GraphCanvas.svelte?raw";

// Graph canvas forgiving clicks: hit-test pad is wider for
// click-to-select and hover, but not for drag-detect, so
// pan-on-empty-space stays usable.

describe("hit-radius slack constants", () => {
  test("PICK_SLACK_DRAG_PX = 4 (tight drag-vs-pan disambiguation)", () => {
    expect(canvas).toMatch(/const PICK_SLACK_DRAG_PX = 4;/);
  });

  test("PICK_SLACK_CLICK_PX = 10 (forgiving click target ~8-12px)", () => {
    expect(canvas).toMatch(/const PICK_SLACK_CLICK_PX = 10;/);
  });
});

describe("pickNode accepts a slack parameter", () => {
  test("pickNode signature has a slackPx parameter defaulting to PICK_SLACK_DRAG_PX", () => {
    expect(canvas).toMatch(
      /function pickNode\([\s\S]*?slackPx: number = PICK_SLACK_DRAG_PX,?\s*\)/,
    );
  });

  test("slack is applied via screen-constant zoom-divided formula", () => {
    // `slackPx / Math.max(0.5, transform.k)` keeps the slack
    // visually constant in screen pixels across zoom levels.
    expect(canvas).toMatch(
      /const r = n\.radius \+ slackPx \/ Math\.max\(0\.5, transform\.k\);/,
    );
  });
});

describe("call-site slack selection", () => {
  test("onMouseUp tap-to-select uses the WIDER click slack", () => {
    expect(canvas).toMatch(
      /A tap on a node \(no drag movement\) selects it\.[\s\S]*?pickNode\(p\.x, p\.y, PICK_SLACK_CLICK_PX\)/,
    );
  });

  test("hover handler (onMouseMove no-drag) uses the WIDER click slack so cursor preview matches the tap target", () => {
    expect(canvas).toMatch(
      /Cheap hover update\.[\s\S]*?pickNode\(p\.x, p\.y, PICK_SLACK_CLICK_PX\)/,
    );
  });

  test("onMouseDown drag-detect uses the DEFAULT (tight) slack so pan-on-empty-space stays usable", () => {
    // The mousedown call doesn't pass a slack arg — it relies on
    // pickNode's default (PICK_SLACK_DRAG_PX). Pin the
    // call-shape: `pickNode(p.x, p.y)` with no third arg in the
    // mouse-down handler.
    expect(canvas).toMatch(
      /function onMouseDown\(e: MouseEvent\): void \{[\s\S]*?const n = pickNode\(p\.x, p\.y\);/,
    );
  });
});

describe("nearest-centroid tie-break preserved", () => {
  test("closer hits win when several discs overlap (`d2 < bestD2`)", () => {
    // The pre-existing tie-break logic; preserved across the
    // slack-parameter refactor.
    expect(canvas).toMatch(/if \(d2 <= r \* r && d2 < bestD2\)/);
  });
});
