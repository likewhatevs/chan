// Fenced code block with always-visible fences and an editable
// language label, all inside a single styled box.
//
// NodeView layout:
//
//   <div class="md-codeblock">           (the styled box)
//     <div class="md-codeblock-fence">
//       ```<input class="md-codeblock-lang" />
//     </div>
//     <pre class="md-codeblock-content">
//       <code>                            (PM contentDOM)
//     </pre>
//     <div class="md-codeblock-fence">```</div>
//   </div>
//
// The language is a real `<input type="text">`. Native inputs are
// black boxes from PM's perspective: it doesn't try to reconcile
// their internal state, and `stopEvent` keeps PM from intercepting
// keystrokes that should go to the input. The fence wrapper rows
// are non-editable; only the input slot and the code content
// accept the caret.
//
// On `input` events we dispatch `setNodeAttribute("language", ...)`
// so the change round-trips through markdown serialization. The
// `update()` callback resyncs the input value when the model
// changes from elsewhere (paste, undo).

import CodeBlock from "@tiptap/extension-code-block";

export const CodeBlockFenced = CodeBlock.extend({
  addNodeView() {
    return ({ node, getPos, editor }) => {
      const wrap = document.createElement("div");
      wrap.className = "md-codeblock";

      // Opening fence: literal "```" plus the language `<input>`.
      const header = document.createElement("div");
      header.className = "md-codeblock-fence is-open";
      header.contentEditable = "false";
      header.appendChild(document.createTextNode("```"));
      const langInput = document.createElement("input");
      langInput.type = "text";
      langInput.className = "md-codeblock-lang";
      langInput.spellcheck = false;
      langInput.autocapitalize = "off";
      langInput.autocomplete = "off";
      langInput.placeholder = "lang";
      langInput.value = (node.attrs.language as string | null) ?? "";
      langInput.addEventListener("input", () => {
        const pos = getPos();
        if (typeof pos !== "number") return;
        const raw = langInput.value;
        // Empty string round-trips as null so the markdown
        // serializer emits a bare ```.
        const next = raw.length === 0 ? null : raw;
        const current = (editor.state.doc.nodeAt(pos)?.attrs.language ??
          null) as string | null;
        if (next === current) return;
        editor.view.dispatch(
          editor.state.tr.setNodeAttribute(pos, "language", next),
        );
      });
      langInput.addEventListener("keydown", (e) => {
        // Enter on the language slot moves the caret to the first
        // line of the code content rather than submitting / firing
        // PM Enter handlers.
        if (e.key === "Enter") {
          e.preventDefault();
          e.stopPropagation();
          const pos = getPos();
          if (typeof pos !== "number") return;
          editor.commands.focus(pos + 1);
        }
      });
      header.appendChild(langInput);
      wrap.appendChild(header);

      // PM content lives inside `<pre><code>`. Same shape
      // StarterKit emits so syntax-highlighting extensions stay
      // compatible if we add one later.
      const pre = document.createElement("pre");
      pre.className = "md-codeblock-content";
      const code = document.createElement("code");
      pre.appendChild(code);
      wrap.appendChild(pre);

      // Closing fence row. Non-editable; the user closes the block
      // by exiting the code content (Tiptap's default exits still
      // apply: Enter on an empty trailing line, Mod-Enter, etc.).
      const footer = document.createElement("div");
      footer.className = "md-codeblock-fence is-close";
      footer.contentEditable = "false";
      footer.textContent = "```";
      wrap.appendChild(footer);

      const isOurUI = (target: EventTarget | null): boolean => {
        if (!(target instanceof Node)) return false;
        return (
          header.contains(target) ||
          footer.contains(target) ||
          target === langInput
        );
      };

      return {
        dom: wrap,
        contentDOM: code,
        // PM owns events on the code content; events sourced from
        // the fence rows / language input belong to us and must not
        // reach PM (otherwise PM either swallows keystrokes that
        // should land in the input, or blurs the input on click).
        stopEvent(event) {
          return isOurUI(event.target);
        },
        // Same logic for DOM mutations: changes inside our UI
        // shouldn't trigger PM's reconciliation pass.
        ignoreMutation(mutation) {
          return isOurUI(mutation.target);
        },
        update(updated) {
          if (updated.type !== node.type) return false;
          const next = (updated.attrs.language as string | null) ?? "";
          // Skip the assignment when unchanged so the user's caret
          // position inside the input stays stable as they type.
          if (langInput.value !== next) {
            langInput.value = next;
          }
          return true;
        },
      };
    };
  },
});
