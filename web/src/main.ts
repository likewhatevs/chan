// Entry point. Mounts the Svelte 5 root component.

import { mount } from "svelte";
import App from "./App.svelte";

const target = document.getElementById("app");
if (!target) throw new Error("missing #app element");

mount(App, { target });
