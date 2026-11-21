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
  "operational copies distinguish exact record keys and retained entry IDs",
  { timeout: 45000 },
  async () => {
    const dir = await mkdtemp(`${tmpdir()}/inspector-browser-`);
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
      await page.getByRole("button", { name: "Operational current values", exact: true }).click();
      await page
        .getByRole("button", { name: "Inspect zz-current / shared-key", exact: true })
        .click();
      await page.waitForFunction(
        () => document.querySelector("pre.content")?.textContent === '{"n":2}',
      );
      const key = page.getByRole("button", { name: "Copy exact record key", exact: true });
      await key.waitFor({ timeout: 1000 });
      await key.click();
      await clipboardEquals(page, "shared-key");
      await page
        .getByText(
          "Canonical source and attribute links are not applicable to this operational record.",
          { exact: true },
        )
        .waitFor();
      await page.getByRole("button", { name: "Retained mutation entries", exact: true }).click();
      await page
        .getByRole("button", { name: /^Inspect zz-events \/ shared-key \/ entry \d+$/ })
        .nth(0)
        .click();
      await page.waitForFunction(
        () => document.querySelector("pre.content")?.textContent === '{"n":1}',
      );
      await page.getByRole("button", { name: "Copy exact entry ID", exact: true }).click();
      await clipboardEquals(page, "1");
      await page.getByRole("button", { name: "Copy full entry_id", exact: true }).click();
      await clipboardEquals(page, "1");
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page.getByRole("heading", { name: "log records", exact: true }).waitFor();
      await page
        .getByRole("button", { name: /^Inspect zz-events \/ shared-key \/ entry \d+$/ })
        .nth(1)
        .click();
      await page.waitForFunction(
        () => document.querySelector("pre.content")?.textContent === '{"n":2}',
      );
      await page.getByRole("button", { name: "Copy exact entry ID", exact: true }).click();
      await clipboardEquals(page, "2");
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
