#!/usr/bin/env node
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawn } from "node:child_process";

const BASE = process.env.CHAN_WEB_URL ?? "http://127.0.0.1:8788/";
const CHROME =
  process.env.CHROME_BIN ??
  "/Users/fiorix/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

const checks = [];
function pass(name, details = "") {
  checks.push({ name, ok: true, details });
  console.log(`PASS ${name}${details ? ` - ${details}` : ""}`);
}
function fail(name, err) {
  checks.push({ name, ok: false, details: String(err?.message ?? err) });
  console.error(`FAIL ${name} - ${String(err?.message ?? err)}`);
}

async function launchChrome() {
  const profile = await mkdtemp(join(tmpdir(), "chan-webtest-"));
  const args = [
    "--headless=new",
    "--disable-gpu",
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-background-networking",
    "--remote-debugging-port=0",
    `--user-data-dir=${profile}`,
    "about:blank",
  ];
  const proc = spawn(CHROME, args, { stdio: ["ignore", "ignore", "pipe"] });
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
    this.events = [];
  }
  async open() {
    await new Promise((resolve, reject) => {
      this.ws.addEventListener("open", resolve, { once: true });
      this.ws.addEventListener("error", reject, { once: true });
    });
    this.ws.addEventListener("message", (event) => {
      const msg = JSON.parse(event.data);
      if (msg.id && this.pending.has(msg.id)) {
        const { resolve, reject } = this.pending.get(msg.id);
        this.pending.delete(msg.id);
        if (msg.error) reject(new Error(msg.error.message));
        else resolve(msg.result ?? {});
      } else if (msg.method) {
        this.events.push(msg);
      }
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
      throw new Error(res.exceptionDetails.text ?? "Runtime.evaluate failed");
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
  async center(selector) {
    return await this.eval(`(() => {
      const el = document.querySelector(${JSON.stringify(selector)});
      if (!el) return null;
      const r = el.getBoundingClientRect();
      return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
    })()`);
  }
  async click(selector, button = "left") {
    const p = await this.center(selector);
    if (!p) throw new Error(`missing selector ${selector}`);
    await this.send("Input.dispatchMouseEvent", { type: "mouseMoved", x: p.x, y: p.y });
    await this.send("Input.dispatchMouseEvent", { type: "mousePressed", x: p.x, y: p.y, button, clickCount: 1 });
    await this.send("Input.dispatchMouseEvent", { type: "mouseReleased", x: p.x, y: p.y, button, clickCount: 1 });
  }
  async key(key, code, vk) {
    await this.send("Input.dispatchKeyEvent", { type: "keyDown", key, code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
    await this.send("Input.dispatchKeyEvent", { type: "keyUp", key, code, windowsVirtualKeyCode: vk, nativeVirtualKeyCode: vk });
  }
}

async function createPage(browserWs) {
  const port = new URL(browserWs).port;
  const target = await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: "PUT" }).then((r) => r.json());
  const page = new Cdp(target.webSocketDebuggerUrl);
  await page.open();
  await page.send("Runtime.enable");
  await page.send("Page.enable");
  await page.send("Input.setIgnoreInputEvents", { ignore: false });
  return page;
}

function urlWithHash(hash) {
  const u = new URL(BASE);
  u.hash = hash;
  return u.toString();
}

async function smokeSearch(page, width, height) {
  await page.navigate(urlWithHash("search=1:language%3ATypeScript"), width, height);
  await page.waitFor("!!document.querySelector('.search input')", 10000);
  await page.waitFor("document.body.innerText.includes('TypeScript') && document.body.innerText.includes('SLOC')", 15000);
  const before = await page.eval(`(() => {
    const hits = document.querySelector('.search .hits');
    const rows = [...document.querySelectorAll('.search .hits li')];
    return { count: rows.length, top: hits.scrollTop };
  })()`);
  if (before.count < 2) throw new Error(`expected multiple language rows, got ${before.count}`);
  for (let i = 0; i < Math.min(before.count + 4, 30); i += 1) {
    await page.key("ArrowDown", "ArrowDown", 40);
  }
  const nav = await page.eval(`(() => {
    const hits = document.querySelector('.search .hits');
    const active = document.querySelector('.search .hits li[data-active="true"]');
    if (!hits || !active) return null;
    const hr = hits.getBoundingClientRect();
    const ar = active.getBoundingClientRect();
    return {
      scrollTop: hits.scrollTop,
      visible: ar.top >= hr.top - 1 && ar.bottom <= hr.bottom + 1,
      overflow: document.documentElement.scrollWidth - window.innerWidth,
    };
  })()`);
  if (!nav?.visible) throw new Error("active search row is not visible after arrow navigation");
  if (nav.overflow > 2) throw new Error(`document horizontal overflow ${nav.overflow}px`);
  pass(`Search language + arrow scroll ${width}x${height}`, `${before.count} rows, scrollTop ${nav.scrollTop}`);
}

async function smokeSearchStatus(page) {
  await page.eval(`(() => {
    const btn = document.querySelector('button[aria-label="Show search index status"]');
    if (!btn) throw new Error('search status button missing');
    btn.click();
    return true;
  })()`);
  await page.waitFor(`(() => {
    const panel = [...document.querySelectorAll('.panel')]
      .find((p) => p.innerText.includes('Search Status'));
    return !!panel;
  })()`, 10000);
  await page.waitFor(`(() => {
    const panel = [...document.querySelectorAll('.panel')]
      .find((p) => p.innerText.includes('Search Status'));
    return !!panel && /CODE REPORT/i.test(panel.innerText) && panel.innerText.includes('SLOC');
  })()`, 30000);
  const text = await page.eval(`(() => {
    const panel = [...document.querySelectorAll('.panel')]
      .find((p) => p.innerText.includes('Search Status'));
    return panel?.innerText ?? document.body.innerText;
  })()`);
  if (!/CODE REPORT/i.test(text) || !text.includes("SLOC")) {
    throw new Error(`search status panel missing report fields: ${text.slice(0, 500)}`);
  }
  pass("Search Status overlay", "opened from search and rendered report fields");
}

async function smokeGraphThis(page, width, height) {
  await page.navigate(urlWithHash("files=1:"), width, height);
  await page.waitFor("!!document.querySelector('.browser .row.file, .browser .row.dir')", 15000);
  const selector = await page.eval(`(() => document.querySelector('.browser .row.file') ? '.browser .row.file' : '.browser .row.dir')()`);
  await page.eval(`((selector) => {
    const el = document.querySelector(selector);
    if (!el) throw new Error('browser row missing');
    const r = el.getBoundingClientRect();
    el.dispatchEvent(new MouseEvent('contextmenu', {
      bubbles: true,
      cancelable: true,
      clientX: r.left + Math.min(24, r.width / 2),
      clientY: r.top + r.height / 2,
      button: 2,
    }));
    return true;
  })(${JSON.stringify(selector)})`);
  await page.waitFor("!!document.querySelector('.ctx') && document.querySelector('.ctx').innerText.includes('Graph this')", 10000);
  await page.eval(`(() => {
    const btn = [...document.querySelectorAll('.ctx button')].find((b) => b.innerText.includes('Graph this'));
    if (!btn) throw new Error('Graph this menu item missing');
    btn.click();
    return true;
  })()`);
  await page.waitFor("!!document.querySelector('.graph-tab') || document.body.innerText.includes('Graph')", 10000);
  await page.waitFor(`(() => {
    const graph = document.querySelector('.graph-tab');
    return !!graph &&
      graph.innerText.includes('filesystem graph') &&
      graph.innerText.includes('contains');
  })()`, 15000);
  pass(`File Browser Graph this ${width}x${height}`, `opened filesystem graph from ${selector}`);
}

async function smokeAssistant(page, width, height) {
  const driveInfo = await fetch(new URL("/api/drive", BASE)).then((r) => r.json());
  if (driveInfo?.preferences?.assistant?.effective_enabled === false) {
    pass(`Assistant overlay layout ${width}x${height}`, "skipped: assistant disabled in test drive preferences");
    return;
  }
  await page.navigate(urlWithHash("assist=1:drive"), width, height);
  await page.waitFor("!!document.querySelector('.assistant-shell')", 10000);
  const layout = await page.eval(`(() => {
    const shell = document.querySelector('.assistant-shell');
    const body = document.querySelector('.assistant-body');
    const scroll = document.querySelector('.assistant-body .scroll');
    const prompt = document.querySelector('.assistant-body .prompt-wrap');
    const bubbles = [...document.querySelectorAll('.assistant-body .bubble .body')].map((el) => {
      const r = el.getBoundingClientRect();
      const pr = el.closest('.bubble').getBoundingClientRect();
      return { bodyW: r.width, bubbleW: pr.width };
    });
    return {
      shell: !!shell,
      body: !!body,
      scroll: !!scroll,
      prompt: !!prompt,
      overflow: document.documentElement.scrollWidth - window.innerWidth,
      bubbles,
      statusText: document.querySelector('.assistant-body .status-line')?.innerText ?? '',
      hasLegacyThinking: document.body.innerText.includes('thinking...') && !document.querySelector('.stream-status .dot'),
    };
  })()`);
  if (!layout.shell || !layout.body || !layout.scroll || !layout.prompt) {
    throw new Error("assistant overlay missing expected layout pieces");
  }
  if (layout.overflow > 2) throw new Error(`document horizontal overflow ${layout.overflow}px`);
  if (layout.hasLegacyThinking) throw new Error("legacy thinking text appeared without stream status dot");
  pass(`Assistant overlay layout ${width}x${height}`, "chat, scroll area, prompt, and status line present");
}

async function main() {
  const chrome = await launchChrome();
  try {
    const desktop = await createPage(chrome.wsUrl);
    await smokeSearch(desktop, 1440, 1000);
    await smokeSearchStatus(desktop);
    await smokeGraphThis(desktop, 1440, 1000);
    await smokeAssistant(desktop, 1440, 1000);

    const narrow = await createPage(chrome.wsUrl);
    await smokeSearch(narrow, 390, 844);
    await smokeGraphThis(narrow, 390, 844);
    await smokeAssistant(narrow, 390, 844);
  } finally {
    await chrome.close();
  }
  const failed = checks.filter((c) => !c.ok);
  if (failed.length > 0) process.exitCode = 1;
}

main().catch((err) => {
  fail("webtest smoke runner", err);
  process.exitCode = 1;
});
