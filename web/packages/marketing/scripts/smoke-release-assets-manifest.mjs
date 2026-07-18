#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
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

  // Digest fast-path: an asset carrying a GitHub API "sha256:<hex>" digest
  // uses it verbatim; the on-disk bytes (which hash differently) are ignored.
  const digestNames = new Set(["chan-amd64.deb", `Chan_${version}.dmg`]);
  const dig = runCollect("digest", version, [], digestNames);
  for (const name of digestNames) {
    const asset = dig.assets.find((entry) => entry.name === name);
    assert(asset, `missing digest asset ${name}`);
    assertEqual(asset.sha256, digestFor(tag, name), `digest fast-path sha256 for ${name}`);
  }
  assertEqual(
    dig.assets.find((entry) => entry.name === firstCliAsset).sha256,
    createHash("sha256").update(`asset bytes for ${firstCliAsset}\n`).digest("hex"),
    "fallback sha256 without digest",
  );

  // History mode (--latest-count N): GA releases are kept newest-first,
  // rc tags and prerelease/draft entries are filtered out, and the output is
  // a manifest array that generate-release-metadata.mjs consumes as-is.
  const history = [
    { releaseVersion: "0.15.9" },
    { releaseVersion: "0.16.0-rc1" },
    { releaseVersion: "0.15.8" },
    { releaseVersion: "0.15.7", prerelease: true },
    { releaseVersion: "0.15.6" },
    { releaseVersion: "0.15.5", draft: true },
    { releaseVersion: "0.15.4" },
  ];
  const historyRun = runCollectHistory("history", history);
  assertEqual(
    JSON.stringify(historyRun.manifests.map((manifest) => manifest.tag)),
    JSON.stringify(["v0.15.9", "v0.15.8", "v0.15.6", "v0.15.4"]),
    "history keeps GA releases newest-first",
  );
  const historyUpdater = historyRun.manifests[0].assets.find(
    (asset) => asset.updater_platform === "darwin-aarch64",
  );
  assertEqual(historyUpdater.signature, "fixture-updater-signature", "history updater signature");
  assertEqual(
    historyRun.manifests[0].assets.find((asset) => asset.name === firstCliAsset).sha256,
    digestFor("v0.15.9", firstCliAsset),
    "history asset uses digest",
  );

  // An explicit --tag is forced to the front of the history window.
  const forced = runCollectHistory("forced", history, { latestCount: 2, tag: "v0.15.4" });
  assertEqual(
    JSON.stringify(forced.manifests.map((manifest) => manifest.tag)),
    JSON.stringify(["v0.15.4", "v0.15.9"]),
    "requested tag forced to the front",
  );

  // Integration pin: the collector's history array feeds the generator
  // unchanged, producing the retained /dl tree.
  const dlOut = path.join(root, "history", "dl");
  execFileSync("node", [
    "scripts/generate-release-metadata.mjs",
    "--manifest",
    historyRun.manifestPath,
    "--out",
    dlOut,
  ], { cwd: siteRoot });
  const dlReleases = JSON.parse(readFileSync(path.join(dlOut, "releases.json"), "utf8"));
  assertEqual(dlReleases.releases.length, 4, "releases.json carries all retained releases");
  assertEqual(dlReleases.latest, "0.15.9", "releases.json latest");
  assertEqual(dlReleases.latest_tag, "v0.15.9", "releases.json latest_tag");
  for (const retained of ["v0.15.9", "v0.15.8", "v0.15.6", "v0.15.4"]) {
    assert(existsSync(path.join(dlOut, "cli", `${retained}.json`)), `cli/${retained}.json exists`);
    assert(
      existsSync(path.join(dlOut, "desktop", `${retained}.json`)),
      `desktop/${retained}.json exists`,
    );
  }
  assert(!existsSync(path.join(dlOut, "cli", "v0.16.0-rc1.json")), "rc tag filtered out of /dl");
  assert(!existsSync(path.join(dlOut, "cli", "v0.15.7.json")), "prerelease filtered out of /dl");
  assert(!existsSync(path.join(dlOut, "cli", "v0.15.5.json")), "draft filtered out of /dl");
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

function runCollect(label, releaseVersion, extraNames, digestNames = new Set()) {
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
    const asset = {
      name,
      url: `https://api.github.com/repos/fiorix/chan/releases/assets/${encodeURIComponent(name)}`,
      browser_download_url: `https://github.com/fiorix/chan/releases/download/${releaseTag}/${encodeURIComponent(name)}`,
    };
    if (digestNames.has(name)) asset.digest = `sha256:${digestFor(releaseTag, name)}`;
    release.assets.push(asset);
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

// Fixture-history mode: the --release-json file holds an array of release
// objects standing in for the GitHub releases list. Every non-sig asset
// carries a digest so no asset bytes are needed; only the updater signatures
// are read from disk.
function runCollectHistory(label, entries, { latestCount = 5, tag = "" } = {}) {
  const runRoot = path.join(root, label);
  const assetDir = path.join(runRoot, "assets");
  mkdirSync(assetDir, { recursive: true });
  const releases = entries.map(({ releaseVersion, prerelease = false, draft = false }) => {
    const releaseTag = `v${releaseVersion}`;
    const release = {
      tag_name: releaseTag,
      published_at: "2026-05-27T00:00:00Z",
      body: "Fixture release",
      prerelease,
      draft,
      assets: [],
    };
    for (const name of namesFor(releaseVersion)) {
      const asset = {
        name,
        url: `https://api.github.com/repos/fiorix/chan/releases/assets/${encodeURIComponent(name)}`,
        browser_download_url: `https://github.com/fiorix/chan/releases/download/${releaseTag}/${encodeURIComponent(name)}`,
      };
      if (name.endsWith(".sig")) {
        writeFileSync(path.join(assetDir, name), "fixture-updater-signature\n");
      } else {
        asset.digest = `sha256:${digestFor(releaseTag, name)}`;
      }
      release.assets.push(asset);
    }
    return release;
  });

  const releaseJson = path.join(runRoot, "releases.json");
  const manifestPath = path.join(runRoot, "manifest.json");
  writeFileSync(releaseJson, `${JSON.stringify(releases, null, 2)}\n`);
  const args = [
    "scripts/collect-release-assets.mjs",
    "--release-json",
    releaseJson,
    "--asset-dir",
    assetDir,
    "--latest-count",
    String(latestCount),
    "--out",
    manifestPath,
  ];
  if (tag) args.push("--tag", tag);
  execFileSync("node", args, { cwd: siteRoot });

  return { manifestPath, manifests: JSON.parse(readFileSync(manifestPath, "utf8")) };
}

function digestFor(releaseTag, name) {
  return createHash("sha256").update(`digest for ${releaseTag}/${name}`).digest("hex");
}

function assert(value, message) {
  if (!value) throw new Error(message);
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}
