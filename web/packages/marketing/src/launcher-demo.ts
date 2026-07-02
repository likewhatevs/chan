import "@chan/launcher/styles.css";
import { mount } from "svelte";
import LauncherDemo from "@chan/launcher/demo";

const target = document.getElementById("launcher-demo");

if (target) {
  // Display-only launcher mock: window tiles toggle the demo's own state but
  // open no workspace overlay on the marketing site.
  mount(LauncherDemo, { target });
  target.classList.add("mounted");
}
