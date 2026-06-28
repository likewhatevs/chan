#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";

import { gatewayServices } from "./gateway-services.mjs";

const defaultRepo = "fiorix/chan";

// Every public download the install page links to is collected here so it
// gets a SHA256 + browser URL in the manifest. Must stay in sync with the
// download lists in generate-release-metadata.mjs. The standalone tarballs are
// the musl/darwin self-upgrade targets; the .deb/.rpm are the gnu distro
// packages release.yml renames to a version-less chan-<arch>.<ext>.
function cliAssets() {
  return [
    "chan-x86_64-unknown-linux-musl.tar.gz",
    "chan-aarch64-unknown-linux-musl.tar.gz",
    "chan-aarch64-apple-darwin.tar.gz",
    "chan-amd64.deb",
    "chan-arm64.deb",
    "chan-amd64.rpm",
    "chan-arm64.rpm",
  ];
}

function desktopAssets(version) {
  return [
    `Chan_${version}.dmg`,
    `Chan_${version}_amd64.AppImage`,
    `Chan_${version}_aarch64.AppImage`,
    `Chan_${version}_amd64.deb`,
    `Chan_${version}_arm64.deb`,
    `Chan-${version}-1.x86_64.rpm`,
    `Chan-${version}-1.aarch64.rpm`,
  ];
}

// gatewayServices is single-sourced from the Makefile's GATEWAY_RELEASE_CRATES
// (see ./gateway-services.mjs), the same source release.yml builds from.

function gatewayAssets(version) {
  const assets = [];
  for (const service of gatewayServices) {
    for (const arch of ["amd64", "arm64"]) {
      assets.push(`chan-gateway-${service}_${version}-1_${arch}.deb`);
    }
  }
  return assets;
}

// Optional assets are collected only when the release actually shipped them, so
// a release without them does not fail metadata generation. Windows is the
// first: the desktop NSIS installer and the standalone Windows CLI zip are not
// published yet, so they light up on the install page the moment release.yml
// starts uploading them.
function optionalAssets(version) {
  return [
    `Chan_${version}_x64-setup.exe`,
    "chan-x86_64-pc-windows-msvc.zip",
  ];
}

