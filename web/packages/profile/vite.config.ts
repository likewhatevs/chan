// Build the id.chan.app SPA (@chan/profile).
//
// Output goes to the frozen gateway/crates/identity/web/dist, which the
// gateway identity crate embeds via rust-embed at compile time. This package
// lives at web/packages/profile under the ./web npm-workspaces root, so the
// embed-output path is three levels up; the rust-embed input path is frozen
// (X-3), so the source lifted into ./web and the output path did not, and the
// gateway identity crate is untouched. Backend serves /auth/*, /api/*,
// /healthz; everything else falls back to the SPA, so we keep asset URLs
// relative.

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
    // Frozen rust-embed input path (X-2/X-3): the gateway identity crate's
    // gateway/crates/identity/web/dist, three levels up from this package. The
    // gateway crate's static_files.rs #[folder = "web/dist/"] is untouched.
    outDir: "../../../gateway/crates/identity/web/dist",
    emptyOutDir: true,
    target: "es2022",
    sourcemap: false,
  },
});
