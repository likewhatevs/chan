import "@chan/launcher/styles.css";
import { mount } from "svelte";
import LauncherDemo from "@chan/launcher/demo";

const target = document.getElementById("launcher-demo");

if (target) {
  mount(LauncherDemo, {
    target,
    props: {
      // Any window tile opens the same frontend-only workspace demo. The
      // overlay module (the whole workspace-app bundle) loads on first click.
      onOpenWindow: () => {
        void import("./workspace-demo").then((m) => m.openWorkspaceDemo());
      },
    },
  });
  target.classList.add("mounted");
}
