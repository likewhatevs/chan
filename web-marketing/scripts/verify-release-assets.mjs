#!/usr/bin/env node

import { execFileSync } from "node:child_process";

const repo = "fiorix/chan";
const apiBase = `https://api.github.com/repos/${repo}`;
const githubToken = readGithubToken();

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const release = options.tag
    ? await fetchJson(`${apiBase}/releases/tags/${encodeURIComponent(options.tag)}`)
    : await fetchJson(`${apiBase}/releases/latest`);
  const tag = release.tag_name;
  const version = versionFromTag(tag);
  const assets = new Map((release.assets ?? []).map((asset) => [asset.name, asset]));

  const publicAssets = [
    `Chan_${version}.dmg`,
    `Chan_${version}_amd64.AppImage`,
    `Chan_${version}_amd64.deb`,
    "chan-x86_64-unknown-linux-gnu.tar.gz",
    "chan-aarch64-unknown-linux-gnu.tar.gz",
    "chan-aarch64-apple-darwin.tar.gz",
  ];
  const cliAssets = publicAssets.filter((name) => name.startsWith("chan-") && name.endsWith(".tar.gz"));
  const manualAsset = `chan-manual-${version}.tar.gz`;
  const requiredAssets = [...publicAssets];
  if (!options.allowMissingManual) {
    requiredAssets.push(manualAsset);
  }

  const errors = [];
  const warnings = [];

  for (const name of requiredAssets) {
    if (!assets.has(name)) {
      errors.push(`missing release asset: ${name}`);
    }
  }
  if (options.allowMissingManual && !assets.has(manualAsset)) {
    warnings.push(`missing manual bundle allowed for this run: ${manualAsset}`);
  }

  if (assets.has("VERSION")) {
    const body = (await fetchAssetText(assets.get("VERSION"))).trim();
    if (body !== version) {
      errors.push(`VERSION contains ${JSON.stringify(body)}, expected ${JSON.stringify(version)}`);
    }
  } else {
    warnings.push("VERSION asset absent; release metadata is authoritative");
  }

  if (assets.has("SHA256SUMS")) {
    const body = await fetchAssetText(assets.get("SHA256SUMS"));
    for (const name of cliAssets) {
      if (!checksumContains(body, name)) {
        errors.push(`SHA256SUMS is missing ${name}`);
      }
    }
  } else {
    warnings.push("SHA256SUMS asset absent; /dl metadata carries SHA256 values");
  }

  if (!options.skipAssetUrlHeads) {
    for (const name of publicAssets) {
      await verifyAssetUrl(name, assets.get(name), errors);
    }
    if (!options.allowMissingManual) {
      await verifyAssetUrl(manualAsset, assets.get(manualAsset), errors);
    }
  } else {
    warnings.push("asset URL HEAD checks skipped");
  }

  for (const warning of warnings) {
    console.warn(`warning: ${warning}`);
  }
  if (errors.length > 0) {
    for (const error of errors) {
      console.error(`error: ${error}`);
    }
    process.exitCode = 1;
    return;
  }

  const mode = options.tag ? tag : `${tag} via releases/latest`;
  console.log(`verified release assets for ${mode}`);
}

function parseArgs(args) {
  const options = { tag: null, allowMissingManual: false, skipAssetUrlHeads: false };
  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];
    if (arg === "--tag") {
      options.tag = args[i + 1] ?? "";
      i += 1;
    } else if (arg === "--allow-missing-manual") {
      options.allowMissingManual = true;
    } else if (arg === "--skip-asset-url-heads" || arg === "--skip-latest-download-heads") {
      options.skipAssetUrlHeads = true;
    } else if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    } else {
      throw new Error(`unknown argument: ${arg}`);
    }
  }
  if (options.tag === "") throw new Error("--tag requires a value");
  return options;
}

function printHelp() {
  console.log(`usage: node scripts/verify-release-assets.mjs [--tag vX.Y.Z] [--allow-missing-manual] [--skip-asset-url-heads]

Without --tag, verifies the GitHub latest release and each asset URL exposed by
the GitHub API. VERSION and SHA256SUMS are checked when present, but /dl
metadata is the release source of truth.
`);
}

function versionFromTag(tag) {
  if (!/^v\d+\.\d+\.\d+$/.test(tag ?? "")) {
    throw new Error(`release tag must use vX.Y.Z: ${tag}`);
  }
  return tag.slice(1);
}

function checksumContains(body, name) {
  return body.split(/\r?\n/).some((line) => {
    const match = line.trim().match(/^([a-fA-F0-9]{64})\s+\*?(.+)$/);
    if (!match) return false;
    const path = match[2].trim();
    return path === name || path.endsWith(`/${name}`);
  });
}

async function verifyAssetUrl(name, asset, errors) {
  if (!asset?.browser_download_url) return;
  const url = asset.browser_download_url;
  if (url.includes("/releases/latest/download/")) {
    errors.push(`asset URL uses latest-download route: ${url}`);
    return;
  }
  const response = await request(url, { method: "HEAD", redirect: "manual" });
  if (response.status < 200 || response.status >= 400) {
    errors.push(`asset URL returned HTTP ${response.status}: ${url}`);
  }
}

async function fetchJson(url) {
  const response = await request(url);
  if (!response.ok) {
    throw new Error(`${url} returned HTTP ${response.status}`);
  }
  return response.json();
}

async function fetchAssetText(asset) {
  const response = await request(asset.url, {
    headers: { Accept: "application/octet-stream" },
  });
  if (!response.ok) {
    throw new Error(`${asset.name} returned HTTP ${response.status}`);
  }
  return response.text();
}

async function request(url, init = {}) {
  const headers = {
    Accept: "application/vnd.github+json",
    "User-Agent": "chan-release-asset-verifier",
    ...init.headers,
  };
  if (githubToken) {
    headers.Authorization = `Bearer ${githubToken}`;
  }
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
  console.error(`release asset verification failed: ${err.message}`);
  process.exitCode = 1;
});
