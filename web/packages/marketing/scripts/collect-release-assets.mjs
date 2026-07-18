#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import path from "node:path";

import { gatewayServices } from "./gateway-services.mjs";
import { gatewayPackageVersion, validateReleaseTag, versionFromTag } from "./release-version.mjs";

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
  if (options.latestCount > 0) {
    const releases = await loadReleaseHistory(options);
    if (!releases) return;
    const manifests = [];
    for (const release of releases) {
      manifests.push(await collectManifest(release, options));
    }
    await fs.mkdir(path.dirname(options.out), { recursive: true });
    await fs.writeFile(options.out, `${JSON.stringify(manifests, null, 2)}\n`);
    const tags = manifests.map((manifest) => manifest.tag).join(", ");
    console.log(
      `wrote release asset manifest for ${manifests.length} releases (${tags}) to ${options.out}`,
    );
    return;
  }

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
    latestCount: 0,
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
    } else if (arg === "--latest-count") {
      options.latestCount = parseLatestCount(args[i + 1] ?? "");
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
  if (options.tag) validateReleaseTag(options.tag, "--tag");
  return options;
}

function printHelp() {
  console.log(`usage: node scripts/collect-release-assets.mjs --out release-assets.json [--tag vX.Y.Z[-rcN]] [--latest-count N]

Collects uploaded GitHub Release assets into the manifest consumed by
generate-release-metadata.mjs. The script downloads asset bytes to compute
SHA256 values (unless the GitHub API already reports a "sha256:<hex>" digest
for the asset) and reads updater signature assets for signed platform entries.

With --latest-count N the output is an array of manifests, newest first: the
requested tag (or the latest GA release) plus its GA predecessors, so /dl
keeps the latest release and N-1 previous GA releases upgradeable by explicit
version. Prerelease and draft releases are never collected from the releases
list. With --release-json, the fixture may hold one release object or an
array standing in for the releases list.
`);
}

function parseLatestCount(value) {
  const count = Number(value);
  if (!Number.isInteger(count) || count < 1) {
    throw new Error("--latest-count must be a positive integer");
  }
  return count;
}

async function loadRelease(options) {
  if (options.releaseJson) {
    return JSON.parse(await fs.readFile(options.releaseJson, "utf8"));
  }

  const apiBase = `https://api.github.com/repos/${options.repo}`;
  const url = options.tag
    ? `${apiBase}/releases/tags/${encodeURIComponent(options.tag)}`
    : `${apiBase}/releases/latest`;
  return fetchRelease(url, options);
}

async function fetchRelease(url, options) {
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

// History mode (--latest-count N): select the newest N GA releases. The
// GitHub releases list is newest-first; an explicitly requested --tag (the
// release being published) is forced to the front and kept even when the
// list does not carry it yet. Prerelease and draft entries are filtered out
// of the list so rc builds never land in /dl. With --release-json the
// fixture may hold either one release object (collected as-is, like an
// explicit --tag) or an array standing in for the releases list.
async function loadReleaseHistory(options) {
  let requested = null;
  let listed = [];
  if (options.releaseJson) {
    const raw = JSON.parse(await fs.readFile(options.releaseJson, "utf8"));
    if (Array.isArray(raw)) {
      listed = raw;
    } else {
      requested = raw;
    }
  } else {
    const apiBase = `https://api.github.com/repos/${options.repo}`;
    if (options.tag) {
      requested = await fetchRelease(
        `${apiBase}/releases/tags/${encodeURIComponent(options.tag)}`,
        options,
      );
      if (!requested) return null;
    }
    const listUrl = `${apiBase}/releases?per_page=100`;
    const response = await request(listUrl);
    if (!response.ok) {
      throw new Error(`${listUrl} returned HTTP ${response.status}`);
    }
    listed = await response.json();
  }

  const releases = [];
  const seen = new Set();
  const push = (release) => {
    const tag = requireString(release.tag_name, "release.tag_name");
    if (seen.has(tag)) return;
    seen.add(tag);
    releases.push(release);
  };
  if (requested) push(requested);
  for (const release of listed) {
    if (!isGaRelease(release)) continue;
    push(release);
  }
  if (options.tag && !requested) {
    const index = releases.findIndex((release) => release.tag_name === options.tag);
    if (index === -1) {
      throw new Error(`requested tag ${options.tag} not found in the release history`);
    }
    releases.unshift(...releases.splice(index, 1));
  }

  const selected = releases.slice(0, options.latestCount);
  if (selected.length === 0) {
    if (options.allowMissingRelease) {
      console.warn("warning: no GA GitHub Release found; skipping /dl metadata");
      return null;
    }
    throw new Error("no GA releases found to collect");
  }
  return selected;
}

function isGaRelease(release) {
  if (release?.draft || release?.prerelease) return false;
  const tag = typeof release?.tag_name === "string" ? release.tag_name : "";
  return /^v\d+\.\d+\.\d+$/.test(tag);
}

async function collectManifest(release, options) {
  const tag = requireString(release.tag_name, "release.tag_name");
  const version = versionFromTag(tag);
  const gatewayVersion = gatewayPackageVersion(version);
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
  for (const name of [...cliAssets(), ...desktopAssets(version), ...gatewayAssets(gatewayVersion)]) {
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
  const url = requireString(asset.browser_download_url, `${name} browser_download_url`);
  const digest = digestSha256(asset);
  if (digest) return { name, url, sha256: digest };
  const bytes = await readAssetBytes(asset, options);
  return {
    name,
    url,
    sha256: createHash("sha256").update(bytes).digest("hex"),
  };
}

// The GitHub API reports asset digests as "sha256:<hex>"; trust those instead
// of re-downloading bytes just to hash them. Local fixtures and old API
// responses without a digest still take the download-and-hash path.
function digestSha256(asset) {
  const digest = typeof asset.digest === "string" ? asset.digest.trim() : "";
  const match = digest.match(/^sha256:([0-9a-fA-F]{64})$/);
  return match ? match[1].toLowerCase() : "";
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
