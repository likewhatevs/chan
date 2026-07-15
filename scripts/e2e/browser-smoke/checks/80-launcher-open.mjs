// Item 8: the command-launcher Open, driven like a user would.
//
// Dialog flow: Ctrl+Alt+K raises the launcher, "Open" + Enter (bare pick)
// pops the PathPromptModal in open mode, typing a seeded file and Enter
// rides POST /api/open -> open_file window command -> an editor tab.
// Inline-arg flow: "Open <dir>" typed straight into the launcher opens the
// file browser (open_browser). Error flow: "Open <binary>" lands the
// server's refusal in the status pill ("open failed: cannot open binary
// file ..."), persistent until dismissed.

import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

async function raiseLauncher(page) {
  await page.keyboard.down("Control");
  await page.keyboard.down("Alt");
  await page.keyboard.press("KeyK");
  await page.keyboard.up("Alt");
  await page.keyboard.up("Control");
  await page.waitForSelector(".launcher .search", { timeout: 10_000 });
}

async function launcherRun(page, query) {
  await raiseLauncher(page);
  await page.type(".launcher .search", query);
  // The top Results row is auto-highlighted; wait for it, then Enter.
  await page.waitForSelector(".launcher .results .row", { timeout: 10_000 });
  await page.keyboard.press("Enter");
}

export default {
  name: "launcher-open",
  async run(ctx) {
    const { page } = ctx;
    await page.bringToFront();

    // ---- dialog flow: bare Open -> modal -> seeded file -> editor tab ----
    await launcherRun(page, "Open");
    await page.waitForSelector(".modal input", { timeout: 10_000 });
    await ctx.shot("open-dialog");
    await page.type(".modal input", "doc.md");
    // The open-mode status row discloses the action before submit.
    await page.waitForFunction(
      () =>
        document
          .querySelector(".modal .status")
          ?.textContent?.includes("opens doc.md"),
      { timeout: 10_000 },
    );
    await page.keyboard.press("Enter");
    await page.waitForFunction(
      () =>
        [...document.querySelectorAll(".tab")].some((t) =>
          t.textContent?.includes("doc.md"),
        ),
      { timeout: 15_000 },
    );
    await ctx.shot("opened-file");

    // ---- inline-arg flow: "Open <dir>" opens the file browser ----
    const dir = "smoke-open-dir";
    mkdirSync(join(ctx.workspaceDir, dir), { recursive: true });
    writeFileSync(join(ctx.workspaceDir, dir, "inner.md"), "inner\n");
    await launcherRun(page, `Open ${dir}`);
    await page.waitForFunction(
      (d) =>
        document
          .querySelector(".status-msg")
          ?.textContent?.includes(`opened ${d}`),
      { timeout: 15_000 },
      dir,
    );
    await ctx.shot("opened-dir");

    // ---- error flow: a binary target lands in the status pill ----
    await launcherRun(page, "Open photo.png");
    await page.waitForFunction(
      () =>
        document
          .querySelector(".status-msg")
          ?.textContent?.includes("open failed: cannot open binary file photo.png"),
      { timeout: 15_000 },
    );
    await ctx.shot("binary-error");
    return null;
  },
};
