#!/usr/bin/env node

import { execFileSync } from "node:child_process";

const repo = "fiorix/chan";
const githubRepoUrl = `https://github.com/${repo}`;
const apiBase = `https://api.github.com/repos/${repo}`;
const latestDownloadBase = `${githubRepoUrl}/releases/latest/download`;
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
  const requiredAssets = [...publicAssets, "VERSION", "SHA256SUMS"];
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
  }

  if (assets.has("SHA256SUMS")) {
    const body = await fetchAssetText(assets.get("SHA256SUMS"));
    for (const name of cliAssets) {
      if (!checksumContains(body, name)) {
        errors.push(`SHA256SUMS is missing ${name}`);
      }
    }
  }

  if (!options.tag && !options.skipLatestDownloadHeads) {
    for (const name of publicAssets) {
      await verifyLatestDownload(name, errors);
    }
    if (!options.allowMissingManual) {
      await verifyLatestDownload(manualAsset, errors);
    }
  } else if (!options.tag && options.skipLatestDownloadHeads) {
    warnings.push("latest-download HEAD checks skipped");
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
  const options = { tag: null, allowMissingManual: false, skipLatestDownloadHeads: false };
  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];
    if (arg === "--tag") {
      options.tag = args[i + 1] ?? "";
      i += 1;
    } else if (arg === "--allow-missing-manual") {
      options.allowMissingManual = true;
    } else if (arg === "--skip-latest-download-heads") {
      options.skipLatestDownloadHeads = true;
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
  console.log(`usage: node scripts/verify-release-assets.mjs [--tag chan-vX.Y.Z] [--allow-missing-manual] [--skip-latest-download-heads]

Without --tag, verifies the GitHub latest release and its latest-download URLs.
`);
}

function versionFromTag(tag) {
  if (!tag?.startsWith("chan-v")) {
    throw new Error(`release tag must use chan-v<version>: ${tag}`);
  }
  return tag.slice("chan-v".length);
}

function checksumContains(body, name) {
  return body.split(/\r?\n/).some((line) => {
    const match = line.trim().match(/^([a-fA-F0-9]{64})\s+\*?(.+)$/);
    if (!match) return false;
    const path = match[2].trim();
    return path === name || path.endsWith(`/${name}`);
  });
}

async function verifyLatestDownload(name, errors) {
  const url = `${latestDownloadBase}/${name}`;
  const response = await request(url, { method: "HEAD", redirect: "manual" });
  if (response.status < 200 || response.status >= 400) {
    errors.push(`latest-download URL returned HTTP ${response.status}: ${url}`);
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
