#!/usr/bin/env node

import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { promises as fs } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const siteRoot = path.resolve(path.dirname(scriptPath), "..");
const installer = path.join(siteRoot, "src", "install.sh");

async function main() {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), "chan-install-smoke-"));
  try {
    const fakeBin = path.join(root, "bin");
    const assets = path.join(root, "assets");
    const prefixes = path.join(root, "prefixes");
    await fs.mkdir(fakeBin, { recursive: true });
    await fs.mkdir(assets, { recursive: true });
    await fs.mkdir(prefixes, { recursive: true });
    await writeFakeUname(fakeBin);

    const linuxTar = await makeTarball(assets, "x86_64-unknown-linux-musl", "linux");
    const macTar = await makeTarball(assets, "aarch64-apple-darwin", "mac");
    const metadataPath = path.join(root, "latest.json");
    await fs.writeFile(
      metadataPath,
      JSON.stringify(
        {
          version: "0.14.0",
          tag: "v0.14.0",
          published_at: "2026-05-27T00:00:00Z",
          targets: [
            {
              target: "x86_64-unknown-linux-musl",
              asset: "chan-x86_64-unknown-linux-musl.tar.gz",
              url: pathToFileURL(linuxTar.path).href,
              sha256: linuxTar.sha256,
            },
            {
              target: "aarch64-apple-darwin",
              asset: "chan-aarch64-apple-darwin.tar.gz",
              url: pathToFileURL(macTar.path).href,
              sha256: macTar.sha256,
            },
          ],
        },
        null,
        2,
      ),
    );

    runInstall({
      fakeBin,
      metadataPath,
      prefix: path.join(prefixes, "linux"),
      unameS: "Linux",
      unameM: "x86_64",
      expected: "linux",
    });
    runInstall({
      fakeBin,
      metadataPath,
      prefix: path.join(prefixes, "mac"),
      unameS: "Darwin",
      unameM: "arm64",
      expected: "mac",
    });
    runMetadataFailureFallback({
      fakeBin,
      metadataPath: path.join(root, "missing.json"),
      prefix: path.join(prefixes, "missing"),
    });
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
  console.log("smoked install.sh metadata selection");
}

async function writeFakeUname(fakeBin) {
  const uname = path.join(fakeBin, "uname");
  await fs.writeFile(
    uname,
    `#!/bin/sh
case "$1" in
  -s) printf '%s\\n' "$FAKE_UNAME_S" ;;
  -m) printf '%s\\n' "$FAKE_UNAME_M" ;;
  *) printf '%s\\n' "$FAKE_UNAME_S" ;;
esac
`,
  );
  await fs.chmod(uname, 0o755);
}

async function makeTarball(assets, target, marker) {
  const staging = path.join(assets, `staging-${target}`);
  await fs.mkdir(staging, { recursive: true });
  const bin = path.join(staging, "chan");
  await fs.writeFile(bin, `#!/bin/sh\nprintf '%s\\n' '${marker}'\n`);
  await fs.chmod(bin, 0o755);

  const tarball = path.join(assets, `chan-${target}.tar.gz`);
  execFileSync("tar", ["-czf", tarball, "-C", staging, "."]);
  const bytes = await fs.readFile(tarball);
  return {
    path: tarball,
    sha256: createHash("sha256").update(bytes).digest("hex"),
  };
}

function runInstall({ fakeBin, metadataPath, prefix, unameS, unameM, expected }) {
  const result = spawnSync("sh", [installer], {
    encoding: "utf8",
    env: {
      ...process.env,
      FAKE_UNAME_S: unameS,
      FAKE_UNAME_M: unameM,
      METADATA_URL: metadataPath,
      PATH: `${fakeBin}${path.delimiter}${process.env.PATH ?? ""}`,
      PREFIX: prefix,
    },
  });
  if (result.status !== 0) {
    throw new Error(`install failed for ${unameS}/${unameM}: ${result.stderr || result.stdout}`);
  }
  const output = execFileSync(path.join(prefix, "bin", "chan"), { encoding: "utf8" }).trim();
  if (output !== expected) {
    throw new Error(`installed binary printed ${JSON.stringify(output)}, expected ${expected}`);
  }
}

function runMetadataFailureFallback({ fakeBin, metadataPath, prefix }) {
  const result = spawnSync("sh", [installer], {
    encoding: "utf8",
    env: {
      ...process.env,
      FAKE_UNAME_S: "Linux",
      FAKE_UNAME_M: "x86_64",
      METADATA_URL: metadataPath,
      PATH: `${fakeBin}${path.delimiter}${process.env.PATH ?? ""}`,
      PREFIX: prefix,
    },
  });
  if (result.status === 0) {
    throw new Error("install unexpectedly succeeded with missing metadata");
  }
  if (!result.stderr.includes("manual downloads: https://github.com/fiorix/chan/releases")) {
    throw new Error(`missing metadata fallback message not found: ${result.stderr}`);
  }
}

main().catch((err) => {
  console.error(`install.sh smoke failed: ${err.message}`);
  process.exitCode = 1;
});
