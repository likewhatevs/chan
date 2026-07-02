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

// --- chan-reports (approximation of chan-report / tokei) -----------------
//
// The demo serves per-file SLOC/comment/blank/complexity stats plus language
// roll-ups and COCOMO, exactly like chan-report. The real crate counts via
// tokei; here we approximate with per-language comment syntax. Language names
// match tokei's so the roll-up reads right. Complexity and COCOMO use
// chan-report's exact formulas (crates/chan-report: complexity.rs, cocomo.rs).

const C_LIKE = { line: ["//"], block: [["/*", "*/"]] };
const HASH = { line: ["#"], block: [] };
const NO_COMMENT = { line: [], block: [] };
// ext (no dot) -> { name (tokei), line, block, markdown? }
const REPORT_LANGS = {
  rs: { name: "Rust", ...C_LIKE },
  ts: { name: "TypeScript", ...C_LIKE },
  tsx: { name: "TSX", ...C_LIKE },
  js: { name: "JavaScript", ...C_LIKE },
  mjs: { name: "JavaScript", ...C_LIKE },
  cjs: { name: "JavaScript", ...C_LIKE },
  jsx: { name: "JSX", ...C_LIKE },
  go: { name: "Go", ...C_LIKE },
  c: { name: "C", ...C_LIKE },
  h: { name: "C Header", ...C_LIKE },
  cpp: { name: "C++", ...C_LIKE },
  hpp: { name: "C++ Header", ...C_LIKE },
  css: { name: "CSS", line: [], block: [["/*", "*/"]] },
  scss: { name: "Sass", ...C_LIKE },
  svelte: { name: "Svelte", line: ["//"], block: [["/*", "*/"], ["<!--", "-->"]] },
  html: { name: "HTML", line: [], block: [["<!--", "-->"]] },
  json: { name: "JSON", ...NO_COMMENT },
  toml: { name: "TOML", ...HASH },
  yaml: { name: "YAML", ...HASH },
  yml: { name: "YAML", ...HASH },
  sh: { name: "Shell", ...HASH },
  bash: { name: "BASH", ...HASH },
  py: { name: "Python", ...HASH },
  rb: { name: "Ruby", line: ["#"], block: [["=begin", "=end"]] },
  sql: { name: "SQL", line: ["--"], block: [["/*", "*/"]] },
  lua: { name: "Lua", line: ["--"], block: [["--[[", "]]"]] },
  txt: { name: "Plain Text", ...NO_COMMENT },
  md: { name: "Markdown", ...NO_COMMENT, markdown: true },
  markdown: { name: "Markdown", ...NO_COMMENT, markdown: true },
};
const REPORT_FILENAMES = {
  Makefile: { name: "Makefile", ...HASH },
  makefile: { name: "Makefile", ...HASH },
  GNUmakefile: { name: "Makefile", ...HASH },
  Dockerfile: { name: "Dockerfile", ...HASH },
};

function detectReportLanguage(relPath) {
  const base = relPath.slice(relPath.lastIndexOf("/") + 1);
  if (REPORT_FILENAMES[base]) return REPORT_FILENAMES[base];
  const dot = base.lastIndexOf(".");
  if (dot < 0) return null;
  return REPORT_LANGS[base.slice(dot + 1).toLowerCase()] ?? null;
}

// Line classification approximating tokei: blanks are whitespace-only lines,
// comments are lines that are only comment (line-comment or inside a block),
// everything else is code. A trailing code fragment after a block close is
// left as comment (a minor divergence that keeps the scanner cheap).
function countLines(content, lang) {
  const stripped = content.endsWith("\n") ? content.slice(0, -1) : content;
  if (stripped === "" && content === "") return { code: 0, comments: 0, blanks: 0 };
  const lines = stripped.split("\n");
  let code = 0;
  let comments = 0;
  let blanks = 0;
  let blockClose = null;
  for (const raw of lines) {
    const line = raw.trim();
    if (blockClose !== null) {
      comments++;
      if (line.includes(blockClose)) blockClose = null;
      continue;
    }
    if (line === "") {
      blanks++;
      continue;
    }
    if (lang.line.some((tok) => line.startsWith(tok))) {
      comments++;
      continue;
    }
    const open = lang.block.find(([o]) => line.startsWith(o));
    if (open) {
      comments++;
      const [o, c] = open;
      if (!line.slice(o.length).includes(c)) blockClose = c;
      continue;
    }
    code++;
  }
  return { code, comments, blanks };
}

