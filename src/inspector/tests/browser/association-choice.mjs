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
  "equal logical association targets expose exact revision descriptions for keyboard selection",
  { timeout: 30000 },
  async () => {
    const dir = await mkdtemp(`${tmpdir()}/inspector-related-`);
    let child, browser;
    try {
      const db = execFileSync(`${root}/target/debug/examples/fixture`, [dir, "many-linked"], {
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
      await page.getByRole("button", { name: "Inspect source", exact: true }).click();
      const targets = page.getByRole("button", {
        name: "Open recorded derived derived",
        exact: true,
      });
      await targets.nth(1).waitFor({ timeout: 1000 });
      assert.equal(await targets.count(), 2);
      async function targetFor(revision) {
        for (const button of await targets.all()) {
          const id = await button.getAttribute("aria-describedby");
          assert.ok(id, "every equal-name target must have its own exact reference description");
          const description = await page.locator(`#${id}`).textContent();
          assert.match(description, /node.*internal write cursor \d+/);
          if (description.includes(`recorded revision ${revision}`)) return button;
        }
        assert.fail(`missing exact target ${revision}`);
      }
      const old = await targetFor("derived-r1"),
        newer = await targetFor("derived-r2");
      assert.notEqual(
        await old.getAttribute("aria-describedby"),
        await newer.getAttribute("aria-describedby"),
      );
      await old.focus();
      await page.keyboard.press("Enter");
      await page.waitForFunction(
        () =>
          document.querySelector("pre.content")?.textContent ===
          '{"title":"derived-title","text":"derived body"}',
      );
      await page.getByText("derived-r1", { exact: true }).waitFor();
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page.waitForFunction(() =>
        document.querySelector("pre.content")?.textContent.includes("source body"),
      );
      await targets.nth(1).waitFor();
      const exactNew = await targetFor("derived-r2");
      await exactNew.focus();
      await page.keyboard.press("Enter");
      await page.waitForFunction(
        () =>
          document.querySelector("pre.content")?.textContent ===
          '{"title":"new-derived-title","text":"second derived body"}',
      );
      await page.getByText("derived-r2", { exact: true }).waitFor();
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
