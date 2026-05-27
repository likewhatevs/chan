#!/usr/bin/env node

import { execFile } from "node:child_process";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const scriptsRoot = path.dirname(scriptPath);
const siteRoot = path.resolve(scriptsRoot, "..");
const fixture = path.join(siteRoot, "fixtures", "release-assets", "v0.15.4.json");

async function main() {
  const out = await fs.mkdtemp(path.join(os.tmpdir(), "chan-release-metadata-"));
  try {
    await runNode(path.join(scriptsRoot, "generate-release-metadata.mjs"), [
      "--manifest",
      fixture,
      "--out",
      out,
    ]);

    const releases = await readJson(path.join(out, "releases.json"));
    const cliLatest = await readJson(path.join(out, "cli", "latest.json"));
    const cliVersion = await readJson(path.join(out, "cli", "v0.15.4.json"));
    const desktopLatest = await readJson(path.join(out, "desktop", "latest.json"));
    const desktopVersion = await readJson(path.join(out, "desktop", "v0.15.4.json"));

    assert(releases.latest === "0.15.4", "releases.json latest version");
    assert(releases.latest_tag === "v0.15.4", "releases.json latest tag");
    assert(releases.releases?.[0]?.downloads?.length === 6, "release download count");
    assert(
      new Set(releases.releases[0].downloads.map((download) => download.id)).has("cli-linux-x64"),
      "CLI download id present",
    );
    assert(cliLatest.version === "0.15.4", "CLI latest version");
    assert(JSON.stringify(cliLatest) === JSON.stringify(cliVersion), "CLI latest equals version file");
    assert(cliLatest.targets.length === 3, "CLI target count");
    assert(
      cliLatest.targets.every((target) => /^[a-f0-9]{64}$/.test(target.sha256)),
      "CLI sha256 values",
    );
    assert(desktopLatest.pub_date === "2026-05-27T00:00:00Z", "desktop pub_date");
    assert(
      JSON.stringify(desktopLatest) === JSON.stringify(desktopVersion),
      "desktop latest equals version file",
    );
    assert(
      desktopLatest.platforms["darwin-aarch64"]?.signature === "fixture-signature-darwin-aarch64",
      "desktop updater signature",
    );
  } finally {
    await fs.rm(out, { recursive: true, force: true });
  }
  console.log("smoked release metadata generation");
}

function runNode(file, args) {
  return new Promise((resolve, reject) => {
    execFile(process.execPath, [file, ...args], (err, stdout, stderr) => {
      if (err) {
        reject(new Error(`${path.basename(file)} failed: ${stderr || stdout || err.message}`));
      } else {
        resolve(stdout);
      }
    });
  });
}

async function readJson(file) {
  return JSON.parse(await fs.readFile(file, "utf8"));
}

function assert(condition, message) {
  if (!condition) throw new Error(`assertion failed: ${message}`);
}

main().catch((err) => {
  console.error(`release metadata smoke failed: ${err.message}`);
  process.exitCode = 1;
});
