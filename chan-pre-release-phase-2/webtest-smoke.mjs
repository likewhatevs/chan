#!/usr/bin/env node
import { mkdir, mkdtemp, rm, unlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawn } from "node:child_process";

const BASE = process.env.CHAN_WEB_URL ?? "http://127.0.0.1:8788/";
const DRIVE_ROOT = process.env.CHAN_WEBTEST_DRIVE_ROOT ?? "/tmp/chan-dev";
const SCRATCH_REL = "Scratch/phase2-smoke";
const SCRATCH_ABS = join(DRIVE_ROOT, SCRATCH_REL);
const CHROME =
  process.env.CHROME_BIN ??
  "/Users/fiorix/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

function pass(name, detail = "") {
  console.log(`PASS ${name}${detail ? ` - ${detail}` : ""}`);
}

async function launchChrome() {
  const profile = await mkdtemp(join(tmpdir(), "chan-webtest-p2-"));
  const proc = spawn(CHROME, [
    "--headless=new",
    "--disable-gpu",
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-background-networking",
    "--remote-debugging-port=0",
    `--user-data-dir=${profile}`,
    "about:blank",
  ], { stdio: ["ignore", "ignore", "pipe"] });
  let stderr = "";
  const wsUrl = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error("Chrome did not expose DevTools URL")), 10000);
    proc.once("exit", (code) => {
      clearTimeout(timer);
      reject(new Error(`Chrome exited before startup: ${code}\n${stderr}`));
    });
    proc.stderr.on("data", (buf) => {
      stderr += buf.toString();
      const match = stderr.match(/DevTools listening on (ws:\/\/[^\s]+)/);
      if (match) {
        clearTimeout(timer);
        resolve(match[1]);
      }
    });
  });
  return {
    wsUrl,
    async close() {
      if (!proc.killed) proc.kill("SIGTERM");
      await new Promise((resolve) => {
        const timer = setTimeout(resolve, 1500);
        proc.once("exit", () => {
          clearTimeout(timer);
          resolve();
        });
      });
      await rm(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 }).catch(() => {});
    },
  };
}

class Cdp {
  constructor(wsUrl) {
    this.ws = new WebSocket(wsUrl);
    this.next = 1;
    this.pending = new Map();
  }
  async open() {
    await new Promise((resolve, reject) => {
      this.ws.addEventListener("open", resolve, { once: true });
      this.ws.addEventListener("error", reject, { once: true });
    });
    this.ws.addEventListener("message", (event) => {
      const msg = JSON.parse(event.data);
      if (!msg.id || !this.pending.has(msg.id)) return;
      const { resolve, reject } = this.pending.get(msg.id);
      this.pending.delete(msg.id);
      if (msg.error) reject(new Error(msg.error.message));
      else resolve(msg.result ?? {});
    });
  }
  send(method, params = {}) {
    const id = this.next++;
    this.ws.send(JSON.stringify({ id, method, params }));
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
    });
  }
  async eval(expression) {
    const res = await this.send("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (res.exceptionDetails) {
      throw new Error(
        res.exceptionDetails.exception?.description ??
          res.exceptionDetails.text ??
          "Runtime.evaluate failed",
      );
    }
    return res.result?.value;
  }
  async waitFor(expression, timeoutMs = 10000) {
    const deadline = Date.now() + timeoutMs;
    let last;
    while (Date.now() < deadline) {
      try {
        const value = await this.eval(expression);
        if (value) return value;
        last = value;
      } catch (e) {
        last = e;
      }
      await new Promise((r) => setTimeout(r, 100));
    }
    throw new Error(`Timed out waiting for ${expression}; last=${String(last)}`);
  }
  async navigate(url, width, height) {
    await this.send("Emulation.setDeviceMetricsOverride", {
      width,
      height,
      deviceScaleFactor: 1,
      mobile: width < 700,
    });
    await this.send("Page.navigate", { url });
    await this.waitFor("document.readyState === 'complete'", 15000);
    await this.waitFor("!!document.querySelector('.app')", 15000);
  }
}

async function createPage(browserWs) {
  const port = new URL(browserWs).port;
  const target = await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: "PUT" }).then((r) => r.json());
  const page = new Cdp(target.webSocketDebuggerUrl);
  await page.open();
  await page.send("Runtime.enable");
  await page.send("Page.enable");
  return page;
}

function urlWithHash(hash) {
  const u = new URL(BASE);
  u.searchParams.set("webtest", `${Date.now()}-${Math.random().toString(16).slice(2)}`);
  u.hash = hash;
  return u.toString();
}

function encodedParam(name, value) {
  return `${name}=${encodeURIComponent(value)}`;
}

async function resetScratch() {
  await rm(SCRATCH_ABS, { recursive: true, force: true });
  await mkdir(SCRATCH_ABS, { recursive: true });
}

