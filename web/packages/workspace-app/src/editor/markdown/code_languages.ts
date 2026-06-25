// Language descriptions for fenced code blocks.
//
// Each entry uses CM6's LanguageDescription.of() with an async `load`
// callback - the pack is only fetched the first time a code fence
// claims that language. Vite emits each dynamic import as its own
// chunk, so the main bundle stays small and language support arrives
// on demand.
//
// Coverage targets the common cases a notes app sees: web (js/ts,
// html, css, json), systems (rust, cpp, go), data (python, sql, yaml,
// toml), and shell (bash via legacy-modes/shell). `info` strings are
// matched case-insensitively by LanguageDescription.matchLanguageName;
// aliases are listed so `js` / `javascript`, `py` / `python`, etc.
// all resolve.

import {
  LanguageDescription,
  LanguageSupport,
  StreamLanguage,
} from "@codemirror/language";

export const codeLanguages: LanguageDescription[] = [
  LanguageDescription.of({
    name: "javascript",
    alias: ["js", "jsx", "ts", "tsx", "typescript"],
    extensions: ["js", "jsx", "ts", "tsx", "mjs", "cjs"],
    load: () =>
      import("@codemirror/lang-javascript").then((m) =>
        m.javascript({
          jsx: true,
          typescript: true,
        }),
      ),
  }),
  LanguageDescription.of({
    name: "python",
    alias: ["py"],
    extensions: ["py"],
    load: () => import("@codemirror/lang-python").then((m) => m.python()),
  }),
  LanguageDescription.of({
    name: "rust",
    alias: ["rs"],
    extensions: ["rs"],
    load: () => import("@codemirror/lang-rust").then((m) => m.rust()),
  }),
  LanguageDescription.of({
    name: "go",
    alias: ["golang"],
    extensions: ["go"],
    load: () => import("@codemirror/lang-go").then((m) => m.go()),
  }),
  LanguageDescription.of({
    name: "cpp",
    alias: ["c", "c++", "h", "hpp"],
    extensions: ["c", "cc", "cpp", "h", "hpp"],
    load: () => import("@codemirror/lang-cpp").then((m) => m.cpp()),
  }),
  LanguageDescription.of({
    name: "html",
    alias: ["htm"],
    extensions: ["html", "htm"],
    load: () => import("@codemirror/lang-html").then((m) => m.html()),
  }),
  LanguageDescription.of({
    name: "css",
    extensions: ["css"],
    load: () => import("@codemirror/lang-css").then((m) => m.css()),
  }),
  LanguageDescription.of({
    name: "json",
    extensions: ["json"],
    load: () => import("@codemirror/lang-json").then((m) => m.json()),
  }),
  LanguageDescription.of({
    name: "yaml",
    alias: ["yml"],
    extensions: ["yaml", "yml"],
    load: () => import("@codemirror/lang-yaml").then((m) => m.yaml()),
  }),
  LanguageDescription.of({
    name: "sql",
    extensions: ["sql"],
    load: () => import("@codemirror/lang-sql").then((m) => m.sql()),
  }),
  LanguageDescription.of({
    name: "shell",
    alias: ["sh", "bash", "zsh"],
    extensions: ["sh", "bash"],
    load: () =>
      import("@codemirror/legacy-modes/mode/shell").then(
        (m) => new LanguageSupport(StreamLanguage.define(m.shell)),
      ),
  }),
  LanguageDescription.of({
    name: "toml",
    extensions: ["toml"],
    load: () =>
      import("@codemirror/legacy-modes/mode/toml").then(
        (m) => new LanguageSupport(StreamLanguage.define(m.toml)),
      ),
  }),
];
