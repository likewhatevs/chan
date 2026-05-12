// Bold / Italic / Strike with their auto-render input rules stripped.
//
// StarterKit's default rule applies the mark the moment the user types
// the closing delimiter (`*…*`, `**…**`, `~~…~~`). The chan editor
// renders later — once the caret moves strictly outside the closed
// pattern's range — so the user gets to see the markers they just
// typed for a beat (the `liveSource` plugin syntax-highlights them in
// the meantime). We keep the marks themselves intact (Cmd+B / Cmd+I /
// Cmd+Shift+S still toggle, paste rules still fire, the markdown
// serializer still emits `**…**`); only the typing-rule is dropped.
//
// StarterKit doesn't expose addInputRules as a config knob, so we
// disable bold / italic / strike at the StarterKit level (see
// Wysiwyg.svelte's StarterKit.configure) and re-add these extended
// versions afterwards.

import Bold from "@tiptap/extension-bold";
import Italic from "@tiptap/extension-italic";
import Strike from "@tiptap/extension-strike";

export const BoldNoInputRule = Bold.extend({
  addInputRules() {
    return [];
  },
});

export const ItalicNoInputRule = Italic.extend({
  addInputRules() {
    return [];
  },
});

export const StrikeNoInputRule = Strike.extend({
  addInputRules() {
    return [];
  },
});
