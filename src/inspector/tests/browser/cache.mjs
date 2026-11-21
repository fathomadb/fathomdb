import test from "node:test";
import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { createInterface } from "node:readline";
const root = resolve(import.meta.dirname, "../../../..");
const { chromium } = await import(
  pathToFileURL(
    process.env.INSPECTOR_PLAYWRIGHT ??
      "/home/coreyt/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs",
  )
);
test(
  "Back revalidates cached content after same-owned-Engine lifecycle change",
  { timeout: 30000 },
  async () => {
    const dir = await mkdtemp(`${tmpdir()}/inspector-related-`);
    let child, browser;
    try {
      const db = execFileSync(`${root}/target/debug/examples/fixture`, [dir, "linked"], {
        encoding: "utf8",
      }).trim();
      child = spawn(
        `${root}/target/debug/examples/driver`,
        [db, `${root}/target/debug/fathomdb-inspector`, "--no-show-inactive"],
        { stdio: ["pipe", "pipe", "pipe"] },
      );
      const lines = createInterface({ input: child.stdout });
      const iter = lines[Symbol.asyncIterator]();
      const info = JSON.parse((await iter.next()).value);
      browser = await chromium.launch({
        executablePath:
          process.env.INSPECTOR_CHROMIUM ??
          "/home/coreyt/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome",
        headless: true,
        args: ["--no-sandbox", "--disable-background-networking"],
      });
      const page = await browser.newPage();
      await page.goto(pathToFileURL(info.bootstrap).href);
      await page.waitForURL(`http://${info.address}/`);
      await page.getByRole("button", { name: "Attach database", exact: true }).click();
      await page.getByRole("heading", { name: "Database inventory", exact: true }).waitFor();
      await page.getByRole("button", { name: "Canonical nodes", exact: true }).click();
      await page.getByRole("button", { name: "Inspect derived", exact: true }).click();
      const link = page.getByRole("button", { name: "Open recorded source source", exact: true });
      await link.waitFor({ timeout: 1000 });
      await link.focus();
      await page.keyboard.press("Enter");
      await page.getByText("source-r1", { exact: true }).waitFor();
      await page.getByText("canonical_source", { exact: true }).waitFor();
      child.stdin.write(
        JSON.stringify({ control: "transition", identity: "derived", deleted: true }) + "\n",
      );
      assert.equal(JSON.parse((await iter.next()).value).status, "complete");
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page.waitForFunction(
        () => !document.getElementById("app").textContent.includes("derived body"),
        null,
        { timeout: 1000 },
      );
      assert.doesNotMatch(await page.locator("#app").textContent(), /derived body/);
      assert.equal(
        await page.getByRole("button", { name: "Attach database", exact: true }).count(),
        0,
        "classification change does not expire the database identity",
      );
      await page.getByRole("button", { name: "Refresh this view", exact: true }).waitFor();
    } finally {
      if (browser) await browser.close();
      if (child) {
        child.stdin.end("close\n");
        await new Promise((r) => child.on("exit", r));
      }
      await rm(dir, { recursive: true, force: true });
    }
  },
);
