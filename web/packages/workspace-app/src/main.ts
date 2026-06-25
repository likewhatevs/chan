// Entry point. Mounts the Svelte 5 root component.

import { mount } from "svelte";
import App from "./App.svelte";
// Source Code Pro Regular @font-face declaration for the in-app
// terminal. Imported at app boot rather than at first terminal spawn
// so the face is in flight before xterm.js needs it.
import "./fonts.css";
// Editor themes. base.css declares every `--chan-editor-*` variable
// with a neutral default; per-theme files override under
// `[data-editor-theme="<name>"]`. The active theme is applied as a
// `data-editor-theme` attr on documentElement by state/editorTheme.
import "./editor/themes/base.css";
import "./editor/themes/github.css";
import "./editor/themes/google_docs.css";
import "./editor/themes/word.css";

const target = document.getElementById("app");
if (!target) throw new Error("missing #app element");

mount(App, { target });