async function waitForIndexIdle(timeoutMs = 20000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const status = await fetch(new URL("/api/index/status", BASE)).then((r) => r.json());
    if (status.state === "idle") return status;
    await new Promise((r) => setTimeout(r, 250));
  }
  throw new Error("index did not become idle");
}

async function rebuildIndex() {
  await fetch(new URL("/api/index/rebuild", BASE), { method: "POST" });
  await waitForIndexIdle();
}

async function openGraph(page, value, width = 1440, height = 1000, layout = null) {
  const hash = [layout ? encodedParam("s", JSON.stringify(layout)) : null, encodedParam("graph", value)]
    .filter(Boolean)
    .join("&");
  await page.navigate(urlWithHash(hash), width, height);
  await page.waitFor("!!document.querySelector('.graph-tab canvas')", 15000);
}

async function openDepthMenu(page) {
  await page.eval(`(() => {
    const btn = document.querySelector('.graph-tab .hamburger-trigger');
    if (!btn) throw new Error('graph menu trigger missing');
    btn.click();
    return true;
  })()`);
  await page.waitFor("!!document.querySelector('.hamburger-menu input[aria-label=\"depth\"]')", 5000);
  return await page.eval(`(() => {
    const input = document.querySelector('.hamburger-menu input[aria-label="depth"]');
    return {
      min: Number(input.min),
      max: Number(input.max),
      value: Number(input.value),
      disabled: input.disabled,
    };
  })()`);
}

async function closeDepthMenu(page) {
  await page.eval(`document.body.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }))`);
}

async function smokeDepthCaps(page) {
  await resetScratch();
  await mkdir(join(SCRATCH_ABS, "depth-dir", "a", "b", "c"), { recursive: true });
  await writeFile(join(SCRATCH_ABS, "depth-file.md"), "# Depth file\\n");
  await writeFile(join(SCRATCH_ABS, "group-a.md"), "# Group A\\n");
  await writeFile(join(SCRATCH_ABS, "group-b.md"), "# Group B\\n");
  await writeFile(join(SCRATCH_ABS, "depth-dir", "a", "b", "c", "leaf.md"), "# Leaf\\n");
  await rebuildIndex();

  await openGraph(page, `file:${SCRATCH_REL}/depth-file.md|10`);
  let cap = await openDepthMenu(page);
  if (cap.max !== 1 || cap.value !== 1 || cap.disabled) {
    throw new Error(`file depth cap failed: ${JSON.stringify(cap)}`);
  }
  await closeDepthMenu(page);

  const layout = {
    k: "s",
    d: "r",
    a: { k: "l", t: [{ p: `${SCRATCH_REL}/group-a.md`, a: 1 }], f: 1 },
    b: { k: "l", t: [{ p: `${SCRATCH_REL}/group-b.md`, a: 1 }] },
  };
  await openGraph(page, "missing-scope|10", 1440, 1000, layout);
  cap = await openDepthMenu(page);
  if (cap.max !== 2 || cap.value !== 2 || cap.disabled) {
    throw new Error(`group depth cap failed: ${JSON.stringify(cap)}`);
  }
  await closeDepthMenu(page);

  await openGraph(page, `dir:${SCRATCH_REL}/depth-dir|10||1|fs`);
  cap = await openDepthMenu(page);
  if (cap.max !== 4 || cap.value !== 4 || cap.disabled) {
    throw new Error(`dir depth cap failed: ${JSON.stringify(cap)}`);
  }
  await closeDepthMenu(page);

  await openGraph(page, "drive|10");
  cap = await openDepthMenu(page);
  if (!cap.disabled || cap.max < 1 || cap.max > 6) {
    throw new Error(`drive depth cap failed: ${JSON.stringify(cap)}`);
  }
  pass("Graph depth caps", "file=1 group=2 dir=4 drive<=6");
}

async function graphNodeCount(page) {
  return await page.eval(`(() => {
    const text = document.querySelector('.graph-tab .statusbar .stat')?.textContent ?? "";
    const match = text.match(/(\\d+)\\/(\\d+) nodes/);
    return match ? Number(match[2]) : 0;
  })()`);
}

