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
  "partial catalog keeps truthful metadata and opens populated operational categories",
  { timeout: 30000 },
  async () => {
    const dir = await mkdtemp(`${tmpdir()}/inspector-related-`);
    let child, browser;
    try {
      const db = execFileSync(`${root}/target/debug/examples/fixture`, [dir, "catalog-caps"], {
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
      await page
        .getByText(
          "Projection declarations were not fully enumerated; this response does not prove none are configured.",
          { exact: true },
        )
        .waitFor({ timeout: 1000 });
      assert.equal(
        await page.getByText("No projection declarations configured.", { exact: true }).count(),
        0,
      );
      const truncated = page.getByRole("button", { name: /^Collection 0000-/ });
      assert.equal(await truncated.count(), 2);
      for (const button of await truncated.all()) assert.equal(await button.isDisabled(), true);
      const known = page.getByRole("button", {
        name: "Collection catalog-state-000 (latest_state)",
        exact: true,
      });
      assert.equal(await known.count(), 2);
      for (const button of await known.all()) assert.equal(await button.isEnabled(), true);
      const current = page.getByRole("button", { name: "Operational current values", exact: true });
      await current.waitFor({ timeout: 1000 });
      await current.focus();
      await page.keyboard.press("Enter");
      await page.getByRole("heading", { name: "current records", exact: true }).waitFor();
      assert.equal(
        await page
          .getByRole("button", { name: "Inspect zz-current / shared-key", exact: true })
          .count(),
        1,
      );
      await page
        .getByRole("button", { name: "Inspect zz-current / shared-key", exact: true })
        .click();
      await page.waitForFunction(
        () => document.querySelector("pre.content")?.textContent === '{"n":2}',
      );
      await page.getByText("zz-current", { exact: true }).waitFor();
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page.getByRole("heading", { name: "current records", exact: true }).waitFor();
      const logs = page.getByRole("button", { name: "Retained mutation entries", exact: true });
      await logs.focus();
      await page.keyboard.press("Enter");
      await page.getByRole("heading", { name: "log records", exact: true }).waitFor();
      const entries = page.getByRole("button", {
        name: /^Inspect zz-events \/ shared-key \/ entry \d+$/,
      });
      assert.equal(await entries.count(), 2);
      assert.equal(new Set(await entries.allTextContents()).size, 2);
      await entries.nth(0).click();
      await page.waitForFunction(
        () => document.querySelector("pre.content")?.textContent === '{"n":1}',
      );
      await page.getByText("zz-events", { exact: true }).waitFor();
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page.getByRole("heading", { name: "log records", exact: true }).waitFor();
      await page
        .getByRole("button", { name: /^Inspect zz-events \/ shared-key \/ entry \d+$/ })
        .nth(1)
        .click();
      await page.waitForFunction(
        () => document.querySelector("pre.content")?.textContent === '{"n":2}',
      );
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
