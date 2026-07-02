<script lang="ts">
  // Frontend-only demo wrapper: runs the real workspace app against the
  // in-memory mock, with no backend. Mirrors launcher/LauncherDemo.svelte.
  //
  // The mock transport is installed at module init (top of this script), which
  // runs before the child <App/> is created and therefore before App's
  // onMount bootstrap issues any request. From then on every fetch and
  // WebSocket the app makes is served from the snapshot in memory.

  import App from "./App.svelte";
  import type { MockWorkspaceData } from "./demo/data";
  import { installDemoWorkspace } from "./demo/install";
  // Same global styles the real entry (main.ts) loads.
  import "./fonts.css";
  import "./editor/themes/base.css";
  import "./editor/themes/github.css";
  import "./editor/themes/google_docs.css";
  import "./editor/themes/word.css";

  let { data }: { data: MockWorkspaceData } = $props();

  // Initial-value capture is the point: the snapshot is loaded once before
  // mount and never swapped at runtime.
  // svelte-ignore state_referenced_locally
  installDemoWorkspace(data);
</script>

<!-- The marketing build scopes the bundle's :root variable blocks to this
     frame (scopeDemoCss), keyed on data-theme like the launcher embed. The
     demo always runs dark. -->
<div class="workspace-demo-frame" data-theme="dark">
  <App />
</div>

<style>
  /* Fill the host (the overlay panel on the marketing site, or the full
     viewport in the dev harness). `isolation: isolate` keeps the app's
     stacking context self-contained so its overlays never escape the frame. */
  .workspace-demo-frame {
    width: 100%;
    height: 100%;
    overflow: hidden;
    isolation: isolate;
  }
</style>