async function smokeGraphLiveMutation(page, width, height) {
  await resetScratch();
  await writeFile(join(SCRATCH_ABS, "ghost-probe.md"), "# Ghost probe\\n\\n[[target]]\\n");
  await rebuildIndex();

  await openGraph(page, `dir:${SCRATCH_REL}|1||1|fs`, width, height);
  const beforeAdd = await graphNodeCount(page);
  await writeFile(join(SCRATCH_ABS, `live-add-probe-${Date.now()}.md`), "# Live add\\n");
  await page.waitFor(`(() => {
    const text = document.querySelector('.graph-tab .statusbar .stat')?.textContent ?? "";
    const match = text.match(/(\\d+)\\/(\\d+) nodes/);
    return match && Number(match[2]) > ${beforeAdd};
  })()`, 10000);

  await openGraph(page, `file:${SCRATCH_REL}/ghost-probe.md|1`, width, height);
  await page.waitFor("!!document.querySelector('.graph-tab canvas')", 10000);
  await page.eval(`(() => {
    const canvas = document.querySelector('.graph-tab canvas');
    const r = canvas.getBoundingClientRect();
    canvas.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 }));
    canvas.dispatchEvent(new MouseEvent('mouseup', { bubbles: true, clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 }));
    return true;
  })()`);
  await unlink(join(SCRATCH_ABS, "ghost-probe.md"));
  await page.waitFor("document.body.innerText.includes('file does not exist') || document.body.innerText.includes('not in the current file listing')", 12000);
  pass(`Graph live add + delete ghost ${width}x${height}`, "scratch subtree mutation observed");
}

async function smokeSearchLayout(page) {
  await page.navigate(urlWithHash("search=1:language"), 1440, 1000);
  await page.waitFor("!!document.querySelector('.search .hits li')", 15000);
  const result = await page.eval(`(() => {
    const search = document.querySelector('.search');
    const body = document.querySelector('.search .search-body');
    const results = document.querySelector('.search .results');
    const inspector = document.querySelector('.search .inspector');
    const rows = [...document.querySelectorAll('.search .hits li .path')]
      .map((el) => el.textContent.trim())
      .filter(Boolean);
    const unique = new Set(rows);
    const sr = search?.getBoundingClientRect();
    const br = body?.getBoundingClientRect();
    const rr = results?.getBoundingClientRect();
    const ir = inspector?.getBoundingClientRect();
    return {
      rowCount: rows.length,
      duplicatePaths: rows.length - unique.size,
      hasInspector: !!inspector,
      inspectorInsideBody: !!br && !!ir && ir.top >= br.top - 1 && ir.bottom <= br.bottom + 1,
      bodyInsideSearch: !!sr && !!br && br.left >= sr.left - 1 && br.right <= sr.right + 1,
      resultsLeftOfInspector: !!rr && !!ir && rr.right <= ir.left + 2,
      overflow: document.documentElement.scrollWidth - window.innerWidth,
    };
  })()`);
  if (result.rowCount < 2) throw new Error(`expected multiple search rows, got ${result.rowCount}`);
  if (result.duplicatePaths !== 0) throw new Error(`search rows contain ${result.duplicatePaths} duplicate paths`);
  if (!result.hasInspector || !result.inspectorInsideBody || !result.bodyInsideSearch || !result.resultsLeftOfInspector) {
    throw new Error(`search layout failed: ${JSON.stringify(result)}`);
  }
  if (result.overflow > 2) throw new Error(`document horizontal overflow ${result.overflow}px`);
  pass("Search layout + per-file rows", `${result.rowCount} unique paths`);
}

async function smokeSearchStatusLanguageGraph(page) {
  await page.eval(`(() => {
    const btn = document.querySelector('button[aria-label="Show search index status"]');
    if (!btn) throw new Error('search status button missing');
    btn.click();
    return true;
  })()`);
  await page.waitFor("document.body.innerText.includes('Search Status') && document.body.innerText.includes('Graph this')", 15000);
  await page.eval(`(() => {
    const btn = [...document.querySelectorAll('button')]
      .find((b) => b.textContent.includes('Graph this'));
    if (!btn) throw new Error('Graph this button missing');
    btn.click();
    return true;
  })()`);
  await page.waitFor("!!document.querySelector('.graph-tab')", 10000);
  await page.waitFor("document.body.innerText.includes('language graph')", 15000);
  const graph = await page.eval(`(() => {
    const text = document.body.innerText;
    const canvas = document.querySelector('.graph-tab canvas');
    const r = canvas?.getBoundingClientRect();
    return {
      hasLanguageChip: text.includes('language'),
      hasNodeCount: /\\d+\\/\\d+ nodes/.test(text),
      canvasW: r?.width ?? 0,
      canvasH: r?.height ?? 0,
      overflow: document.documentElement.scrollWidth - window.innerWidth,
    };
  })()`);
  if (!graph.hasLanguageChip || !graph.hasNodeCount || graph.canvasW < 100 || graph.canvasH < 100) {
    throw new Error(`language graph layout failed: ${JSON.stringify(graph)}`);
  }
  if (graph.overflow > 2) throw new Error(`document horizontal overflow ${graph.overflow}px`);
  pass("Search Status Graph this -> language graph", `${Math.round(graph.canvasW)}x${Math.round(graph.canvasH)} canvas`);
}

