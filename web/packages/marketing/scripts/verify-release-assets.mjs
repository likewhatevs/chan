#!/usr/bin/env node

import { execFileSync } from "node:child_process";

import { gatewayServices } from "./gateway-services.mjs";
import { gatewayPackageVersion, versionFromTag } from "./release-version.mjs";

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
  const gatewayVersion = gatewayPackageVersion(version);
  const assets = new Map((release.assets ?? []).map((asset) => [asset.name, asset]));

  const publicAssets = [
    // chan CLI
    "chan-x86_64-unknown-linux-musl.tar.gz",
    "chan-aarch64-unknown-linux-musl.tar.gz",
    "chan-aarch64-apple-darwin.tar.gz",
    "chan-amd64.deb",
    "chan-arm64.deb",
    "chan-amd64.rpm",
    "chan-arm64.rpm",
    // chan-desktop
    `Chan_${version}.dmg`,
    `Chan_${version}_amd64.AppImage`,
    `Chan_${version}_aarch64.AppImage`,
    `Chan_${version}_amd64.deb`,
    `Chan_${version}_arm64.deb`,
    `Chan-${version}-1.x86_64.rpm`,
    `Chan-${version}-1.aarch64.rpm`,
    // chan-gateway: one .deb per service per arch, single-sourced from the
    // Makefile's GATEWAY_RELEASE_CRATES (see ./gateway-services.mjs).
    ...gatewayServices.flatMap((service) =>
      ["amd64", "arm64"].map(
        (arch) => `chan-gateway-${service}_${gatewayVersion}-1_${arch}.deb`,
      ),
    ),
  ];
  const updaterAssets = [
    `Chan_${version}_aarch64.app.tar.gz`,
    `Chan_${version}_aarch64.app.tar.gz.sig`,
  ];
  const cliAssets = publicAssets.filter((name) => name.startsWith("chan-") && name.endsWith(".tar.gz"));
  const requiredAssets = [...publicAssets, ...updaterAssets];

  const errors = [];
  const warnings = [];

  for (const name of requiredAssets) {
    if (!assets.has(name)) {
      errors.push(`missing release asset: ${name}`);
    }
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

  if (assets.has(updaterAssets[1])) {
    const signature = (await fetchAssetText(assets.get(updaterAssets[1]))).trim();
    if (!signature) {
      errors.push(`${updaterAssets[1]} is empty`);
    }
  }

  if (!options.skipAssetUrlHeads) {
    for (const name of [...publicAssets, ...updaterAssets]) {
      await verifyAssetUrl(name, assets.get(name), errors);
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
  const options = { tag: null, skipAssetUrlHeads: false };
  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];
    if (arg === "--tag") {
      options.tag = args[i + 1] ?? "";
      i += 1;
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
  console.log(`usage: node scripts/verify-release-assets.mjs [--tag vX.Y.Z[-rcN]] [--skip-asset-url-heads]

Without --tag, verifies the GitHub latest release and each asset URL exposed by
the GitHub API. Desktop updater payloads must include detached signature
assets. VERSION and SHA256SUMS are checked when present, but /dl metadata is
the release source of truth.
`);
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
