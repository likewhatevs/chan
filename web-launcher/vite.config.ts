// vite.config.ts
//
// Build the launcher web app (the workspace/devserver launcher).
//
// Output goes to /web-launcher/dist/, which chan-library embeds via
// rust-embed at compile time and serves from its loopback HTTP server.
// The library serves /api/library/* and the /api/library/windows/watch
// socket; everything else is this SPA, so base is "./" to keep asset URLs
// relative. Mirrors web/vite.config.ts. The launcher is a pure HTTP
// client, so it carries none of web/'s editor (CodeMirror), terminal
// (xterm), or graph (cytoscape) weight.

import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

const svelteClient = fileURLToPath(new URL("./node_modules/svelte/src/index-client.js", import.meta.url));

export default defineConfig({
  base: "./",
  plugins: [svelte()],
  server: {
    port: 5174,
    // While iterating, proxy the library HTTP surface to a running
    // `chan devserver` (or the local loopback library) so the SPA talks
    // to the real backend without rebuilding the binary on every change.
    proxy: {
      "/api/library/windows/watch": { target: "ws://127.0.0.1:8787", ws: true },
      "/api": "http://127.0.0.1:8787",
    },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
    sourcemap: false,
  },
  test: {
    environment: "jsdom",
    alias: [{ find: /^svelte$/, replacement: svelteClient }],
    testTimeout: 30_000,
  },
});
