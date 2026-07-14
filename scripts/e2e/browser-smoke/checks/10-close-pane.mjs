// Item 5: the pane hamburger ends with a Close pane row that closes
// the pane. Split first so the close has a pane to remove, then drive
// the menu like a user would.

async function paneCount(page) {
  return page.evaluate(() => document.querySelectorAll(".pane").length);
}

export default {
  name: "close-pane-row",
  async run(ctx) {
    const { page } = ctx;
    const before = await paneCount(page);

    await page.evaluate(() => {
      window.dispatchEvent(
        new CustomEvent("chan:command", { detail: { name: "app.pane.splitRight" } }),
      );
    });
    await page.waitForFunction(
      (n) => document.querySelectorAll(".pane").length === n + 1,
      { timeout: 10_000 },
      before,
    );
    await ctx.shot("split");

    await page.click(".pane .hamburger-trigger");
    await page.waitForSelector(".hamburger-menu", { timeout: 5_000 });
    await ctx.shot("menu-open");

    const rows = await page.$$eval(".hamburger-menu li", (items) =>
      items.map((li) => ({
        sep: li.classList.contains("sep"),
        label: li.querySelector(".menu-row-label")?.textContent?.trim() ?? null,
      })),
    );
    const last = rows[rows.length - 1];
    if (last?.label !== "Close pane") {
      throw new Error(`last menu row is ${JSON.stringify(last)}, expected Close pane`);
    }
    if (!rows[rows.length - 2]?.sep) {
      throw new Error("Close pane row is not preceded by a separator");
    }

    const clicked = await page.evaluate(() => {
      const row = [...document.querySelectorAll(".hamburger-menu button")].find(
        (b) => b.querySelector(".menu-row-label")?.textContent?.trim() === "Close pane",
      );
      if (!row) return false;
      row.click();
      return true;
    });
    if (!clicked) throw new Error("Close pane row not clickable");

    await page.waitForFunction(
      (n) => document.querySelectorAll(".pane").length === n,
      { timeout: 10_000 },
      before,
    );
    await ctx.shot("closed");
    return { panesBefore: before, rows: rows.length };
  },
};
