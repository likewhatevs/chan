// Inline `#tag` rendering as clickable pills.
//
// A ProseMirror plugin that scans the doc for `#word` tokens and
// adds an inline decoration with class `md-tag-pill`. CSS turns
// the run into a rounded chip; the plugin's `handleClick` prop
// translates a click on the chip into a tag-inspector open via
// the host-supplied callback (typically `openGraphAtNode`).
//
// Why a plugin (not an inline node):
//   - Tags don't need attrs of their own; the `#name` text IS the
//     identity. Wrapping each occurrence in an inline node would
//     require a markdown parser hook + serializer + input rule
//     and would round-trip awkwardly through tiptap-markdown.
//   - Decorations rebuild on every transaction so freshly typed
//     tags style live without extra wiring; deletion is automatic.
//
// The match is bounded: `#` must be at start-of-text-node or
// preceded by a non-word character (so URLs like `#section` inside
// `https://…` and ids like `foo#bar` don't accidentally pill).

import { Extension } from "@tiptap/core";
import type { Node as PMNode } from "@tiptap/pm/model";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import type { EditorState, Transaction } from "@tiptap/pm/state";
import type { EditorView } from "@tiptap/pm/view";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

const tagKey = new PluginKey<DecorationSet>("tagDecoration");

// `(?:^|[^A-Za-z0-9_])` is a non-capturing prefix that the regex
// engine is allowed to consume; we re-derive the actual `#` start
// from m.index + the prefix length so the decoration covers only
// the `#name` substring, not the leading separator.
const TAG_RE = /(?:^|[^A-Za-z0-9_])(#[A-Za-z0-9_-]+)/g;

function computeDecos(doc: PMNode): DecorationSet {
  const decos: Decoration[] = [];
  doc.descendants((node, pos, parent) => {
    if (!node.isText || !node.text) return;
    // Skip code spans + code blocks: a `#` inside a snippet is
    // never a tag.
    if (parent?.type.name === "codeBlock") return false;
    if (node.marks.some((m) => m.type.name === "code")) return;
    const text = node.text;
    let m: RegExpExecArray | null;
    TAG_RE.lastIndex = 0;
    while ((m = TAG_RE.exec(text)) !== null) {
      const tagWithHash = m[1] ?? "";
      if (!tagWithHash) continue;
      const startInText = m.index + (m[0].length - tagWithHash.length);
      const from = pos + startInText;
      const to = from + tagWithHash.length;
      decos.push(
        Decoration.inline(from, to, {
          class: "md-tag-pill",
          "data-tag": tagWithHash.slice(1),
        }),
      );
    }
  });
  return DecorationSet.create(doc, decos);
}

export interface TagDecorationOptions {
  /// Called when the user clicks a tag pill in the editor. The
  /// argument is the bare tag name (no leading `#`); the host is
  /// responsible for whatever surfaces the inspector for it.
  onTagClick?: (name: string) => void;
}

export function createTagDecorationExtension(opts: TagDecorationOptions = {}) {
  return Extension.create({
    name: "tagDecoration",
    addProseMirrorPlugins() {
      return [
        new Plugin<DecorationSet>({
          key: tagKey,
          state: {
            init(_config, instance: EditorState): DecorationSet {
              return computeDecos(instance.doc);
            },
            apply(
              tr: Transaction,
              prev: DecorationSet,
              _oldState: EditorState,
              newState: EditorState,
            ): DecorationSet {
              if (!tr.docChanged) {
                return prev.map(tr.mapping, tr.doc);
              }
              return computeDecos(newState.doc);
            },
          },
          props: {
            decorations(state: EditorState) {
              return tagKey.getState(state) ?? null;
            },
            handleClick(_view: EditorView, _pos: number, event: MouseEvent) {
              const target = event.target as HTMLElement | null;
              if (!target) return false;
              const pill = target.closest(".md-tag-pill") as HTMLElement | null;
              if (!pill) return false;
              const name = pill.getAttribute("data-tag");
              if (!name) return false;
              event.preventDefault();
              opts.onTagClick?.(name);
              return true;
            },
          },
        }),
      ];
    },
  });
}
