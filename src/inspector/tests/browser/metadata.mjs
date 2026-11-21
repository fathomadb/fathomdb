import { clipboardEquals } from "./helpers.mjs";
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
const executablePath =
  process.env.INSPECTOR_CHROMIUM ??
  "/home/coreyt/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome";
test(
  "long metadata previews open full fields with exact copy and preserve selection through Back and refresh",
  { timeout: 45000 },
  async () => {
    const dir = await mkdtemp(`${tmpdir()}/inspector-browser-`);
    let child, browser;
    try {
      const db = execFileSync(`${root}/target/debug/examples/fixture`, [dir, "metadata"], {
        encoding: "utf8",
      }).trim();
      child = spawn(
        `${root}/target/debug/examples/driver`,
        [db, `${root}/target/debug/fathomdb-inspector`],
        { stdio: ["pipe", "pipe", "pipe"] },
      );
      const lines = createInterface({ input: child.stdout });
      const info = JSON.parse((await lines[Symbol.asyncIterator]().next()).value);
      const base = `http://${info.address}`;
      browser = await chromium.launch({
        executablePath,
        headless: true,
        env: { LANG: "C.UTF-8", LC_ALL: "C.UTF-8", TMPDIR: dir },
        args: ["--no-sandbox", "--disable-background-networking", "--no-proxy-server"],
      });
      const context = await browser.newContext();
      await context.grantPermissions(["clipboard-read", "clipboard-write"], { origin: base });
      const page = await context.newPage();
      const outside = [];
      page.on("request", (r) => {
        if (!r.url().startsWith(base) && !r.url().startsWith("file:")) outside.push(r.url());
      });
      await page.goto(pathToFileURL(info.bootstrap).href);
      await page.waitForURL(`${base}/`, { timeout: 10000 });
      const attach = page.getByRole("button", { name: "Attach database", exact: true });
      await attach.waitFor({ timeout: 1000 });
      await attach.focus();
      await page.keyboard.press("Enter");
      await page.getByRole("heading", { name: "Database inventory", exact: true }).waitFor();
      await page.getByRole("button", { name: "Canonical nodes", exact: true }).click();
      await page.getByRole("button", { name: "Inspect metadata", exact: true }).click();
      await page.getByRole("heading", { name: "Record detail", exact: true }).waitFor();
      assert.match(await page.locator("dl").first().textContent(), /Preview/);
      await page
        .getByText("Revision evidence: recorded", { exact: true })
        .waitFor({ timeout: 1000 });
      const open = page.getByRole("button", { name: "Open full reason", exact: true });
      await open.waitFor({ timeout: 1000 });
      await open.focus();
      await page.keyboard.press("Enter");
      await page
        .getByRole("heading", { name: "Full field: reason", exact: true })
        .waitFor({ timeout: 1000 });
      const expected = "retained reason é🦀".repeat(20000);
      assert.ok(expected.startsWith(await page.locator("pre.content").textContent()));
      await page.getByRole("button", { name: "Copy content", exact: true }).click();
      await clipboardEquals(page, expected);
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page.waitForFunction(
        () => document.querySelector("pre.content")?.textContent === "metadata fixture body",
      );
      await page.getByRole("button", { name: "Open full reason", exact: true }).click();
      await page.getByRole("heading", { name: "Full field: reason", exact: true }).waitFor();
      for (let n = 0; n < 15; n++) {
        const copy = page.getByRole("button", { name: "Copy exact identity", exact: true });
        if (!(await copy.count())) break;
        const response = page.waitForResponse(
          (r) => r.url().endsWith("/api") && r.request().postDataJSON()?.request?.op === "chunk",
        );
        await copy.click();
        await response;
        await page.waitForFunction(
          () => document.getElementById("app").getAttribute("aria-busy") === "false",
        );
      }
      await page
        .getByRole("button", { name: "Refresh this view", exact: true })
        .waitFor({ timeout: 1000 });
      await page.getByRole("button", { name: "Refresh this view", exact: true }).click();
      await page.getByRole("heading", { name: "Full field: reason", exact: true }).waitFor();
      assert.ok(expected.startsWith(await page.locator("pre.content").textContent()));
      assert.notEqual(await page.locator("pre.content").textContent(), "metadata fixture body");
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
