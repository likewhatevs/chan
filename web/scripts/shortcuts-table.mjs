#!/usr/bin/env node
// Print the keyboard-shortcut table from web/src/state/shortcuts.ts.
// Two modes:
//   default                   plain ASCII (what the empty pane renders)
//   --serve-long-about        wrapped in the `chan serve --help` framing
//                             so the output drops straight into
//                             crates/chan/src/main.rs.
//
// Usage:
//   node web/scripts/shortcuts-table.mjs
//   node web/scripts/shortcuts-table.mjs --serve-long-about
//
// Pick `--platform` (web | native) and `--os` (mac | linux | windows)
// to render alternate variants; defaults to the web fallback set with
// `Mod` rendered as `Cmd` (the same shape the existing SERVE_LONG_ABOUT
// uses).

import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { execSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { pathToFileURL } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const tsPath = join(here, "..", "src", "state", "shortcuts.ts");
const webDir = join(here, "..");

// Compile shortcuts.ts to a temp .mjs and import. Keeps shortcuts.ts
// as the only place chord data lives — no parallel JS shadow file
// to drift out of sync. tsc is already installed for the web build.
const work = mkdtempSync(join(tmpdir(), "chan-shortcuts-"));
try {
  const inFile = join(work, "shortcuts.ts");
  writeFileSync(inFile, readFileSync(tsPath, "utf8"));
  execSync(
    `node_modules/.bin/tsc --target es2022 --module es2022 --moduleResolution bundler --strict --outDir ${JSON.stringify(work)} ${JSON.stringify(inFile)}`,
    { cwd: webDir, stdio: ["ignore", "ignore", "inherit"] },
  );
  const outFile = join(work, "shortcuts.js");
  const mod = await import(pathToFileURL(outFile).href);

  const args = process.argv.slice(2);
  const flag = (name, fallback) => {
    const i = args.indexOf(name);
    return i < 0 ? fallback : args[i + 1];
  };
  const wrap = args.includes("--serve-long-about");
  const platform = flag("--platform", "web");
  const os = flag("--os", "mac");

  const table = mod.renderTable(platform, os);

  if (!wrap) {
    process.stdout.write(table + "\n");
  } else {
    // Frame matching the existing SERVE_LONG_ABOUT block in
    // crates/chan/src/main.rs. Sync this prose if main.rs's framing
    // ever changes.
    const indented = table.replace(/^/gm, "  ");
    process.stdout.write(`Run the HTTP server. Defaults to 127.0.0.1 (loopback only).

In-app keybindings (Cmd = Ctrl on Linux / Windows):

${indented}
`);
  }
} finally {
  rmSync(work, { recursive: true, force: true });
}
