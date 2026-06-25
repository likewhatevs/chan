// vite.config.ts
//
// Build the editor web app.
//
// Output goes to the repo-root /web/dist/, which chan-server embeds via
// rust-embed at compile time. This package lives at web/packages/workspace-app
// under the ./web npm-workspaces root, so the embed-output path is two levels
// up (../../dist); the rust-embed input path is frozen, so the source layout
// moves and the output path does not. The Rust backend serves /api/* and the
// WS at /ws; everything else is the SPA, so we set base to "./" to keep asset
// URLs relative.

import { svelte } from "@sveltejs/vite-plugin-svelte";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { defineConfig } from "vitest/config";

// Resolve svelte's client entry through node resolution so it works whether
// svelte is hoisted to the workspace-root node_modules or kept package-local;
// svelte's exports map does not expose ./src, so reach it via the package dir.
const require = createRequire(import.meta.url);
const svelteClient = join(dirname(require.resolve("svelte/package.json")), "src/index-client.js");

export default defineConfig({
  base: "./",
  plugins: [svelte()],
  server: {
    port: 5173,
    // Allow vite to serve files from the chan repo root + the
    // docs/templates/ tree: the team orchestrator imports
    // `docs/templates/team-process/*.tpl?raw` to bundle the process
    // docs into the SPA build at compile time (vite `?raw`, no
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
    // Frozen rust-embed input path (X-2): repo-root web/dist, two levels up
    // from this package. crates/chan-server/build.rs + static_assets.rs are
    // untouched; only the source location moved under ./web.
    outDir: "../../dist",
    emptyOutDir: true,
    target: "es2022",
    sourcemap: false,
    // The SPA ships as one embedded bundle served from localhost by
    // chan-server; aggressive code-splitting is a non-goal. The main
    // chunk sits near 1.5 MB (CodeMirror + xterm + svelte), so the
    // default 500 kB advisory would always fire. Keep a ceiling so
    // real regressions still warn.
    chunkSizeWarningLimit: 1600,
    rollupOptions: {
      onwarn(warning, warn) {
        // Known-cosmetic: store.svelte.ts is imported dynamically as a
        // deliberate module-eval cycle-breaker (tabs.svelte.ts), and
        // @codemirror/lang-html statically pulls lang-css/js that
        // code_languages.ts loads lazily. Neither dynamic import is
        // meant to create a chunk, so the "ineffective" advisory is
        // expected. Everything else still warns.
        if (warning.code === "INEFFECTIVE_DYNAMIC_IMPORT") return;
        warn(warning);
      },
    },
  },
  test: {
    environment: "jsdom",
    alias: [{ find: /^svelte$/, replacement: svelteClient }],
    testTimeout: 30_000,
  },
});
