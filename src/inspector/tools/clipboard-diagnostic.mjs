import test from "node:test";
import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { createInterface } from "node:readline";
const root = resolve(import.meta.dirname, "../../..");
const interval = Number(process.env.INSPECTOR_COPY_INTERVAL_MS ?? 100),
  samples = Number(process.env.INSPECTOR_COPY_SAMPLES ?? 10);
assert.ok(Number.isInteger(interval) && interval >= 1 && interval <= 1000);
assert.ok(Number.isInteger(samples) && samples >= 1 && samples <= 100);
if (interval * samples > 60000)
  assert.equal(process.env.AGENT_LONG, "1", "AGENT_LONG=1 required for a long diagnostic");
const screenshot = process.env.INSPECTOR_SCREENSHOT;
if (screenshot) assert.ok(screenshot.startsWith("/"), "screenshot path must be absolute");
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
  "owned fixture clipboard timing diagnostic",
  { timeout: 45000 + interval * samples },
  async () => {
    const dir = await mkdtemp(`${tmpdir()}/inspector-browser-`);
    let child, browser;
    try {
      const db = execFileSync(`${root}/target/debug/examples/fixture`, [dir], {
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
      const inspect = page.getByRole("button", { name: "Inspect structured", exact: true });
      await inspect.waitFor();
      await inspect.focus();
      await page.keyboard.press("Enter");
      await page.getByRole("heading", { name: "Record detail", exact: true }).waitFor();
      await page.getByText("9007199254740993123456789", { exact: true }).waitFor();
      await page.getByRole("button", { name: "Expand nested", exact: true }).focus();
      await page.keyboard.press("Enter");
      await page.getByText("null", { exact: true }).last().waitFor();
      const responses = [];
      page.on("response", async (r) => {
        if (r.url().endsWith("/api")) responses.push(await r.json());
      });
      await page.evaluate(() => navigator.clipboard.writeText("inspector-owned-copy-sentinel"));
      await page.getByRole("button", { name: "Copy exact identity", exact: true }).click();
      for (let n = 0; n < samples; n++) {
        await new Promise((r) => setTimeout(r, interval));
        console.log(
          JSON.stringify({
            n,
            clipboard: await page.evaluate(() => navigator.clipboard.readText()),
            notice: await page.locator("#notice").textContent(),
            responses: responses.map((r) => ({
              status: r.status,
              error: r.error,
              context: r.context?.visibility_generation,
            })),
          }),
        );
      }
      assert.equal(await page.evaluate(() => navigator.clipboard.readText()), "structured");
      if (screenshot) await page.screenshot({ path: screenshot, fullPage: true });
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
