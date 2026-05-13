// Syntax-highlight palettes for code in fenced blocks (and any other
// lang-mode code the editor renders). Two HighlightStyle objects,
// one per light / dark; base.ts picks one per ChanTheme.
//
// Palette source: GitHub Primer "color/syntax/*" tokens. The dark
// palette matches github.com's current dark code view; the light
// palette matches its current light view. The editor-theme dimension
// (github / google_docs / word) does NOT branch here on purpose: a
// shared syntax palette across all three editor themes keeps code
// snippets reading the same regardless of which document chrome is
// active. Themes only differ in body typography, headings, slab bg.

import { HighlightStyle } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";

type Palette = {
  comment: string;
  keyword: string;
  function: string;
  string: string;
  constant: string;
  variable: string;
  tag: string;
  regexp: string;
  invalid: string;
};

const DARK: Palette = {
  comment: "#8b949e",
  keyword: "#ff7b72",
  function: "#d2a8ff",
  string: "#a5d6ff",
  constant: "#79c0ff",
  variable: "#ffa657",
  tag: "#7ee787",
  regexp: "#7ee787",
  invalid: "#ffa198",
};

const LIGHT: Palette = {
  comment: "#6e7781",
  keyword: "#cf222e",
  function: "#8250df",
  string: "#0a3069",
  constant: "#0550ae",
  variable: "#953800",
  tag: "#116329",
  regexp: "#116329",
  invalid: "#82071e",
};

function paletteHighlight(p: Palette): HighlightStyle {
  return HighlightStyle.define([
    { tag: [t.comment, t.lineComment, t.blockComment, t.docComment], color: p.comment, fontStyle: "italic" },
    { tag: [t.keyword, t.controlKeyword, t.operatorKeyword, t.modifier, t.definitionKeyword], color: p.keyword },
    { tag: [t.function(t.variableName), t.function(t.propertyName), t.macroName], color: p.function },
    { tag: [t.string, t.special(t.string), t.character], color: p.string },
    { tag: [t.number, t.bool, t.atom, t.literal, t.constant(t.variableName), t.standard(t.variableName)], color: p.constant },
    // Intentionally no color on plain variable / property /
    // attribute names so bare identifiers (e.g. `foo` on its own
    // line in a python block) fall through to the editor's body
    // color: white on dark, near-black on light. GitHub Primer
    // paints these orange, but #ffa657 collides with the chan
    // brand orange (--assistant-accent / ensō logo), so we
    // deliberately diverge.
    { tag: [t.typeName, t.namespace, t.className, t.tagName], color: p.tag },
    { tag: [t.regexp, t.escape], color: p.regexp },
    { tag: t.invalid, color: p.invalid },
    // Markdown-specific (the doc IS markdown so these fire on the
    // outer surface too). Heading color comes from the editor-theme
    // CSS var (--chan-editor-heading-color); we only re-add weight
    // here because Wysiwyg.svelte's `[class*="cm-md-h"] *` rule
    // forces descendants to inherit the heading color. Emphasis /
    // strong stay style-only so *italic* / **bold** still render
    // correctly in the source view.
    { tag: t.heading, fontWeight: "bold" },
    { tag: t.emphasis, fontStyle: "italic" },
    { tag: t.strong, fontWeight: "bold" },
    { tag: t.strikethrough, textDecoration: "line-through" },
  ]);
}

export const githubLightHighlight = paletteHighlight(LIGHT);
export const githubDarkHighlight = paletteHighlight(DARK);
