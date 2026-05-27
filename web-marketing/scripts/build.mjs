#!/usr/bin/env node
import { promises as fs } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const siteRoot = path.resolve(path.dirname(scriptPath), "..");
const repoRoot = path.resolve(siteRoot, "..");
const srcRoot = path.join(siteRoot, "src");
const distRoot = path.join(siteRoot, "dist");
const manualRoot = path.join(repoRoot, "docs", "manual");
const githubRepoUrl = "https://github.com/fiorix/chan";
const cliMetadataBase = "https://chan.app/dl/cli";
const releasesMetadataPath = "/dl/releases.json";
const requiredDownloadIds = [
  "desktop-macos-dmg",
  "desktop-linux-appimage",
  "desktop-linux-deb",
  "cli-linux-x64",
  "cli-linux-arm64",
  "cli-macos-arm64",
];

const requiredInputs = [
  path.join(srcRoot, "templates", "base.html"),
  path.join(srcRoot, "pages", "home.html"),
  path.join(srcRoot, "pages", "install.html"),
  path.join(srcRoot, "install.sh"),
  path.join(srcRoot, "styles.css"),
  path.join(srcRoot, "site.js"),
  path.join(siteRoot, "favicon.ico"),
  path.join(siteRoot, "chan-mark.png"),
  path.join(siteRoot, "qr-donate.png"),
  path.join(manualRoot, "index.md"),
];

async function main() {
  await Promise.all(requiredInputs.map(assertFile));

  const version = await readWorkspaceVersion();
  const manualPages = await readManualPages();
  const baseTemplate = await fs.readFile(path.join(srcRoot, "templates", "base.html"), "utf8");
  const homeTemplate = await fs.readFile(path.join(srcRoot, "pages", "home.html"), "utf8");
  const installTemplate = await fs.readFile(path.join(srcRoot, "pages", "install.html"), "utf8");

  await fs.rm(distRoot, { recursive: true, force: true });
  await fs.mkdir(path.join(distRoot, "assets"), { recursive: true });

  await copyStaticAssets();
  await copyInstaller();
  await fs.writeFile(path.join(distRoot, "CNAME"), "chan.app\n");

  const releaseTemplateValues = {
    githubReleasesUrl: githubRepoUrl + "/releases",
  };
  await writePage(
    "index.html",
    renderPage(baseTemplate, {
      active: "home",
      bodyClass: "home-page",
      title: "chan - local-first markdown editor",
      description:
        "Chan is a local-first markdown editor for plain-file drives, wiki-links, search, graph, terminal, and MCP workflows.",
      content: fillTemplate(homeTemplate, { version, ...releaseTemplateValues }),
    }),
  );

  await writePage(
    "install/index.html",
    renderPage(baseTemplate, {
      active: "install",
      bodyClass: "install-page",
      title: "Install chan",
      description: "Install Chan Desktop or the standalone chan CLI.",
      content: fillTemplate(installTemplate, { version, ...releaseTemplateValues }),
    }),
  );

  const manualNav = renderManualNav(manualPages);
  for (const page of manualPages) {
    await writePage(
      page.output,
      renderPage(baseTemplate, {
        active: "manual",
        bodyClass: "manual-page",
        title: `${page.title} - chan manual`,
        description: `Chan manual: ${page.title}.`,
        content: renderManualPage(page, manualNav),
      }),
    );
  }

  await validateDist(version);
  console.log(`built web-marketing/dist for chan ${version}`);
}

async function assertFile(file) {
  try {
    const stat = await fs.stat(file);
    if (!stat.isFile()) throw new Error(`${file} is not a file`);
  } catch (err) {
    throw new Error(`missing required input: ${path.relative(repoRoot, file)} (${err.message})`);
  }
}

async function readWorkspaceVersion() {
  const cargoToml = await fs.readFile(path.join(repoRoot, "Cargo.toml"), "utf8");
  const match = cargoToml.match(/^\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m);
  if (!match) throw new Error("workspace package version not found in Cargo.toml");
  return match[1];
}

async function copyStaticAssets() {
  await fs.copyFile(path.join(siteRoot, "favicon.ico"), path.join(distRoot, "favicon.ico"));
  await fs.copyFile(path.join(siteRoot, "chan-mark.png"), path.join(distRoot, "chan-mark.png"));
  await fs.copyFile(path.join(siteRoot, "qr-donate.png"), path.join(distRoot, "qr-donate.png"));
  await fs.copyFile(path.join(srcRoot, "styles.css"), path.join(distRoot, "assets", "site.css"));
  await fs.copyFile(path.join(srcRoot, "site.js"), path.join(distRoot, "assets", "site.js"));
  await copyDir(path.join(siteRoot, "assets"), path.join(distRoot, "assets"));
}