// chan-report/complexity.rs: count keyword occurrences. Alphabetic keywords on
// word boundaries (word byte = [A-Za-z0-9_]); `&&`/`||` as raw substrings.
const CX_WORDS = [
  "if", "else", "elsif", "elif", "for", "while", "switch", "case", "match",
  "do", "goto", "continue", "break", "try", "catch", "except", "and", "or",
];
const CX_WORD_RE = new RegExp(`(?<![A-Za-z0-9_])(?:${CX_WORDS.join("|")})(?![A-Za-z0-9_])`, "g");
function complexityScore(content) {
  let n = (content.match(CX_WORD_RE) ?? []).length;
  for (const sym of ["&&", "||"]) n += content.split(sym).length - 1;
  return n;
}

// One chan-report `file` row from full (uncapped) content.
function reportRow(relPath, content, bytes, mtimeMs, lang) {
  const { code, comments, blanks } = countLines(content, lang);
  return {
    path: relPath,
    language: lang.name,
    code,
    comments,
    blanks,
    complexity: complexityScore(content),
    bytes,
    mtime: new Date(mtimeMs).toISOString(),
  };
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
  const reportFiles = [];
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
    const reportLang = text ? detectReportLanguage(rel) : null;
    const wantStore =
      text && !NO_CONTENT_BASENAMES.has(base) && bytesIncluded < totalBudget;

    // Read once when either storage or the report needs the bytes. Reports are
    // computed from FULL content (chan-report counts the whole file), even for
    // files whose content is too big to store (e.g. package-lock.json).
    let content = null;
    if (text && (wantStore || reportLang)) {
      const buf = await fs.readFile(abs);
      if (looksBinary(buf)) entry.kind = "binary";
      else content = buf.toString("utf8");
    }

    if (reportLang && content !== null) {
      reportFiles.push(reportRow(rel, content, stat.size, stat.mtimeMs, reportLang));
    }

    if (content !== null && wantStore) {
      const cap = kind === "document" ? maxMd : maxText;
      let stored = content;
      if (stored.length > cap) {
        stored = stored.slice(0, cap);
        entry.truncated = true;
        truncatedCount++;
      }
      entry.content = stored;
      bytesIncluded += Buffer.byteLength(stored, "utf8");
      textCount++;
    } else if (text && entry.kind !== "binary" && !wantStore) {
      skippedContentCount++;
    }

    files.push(entry);
  }

  reportFiles.sort((a, z) => a.path.localeCompare(z.path));
  const reportCode = reportFiles.reduce((n, r) => n + r.code, 0);

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
      reportFileCount: reportFiles.length,
      reportCode,
    },
    files,
    reports: { files: reportFiles },
  };

  const json = JSON.stringify(snapshot);
  await fs.mkdir(path.dirname(args.out), { recursive: true });
  await fs.writeFile(args.out, json);

  const mb = (Buffer.byteLength(json, "utf8") / (1024 * 1024)).toFixed(2);
  process.stdout.write(
    `snapshot: ${files.length} files, ${textCount} with content ` +
      `(${truncatedCount} truncated, ${skippedContentCount} content-skipped), ` +
      `${reportFiles.length} report rows (${reportCode.toLocaleString()} SLOC), ` +
      `asset ${mb} MB -> ${args.out}\n`,
  );
}

main().catch((err) => {
  process.stderr.write(`snapshot failed: ${err?.stack ?? err}\n`);
  process.exit(1);
});
