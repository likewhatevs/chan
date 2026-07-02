import "@chan/launcher/styles.css";
import { mount } from "svelte";
import LauncherDemo from "@chan/launcher/demo";

const target = document.getElementById("launcher-demo");

if (target) {
  mount(LauncherDemo, {
    target,
  });
  target.classList.add("mounted");
}
