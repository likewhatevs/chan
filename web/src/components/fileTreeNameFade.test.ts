import { describe, expect, test } from "vitest";
import source from "./FileTree.svelte?raw";

// `fullstack-a-62`: docked file browser rows fade long filenames
// at the edge (linear-gradient mask) instead of wrapping to a
// second line. Mirrors `Pane.svelte`'s tab-name mask pattern.
// Right-dock variant flips the fade direction (right → left).

describe("FileTree .name fade-mask", () => {
  test(".name keeps content on a single line", () => {
    // Pin the load-bearing whitespace + overflow rules: `nowrap`
    // prevents the second-line wrap @@Alex flagged; `overflow:
    // hidden` clips beyond the row's width so the mask has
    // something to fade.
    expect(source).toMatch(/\.name \{[\s\S]*?white-space: nowrap;[\s\S]*?overflow: hidden;/);
  });

  test(".name applies a linear-gradient mask fading to the right edge", () => {
    // Default left-dock + overlay: text left-aligns; fade-out is
    // on the right edge. 1.25rem matches Pane.svelte's tab-name
    // mask width for visual parity across surfaces.
    expect(source).toMatch(
      /\.name \{[\s\S]*?mask-image: linear-gradient\(to right, black calc\(100% - 1\.25rem\), transparent\);/,
    );
    expect(source).toMatch(
      /\.name \{[\s\S]*?-webkit-mask-image: linear-gradient\(to right, black calc\(100% - 1\.25rem\), transparent\);/,
    );
  });

  test(".tree.right-dock .name flips the fade direction (to LEFT edge)", () => {
    // Right-dock: text right-aligns, so the LEFT edge is where
    // the long part gets truncated. Mirrored mask direction.
    expect(source).toMatch(
      /\.tree\.right-dock \.name \{[\s\S]*?mask-image: linear-gradient\(to left, black calc\(100% - 1\.25rem\), transparent\);/,
    );
    expect(source).toMatch(
      /\.tree\.right-dock \.name \{[\s\S]*?-webkit-mask-image: linear-gradient\(to left, black calc\(100% - 1\.25rem\), transparent\);/,
    );
  });

  test(".name retains its existing flex + button-reset rules", () => {
    // The fade should be ADDITIVE — don't drop the flex:1 / button
    // reset / cursor / inherit-style rules that make the row a
    // clickable filename button.
    expect(source).toMatch(/\.name \{[\s\S]*?flex: 1;/);
    expect(source).toMatch(/\.name \{[\s\S]*?background: none;[\s\S]*?border: 0;/);
    expect(source).toMatch(/\.name \{[\s\S]*?cursor: pointer;/);
  });
});