function updaterAssets(version) {
  return [
    {
      name: `Chan_${version}_aarch64.app.tar.gz`,
      platform: "darwin-aarch64",
    },
  ];
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const release = await loadRelease(options);
  if (!release) return;

  const manifest = await collectManifest(release, options);
  await fs.mkdir(path.dirname(options.out), { recursive: true });
  await fs.writeFile(options.out, `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`wrote release asset manifest for ${manifest.tag} to ${options.out}`);
}

function parseArgs(args) {
  const options = {
    allowMissingRelease: false,
    assetDir: "",
    out: "",
    releaseJson: "",
    repo: defaultRepo,
    tag: "",
  };
  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];
    if (arg === "--allow-missing-release") {
      options.allowMissingRelease = true;
    } else if (arg === "--asset-dir") {
      options.assetDir = args[i + 1] ?? "";
      i += 1;
    } else if (arg === "--out") {
      options.out = args[i + 1] ?? "";
      i += 1;
    } else if (arg === "--release-json") {
      options.releaseJson = args[i + 1] ?? "";
      i += 1;
    } else if (arg === "--repo") {
      options.repo = args[i + 1] ?? "";
      i += 1;
    } else if (arg === "--tag") {
      options.tag = args[i + 1] ?? "";
      i += 1;
    } else if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  if (!options.out) throw new Error("--out is required");
  if (!options.repo) throw new Error("--repo requires a value");
  if (options.assetDir && !options.releaseJson) {
    throw new Error("--asset-dir is only valid with --release-json");
  }
  if (options.releaseJson && !options.assetDir) {
    throw new Error("--release-json requires --asset-dir");
  }
  if (options.tag && !/^v\d+\.\d+\.\d+$/.test(options.tag)) {
    throw new Error("--tag must use vX.Y.Z");
  }
  return options;
}

function printHelp() {
  console.log(`usage: node scripts/collect-release-assets.mjs --out release-assets.json [--tag vX.Y.Z]

Collects uploaded GitHub Release assets into the manifest consumed by
generate-release-metadata.mjs. The script downloads asset bytes to compute
SHA256 values and reads updater signature assets for signed platform entries.
`);
}

async function loadRelease(options) {
  if (options.releaseJson) {
    return JSON.parse(await fs.readFile(options.releaseJson, "utf8"));
  }

  const apiBase = `https://api.github.com/repos/${options.repo}`;
  const url = options.tag
    ? `${apiBase}/releases/tags/${encodeURIComponent(options.tag)}`
    : `${apiBase}/releases/latest`;
  const response = await request(url);
  if (response.status === 404 && options.allowMissingRelease) {
    console.warn("warning: no GitHub Release found; skipping /dl metadata");
    return null;
  }
  if (!response.ok) {
    throw new Error(`${url} returned HTTP ${response.status}`);
  }
  return response.json();
}

async function collectManifest(release, options) {
  const tag = requireString(release.tag_name, "release.tag_name");
  if (!/^v\d+\.\d+\.\d+$/.test(tag)) {
    throw new Error(`release tag must use vX.Y.Z: ${tag}`);
  }
  const version = tag.slice(1);
  const publishedAt = requireString(
    release.published_at ?? release.created_at,
    "release.published_at",
  );
  const releaseAssets = new Map();
  for (const asset of release.assets ?? []) {
    const name = requireString(asset.name, "asset.name");
    if (releaseAssets.has(name)) throw new Error(`duplicate release asset ${name}`);
    releaseAssets.set(name, asset);
  }

  const assets = [];
  for (const name of [...cliAssets(), ...desktopAssets(version), ...gatewayAssets(version)]) {
    assets.push(await collectAsset(name, releaseAssets, options));
  }
  for (const name of optionalAssets(version)) {
    if (!releaseAssets.has(name)) continue;
    assets.push(await collectAsset(name, releaseAssets, options));
  }
  for (const updater of updaterAssets(version)) {
    const payload = await collectAsset(updater.name, releaseAssets, options);
    const signature = await collectSignature(`${updater.name}.sig`, releaseAssets, options);
    assets.push({
      ...payload,
      signature,
      updater_platform: updater.platform,
    });
  }

  return {
    version,
    tag,
    published_at: publishedAt,
    notes: String(release.body ?? ""),
    assets,
  };
}

async function collectAsset(name, releaseAssets, options) {
  const asset = releaseAssets.get(name);
  if (!asset) throw new Error(`missing release asset: ${name}`);
  const bytes = await readAssetBytes(asset, options);
  return {
    name,
    url: requireString(asset.browser_download_url, `${name} browser_download_url`),
    sha256: createHash("sha256").update(bytes).digest("hex"),
  };
}

async function collectSignature(name, releaseAssets, options) {
  const asset = releaseAssets.get(name);
  if (!asset) throw new Error(`missing release asset: ${name}`);
  const bytes = await readAssetBytes(asset, options);
  const signature = bytes.toString("utf8").trim();
  if (!signature) throw new Error(`empty updater signature asset: ${name}`);
  return signature;
}

async function readAssetBytes(asset, options) {
  if (options.assetDir) {
    return fs.readFile(path.join(options.assetDir, asset.name));
  }
  const response = await request(requireString(asset.url, `${asset.name} url`), {
    headers: { Accept: "application/octet-stream" },
  });
  if (!response.ok) {
    throw new Error(`${asset.name} returned HTTP ${response.status}`);
  }
  return Buffer.from(await response.arrayBuffer());
}

function requireString(value, label) {
  if (typeof value !== "string" || value.trim() === "") {
    throw new Error(`${label} is required`);
  }
  return value.trim();
}

async function request(url, init = {}) {
  const headers = {
    Accept: "application/vnd.github+json",
    "User-Agent": "chan-release-asset-collector",
    ...init.headers,
  };
  const token = readGithubToken();
  if (token) headers.Authorization = `Bearer ${token}`;
  return fetch(url, { ...init, headers });
}

function readGithubToken() {
  if (process.env.GH_TOKEN) return process.env.GH_TOKEN;
  if (process.env.GITHUB_TOKEN) return process.env.GITHUB_TOKEN;
  try {
    return execFileSync("gh", ["auth", "token"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return "";
  }
}

main().catch((err) => {
  console.error(`release asset collection failed: ${err.message}`);
  process.exitCode = 1;
});
