// Heading folding via a ProseMirror plugin.
//
// Each heading gets a clickable chevron rendered as a widget
// decoration just inside the heading element. Clicking toggles the
// heading's "folded" state; folded headings hide every following
// sibling block until the next heading of equal-or-higher level
// (a node decoration applies a `.md-fold-hidden` class which CSS
// resolves to `display: none`).
//
// Why a plugin (not a Heading node-view extension): the fold state
// is purely local UI, doesn't round-trip through the markdown
// source, and the doc structure (heading + following siblings)
// doesn't fit a single node's nodeView. A plugin keeps the state
// outside the doc and uses PM decorations for the visual layer,
// which composes cleanly with StarterKit's stock Heading.
//
// State persistence: positions move when the user edits above a
// folded heading, so on every transaction we map the folded
// position set through `tr.mapping`. State doesn't survive a tab
// switch (the editor is recreated); that matches how desktop
// editors behave for ephemeral fold state.

import { Extension } from "@tiptap/core";
import type { Node as PMNode } from "@tiptap/pm/model";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import type { EditorState, Transaction } from "@tiptap/pm/state";
import type { EditorView } from "@tiptap/pm/view";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

const foldKey = new PluginKey<FoldState>("foldHeading");

interface FoldState {
  folded: Set<number>;
  decos: DecorationSet;
}

interface ToggleMeta {
  type: "toggle";
  pos: number;
}

function computeDecos(doc: PMNode, folded: Set<number>): DecorationSet {
  const decos: Decoration[] = [];

  // Walk top-level children once to learn the heading layout. We
  // iterate level-by-level so a folded H1 hides everything until
  // the next H1 (or end of doc), a folded H2 hides until next H2/H1,
  // and so on. Headings nested inside other blocks (table cell,
  // blockquote) are skipped here; folding only affects the
  // top-level outline.
  const blocks: { pos: number; node: PMNode }[] = [];
  doc.forEach((node, offset) => {
    blocks.push({ pos: offset, node });
  });

  let foldUntilLevel = 0;
  for (const block of blocks) {
    const isHeading = block.node.type.name === "heading";
    const lvl = isHeading ? ((block.node.attrs.level as number) || 1) : 0;

    // A heading of equal or higher level closes any active fold,
    // even if it isn't itself folded.
    if (foldUntilLevel > 0 && isHeading && lvl <= foldUntilLevel) {
      foldUntilLevel = 0;
    }

    if (foldUntilLevel > 0) {
      decos.push(
        Decoration.node(block.pos, block.pos + block.node.nodeSize, {
          class: "md-fold-hidden",
        }),
      );
    }

    if (isHeading) {
      const isFolded = folded.has(block.pos);
      // Chevron widget sits just inside the heading (side: -1),
      // so PM places it before the heading text. The closure key
      // includes the fold state so PM rebuilds the DOM when it
      // changes (rotating the glyph + flipping the data attr).
      const headingPos = block.pos;
      decos.push(
        Decoration.widget(
          block.pos + 1,
          () => {
            const span = document.createElement("span");
            span.className = "md-fold-chevron";
            span.setAttribute("data-fold-pos", String(headingPos));
            span.setAttribute("contenteditable", "false");
            span.textContent = isFolded ? "▸" : "▾";
            if (isFolded) span.setAttribute("data-folded", "true");
            return span;
          },
          {
            side: -1,
            key: `fold-${headingPos}-${isFolded ? "1" : "0"}`,
            ignoreSelection: true,
          },
        ),
      );
      if (isFolded) {
        // Ellipsis at the end of the heading content cues the user
        // that there's hidden content below.
        decos.push(
          Decoration.widget(
            block.pos + 1 + block.node.content.size,
            () => {
              const span = document.createElement("span");
              span.className = "md-fold-ellipsis";
              span.setAttribute("contenteditable", "false");
              span.textContent = " …";
              return span;
            },
            {
              side: 1,
              key: `ellipsis-${headingPos}`,
              ignoreSelection: true,
            },
          ),
        );
      }
      if (isFolded) {
        foldUntilLevel = lvl;
      }
    }
  }

  return DecorationSet.create(doc, decos);
}

export const FoldHeadingExtension = Extension.create({
  name: "foldHeading",

  addProseMirrorPlugins() {
    return [
      new Plugin<FoldState>({
        key: foldKey,
        state: {
          init(_config, instance: EditorState): FoldState {
            const folded = new Set<number>();
            return { folded, decos: computeDecos(instance.doc, folded) };
          },
          apply(
            tr: Transaction,
            prev: FoldState,
            _oldState: EditorState,
            newState: EditorState,
          ): FoldState {
            // Map every folded position through the transaction so
            // edits above a folded heading don't desync the state
            // from the doc. `mapping.map` returns the new position
            // (or the closest survivor when the heading itself was
            // deleted; the next compute pass drops stale entries
            // because they no longer point at a heading node).
            const folded = new Set<number>();
            for (const pos of prev.folded) {
              const mapped = tr.mapping.map(pos);
              const node = newState.doc.nodeAt(mapped);
              if (node && node.type.name === "heading") {
                folded.add(mapped);
              }
            }
            const meta = tr.getMeta(foldKey) as ToggleMeta | undefined;
            if (meta?.type === "toggle") {
              if (folded.has(meta.pos)) folded.delete(meta.pos);
              else folded.add(meta.pos);
            }
            // Recompute when the doc changed OR the folded set
            // changed; otherwise return the prior decoration set
            // so the caller benefits from PM's reference-equality
            // shortcut.
            const setChanged =
              folded.size !== prev.folded.size ||
              [...folded].some((p) => !prev.folded.has(p));
            if (!tr.docChanged && !setChanged && !meta) {
              return { folded, decos: prev.decos };
            }
            return { folded, decos: computeDecos(newState.doc, folded) };
          },
        },
        props: {
          decorations(state) {
            return foldKey.getState(state)?.decos ?? null;
          },
          handleClick(view: EditorView, _pos: number, event: MouseEvent) {
            const target = event.target as HTMLElement | null;
            if (!target) return false;
            const chevron = target.closest(".md-fold-chevron") as HTMLElement | null;
            if (!chevron) return false;
            const raw = chevron.getAttribute("data-fold-pos");
            if (raw == null) return false;
            const headingPos = parseInt(raw, 10);
            if (Number.isNaN(headingPos)) return false;
            event.preventDefault();
            view.dispatch(
              view.state.tr.setMeta(foldKey, {
                type: "toggle",
                pos: headingPos,
              } as ToggleMeta),
            );
            return true;
          },
        },
      }),
    ];
  },
});
