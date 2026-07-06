import { describe, expect, test } from "vitest";
import { ancestorsExpanded } from "./pathVisibility";

describe("graph path visibility", () => {
  test("directory scope keeps its workspace-root spine ancestors visible", () => {
    const root = "crates/chan-tunnel-proto/src";
    const expanded = { "": true, [root]: true };

    expect(ancestorsExpanded(root, "", expanded)).toBe(true);
    expect(ancestorsExpanded(root, "crates", expanded)).toBe(true);
    expect(ancestorsExpanded(root, "crates/chan-tunnel-proto", expanded)).toBe(true);
    expect(ancestorsExpanded(root, root, expanded)).toBe(true);
  });

  test("directory scope hides unrelated siblings outside the scoped tree", () => {
    const root = "crates/chan-tunnel-proto/src";
    const expanded = { "": true, [root]: true };

    expect(ancestorsExpanded(root, "crates/chan-server", expanded)).toBe(false);
    expect(ancestorsExpanded(root, "README.md", expanded)).toBe(false);
  });

  test("descendants below the scope root still require expanded ancestors", () => {
    const root = "crates/chan-tunnel-proto/src";

    expect(
      ancestorsExpanded(root, `${root}/h2_duplex.rs`, { "": true, [root]: true }),
    ).toBe(true);
    expect(
      ancestorsExpanded(root, `${root}/nested/mod.rs`, { "": true, [root]: true }),
    ).toBe(false);
    expect(
      ancestorsExpanded(root, `${root}/nested/mod.rs`, {
        "": true,
        [root]: true,
        [`${root}/nested`]: true,
      }),
    ).toBe(true);
  });
});
