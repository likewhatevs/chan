#!/usr/bin/env node

import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
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
    assert(releases.releases?.[0]?.downloads?.length === 22, "release download count");
    const downloadIds = new Set(releases.releases[0].downloads.map((download) => download.id));
    assert(downloadIds.has("cli-linux-x64"), "CLI tarball download id present");
    assert(downloadIds.has("cli-linux-deb-arm64"), "CLI deb download id present");
    assert(downloadIds.has("desktop-linux-rpm-amd64"), "desktop rpm download id present");
    assert(downloadIds.has("gateway-profile-deb-amd64"), "gateway download id present");
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
    // The fixture ships no Windows assets, so the optional Windows downloads
    // stay out of the manifest (the install-page buttons fall back).
    assert(!downloadIds.has("desktop-windows-nsis"), "windows installer absent without asset");
    assert(!downloadIds.has("cli-windows-x64"), "windows cli absent without asset");

    // With the Windows assets present, the optional downloads light up.
    const withWindows = JSON.parse(await fs.readFile(fixture, "utf8"));
    const windowsNames = ["Chan_0.15.4_x64-setup.exe", "chan-x86_64-pc-windows-msvc.zip"];
    for (const name of windowsNames) {
      withWindows.assets.push({
        name,
        url: `https://github.com/fiorix/chan/releases/download/v0.15.4/${name}`,
        sha256: createHash("sha256").update(name).digest("hex"),
      });
    }
    const winManifest = path.join(out, "with-windows.json");
    await fs.writeFile(winManifest, `${JSON.stringify(withWindows)}\n`);
    const winOut = path.join(out, "win");
    await runNode(path.join(scriptsRoot, "generate-release-metadata.mjs"), [
      "--manifest",
      winManifest,
      "--out",
      winOut,
    ]);
    const winDownloads = (await readJson(path.join(winOut, "releases.json"))).releases[0].downloads;
    assert(winDownloads.length === 24, "windows present adds two downloads");
    const winById = new Map(winDownloads.map((download) => [download.id, download]));
    assert(winById.has("desktop-windows-nsis"), "windows installer download present");
    assert(winById.has("cli-windows-x64"), "windows cli download present");
    assert(
      winById.get("desktop-windows-nsis").url.endsWith("/Chan_0.15.4_x64-setup.exe"),
      "windows installer url",
    );
    assert(
      /^[a-f0-9]{64}$/.test(winById.get("cli-windows-x64").sha256),
      "windows cli sha256",
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
