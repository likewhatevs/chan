#!/usr/bin/env node

import { promises as fs } from "node:fs";
import path from "node:path";

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const releases = await fetchOptionalJson(`${options.base}/releases.json`);
  if (!releases) {
    console.warn("warning: no existing /dl metadata to preserve");
    return;
  }
  const latestTag = requireString(releases.latest_tag, "releases.latest_tag");
  if (!/^v\d+\.\d+\.\d+$/.test(latestTag)) {
    throw new Error(`existing releases.json has invalid latest_tag: ${latestTag}`);
  }

  const files = [
    "releases.json",
    "cli/latest.json",
    `cli/${latestTag}.json`,
    "desktop/latest.json",
    `desktop/${latestTag}.json`,
  ];
  for (const file of files) {
    const body = file === "releases.json"
      ? `${JSON.stringify(releases, null, 2)}\n`
      : await fetchRequiredText(`${options.base}/${file}`);
    const target = path.join(options.out, file);
    await fs.mkdir(path.dirname(target), { recursive: true });
    await fs.writeFile(target, body);
  }
  console.log(`preserved existing /dl metadata for ${latestTag}`);
}

function parseArgs(args) {
  const options = {
    base: "https://chan.app/dl",
    out: "dist/dl",
  };
  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];
    if (arg === "--base") {
      options.base = trimRightSlash(args[i + 1] ?? "");
      i += 1;
    } else if (arg === "--out") {
      options.out = args[i + 1] ?? "";
      i += 1;
    } else if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  if (!options.base) throw new Error("--base requires a value");
  if (!options.out) throw new Error("--out requires a value");
  return options;
}

function printHelp() {
  console.log(`usage: node scripts/preserve-release-metadata.mjs [--base https://chan.app/dl] [--out dist/dl]

Copies already-published /dl metadata into a freshly built Pages artifact. This
does not generate new release metadata; release.yml owns that gated path.
`);
}

async function fetchOptionalJson(url) {
  const response = await fetch(url, { headers: { Accept: "application/json" } });
  if (response.status === 404) return null;
  if (!response.ok) throw new Error(`${url} returned HTTP ${response.status}`);
  return response.json();
}

async function fetchRequiredText(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url} returned HTTP ${response.status}`);
  return response.text();
}

function requireString(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(`${label} is required`);
  }
  return value.trim();
}

function trimRightSlash(value) {
  return value.trim().replace(/\/+$/, "");
}

main().catch((err) => {
  console.error(`release metadata preservation failed: ${err.message}`);
  process.exitCode = 1;
});
