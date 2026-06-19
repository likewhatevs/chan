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

// The backend port the dev server proxies to. Defaults to 8787 (the local
// loopback library / a default `chan devserver`); override with VITE_PROXY_PORT
// to point at a devserver on another port.
const proxyPort = process.env.VITE_PROXY_PORT ?? "8787";

export default defineConfig({
  base: "./",
  plugins: [svelte()],
  server: {
    port: 5174,
    // While iterating, proxy the library HTTP surface to a running
    // `chan devserver` (or the local loopback library) so the SPA talks
    // to the real backend without rebuilding the binary on every change.
    proxy: {
      "/api/library/windows/watch": { target: `ws://127.0.0.1:${proxyPort}`, ws: true },
      "/api": `http://127.0.0.1:${proxyPort}`,
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
