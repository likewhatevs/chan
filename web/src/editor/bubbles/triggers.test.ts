import { EditorState } from "@codemirror/state";
import { describe, expect, test } from "vitest";
import { computeBubbleSpec } from "./triggers";

function specAtEnd(doc: string) {
  return computeBubbleSpec(
    EditorState.create({
      doc,
      selection: { anchor: doc.length },
    }),
  );
}

describe("macro trigger reservations", () => {
  test("@today / @date stay out of the contact bubble", () => {
    expect(specAtEnd("@today")).toBeNull();
    expect(specAtEnd("@date")).toBeNull();
  });

  test("@pagebreak / @break stay out of the contact bubble", () => {
    expect(specAtEnd("@pagebreak")).toBeNull();
    expect(specAtEnd("@break")).toBeNull();
  });

  test("macro prefixes still open contact completion", () => {
    expect(specAtEnd("@pageb")).toMatchObject({
      kind: "contact",
      query: "pageb",
    });
  });
});
