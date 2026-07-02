#!/usr/bin/env node
// Snapshot a git repo into the mock-workspace data the frontend-only demo
// loads into memory. The demo runs the real workspace-app with no backend
// (see src/demo/), so this asset stands in for the whole filesystem: the file
// browser, editor, graph, and search all read from it.
//
// This script stays a dumb file-walker on purpose. It emits a flat list of
// files (path + kind + capped content); every chan-specific derivation (tree
// listing, the wikilink/tag graph, headings, search) is computed on demand in
// TypeScript by the mock client, so the domain logic lives in one testable
// place and this script never drifts from the real graph rules.
//
// Usage:
//   node snapshot-workspace.mjs --repo <dir> --out <file.json> \
//     [--max-md-kb 64] [--max-text-kb 24] [--max-total-mb 6]

import { execFileSync } from "node:child_process";
import { promises as fs } from "node:fs";
import path from "node:path";

function parseArgs(argv) {
  const args = {
    repo: process.cwd(),
    out: "",
    maxMdKb: 64,
    maxTextKb: 24,
    maxTotalMb: 6,
  };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const next = () => argv[++i];
    if (a === "--repo") args.repo = path.resolve(next());
    else if (a === "--out") args.out = path.resolve(next());
    else if (a === "--max-md-kb") args.maxMdKb = Number(next());
    else if (a === "--max-text-kb") args.maxTextKb = Number(next());
    else if (a === "--max-total-mb") args.maxTotalMb = Number(next());
    else throw new Error(`unknown arg: ${a}`);
  }
  if (!args.out) throw new Error("--out is required");
  return args;
}

// Image / document binaries the tree shows but the demo has no bytes for.
const MEDIA_EXTS = new Set([
  ".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".avif", ".bmp", ".ico",
  ".tif", ".tiff", ".heic",
]);
// Non-image binaries: keep the tree entry, drop the bytes.
const BINARY_EXTS = new Set([
  ".pdf", ".zst", ".gz", ".zip", ".tar", ".woff", ".woff2", ".ttf", ".otf",
  ".eot", ".mp4", ".mov", ".webm", ".mp3", ".wav", ".wasm", ".bin", ".png@2x",
]);
// Big generated/lock files: keep the entry, skip the (huge) content.
const NO_CONTENT_BASENAMES = new Set([
  "package-lock.json", "Cargo.lock",
]);

function classify(relPath) {
  const ext = path.extname(relPath).toLowerCase();
  if (ext === ".md" || ext === ".markdown") return { kind: "document", text: true };
  if (MEDIA_EXTS.has(ext)) return { kind: "media", text: false };
  if (BINARY_EXTS.has(ext)) return { kind: "binary", text: false };
  return { kind: "text", text: true };
}

// A NUL byte in the head is the cheap, reliable "this is binary" signal.
function looksBinary(buf) {
  const n = Math.min(buf.length, 8192);
  for (let i = 0; i < n; i++) if (buf[i] === 0) return true;
  return false;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const repo = args.repo;
  const label = path.basename(repo);
  const maxMd = args.maxMdKb * 1024;
  const maxText = args.maxTextKb * 1024;
  const totalBudget = args.maxTotalMb * 1024 * 1024;

  // Tracked files only: git already excludes node_modules/target/dist/.git and
  // everything else gitignored, so the tree matches what a contributor sees.
  const listed = execFileSync("git", ["-C", repo, "ls-files", "-z"], {
    maxBuffer: 256 * 1024 * 1024,
  })
    .toString("utf8")
    .split("\0")
    .filter(Boolean)
    .sort();

  const files = [];
  let bytesIncluded = 0;
  let textCount = 0;
  let truncatedCount = 0;
  let skippedContentCount = 0;

  for (const rel of listed) {
    const abs = path.join(repo, rel);
    let stat;
    try {
      stat = await fs.stat(abs);
    } catch {
      continue; // listed-but-missing (broken symlink); skip.
    }
    if (!stat.isFile()) continue;

    const { kind, text } = classify(rel);
    const entry = {
      path: rel,
      kind,
      size: stat.size,
      mtime: Math.floor(stat.mtimeMs / 1000),
    };

    const base = path.basename(rel);
    const wantContent =
      text && !NO_CONTENT_BASENAMES.has(base) && bytesIncluded < totalBudget;

    if (wantContent) {
      const buf = await fs.readFile(abs);
      if (looksBinary(buf)) {
        entry.kind = "binary";
      } else {
        const cap = kind === "document" ? maxMd : maxText;
        let content = buf.toString("utf8");
        if (content.length > cap) {
          content = content.slice(0, cap);
          entry.truncated = true;
          truncatedCount++;
        }
        entry.content = content;
        bytesIncluded += Buffer.byteLength(content, "utf8");
        textCount++;
      }
    } else if (text) {
      skippedContentCount++;
    }

    files.push(entry);
  }

  const snapshot = {
    metadata: {
      workspaceRoot: label,
      label,
      generatedAt: Date.now(),
      sourceRepo: repo,
      fileCount: files.length,
      textCount,
      truncatedCount,
      skippedContentCount,
      bytesIncluded,
    },
    files,
  };

  const json = JSON.stringify(snapshot);
  await fs.mkdir(path.dirname(args.out), { recursive: true });
  await fs.writeFile(args.out, json);

  const mb = (Buffer.byteLength(json, "utf8") / (1024 * 1024)).toFixed(2);
  process.stdout.write(
    `snapshot: ${files.length} files, ${textCount} with content ` +
      `(${truncatedCount} truncated, ${skippedContentCount} content-skipped), ` +
      `asset ${mb} MB -> ${args.out}\n`,
  );
}

main().catch((err) => {
  process.stderr.write(`snapshot failed: ${err?.stack ?? err}\n`);
  process.exit(1);
});
