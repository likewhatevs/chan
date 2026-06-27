import { describe, expect, test } from "vitest";

import { rewriteImagePathsForDelivery } from "./deliver_images";

// A pasted image in the rich prompt lands at `.Drafts/{name}/image.png`
// and is inserted draft-file-relative (`./image.png`) so the in-compose
// preview resolves it. At submit the delivered text carries a PLAIN path
// (no `![]()` wrapper, no `#w=N` hint) computed for the terminal's live
// CWD: relative to that CWD when it is known and inside the workspace
// root, else an absolute on-disk path. The draft text (preview) is
// untouched.
describe("rewriteImagePathsForDelivery", () => {
  const draft = ".Drafts/abc123/draft.md";
  const root = "/home/u/ws";

  test("CWD at root: delivers a plain path relative to the root", () => {
    const out = rewriteImagePathsForDelivery(
      "see ![](./image.png#w=250) please",
      draft,
      "",
      root,
    );
    expect(out).toBe("see ./.Drafts/abc123/image.png please");
  });

  test("CWD inside root: delivers a path relative to that CWD", () => {
    const out = rewriteImagePathsForDelivery(
      "![](./shot.png#w=120)",
      draft,
      "Recipes",
      root,
    );
    expect(out).toBe("../.Drafts/abc123/shot.png");
  });

  test("drops the `![]()` wrapper, alt text, and the #w fragment", () => {
    const out = rewriteImagePathsForDelivery(
      "![a diagram](./d.png#w=300)",
      draft,
      "",
      root,
    );
    expect(out).toBe("./.Drafts/abc123/d.png");
    expect(out).not.toContain("![");
    expect(out).not.toContain("#w=");
  });

  test("CWD unknown / outside root: delivers an absolute on-disk path", () => {
    const out = rewriteImagePathsForDelivery(
      "![](./image.png#w=250)",
      draft,
      null,
      root,
    );
    expect(out).toBe("/home/u/ws/.Drafts/abc123/image.png");
  });

  test("absolute fallback normalizes a Windows root + trailing slash", () => {
    const out = rewriteImagePathsForDelivery(
      "![](./image.png)",
      draft,
      null,
      "C:\\Users\\me\\ws\\",
    );
    expect(out).toBe("C:/Users/me/ws/.Drafts/abc123/image.png");
  });

  test("rewrites every ref in a multi-image prompt", () => {
    const out = rewriteImagePathsForDelivery(
      "![](./a.png#w=250)\nand\n![](./b.png#w=250)",
      draft,
      "",
      root,
    );
    expect(out).toBe("./.Drafts/abc123/a.png\nand\n./.Drafts/abc123/b.png");
  });

  test("delivers a real (decoded) filename, not a percent-encoded one", () => {
    const out = rewriteImagePathsForDelivery(
      "![](./My%20Photo.png#w=250)",
      draft,
      "",
      root,
    );
    expect(out).toBe("./.Drafts/abc123/My Photo.png");
  });

  test("resolves a parent-relative ref out of a nested draft dir", () => {
    const out = rewriteImagePathsForDelivery(
      "![](../shared/logo.png#w=64)",
      ".Drafts/x/y/draft.md",
      "",
      root,
    );
    expect(out).toBe("./.Drafts/x/shared/logo.png");
  });

  test("leaves external (http/data/blob) refs untouched", () => {
    const md =
      "![](https://example.com/x.png) ![](data:image/png;base64,AAAA) ![](blob:abc)";
    expect(rewriteImagePathsForDelivery(md, draft, "", root)).toBe(md);
  });

  test("leaves non-image text untouched", () => {
    const md = "just some [a link](./note.md) and prose";
    expect(rewriteImagePathsForDelivery(md, draft, "", root)).toBe(md);
  });

  test("no-ops when there is no draft path (no base to resolve against)", () => {
    const md = "![](./image.png#w=250)";
    expect(rewriteImagePathsForDelivery(md, null, "", root)).toBe(md);
  });

  test("leaves a ref that escapes the workspace root untouched", () => {
    // `normalizeHref` rejects a `..` past the root; the ref is left as-is
    // (wrapper and all) rather than emitting a broken path.
    const out = rewriteImagePathsForDelivery(
      "![](../../../etc/passwd#w=1)",
      ".Drafts/x/draft.md",
      "",
      root,
    );
    expect(out).toBe("![](../../../etc/passwd#w=1)");
  });
});
