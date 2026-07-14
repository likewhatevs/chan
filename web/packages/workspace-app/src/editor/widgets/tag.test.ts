// @vitest-environment jsdom

import { afterEach, describe, expect, test } from "vitest";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { chanMarkdown } from "../markdown/grammar";
import { tagDecorations } from "./tag";

function mount(doc: string): { view: EditorView; cleanup: () => void } {
  const target = document.createElement("div");
  document.body.append(target);
  const state = EditorState.create({
    doc,
    extensions: [chanMarkdown(), tagDecorations({ onTagClick: () => {} })],
  });
  const view = new EditorView({ state, parent: target });
  return {
    view,
    cleanup: () => {
      view.destroy();
      target.remove();
    },
  };
}

function tagPills(view: EditorView): string[] {
  return [...view.dom.querySelectorAll(".cm-md-tag")].map(
    (el) => el.textContent ?? "",
  );
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("tag decorations skip link and image labels", () => {
  test("a tag-lookalike link label is not decorated", () => {
    const { view, cleanup } = mount("[#999](https://google.com)");
    expect(tagPills(view)).toEqual([]);
    cleanup();
  });

  test("a bare tag outside a link still decorates", () => {
    const { view, cleanup } = mount("see #999 in the graph");
    expect(tagPills(view)).toEqual(["#999"]);
    cleanup();
  });

  test("a link label and a bare tag on one line decorate only the tag", () => {
    const { view, cleanup } = mount("[#999](https://google.com) and #topic");
    expect(tagPills(view)).toEqual(["#topic"]);
    cleanup();
  });

  test("image alt text is not decorated", () => {
    const { view, cleanup } = mount("![#999](photo.png)");
    expect(tagPills(view)).toEqual([]);
    cleanup();
  });

  test("a wikilink anchor is not decorated", () => {
    const { view, cleanup } = mount("[[doc#999]]");
    expect(tagPills(view)).toEqual([]);
    cleanup();
  });
});
