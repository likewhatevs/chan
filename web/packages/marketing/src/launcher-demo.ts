import "@chan/launcher/styles.css";
import { mount } from "svelte";
import LauncherDemo from "@chan/launcher/demo";

const target = document.getElementById("launcher-demo");

if (target) {
  const variant = target.dataset.variant;
  mount(LauncherDemo, {
    target,
    // Per-page config rides on the mount node so one bundle serves the home
    // hero (no data attributes, populated library), the manual's empty
    // first-run embed (data-variant="empty" data-hints="true"), and the
    // devserver-form embed page (data-variant="devserver").
    props: {
      variant: variant === "empty" || variant === "devserver" ? variant : "populated",
      hints: target.dataset.hints === "true",
    },
  });
  target.classList.add("mounted");
}
