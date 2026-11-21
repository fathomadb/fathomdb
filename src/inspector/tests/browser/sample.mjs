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
  "observed-field sample shows actual scoped fields and opens exact documents with Back",
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
        [db, `${root}/target/debug/fathomdb-inspector`],
        { stdio: ["pipe", "pipe", "pipe"] },
      );
      const lines = createInterface({ input: child.stdout });
      const info = JSON.parse((await lines[Symbol.asyncIterator]().next()).value);
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
      const sample = page.getByRole("button", { name: "Sample observed fields", exact: true });
      await sample.waitFor({ timeout: 1000 });
      await sample.focus();
      await page.keyboard.press("Enter");
      await page
        .getByRole("heading", { name: "node observed-field sample", exact: true })
        .waitFor();
      const fields = page.getByRole("region", { name: "Observed fields for derived", exact: true });
      assert.match(await fields.textContent(), /title/);
      assert.match(await fields.textContent(), /text/);
      assert.match(
        await page.locator("section.panel > pre.scope").textContent(),
        /"absence_proof": false/,
      );
      await page.getByRole("button", { name: "Inspect derived", exact: true }).focus();
      await page.keyboard.press("Enter");
      await page.getByText("derived-r1", { exact: true }).waitFor();
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page
        .getByRole("heading", { name: "node observed-field sample", exact: true })
        .waitFor();
      assert.match(await fields.textContent(), /title/);
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
