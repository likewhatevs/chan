#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { gatewayServices } from "./gateway-services.mjs";
import { gatewayPackageVersion } from "./release-version.mjs";

const siteRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const version = "0.15.4";
const tag = `v${version}`;
const firstCliAsset = "chan-x86_64-unknown-linux-musl.tar.gz";

const root = mkdtempSync(path.join(tmpdir(), "chan-release-assets-"));
try {
  // No Windows assets: the optional Windows entries are skipped, not an error.
  const base = runCollect("base", version, []);
  assertEqual(base.version, version, "version");
  assertEqual(base.tag, tag, "tag");
  assertEqual(base.assets.length, 23, "asset count excludes detached sig");
  assert(
    !base.assets.some((asset) => asset.name.endsWith("-setup.exe")),
    "windows installer absent when not in the release",
  );
  assert(
    !base.assets.some((asset) => asset.name.endsWith("windows-msvc.zip")),
    "windows cli absent when not in the release",
  );

  const cli = base.assets.find((asset) => asset.name === firstCliAsset);
  assert(cli, "missing CLI asset");
  assertEqual(
    cli.sha256,
    createHash("sha256").update(`asset bytes for ${firstCliAsset}\n`).digest("hex"),
    "CLI sha256",
  );

  const updater = base.assets.find((asset) => asset.updater_platform === "darwin-aarch64");
  assert(updater, "missing updater asset");
  assertEqual(updater.signature, "fixture-updater-signature", "updater signature");

  // Windows assets present: the optional entries are collected.
  const windowsNames = optionalNames(version);
  const win = runCollect("windows", version, windowsNames);
  assertEqual(win.assets.length, 25, "windows assets collected when present");
  assert(
    win.assets.some((asset) => asset.name === windowsNames[0]),
    "windows installer collected",
  );
  assert(
    win.assets.some((asset) => asset.name === windowsNames[1]),
    "windows cli collected",
  );

  // Prerelease assets keep the Cargo version in desktop names but gateway
  // debs use cargo-deb's package-version spelling.
  const prereleaseVersion = "0.56.0-rc1";
  const prerelease = runCollect("prerelease", prereleaseVersion, optionalNames(prereleaseVersion));
  assertEqual(prerelease.version, prereleaseVersion, "prerelease version");
  assertEqual(prerelease.tag, `v${prereleaseVersion}`, "prerelease tag");
  assert(
    prerelease.assets.some((asset) => asset.name === "Chan-0.56.0-rc1-1.x86_64.rpm"),
    "prerelease desktop rpm collected",
  );
  assert(
    prerelease.assets.some((asset) => asset.name === "chan-gateway-admin_0.56.0.rc1-1_amd64.deb"),
    "prerelease gateway deb collected",
  );
  console.log("smoked release asset manifest collection");
} finally {
  rmSync(root, { force: true, recursive: true });
}

function namesFor(releaseVersion) {
  const gatewayVersion = gatewayPackageVersion(releaseVersion);
  return [
    // chan CLI
    firstCliAsset,
    "chan-aarch64-unknown-linux-musl.tar.gz",
    "chan-aarch64-apple-darwin.tar.gz",
    "chan-amd64.deb",
    "chan-arm64.deb",
    "chan-amd64.rpm",
    "chan-arm64.rpm",
    // chan-desktop
    `Chan_${releaseVersion}.dmg`,
    `Chan_${releaseVersion}_amd64.AppImage`,
    `Chan_${releaseVersion}_aarch64.AppImage`,
    `Chan_${releaseVersion}_amd64.deb`,
    `Chan_${releaseVersion}_arm64.deb`,
    `Chan-${releaseVersion}-1.x86_64.rpm`,
    `Chan-${releaseVersion}-1.aarch64.rpm`,
    // chan-gateway
    ...gatewayServices.flatMap((service) =>
      ["amd64", "arm64"].map(
        (arch) => `chan-gateway-${service}_${gatewayVersion}-1_${arch}.deb`,
      ),
    ),
    // signed desktop updater payload + detached signature
    `Chan_${releaseVersion}_aarch64.app.tar.gz`,
    `Chan_${releaseVersion}_aarch64.app.tar.gz.sig`,
  ];
}

function optionalNames(releaseVersion) {
  return [
    `Chan_${releaseVersion}_x64-setup.exe`,
    "chan-x86_64-pc-windows-msvc.zip",
  ];
}

function runCollect(label, releaseVersion, extraNames) {
  const releaseTag = `v${releaseVersion}`;
  const runRoot = path.join(root, label);
  const assetDir = path.join(runRoot, "assets");
  mkdirSync(assetDir, { recursive: true });
  const release = {
    tag_name: releaseTag,
    published_at: "2026-05-27T00:00:00Z",
    body: "Fixture release",
    assets: [],
  };
  for (const name of [...namesFor(releaseVersion), ...extraNames]) {
    const body = name.endsWith(".sig")
      ? "fixture-updater-signature\n"
      : `asset bytes for ${name}\n`;
    writeFileSync(path.join(assetDir, name), body);
    release.assets.push({
      name,
      url: `https://api.github.com/repos/fiorix/chan/releases/assets/${encodeURIComponent(name)}`,
      browser_download_url: `https://github.com/fiorix/chan/releases/download/${releaseTag}/${encodeURIComponent(name)}`,
    });
  }

  const releaseJson = path.join(runRoot, "release.json");
  const out = path.join(runRoot, "manifest.json");
  writeFileSync(releaseJson, `${JSON.stringify(release, null, 2)}\n`);
  execFileSync("node", [
    "scripts/collect-release-assets.mjs",
    "--release-json",
    releaseJson,
    "--asset-dir",
    assetDir,
    "--out",
    out,
  ], { cwd: siteRoot });

  return JSON.parse(readFileSync(out, "utf8"));
}

function assert(value, message) {
  if (!value) throw new Error(message);
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}
