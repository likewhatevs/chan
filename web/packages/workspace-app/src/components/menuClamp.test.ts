import { describe, expect, test } from "vitest";
import { triggerMenuX } from "./menuClamp";

describe("triggerMenuX", () => {
  test("opens right from a left-edge Hybrid hamburger", () => {
    expect(triggerMenuX({ left: 6, right: 30 }, 250)).toBe(6);
  });

  test("aligns the menu right edge on normal right-side chrome", () => {
    expect(triggerMenuX({ left: 760, right: 784 }, 250)).toBe(534);
  });
});
