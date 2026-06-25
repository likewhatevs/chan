import { describe, expect, test } from "vitest";
import src from "./click_caret.ts?raw";
import wysiwyg from "./Wysiwyg.svelte?raw";

// click_caret places the caret for blank-area clicks (right of a short
// line, in a row's trailing space, below the last line) that CodeMirror's
// precise hit-test misses. These pin the conservative contract: act only
// when the precise hit is null, and otherwise stay out of the way.
describe("click-to-place-caret handler", () => {
  test("only acts when the precise hit-test misses", () => {
    // A precise hit means CM6 already placed the caret; bail out so
    // normal clicks (and widget/pill handlers) are untouched.
    expect(src).toMatch(/if \(view\.posAtCoords\(coords\) !== null\) return false;/);
  });

  test("resolves the nearest position for a blank-area click", () => {
    // The non-precise resolve snaps a past-EOL click to the row's text end.
    expect(src).toMatch(/const near = view\.posAtCoords\(coords, false\);/);
    expect(src).toMatch(/view\.dispatch\(\{ selection: \{ anchor: near \} \}\);/);
  });

  test("ignores modified, multi-, and non-primary clicks", () => {
    expect(src).toMatch(/event\.button !== 0 \|\| event\.detail > 1/);
    expect(src).toMatch(
      /event\.shiftKey \|\| event\.altKey \|\| event\.metaKey \|\| event\.ctrlKey/,
    );
  });

  test("is wired into the editor", () => {
    expect(wysiwyg).toMatch(/clickToPlaceCaret\(\),/);
    expect(wysiwyg).toMatch(/import \{ clickToPlaceCaret \} from "\.\/click_caret";/);
    // There is no bullet caret-snap (listCaretGuard): markers are
    // real-width glyphs, so only the blank-area helper is wired in.
    expect(wysiwyg).not.toContain("listCaretGuard");
  });
});