// Wire-shape prep for frontend-10 (folder-glyph swap). Pre-swap,
// GraphPanel.svelte::mapFsNodes coerces fs-graph `kind: "folder"`
// nodes into RenderedNode `kind: "tag"`, so the canvas draws them
// with the same `#` glyph used for semantic-graph tag nodes.
// Post-swap, mapFsNodes will emit `kind: "folder"` and the canvas
// picks up the PATH_FOLDER stroke icon already wired at
// GraphCanvas.svelte (iconImages.folder = svgStrokeIcon(PATH_FOLDER)).
//
// This probe captures two canvas pixel signatures:
//   sig.fsFolder  = ROI signature of an fs-graph folder-scope canvas
//   sig.tagSearch = ROI signature of a semantic-graph canvas opened
//                   from a tag-bearing scope
// and asserts they differ in non-disc, non-bg pixel content. Pre-swap
// the signatures collide on the `#` glyph; post-swap they diverge.
// Gated behind CHAN_WEBTEST_GLYPH_PROBE=1 until @@Frontend lands the
// swap, because pre-swap the assertion is expected to fail.
async function captureCanvasSignature(page) {
  return await page.eval(`(() => {
    const canvas = document.querySelector('.graph-tab canvas');
    if (!canvas) return null;
    const ctx = canvas.getContext('2d');
    const w = canvas.width;
    const h = canvas.height;
    const img = ctx.getImageData(0, 0, w, h).data;
    // Coarse 16-bin luminance histogram, normalised. Stable enough
    // for "does this canvas contain the # glyph vs the folder glyph"
    // to register as a signature drift; tolerant to layout jitter
    // because the bins integrate across the whole canvas.
    const bins = new Array(16).fill(0);
    let total = 0;
    for (let i = 0; i < img.length; i += 4) {
      const a = img[i + 3];
      if (a < 16) continue;
      const lum = (img[i] * 30 + img[i + 1] * 59 + img[i + 2] * 11) / 100;
      bins[Math.min(15, Math.floor(lum / 16))] += 1;
      total += 1;
    }
    if (total === 0) return null;
    return bins.map((n) => n / total);
  })()`);
}

function signatureDistance(a, b) {
  if (!a || !b) return Infinity;
  let sum = 0;
  for (let i = 0; i < a.length; i += 1) sum += Math.abs(a[i] - b[i]);
  return sum;
}

async function smokeFolderGlyphWireShape(page) {
  await resetScratch();
  await mkdir(join(SCRATCH_ABS, "glyph-probe", "sub"), { recursive: true });
  await writeFile(join(SCRATCH_ABS, "glyph-probe", "root.md"), "# Root\\n\\n#alpha\\n");
  await writeFile(join(SCRATCH_ABS, "glyph-probe", "sub", "child.md"), "# Child\\n");
  await rebuildIndex();

  await openGraph(page, `dir:${SCRATCH_REL}/glyph-probe|2||1|fs`);
  await page.waitFor("!!document.querySelector('.graph-tab canvas')", 10000);
  // Let the force layout settle so the icons are rasterised.
  await new Promise((r) => setTimeout(r, 1500));
  const fsFolderSig = await captureCanvasSignature(page);

  await openGraph(page, `dir:${SCRATCH_REL}/glyph-probe|2`);
  await page.waitFor("!!document.querySelector('.graph-tab canvas')", 10000);
  await new Promise((r) => setTimeout(r, 1500));
  const semanticSig = await captureCanvasSignature(page);

  const drift = signatureDistance(fsFolderSig, semanticSig);
  const post = process.env.CHAN_WEBTEST_GLYPH_PROBE === "1";
  if (post && drift < 0.05) {
    throw new Error(
      `folder glyph still indistinguishable from tag glyph: drift=${drift.toFixed(4)}`,
    );
  }
  pass(
    `Folder glyph wire-shape (${post ? "post-swap" : "pre-swap prep"})`,
    `signature drift=${drift.toFixed(4)}`,
  );
}

async function main() {
  const chrome = await launchChrome();
  try {
    const page = await createPage(chrome.wsUrl);
    await smokeSearchLayout(page);
    await smokeSearchStatusLanguageGraph(page);
    await smokeDepthCaps(page);
    await smokeGraphLiveMutation(page, 1440, 1000);
    await smokeGraphLiveMutation(page, 390, 844);
    await smokeFolderGlyphWireShape(page);
  } finally {
    await chrome.close();
    await rm(SCRATCH_ABS, { recursive: true, force: true }).catch(() => {});
  }
}

main().catch((err) => {
  console.error(`FAIL phase2 smoke - ${err.message ?? err}`);
  process.exitCode = 1;
});
