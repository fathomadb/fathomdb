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
  "projection declaration opens verified scalar materializations with preserved scoped constraints",
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
      await page.getByRole("button", { name: "Projection title", exact: true }).click();
      const browse = page.getByRole("button", {
        name: "Browse verified attribute origins",
        exact: true,
      });
      await browse.waitFor({ timeout: 1000 });
      await browse.focus();
      await page.keyboard.press("Enter");
      await page.getByRole("heading", { name: "node records", exact: true }).waitFor();
      assert.equal(
        await page.getByRole("button", { name: "Inspect alpha", exact: true }).count(),
        0,
      );
      await page.getByRole("button", { name: "Inspect source", exact: true }).waitFor();
      assert.equal(
        await page.getByLabel("Attribute filter", { exact: true }).inputValue(),
        "title",
      );
      assert.match(
        await page.locator("#app").textContent(),
        /does not prove body-field absence or vector readiness/,
      );
      await page.getByLabel("Kind filter", { exact: true }).fill("document");
      await page.getByRole("button", { name: "Apply filters", exact: true }).click();
      await page.getByRole("button", { name: "Inspect derived", exact: true }).waitFor();
      assert.equal(
        await page.getByLabel("Attribute filter", { exact: true }).inputValue(),
        "title",
      );
      assert.equal(
        await page.getByRole("button", { name: "Inspect alpha", exact: true }).count(),
        0,
      );
      await page.getByRole("button", { name: "Inspect derived", exact: true }).click();
      await page
        .getByRole("button", { name: "Open materialized attribute title", exact: true })
        .click();
      await page.waitForFunction(
        () => document.querySelector("pre.content")?.textContent === "derived-title",
      );
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page.waitForFunction(() =>
        document.querySelector("pre.content")?.textContent.includes("derived body"),
      );
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page.getByRole("heading", { name: "node records", exact: true }).waitFor();
      assert.equal(
        await page.getByLabel("Attribute filter", { exact: true }).inputValue(),
        "title",
      );
      await page.getByRole("button", { name: "Sample observed fields", exact: true }).click();
      await page
        .getByRole("heading", { name: "node observed-field sample", exact: true })
        .waitFor();
      assert.equal(
        await page.getByRole("button", { name: "Inspect alpha", exact: true }).count(),
        0,
      );
      assert.equal(
        await page.getByLabel("Attribute filter", { exact: true }).inputValue(),
        "title",
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
