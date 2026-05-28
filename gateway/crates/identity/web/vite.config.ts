// Build the id.chan.app SPA.
//
// Output goes to /web/dist/, which identity-service embeds via
// rust-embed at compile time. Backend serves /auth/*, /api/*,
// /healthz; everything else falls back to the SPA, so we keep
// asset URLs relative.

import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

export default defineConfig({
  base: "./",
  plugins: [svelte()],
  server: {
    port: 5173,
    // Proxy backend routes to a running identity-service so
    // `npm run dev` works end-to-end against real OAuth.
    proxy: {
      "/api": "http://127.0.0.1:7000",
      "/auth": "http://127.0.0.1:7000",
      "/healthz": "http://127.0.0.1:7000",
    },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
    sourcemap: false,
  },
});
