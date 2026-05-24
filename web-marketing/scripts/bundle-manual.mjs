#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const siteRoot = path.resolve(path.dirname(scriptPath), "..");
const repoRoot = path.resolve(siteRoot, "..");
const distRoot = path.join(siteRoot, "dist");

async function main() {
  const options = await parseArgs(process.argv.slice(2));
  const version = options.version ?? (await readWorkspaceVersion());
  const output = options.out ?? path.join(distRoot, `chan-manual-${version}.tar.gz`);
  const outputPath = path.resolve(output);
  const staging = await fs.mkdtemp(path.join(os.tmpdir(), "chan-manual-bundle-"));

  try {
    await stageBundle(staging);
    await fs.mkdir(path.dirname(outputPath), { recursive: true });
    execFileSync("tar", ["-czf", outputPath, "-C", staging, "."]);
    const entries = listTar(outputPath);
    validateBundleEntries(entries);
    if (options.list) {
      for (const entry of entries.slice(0, 80)) {
        console.log(entry);
      }
    }
    console.log(`wrote ${path.relative(repoRoot, outputPath)}`);
  } finally {
    await fs.rm(staging, { recursive: true, force: true });
    if (options.check) {
      await fs.rm(outputPath, { force: true });
    }
  }
}

async function parseArgs(args) {
  const options = { check: false, list: false, out: null, version: null };
  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];
    if (arg === "--check") {
      options.check = true;
      options.out = path.join(os.tmpdir(), `chan-manual-check-${Date.now()}.tar.gz`);
    } else if (arg === "--list") {
      options.list = true;
    } else if (arg === "--out") {
      options.out = args[i + 1] ?? "";
      i += 1;
    } else if (arg === "--version") {
      options.version = args[i + 1] ?? "";
      i += 1;
    } else if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  if (options.out === "") throw new Error("--out requires a value");
  if (options.version === "") throw new Error("--version requires a value");
  if (options.check && args.includes("--out")) {
    throw new Error("--check and --out cannot be used together");
  }
  return options;
}

function printHelp() {
  console.log(`usage: node scripts/bundle-manual.mjs [--version X.Y.Z] [--out PATH] [--check] [--list]

Builds chan-manual-<version>.tar.gz from web-marketing/dist/manual and shared
site assets. Run npm run build first.
`);
}

async function readWorkspaceVersion() {
  const cargoToml = await fs.readFile(path.join(repoRoot, "Cargo.toml"), "utf8");
  const match = cargoToml.match(/^\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m);
  if (!match) throw new Error("workspace package version not found in Cargo.toml");
  return match[1];
}

async function stageBundle(staging) {
  const required = [
    "manual/index.html",
    "manual/install/index.html",
    "assets/site.css",
    "assets/site.js",
    "favicon.ico",
    "chan-mark.png",
  ];
  for (const relative of required) {
    const stat = await fs.stat(path.join(distRoot, relative)).catch(() => null);
    if (!stat?.isFile()) {
      throw new Error(`dist is missing ${relative}; run npm run build first`);
    }
  }

  await fs.cp(path.join(distRoot, "manual"), path.join(staging, "manual"), {
    recursive: true,
  });
  await fs.cp(path.join(distRoot, "assets"), path.join(staging, "assets"), {
    recursive: true,
  });
  await fs.copyFile(path.join(distRoot, "favicon.ico"), path.join(staging, "favicon.ico"));
  await fs.copyFile(path.join(distRoot, "chan-mark.png"), path.join(staging, "chan-mark.png"));
}

function listTar(outputPath) {
  return execFileSync("tar", ["-tzf", outputPath], { encoding: "utf8" })
    .split(/\r?\n/)
    .filter(Boolean)
    .sort();
}

function validateBundleEntries(entries) {
  const required = [
    "./manual/index.html",
    "./manual/install/index.html",
    "./assets/site.css",
    "./assets/site.js",
    "./favicon.ico",
    "./chan-mark.png",
  ];
  for (const entry of required) {
    if (!entries.includes(entry)) {
      throw new Error(`manual bundle is missing ${entry}`);
    }
  }
}

main().catch((err) => {
  console.error(`manual bundle failed: ${err.message}`);
  process.exitCode = 1;
});
