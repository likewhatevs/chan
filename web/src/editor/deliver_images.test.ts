import { describe, expect, test } from "vitest";

import { rewriteImagePathsForDelivery } from "./deliver_images";

// C1: a pasted image in the rich prompt lands at `.Drafts/{name}/image.png`
// and is inserted draft-file-relative (`./image.png`) so the in-compose
// preview resolves it. But the receiving agent runs at `$CWD` = workspace
// root, so `./image.png` would 404. `rewriteImagePathsForDelivery` rewrites
// the delivered ref to the workspace-rooted path the agent can read, dropping
// the `#w=N` render hint, while leaving the draft text (preview) untouched.
describe("rewriteImagePathsForDelivery", () => {
  const draft = ".Drafts/abc123/draft.md";

  test("rewrites a draft-relative paste to a workspace-rooted path", () => {
    const out = rewriteImagePathsForDelivery(
      "see ![](./image.png#w=250) please",
      draft,
    );
    expect(out).toBe("see ![](.Drafts/abc123/image.png) please");
  });

  test("strips the #w fragment so the agent reads a real filename", () => {
    const out = rewriteImagePathsForDelivery("![](./shot.png#w=120)", draft);
    expect(out).toBe("![](.Drafts/abc123/shot.png)");
    expect(out).not.toContain("#w=");
  });

  test("preserves alt text", () => {
    const out = rewriteImagePathsForDelivery("![a diagram](./d.png#w=300)", draft);
    expect(out).toBe("![a diagram](.Drafts/abc123/d.png)");
  });

  test("rewrites every ref in a multi-image prompt", () => {
    const out = rewriteImagePathsForDelivery(
      "![](./a.png#w=250)\nand\n![](./b.png#w=250)",
      draft,
    );
    expect(out).toBe(
      "![](.Drafts/abc123/a.png)\nand\n![](.Drafts/abc123/b.png)",
    );
  });

  test("re-encodes a spaced name so the destination stays valid markdown", () => {
    // The draft stores the path percent-encoded; the rewrite resolves it and
    // re-encodes the same way, so a space survives as `%20` (not a truncated
    // destination).
    const out = rewriteImagePathsForDelivery(
      "![](./My%20Photo.png#w=250)",
      draft,
    );
    expect(out).toBe("![](.Drafts/abc123/My%20Photo.png)");
  });

  test("resolves a parent-relative ref out of a nested draft dir", () => {
    const out = rewriteImagePathsForDelivery(
      "![](../shared/logo.png#w=64)",
      ".Drafts/x/y/draft.md",
    );
    expect(out).toBe("![](.Drafts/x/shared/logo.png)");
  });

  test("leaves external (http/data/blob) refs untouched", () => {
    const md =
      "![](https://example.com/x.png) ![](data:image/png;base64,AAAA) ![](blob:abc)";
    expect(rewriteImagePathsForDelivery(md, draft)).toBe(md);
  });

  test("leaves non-image text untouched", () => {
    const md = "just some [a link](./note.md) and prose";
    expect(rewriteImagePathsForDelivery(md, draft)).toBe(md);
  });

  test("no-ops when there is no draft path (no base to resolve against)", () => {
    const md = "![](./image.png#w=250)";
    expect(rewriteImagePathsForDelivery(md, null)).toBe(md);
  });

  test("leaves a ref that escapes the workspace root untouched", () => {
    // `normalizeHref` rejects a `..` past the root; the ref is left as-is
    // rather than emitting a broken path.
    const out = rewriteImagePathsForDelivery(
      "![](../../../etc/passwd#w=1)",
      ".Drafts/x/draft.md",
    );
    expect(out).toBe("![](../../../etc/passwd#w=1)");
  });
});
