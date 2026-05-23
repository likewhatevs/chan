import { describe, expect, test } from "vitest";
import imageModule from "./image.ts?raw";

// `fullstack-a-71`: editor auto-scroll cursor-lost when image
// renders above the caret + pushes layout down. The pre-`-a-71`
// gate `Math.abs(headLine - imgLine) > 1 return` was too
// restrictive — assumed far-away images don't disturb the
// caret, but a tall image above the caret line still pushes
// the caret off-screen via layout-shift.

describe("fullstack-a-71: image-load scroll restore", () => {
  test("removes the headline-distance gate (no early-return on distant images)", () => {
    // The pre-fix `if (Math.abs(headLine - imgLine) > 1) return;`
    // must NOT appear in the load handler. Pin its absence.
    expect(imageModule).not.toMatch(
      /Math\.abs\(headLine - imgLine\) > 1\) return;/,
    );
  });

  test("viewport-visibility gate preserved (no disturbance when caret already visible)", () => {
    // The `cb.top >= sb.top && cb.bottom <= sb.bottom` check must
    // stay — it's the "deliberate position" safeguard. When the
    // caret is on-screen, we don't dispatch.
    expect(imageModule).toMatch(
      /if \(cb\.top >= sb\.top && cb\.bottom <= sb\.bottom\) return;/,
    );
  });

  test("scrollIntoView with nearest dispatched when caret is off-screen", () => {
    expect(imageModule).toMatch(
      /view\.dispatch\(\{\s*effects: EditorView\.scrollIntoView\(head, \{ y: "nearest" \}\),\s*\}\);/,
    );
  });

  test("recent user scroll intent suppresses image-load recovery", () => {
    expect(imageModule).toMatch(/const USER_SCROLL_QUIET_MS = 900;/);
    expect(imageModule).toMatch(/scrollDOM\.addEventListener\("wheel", mark, \{ passive: true \}\)/);
    expect(imageModule).toMatch(/function userScrollIntentActive\(scrollDOM: HTMLElement\): boolean/);
    expect(imageModule).toMatch(
      /installUserScrollIntentTracker\(view\.scrollDOM\);[\s\S]*?if \(userScrollIntentActive\(view\.scrollDOM\)\) return;[\s\S]*?const head = view\.state\.selection\.main\.head;/,
    );
  });

  test("rationale comment cites @@Alex's repro pattern", () => {
    // The new comment block documents the "list at bottom + image
    // above" repro so future readers know why the gate was
    // dropped.
    expect(imageModule).toMatch(
      /list-at-bottom[\s\S]*?image above[\s\S]*?caret vanishes from viewport/i,
    );
  });
});