async function copyInstaller() {
  const source = path.join(srcRoot, "install.sh");
  const target = path.join(distRoot, "install.sh");
  await fs.copyFile(source, target);
  await fs.chmod(target, 0o755);
}

async function copyDir(source, target) {
  const entries = await fs.readdir(source, { withFileTypes: true });
  await fs.mkdir(target, { recursive: true });
  for (const entry of entries) {
    const from = path.join(source, entry.name);
    const to = path.join(target, entry.name);
    if (entry.isDirectory()) {
      await copyDir(from, to);
    } else if (entry.isFile()) {
      await fs.copyFile(from, to);
    }
  }
}

async function writePage(output, html) {
  const target = path.join(distRoot, output);
  await fs.mkdir(path.dirname(target), { recursive: true });
  await fs.writeFile(target, html);
}

function renderPage(template, { active, bodyClass, title, description, content }) {
  return fillTemplate(template, {
    bodyClass,
    content,
    description: escapeHtml(description),
    siteNav: renderSiteNav(active),
    title: escapeHtml(title),
  });
}

function renderSiteNav(active) {
  const links = [
    ["home", "/", "Home"],
    ["install", "/install/", "Install"],
    ["manual", "/manual/", "Manual"],
    ["github", githubRepoUrl, "GitHub"],
  ];
  return links
    .map(([key, href, label]) => {
      const attrs = key === active ? ' class="active" aria-current="page"' : "";
      return `<a${attrs} href="${href}">${label}</a>`;
    })
    .join("\n        ");
}

function fillTemplate(template, values) {
  let rendered = template;
  for (const [key, value] of Object.entries(values)) {
    rendered = rendered.split(`{{${key}}}`).join(String(value));
  }
  const missing = rendered.match(/{{[a-zA-Z0-9_]+}}/g);
  if (missing) throw new Error(`unfilled template values: ${[...new Set(missing)].join(", ")}`);
  return rendered;
}

async function readManualPages() {
  const files = await walkMarkdown(manualRoot);
  if (!files.some((file) => path.relative(manualRoot, file) === "index.md")) {
    throw new Error("docs/manual/index.md is required");
  }

  const pages = [];
  let indexLinkOrder = new Map();
  for (const file of files) {
    const source = path.relative(repoRoot, file);
    const raw = await fs.readFile(file, "utf8");
    const { attrs, body } = parseFrontMatter(raw, source);
    const title = firstH1(body, source);
    const rel = path.relative(manualRoot, file).split(path.sep).join("/");
    const url = manualUrlFor(rel);
    if (rel === "index.md") {
      indexLinkOrder = manualIndexLinkOrder(body);
    }
    pages.push({
      attrs,
      depth: url.split("/").filter(Boolean).length - 1,
      html: renderMarkdown(body, source, rel),
      output: outputForUrl(url),
      rel,
      source,
      title,
      url,
    });
  }

  pages.sort((a, b) => {
    if (a.rel === "index.md") return -1;
    if (b.rel === "index.md") return 1;
    const aOrder = manualSortOrder(a, indexLinkOrder);
    const bOrder = manualSortOrder(b, indexLinkOrder);
    if (Number.isFinite(aOrder) && Number.isFinite(bOrder) && aOrder !== bOrder) {
      return aOrder - bOrder;
    }
    if (Number.isFinite(aOrder) !== Number.isFinite(bOrder)) {
      return Number.isFinite(aOrder) ? -1 : 1;
    }
    return a.rel.localeCompare(b.rel);
  });
  return pages;
}

function manualIndexLinkOrder(markdown) {
  const order = new Map();
  for (const m of markdown.matchAll(/\[[^\]]+]\(([^)#?]+\.md(?:#[^)]*)?)\)/g)) {
    const url = manualHrefToCleanUrl(m[1], "index.md");
    if (!url || url === "/manual/" || order.has(url)) continue;
    order.set(url, order.size + 1);
  }
  return order;
}

function manualSortOrder(page, indexLinkOrder) {
  const explicit = Number(page.attrs.order ?? Number.NaN);
  if (Number.isFinite(explicit)) return explicit;
  return indexLinkOrder.get(page.url) ?? Number.NaN;
}

async function walkMarkdown(dir) {
  const entries = await fs.readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name))) {
    if (entry.name.startsWith(".") || entry.name.startsWith("_")) continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await walkMarkdown(full)));
    } else if (entry.isFile() && entry.name.endsWith(".md")) {
      files.push(full);
    }
  }
  return files;
}

