<script lang="ts">
  // The SPA's one React island: an interactive Excalidraw board mounted
  // inside a Svelte file tab. createRoot ONCE in onMount; every later
  // Svelte-side change goes through the imperative API or a themed
  // re-render, never a root re-creation (the svelte-excalidraw demo's
  // bug). react, react-dom, and @excalidraw/excalidraw are dynamic
  // imports so the eager editor bundle never pulls React; this module is
  // itself reached only via a dynamic import from FileEditorTab, so the
  // static index.css import rides its async chunk instead of the eager
  // CSS. See ../editor/ExcalidrawCanvas source-pin test.
  import { onDestroy, onMount } from "svelte";
  import type {
    ExcalidrawImperativeAPI,
    ExcalidrawInitialDataState,
  } from "@excalidraw/excalidraw/types";
  import { configureExcalidrawAssets } from "./excalidrawAssets";
  import "@excalidraw/excalidraw/index.css";

  type Props = {
    /// The FileTab content buffer: a serialized .excalidraw scene ("" =
    /// a fresh, empty board).
    content: string;
    dark: boolean;
    /// The board changed. The host serializes into the tab buffer, which
    /// the existing autosave path persists.
    onSceneChange: (json: string) => void;
  };
  let { content, dark, onSceneChange }: Props = $props();

  let host: HTMLDivElement | undefined = $state();
  let root: import("react-dom/client").Root | null = null;
  let react: typeof import("react") | null = null;
  let ex: typeof import("@excalidraw/excalidraw") | null = null;
  let api: ExcalidrawImperativeAPI | null = null;

  // Last scene JSON we know the buffer holds. Distinguishes our own
  // serialized output from an external buffer write (reload, 409
  // resolution, sibling-pane mirror) so we neither reparse bytes we just
  // emitted nor dirty the tab on a write we did not make. Mirrors
  // CsvTable's lastSerialized guard. Captures the initial buffer on
  // purpose; the effects below track later content changes.
  // svelte-ignore state_referenced_locally
  let lastSerialized = content;

  // Serialization is debounced: excalidraw's onChange fires per pointer
  // event, and serializing the whole scene on each would jank a drag.
  // serializeAsJSON("local") keeps only elements plus a few persistent
  // appState keys (grid, background), so pan / zoom / selection / theme
  // churn produce an identical string and never dirty the buffer.
  let serializeTimer: ReturnType<typeof setTimeout> | null = null;

  function parseScene(json: string): ExcalidrawInitialDataState | null {
    if (!json.trim()) return null;
    try {
      return JSON.parse(json) as ExcalidrawInitialDataState;
    } catch {
      // A corrupt scene (a hand-edited source-mode typo the save gate
      // somehow let through) opens as an empty board rather than throwing.
      return null;
    }
  }

  function scheduleSerialize(): void {
    if (serializeTimer !== null) clearTimeout(serializeTimer);
    serializeTimer = setTimeout(flushSerialize, 200);
  }

  function flushSerialize(): void {
    serializeTimer = null;
    if (!api || !ex) return;
    const json = ex.serializeAsJSON(
      api.getSceneElements(),
      api.getAppState(),
      api.getFiles(),
      "local",
    );
    if (json === lastSerialized) return;
    lastSerialized = json;
    onSceneChange(json);
  }

  // withInitial mounts the scene on the first render; a themed re-render
  // omits it (excalidraw consumes initialData once, so re-passing would
  // be ignored anyway) to keep the drawn scene.
  function renderExcalidraw(withInitial: boolean): void {
    if (!root || !react || !ex) return;
    root.render(
      react.createElement(ex.Excalidraw, {
        ...(withInitial ? { initialData: parseScene(content) } : {}),
        theme: dark ? "dark" : "light",
        excalidrawAPI: (a: ExcalidrawImperativeAPI) => (api = a),
        onChange: scheduleSerialize,
      }),
    );
  }

  onMount(async () => {
    // Set the asset path before the package loads so the font registry
    // resolves label fonts from the self-hosted bundle, not the CDN.
    configureExcalidrawAssets();
    const [reactDom, r, e] = await Promise.all([
      import("react-dom/client"),
      import("react"),
      import("@excalidraw/excalidraw"),
    ]);
    if (!host) return; // tab closed while the chunk loaded
    react = r;
    ex = e;
    root = reactDom.createRoot(host);
    renderExcalidraw(true);
  });

  // Theme follows the app surface. The theme prop is controlled, so a
  // re-render (not updateScene) is what re-themes; the React root is
  // reused and the scene survives. Reads dark only, so an external
  // content change does not trigger a re-render here.
  $effect(() => {
    void dark;
    if (root) renderExcalidraw(false);
  });

  // External buffer change (reload, 409 resolution, sibling mirror):
  // load the new scene into the live board. Skipped for our own writes
  // via lastSerialized. Reads content only.
  $effect(() => {
    const c = content;
    if (c === lastSerialized) return;
    lastSerialized = c;
    if (!api) return;
    const scene = parseScene(c);
    api.updateScene({ elements: scene?.elements ?? [] });
    const files = scene?.files;
    if (files) api.addFiles(Object.values(files));
  });

  /// Move keyboard focus into the board, the canvas analogue of the
  /// editor caret-grab on tab activation.
  export function focusCanvas(): void {
    host?.focus();
  }

  onDestroy(() => {
    if (serializeTimer !== null) clearTimeout(serializeTimer);
    root?.unmount();
    root = null;
    api = null;
  });
</script>

<div class="excalidraw-shell">
  <div class="excalidraw-host" bind:this={host} tabindex="-1"></div>
</div>

<style>
  .excalidraw-shell {
    position: absolute;
    inset: 0;
    background: var(--bg);
  }
  :global(.chan-page-capped) .excalidraw-shell {
    background: var(--page-shade);
  }
  .excalidraw-host {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    width: min(100%, var(--chan-page-max-width, 100%));
    transform: translateX(-50%);
    background: var(--bg);
    outline: none;
    overflow: hidden;
  }
</style>
