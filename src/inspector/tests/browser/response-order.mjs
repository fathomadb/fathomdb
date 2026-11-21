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
  "late real detail response cannot restore content after newer visibility invalidation",
  { timeout: 30000 },
  async () => {
    const dir = await mkdtemp(`${tmpdir()}/inspector-related-`);
    let child, browser, release;
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
      const released = new Promise((resolve) => {
        release = resolve;
      });
      let captured;
      const held = new Promise((resolve) => {
        captured = resolve;
      });
      await page.route("**/api", async (route) => {
        const request = route.request().postDataJSON().request;
        if (request.op !== "detail") return route.continue();
        const response = await route.fetch();
        const actual = await response.json();
        assert.equal(actual.status, "complete");
        assert.match(actual.content.text, /derived body/);
        captured();
        await released;
        await route.fulfill({ response });
      });
      await page.getByRole("button", { name: "Inspect derived", exact: true }).click();
      await held;
      child.stdin.write(
        JSON.stringify({ control: "transition", identity: "derived", deleted: true }) + "\n",
      );
      assert.equal(JSON.parse((await iter.next()).value).status, "complete");
      await page.getByRole("button", { name: "Canonical nodes", exact: true }).click();
      await page
        .getByText("Database observation changed. Cached values cleared; refresh explicitly.", {
          exact: true,
        })
        .waitFor();
      assert.equal(
        await page.getByRole("button", { name: "Inspect derived", exact: true }).count(),
        0,
      );
      await page.evaluate(() => {
        window.latePayloadSeen = false;
        new MutationObserver(() => {
          if (document.getElementById("app").textContent.includes("derived body"))
            window.latePayloadSeen = true;
        }).observe(document.getElementById("app"), {
          subtree: true,
          childList: true,
          characterData: true,
        });
      });
      const delivered = page.waitForResponse(
        (response) =>
          response.url().endsWith("/api") &&
          response.request().postDataJSON().request.op === "detail",
      );
      release();
      await delivered;
      await page.waitForFunction(
        () => document.getElementById("app").getAttribute("aria-busy") === "false",
      );
      assert.equal(
        await page.evaluate(() => window.latePayloadSeen),
        false,
        "no invalidated body may reappear even transiently after a late response",
      );
      assert.doesNotMatch(
        await page.locator("#app").textContent(),
        /derived body/,
        "an older real response must remain discarded after newer visibility invalidation",
      );
    } finally {
      release?.();
      if (browser) await browser.close();
      if (child) {
        child.stdin.end("close\n");
        await new Promise((r) => child.on("exit", r));
      }
      await rm(dir, { recursive: true, force: true });
    }
  },
);
