// Entry point. Mounts the Svelte 5 root component.

import { mount } from "svelte";
import App from "./App.svelte";
// `fullstack-b-12`: Source Code Pro Regular @font-face declaration
// for the in-app terminal. Imported here so the face starts loading
// at app boot rather than at first terminal spawn; xterm.js still
// renders against the fallback chain while the woff2 is in flight.
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
