#!/usr/bin/env node

import { createServer } from "node:http";
import { promises as fs } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const siteRoot = path.resolve(path.dirname(scriptPath), "..");
const distRoot = path.join(siteRoot, "dist");

const checks = [
  { path: "/", status: 200, includes: 'src="/assets/home-hero.png"' },
  { path: "/", status: 200, includes: 'href="/install/">Install' },
  { path: "/install/", status: 200, includes: "Install Chan" },
  { path: "/install/", status: 200, includes: 'data-release-download="cli-linux-x64"' },
  { path: "/install/", status: 200, includes: 'data-release-download="desktop-linux-rpm-amd64"' },
  { path: "/install/", status: 200, includes: 'data-release-download="gateway-profile-deb-amd64"' },
  {
    path: "/install.sh",
    status: 200,
    includes: 'DEFAULT_METADATA_BASE="https://chan.app/dl/cli"',
  },
  { path: "/install.ps1", status: 404 },
];

async function main() {
  await assertDistReady();
  const server = createServer(handleRequest);
  await listen(server);
  const { port } = server.address();
  try {
    for (const check of checks) {
      await runCheck(port, check);
    }
  } finally {
    await close(server);
  }
  console.log(`smoked marketing dist routes on 127.0.0.1:${port}`);
}

async function assertDistReady() {
  const stat = await fs.stat(distRoot).catch(() => null);
  if (!stat?.isDirectory()) {
    throw new Error("marketing dist is missing; run npm run build first");
  }
}

async function handleRequest(req, res) {
  try {
    const file = resolveRequestPath(req.url ?? "/");
    if (!file) {
      res.writeHead(404).end("not found");
      return;
    }
    const body = await fs.readFile(file);
    res.writeHead(200, { "content-type": contentType(file) });
    res.end(body);
  } catch (err) {
    if (err?.code === "ENOENT" || err?.code === "EISDIR") {
      res.writeHead(404).end("not found");
    } else {
      res.writeHead(500).end("error");
    }
  }
}

function resolveRequestPath(rawUrl) {
  const url = new URL(rawUrl, "http://127.0.0.1");
  const decoded = decodeURIComponent(url.pathname);
  const relative = decoded.endsWith("/")
    ? path.posix.join(decoded.slice(1), "index.html")
    : decoded.slice(1);
  const candidate = path.resolve(distRoot, relative);
  if (candidate !== distRoot && !candidate.startsWith(`${distRoot}${path.sep}`)) {
    return null;
  }
  return candidate;
}

function contentType(file) {
  if (file.endsWith(".html")) return "text/html; charset=utf-8";
  if (file.endsWith(".css")) return "text/css; charset=utf-8";
  if (file.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (file.endsWith(".sh")) return "text/x-shellscript; charset=utf-8";
  if (file.endsWith(".png")) return "image/png";
  if (file.endsWith(".ico")) return "image/x-icon";
  return "application/octet-stream";
}

function listen(server) {
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      server.off("error", reject);
      resolve();
    });
  });
}

function close(server) {
  return new Promise((resolve, reject) => {
    server.close((err) => (err ? reject(err) : resolve()));
  });
}

async function runCheck(port, check) {
  const url = `http://127.0.0.1:${port}${check.path}`;
  const response = await fetch(url);
  if (response.status !== check.status) {
    throw new Error(`${check.path} returned HTTP ${response.status}, expected ${check.status}`);
  }
  if (check.includes) {
    const body = await response.text();
    if (!body.includes(check.includes)) {
      throw new Error(`${check.path} did not contain ${JSON.stringify(check.includes)}`);
    }
  }
}

main().catch((err) => {
  console.error(`dist smoke failed: ${err.message}`);
  process.exitCode = 1;
});
