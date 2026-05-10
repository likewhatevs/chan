// Fenced code block with always-visible fences and an editable
// language label.
//
// Extends StarterKit's CodeBlock with a NodeView that renders:
//
//   ```<lang>
//   <code content (PM contentDOM)>
//   ```
//
// `<lang>` is a small `contenteditable=true` span living next to
// the opening fence. Typing into it dispatches a transaction that
// updates the node's `language` attribute. The closing fence is a
// `contenteditable=false` footer.
//
// This replaces the `::before` / `::after` pseudo-element fence
// reveal the `liveSource` plugin used to drive: the fences are
// always present (matching how a markdown source view looks) and
// the language is directly editable instead of needing a separate
// picker UI.

import CodeBlock from "@tiptap/extension-code-block";

export const CodeBlockFenced = CodeBlock.extend({
  addNodeView() {
    return ({ node, getPos, editor }) => {
      const wrap = document.createElement("div");
      wrap.className = "md-codeblock-wrap";

      // Opening fence row. The "```" prefix is plain text; the
      // language span is a tiny editable region that mirrors the
      // node's `language` attribute. `contentEditable=false` on
      // the row prevents PM from putting the caret in the literal
      // "```" text; `contentEditable=true` on the span carves out
      // the editable language slot.
      const header = document.createElement("div");
      header.className = "md-codeblock-fence is-open";
      header.contentEditable = "false";
      header.appendChild(document.createTextNode("```"));
      const langInput = document.createElement("span");
      langInput.className = "md-codeblock-lang";
      langInput.contentEditable = "true";
      langInput.spellcheck = false;
      langInput.textContent = (node.attrs.language as string | null) ?? "";
      langInput.addEventListener("input", () => {
        const pos = getPos();
        if (typeof pos !== "number") return;
        const raw = langInput.textContent ?? "";
        // Empty string round-trips as null (no language) so the
        // markdown serializer emits a bare ```.
        const next = raw.length === 0 ? null : raw;
        const current = (editor.state.doc.nodeAt(pos)?.attrs.language ??
          null) as string | null;
        if (next === current) return;
        editor.view.dispatch(
          editor.state.tr.setNodeAttribute(pos, "language", next),
        );
      });
      langInput.addEventListener("keydown", (e) => {
        // Enter on the language slot jumps the caret to the first
        // line of the code content rather than inserting a newline
        // into the language attribute.
        if (e.key === "Enter") {
          e.preventDefault();
          const pos = getPos();
          if (typeof pos !== "number") return;
          editor.commands.focus(pos + 1);
        }
        // Stop Backspace from bubbling out and deleting the
        // codeBlock node when the language slot is empty.
        if (e.key === "Backspace" && langInput.textContent === "") {
          e.preventDefault();
        }
      });
      header.appendChild(langInput);
      wrap.appendChild(header);

      // PM content lives in the `<code>` inside `<pre>`. Same
      // structure StarterKit's default renderHTML produces, so
      // syntax-highlight extensions stay compatible.
      const pre = document.createElement("pre");
      const code = document.createElement("code");
      pre.appendChild(code);
      wrap.appendChild(pre);

      // Closing fence row. Non-editable; the user types `` ``` ``
      // on a fresh line inside the code content to close, or
      // simply navigates out of the block (PM's existing exits
      // still apply).
      const footer = document.createElement("div");
      footer.className = "md-codeblock-fence is-close";
      footer.contentEditable = "false";
      footer.textContent = "```";
      wrap.appendChild(footer);

      return {
        dom: wrap,
        contentDOM: code,
        update(updated) {
          if (updated.type !== node.type) return false;
          const next = (updated.attrs.language as string | null) ?? "";
          // Only resync from the model when it diverges. Skipping
          // the assignment otherwise keeps the user's caret in the
          // language input stable while they type.
          if (langInput.textContent !== next) {
            langInput.textContent = next;
          }
          return true;
        },
      };
    };
  },
});
