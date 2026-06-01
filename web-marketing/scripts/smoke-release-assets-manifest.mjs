#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const version = "0.15.4";
const tag = `v${version}`;
const names = [
  "chan-x86_64-unknown-linux-musl.tar.gz",
  "chan-aarch64-unknown-linux-musl.tar.gz",
  "chan-aarch64-apple-darwin.tar.gz",
  `Chan_${version}.dmg`,
  `Chan_${version}_amd64.AppImage`,
  `Chan_${version}_amd64.deb`,
  `Chan_${version}_aarch64.app.tar.gz`,
  `Chan_${version}_aarch64.app.tar.gz.sig`,
];

const root = mkdtempSync(path.join(tmpdir(), "chan-release-assets-"));
try {
  const assetDir = path.join(root, "assets");
  mkdirSync(assetDir, { recursive: true });
  const release = {
    tag_name: tag,
    published_at: "2026-05-27T00:00:00Z",
    body: "Fixture release",
    assets: [],
  };
  for (const name of names) {
    const body = name.endsWith(".sig")
      ? "fixture-updater-signature\n"
      : `asset bytes for ${name}\n`;
    writeFileSync(path.join(assetDir, name), body);
    release.assets.push({
      name,
      url: `https://api.github.com/repos/fiorix/chan/releases/assets/${encodeURIComponent(name)}`,
      browser_download_url: `https://github.com/fiorix/chan/releases/download/${tag}/${encodeURIComponent(name)}`,
    });
  }

  const releaseJson = path.join(root, "release.json");
  const out = path.join(root, "manifest.json");
  writeFileSync(releaseJson, `${JSON.stringify(release, null, 2)}\n`);
  execFileSync("node", [
    "scripts/collect-release-assets.mjs",
    "--release-json",
    releaseJson,
    "--asset-dir",
    assetDir,
    "--out",
    out,
  ], { cwd: path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..") });

  const manifest = JSON.parse(readFileSync(out, "utf8"));
  assertEqual(manifest.version, version, "version");
  assertEqual(manifest.tag, tag, "tag");
  assertEqual(manifest.assets.length, 7, "asset count excludes detached sig");

  const cli = manifest.assets.find((asset) => asset.name === names[0]);
  assert(cli, "missing CLI asset");
  assertEqual(
    cli.sha256,
    createHash("sha256").update(`asset bytes for ${names[0]}\n`).digest("hex"),
    "CLI sha256",
  );

  const updater = manifest.assets.find((asset) => asset.updater_platform === "darwin-aarch64");
  assert(updater, "missing updater asset");
  assertEqual(updater.signature, "fixture-updater-signature", "updater signature");
  console.log("smoked release asset manifest collection");
} finally {
  rmSync(root, { force: true, recursive: true });
}

function assert(value, message) {
  if (!value) throw new Error(message);
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}
