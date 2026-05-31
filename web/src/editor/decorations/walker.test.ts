import { describe, expect, test } from "vitest";
import walker from "./walker.ts?raw";

// BUG-EDITOR (round-2 part-2): conceal decorations reverted to raw markdown
// markers in the lower viewport after a tab switch until a click/scroll. The
// ViewPlugin recomputed only on docChanged / viewportChanged / selectionSet,
// so the post-remount geometry-settle (which does not reliably fire
// viewportChanged) never re-decorated the corrected viewport. This pins the
// geometryChanged arm of the fix; the real timing is browser-smoked.

describe("decoration walker recompute condition", () => {
  test("recomputes on geometryChanged so a tab-switch remount re-decorates", () => {
    expect(walker).toMatch(
      /u\.docChanged \|\|[\s\S]{1,40}u\.viewportChanged \|\|[\s\S]{1,40}u\.selectionSet \|\|[\s\S]{1,40}u\.geometryChanged/,
    );
  });
});
