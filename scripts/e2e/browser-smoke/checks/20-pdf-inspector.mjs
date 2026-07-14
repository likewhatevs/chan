// Item 6, surface (a): export documents and decks to PDF through the
// Inspector action and assert the downloaded bytes: page counts, A4
// orientation, and per-page nonzero raster ink.

import { existsSync, rmSync } from "node:fs";
import { join } from "node:path";

async function openFileBrowser(page) {
  const open = await page.$(".file-tree, [role=tree]");
  if (open) return;
  await page.evaluate(() => {
    window.dispatchEvent(
      new CustomEvent("chan:command", { detail: { name: "app.files.toggle" } }),
    );
  });
  await page.waitForSelector('[role="treeitem"]', { timeout: 15_000 });
}

async function selectTreeFile(page, filename) {
  const clicked = await page.evaluate((name) => {
    const row = [...document.querySelectorAll('[role="treeitem"] button.name')].find(
      (b) => b.textContent?.trim() === name,
    );
    if (!row) return false;
    row.click();
    return true;
  }, filename);
  if (!clicked) throw new Error(`tree row not found: ${filename}`);
}

async function clickExportToPdf(page) {
  await page.waitForSelector(".pill-caret", { timeout: 10_000 });
  await page.click(".pill-caret");
  await page.waitForSelector(".action-menu-item", { timeout: 5_000 });
  const clicked = await page.evaluate(() => {
    const item = [...document.querySelectorAll(".action-menu-item")].find((b) =>
      b.textContent?.includes("Export to PDF"),
    );
    if (!item) return false;
    item.click();
    return true;
  });
  if (!clicked) throw new Error("Export to PDF menu item not found");
}

export default {
  name: "pdf-inspector",
  async run(ctx) {
    const { page } = ctx;
    await openFileBrowser(page);
    await ctx.shot("file-browser");

    const cases = [
      { file: "doc.md", pdf: "doc.pdf", orientation: "portrait", minPages: 2 },
      { file: "deck-169.md", pdf: "deck-169.pdf", orientation: "landscape", pages: 3 },
      { file: "deck-43.md", pdf: "deck-43.pdf", orientation: "landscape", pages: 3 },
    ];
    const details = {};
    for (const c of cases) {
      const target = join(ctx.downloadDir, c.pdf);
      if (existsSync(target)) rmSync(target);

      await selectTreeFile(page, c.file);
      await clickExportToPdf(page);
      const bytes = await ctx.pollFile(target, 90_000);
      await ctx.shot(`exported-${c.file}`);

      if (c.pages !== undefined) {
        details[c.file] = await ctx.assertPdf(bytes, {
          pages: c.pages,
          orientation: c.orientation,
        });
      } else {
        // Documents paginate by content height; pin a floor, not an
        // exact count, so copy tweaks don't flake the smoke.
        const { PDFDocument } = await import("pdf-lib");
        const count = (await PDFDocument.load(bytes)).getPageCount();
        if (count < c.minPages) {
          throw new Error(`${c.pdf}: expected >=${c.minPages} pages, got ${count}`);
        }
        details[c.file] = await ctx.assertPdf(bytes, {
          pages: count,
          orientation: c.orientation,
        });
      }
    }
    return details;
  },
};
