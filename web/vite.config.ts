// vite.config.ts
//
// Build the editor web app.
//
// Output goes to /web/dist/, which md-cli embeds via rust-embed at compile
// time. The Rust backend serves /api/* and the WS at /ws; everything else
// is the SPA, so we set base to "./" to keep asset URLs relative.
//
// Wasm: the shared logic crate is built into /web/pkg/ by `npm run wasm`
// (which runs wasm-pack on ../crates/md-shared). Vite picks it up as a
// regular ES module. If the package isn't built yet, the editor falls back
// to TS-only behavior (see src/api/wasm.ts).

import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

const svelteClient = fileURLToPath(new URL("./node_modules/svelte/src/index-client.js", import.meta.url));

export default defineConfig({
  base: "./",
  plugins: [svelte()],
  server: {
    port: 5173,
    // `fullstack-a-79` slice 3: allow vite to serve files from the
    // chan repo root + the docs/templates/ tree. The team
    // orchestrator imports `docs/templates/team-process/*.tpl?raw`
    // to bundle the process docs into the SPA build at compile
    // time (architect-routed delivery shape: vite `?raw`, no
    // chan-server endpoint). Without this `fs.allow`, vite's
    // default `fs.strict` blocks the parent-dir traversal.
    fs: {
      allow: [".", ".."],
    },
    // While iterating, proxy API + WS to a `md serve` instance so we get
    // the real backend without rebuilding the binary on every change.
    proxy: {
      "/api/terminal/ws": { target: "ws://127.0.0.1:8787", ws: true },
      "/api": "http://127.0.0.1:8787",
      "/ws": { target: "ws://127.0.0.1:8787", ws: true },
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
  },
});