function parseFrontMatter(raw, source) {
  const normalized = raw.replace(/\r\n/g, "\n");
  if (!normalized.startsWith("---\n")) return { attrs: {}, body: normalized };
  const end = normalized.indexOf("\n---\n", 4);
  if (end === -1) throw new Error(`${source}: front matter starts but never closes`);
  const block = normalized.slice(4, end).trim();
  const attrs = {};
  for (const line of block.split("\n").filter(Boolean)) {
    const match = line.match(/^([A-Za-z][A-Za-z0-9_-]*):\s*(.*)$/);
    if (!match) throw new Error(`${source}: invalid front matter line: ${line}`);
    const [, key, value] = match;
    if (!["order"].includes(key)) {
      throw new Error(`${source}: unsupported front matter key: ${key}`);
    }
    if (key === "order" && !/^\d+$/.test(value)) {
      throw new Error(`${source}: front matter order must be an integer`);
    }
    attrs[key] = value;
  }
  return { attrs, body: normalized.slice(end + "\n---\n".length) };
}

function firstH1(markdown, source) {
  const match = markdown.match(/^#\s+(.+?)\s*$/m);
  if (!match) throw new Error(`${source}: first H1 title is required`);
  return stripMarkdown(match[1]);
}

function manualUrlFor(rel) {
  const withoutExt = rel.replace(/\.md$/, "");
  if (withoutExt === "index") return "/manual/";
  if (withoutExt.endsWith("/index")) return `/manual/${withoutExt.slice(0, -"/index".length)}/`;
  return `/manual/${withoutExt}/`;
}

// Maps a drive-relative `.md` link (as authored in docs/manual) to the
// clean published URL. Returns null for hrefs that are not drive-relative
// .md targets (external, root-absolute, anchor-only) so callers leave
// them untouched. Manual pages are flat siblings today, but this resolves
// against the source page's dir so nested pages would work too.
function manualHrefToCleanUrl(href, pageRel) {
  if (/^[a-z][a-z0-9+.-]*:/i.test(href)) return null; // scheme (http:, mailto:)
  if (href.startsWith("#") || href.startsWith("/")) return null;
  const hash = href.indexOf("#");
  const pathPart = hash === -1 ? href : href.slice(0, hash);
  const anchor = hash === -1 ? "" : href.slice(hash);
  if (!/\.md$/.test(pathPart)) return null;
  const pageDir = path.posix.dirname(pageRel); // "." for root pages
  const base = pageDir === "." ? "" : `${pageDir}/`;
  const targetRel = path.posix.normalize(`${base}${pathPart}`);
  return `${manualUrlFor(targetRel)}${anchor}`;
}

function outputForUrl(url) {
  if (!url.startsWith("/") || !url.endsWith("/")) throw new Error(`invalid clean URL: ${url}`);
  return path.posix.join(url.slice(1), "index.html");
}

function renderManualNav(pages) {
  const links = pages
    .map((page) => {
      const style = page.depth > 0 ? ` style="--depth:${page.depth}"` : "";
      return `<a${style} href="${page.url}">${escapeHtml(page.title)}</a>`;
    })
    .join("\n");
  return `<nav class="manual-nav" aria-label="Manual pages">\n${links}\n</nav>`;
}

function renderManualPage(page, manualNav) {
  return `<div class="manual-layout">
${manualNav}
<article class="prose">
${page.html}
</article>
</div>`;
}

function renderMarkdown(markdown, source, pageRel) {
  const lines = markdown.replace(/\r\n/g, "\n").trimEnd().split("\n");
  const html = [];
  const usedIds = new Map();
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (!line.trim()) {
      i += 1;
      continue;
    }

    if (line.startsWith("```")) {
      const lang = line.slice(3).trim();
      const code = [];
      i += 1;
      while (i < lines.length && !lines[i].startsWith("```")) {
        code.push(lines[i]);
        i += 1;
      }
      if (i >= lines.length) throw new Error(`${source}: unterminated code fence`);
      i += 1;
      const langClass = lang ? ` class="language-${escapeAttribute(lang)}"` : "";
      html.push(`<pre><code${langClass}>${escapeHtml(code.join("\n"))}</code></pre>`);
      continue;
    }

    const heading = line.match(/^(#{1,4})\s+(.+)$/);
    if (heading) {
      const level = heading[1].length;
      const text = heading[2].trim();
      const id = uniqueId(slugify(stripMarkdown(text)), usedIds);
      html.push(`<h${level} id="${id}">${renderInline(text, pageRel)}</h${level}>`);
      i += 1;
      continue;
    }

    if (/^-\s+/.test(line)) {
      const items = [];
      while (i < lines.length && /^-\s+/.test(lines[i])) {
        items.push(`<li>${renderInline(lines[i].replace(/^-\s+/, ""), pageRel)}</li>`);
        i += 1;
      }
      html.push(`<ul>\n${items.join("\n")}\n</ul>`);
      continue;
    }

    const paragraph = [line.trim()];
    i += 1;
    while (
      i < lines.length &&
      lines[i].trim() &&
      !lines[i].startsWith("```") &&
      !/^(#{1,4})\s+/.test(lines[i]) &&
      !/^-+\s+/.test(lines[i])
    ) {
      paragraph.push(lines[i].trim());
      i += 1;
    }
    html.push(`<p>${renderInline(paragraph.join(" "), pageRel)}</p>`);
  }
  return html.join("\n");
}

function renderInline(text, pageRel) {
  let rendered = escapeHtml(text);
  rendered = rendered.replace(/`([^`]+)`/g, "<code>$1</code>");
  rendered = rendered.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  rendered = rendered.replace(/\[([^\]]+)]\(([^)]+)\)/g, (_match, label, href) => {
    const finalHref = manualHrefToCleanUrl(href, pageRel) ?? href;
    return `<a href="${escapeAttribute(finalHref)}">${label}</a>`;
  });
  return rendered;
}

function stripMarkdown(text) {
  return text.replace(/`([^`]+)`/g, "$1").replace(/\*\*([^*]+)\*\*/g, "$1").trim();
}

function slugify(text) {
  const slug = text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return slug || "section";
}

function uniqueId(base, used) {
  const count = used.get(base) ?? 0;
  used.set(base, count + 1);
  return count === 0 ? base : `${base}-${count + 1}`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function escapeAttribute(value) {
  return escapeHtml(value).replaceAll("'", "&#39;");
}

async function validateDist(version) {
  const files = await collectFiles(distRoot);
  const htmlFiles = files.filter((file) => file.endsWith(".html"));
  const allDistPaths = new Set(files.map((file) => path.relative(distRoot, file).split(path.sep).join("/")));
  const textByDistPath = new Map();

  for (const htmlFile of htmlFiles) {
    const html = await fs.readFile(htmlFile, "utf8");
    validateLocalLinks(htmlFile, html, allDistPaths);
  }

  const textFiles = files.filter((file) => /\.(html|css|js|sh|txt|xml)$/.test(file) || path.basename(file) === "CNAME");
  for (const file of textFiles) {
    const text = await fs.readFile(file, "utf8");
    textByDistPath.set(path.relative(distRoot, file).split(path.sep).join("/"), text);
    validateNoRemovedInstallSurface(file, text);
    validateNoStalePublicCopy(file, text);
  }

  for (const required of ["index.html", "install/index.html", "manual/index.html", "install.sh", "CNAME"]) {
    if (!allDistPaths.has(required)) throw new Error(`dist is missing ${required}`);
  }

  validateReleaseDownloadContract(textByDistPath);
}

async function collectFiles(dir) {
  const entries = await fs.readdir(dir, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      files.push(...(await collectFiles(full)));
    } else if (entry.isFile()) {
      files.push(full);
    }
  }
  return files;
}

function validateLocalLinks(htmlFile, html, allDistPaths) {
  const pageUrl = urlForOutput(path.relative(distRoot, htmlFile).split(path.sep).join("/"));
  const attrPattern = /\b(?:href|src)="([^"]+)"/g;
  for (const match of html.matchAll(attrPattern)) {
    validatePublicLink(htmlFile, match[1]);
    const target = normalizeLink(pageUrl, match[1]);
    if (!target) continue;
    const distPath = distPathForUrl(target);
    if (!allDistPaths.has(distPath)) {
      throw new Error(`${path.relative(repoRoot, htmlFile)} links to missing ${match[1]}`);
    }
  }
}

function validatePublicLink(htmlFile, raw) {
  const forbidden = [
    /(?:https?:\/\/chan\.app)?\/dl\/latest\//i,
    /github\.com\/fiorix\/chan\/releases\/latest\/download/i,
    /github\.com\/chan-writer\/chan/i,
  ];
  for (const pattern of forbidden) {
    if (pattern.test(raw)) {
      throw new Error(`${path.relative(repoRoot, htmlFile)} links to stale release route: ${raw}`);
    }
  }
}

function normalizeLink(pageUrl, raw) {
  if (
    raw.startsWith("#") ||
    raw.startsWith("mailto:") ||
    raw.startsWith("http://") ||
    raw.startsWith("https://") ||
    raw.startsWith("data:")
  ) {
    return null;
  }
  const clean = raw.split("#")[0].split("?")[0];
  if (!clean) return null;
  if (clean.startsWith("/")) return clean;
  const base = pageUrl.endsWith("/") ? pageUrl : `${path.posix.dirname(pageUrl)}/`;
  return path.posix.normalize(path.posix.join(base, clean));
}

function urlForOutput(output) {
  if (output === "index.html") return "/";
  if (output.endsWith("/index.html")) return `/${output.slice(0, -"index.html".length)}`;
  return `/${output}`;
}

function distPathForUrl(url) {
  if (url.endsWith("/")) return path.posix.join(url.slice(1), "index.html");
  return url.slice(1);
}

function validateNoRemovedInstallSurface(file, text) {
  const forbidden = [
    /install\.ps1/i,
    /PowerShell/i,
    /irm\s+https?:/i,
    /Windows\s+installer/i,
  ];
  for (const pattern of forbidden) {
    if (pattern.test(text)) {
      throw new Error(`${path.relative(repoRoot, file)} contains removed install surface: ${pattern}`);
    }
  }
}

function validateNoStalePublicCopy(file, text) {
  const forbidden = [
    /\bCLI[- ]only\b/i,
    /assistant pane/i,
    /in-app assistant/i,
    /no telemetry/i,
    /\blegacy\b/i,
    /\bmigrations?\b/i,
    /backward[- ]compatibility/i,
    /\bbackcompat\b/i,
    /github\.com\/chan-writer\/chan/i,
    /chan\.app\/dl\/(?:latest|v[0-9])/i,
    /(?:^|[\s"'(])\/dl\/(?:latest|v[0-9][^\s"'<>)]*)/i,
  ];
  for (const pattern of forbidden) {
    if (pattern.test(text)) {
      throw new Error(`${path.relative(repoRoot, file)} contains stale public copy: ${pattern}`);
    }
  }
}

function validateReleaseDownloadContract(textByDistPath) {
  const html = [...textByDistPath.entries()]
    .filter(([file]) => file.endsWith(".html"))
    .map(([, text]) => text)
    .join("\n");

  if (/github\.com\/fiorix\/chan\/releases\/latest\/download/i.test(html)) {
    throw new Error("generated pages must not use GitHub latest-download URLs");
  }
  if (/github\.com\/fiorix\/chan\/releases\/download\/v\d+\.\d+\.\d+\//i.test(html)) {
    throw new Error("generated pages must not infer concrete release asset URLs");
  }
  for (const id of requiredDownloadIds) {
    if (!html.includes(`data-release-download="${id}"`)) {
      throw new Error(`generated pages are missing metadata download hook: ${id}`);
    }
  }

  const siteJs = textByDistPath.get("assets/site.js") ?? "";
  if (!siteJs.includes(releasesMetadataPath)) {
    throw new Error(`site.js must read release metadata from ${releasesMetadataPath}`);
  }
  if (!siteJs.includes(`${githubRepoUrl}/releases`)) {
    throw new Error("site.js must keep GitHub Releases as the metadata failure fallback");
  }

  const installer = textByDistPath.get("install.sh") ?? "";
  const expectedMetadataBase = `DEFAULT_METADATA_BASE="${cliMetadataBase}"`;
  if (!installer.includes(expectedMetadataBase)) {
    throw new Error(`install.sh must default metadata base to ${cliMetadataBase}`);
  }
  if (installer.includes("/releases/latest/download")) {
    throw new Error("install.sh must not depend on GitHub latest-download URLs");
  }
}

main().catch((err) => {
  console.error(`site build failed: ${err.message}`);
  process.exitCode = 1;
});
