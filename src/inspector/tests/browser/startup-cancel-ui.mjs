import { clipboardEquals } from "./helpers.mjs";
import test from "node:test";
import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
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
  "keyboard cancellation remains available before attachment readiness",
  { timeout: 45000 },
  async () => {
    const dir = await mkdtemp(`${tmpdir()}/inspector-browser-`);
    let child, browser;
    try {
      const db = execFileSync(`${root}/target/debug/examples/fixture`, [dir], {
        encoding: "utf8",
      }).trim();
      child = spawn(
        `${root}/target/debug/examples/driver`,
        [db, `${root}/target/debug/examples/blocked-startup`],
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
      const cancel = page.getByRole("button", { name: "Cancel request", exact: true });
      await cancel.waitFor({ timeout: 1000 });
      const reply = page.waitForResponse(
        (r) => r.url().endsWith("/api") && r.request().postDataJSON()?.request?.op === "cancel",
      );
      const began = performance.now();
      await cancel.focus();
      await page.keyboard.press("Enter");
      const result = await (await reply).json();
      assert.equal(result.status, "cancelled");
      assert.ok(performance.now() - began <= 250);
      await page
        .getByRole("button", { name: "Attach database", exact: true })
        .waitFor({ timeout: 1000 });
      assert.equal(await page.locator("pre.content").count(), 0);
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
