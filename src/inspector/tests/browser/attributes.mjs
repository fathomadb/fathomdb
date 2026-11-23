import test from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { createInterface } from "node:readline";
import { installedWorkspace } from "./installed-harness.mjs";
import {
  boundedExecFileSync,
  clipboardEquals,
  closeBrowserBounded,
  stopChildBounded,
} from "./helpers.mjs";

const root = resolve(import.meta.dirname, "../../../..");
const { chromium } = await import(
  pathToFileURL(
    process.env.INSPECTOR_PLAYWRIGHT ??
      "/home/coreyt/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs",
  )
);

async function workspace(action, fixture = "linked", flags = []) {
  if (process.env.INSPECTOR_QUALIFY_BINARY) {
    return installedWorkspace(action, fixture, flags, true);
  }
  const dir = await mkdtemp(`${tmpdir()}/inspector-attributes-`);
  let child, browser;
  try {
    const db = boundedExecFileSync(
      `${root}/target/debug/examples/fixture`,
      [dir, fixture],
      { encoding: "utf8" },
      120000,
    ).trim();
    child = spawn(
      `${root}/target/debug/examples/driver`,
      [db, `${root}/target/debug/fathomdb-inspector`, ...flags],
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
    const context = await browser.newContext();
    const page = await context.newPage();
    await page.goto(pathToFileURL(info.bootstrap).href);
    await page.waitForURL(`http://${info.address}/`);
    await page.getByRole("button", { name: "Attach database", exact: true }).click();
    await page.getByRole("heading", { name: "Database inventory", exact: true }).waitFor();
    await page.getByRole("button", { name: "Canonical nodes", exact: true }).click();
    await action(page, child, iter, { db, context });
  } finally {
    await closeBrowserBounded(browser, 5000);
    await stopChildBounded(child);
    await rm(dir, { recursive: true, force: true });
  }
}

test(
  "materialized attribute opens its own detail and returns to exact canonical origin",
  { timeout: 30000 },
  async () => {
    await workspace(async (page) => {
      await page.getByRole("button", { name: "Inspect derived", exact: true }).click();
      const attribute = page.getByRole("button", {
        name: "Open materialized attribute title",
        exact: true,
      });
      await attribute.waitFor({ timeout: 2000 });
      await attribute.focus();
      await page.keyboard.press("Enter");
      await page.getByText("attribute", { exact: true }).waitFor();
      assert.equal(await page.locator("pre.content").textContent(), "derived-title");
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page.waitForFunction(
        () => document.querySelector("pre.content")?.textContent.includes("derived body"),
        null,
        { timeout: 5000 },
      );
      await page.getByText("derived-r1", { exact: true }).waitFor();
      assert.match(await page.locator("pre.content").textContent(), /derived body/);
    }, "linked");
  },
);

test(
  "outside-body node and edge properties inspect, render scope, property values, and block XSS",
  { timeout: 45000 },
  async () => {
    await workspace(async (page, _child, _iter, meta) => {
      const pageErrors = [];
      const dialogs = [];
      page.on("pageerror", (err) => pageErrors.push(String(err)));
      page.on("dialog", async (dialog) => {
        dialogs.push(dialog.message());
        await dialog.dismiss();
      });
      await meta.context.grantPermissions(["clipboard-read", "clipboard-write"], {
        origin: new URL(page.url()).origin,
      });

      await page.getByRole("button", { name: "Inspect prop-node", exact: true }).click();
      await page.getByRole("heading", { name: "Record detail", exact: true }).waitFor();
      assert.equal(await page.locator("pre.content").textContent(), "plain text body without json");
      assert.match(await page.locator(".record-scope").textContent(), /Scope:\s*""/);

      const propsRegion = page.getByRole("region", { name: "Properties", exact: true });
      await propsRegion.waitFor();
      assert.match(
        await propsRegion.locator(".properties-meta").textContent(),
        /storage type:\s*text/i,
      );
      await propsRegion.getByText("9007199254740993", { exact: true }).waitFor();
      await propsRegion.getByRole("button", { name: "Copy value big_i64", exact: true }).click();
      await clipboardEquals(page, "9007199254740993");
      await propsRegion.getByRole("button", { name: "Copy value xss_img", exact: true }).click();
      await clipboardEquals(page, "<img src=x onerror=alert(1)>");

      const associations = page.getByRole("region", { name: "Recorded associations", exact: true });
      await associations.getByText(/topic \(string · node\)/).waitFor();
      assert.match(await associations.textContent(), /botany/);

      await page.getByRole("button", { name: "Canonical nodes", exact: true }).click();
      await page.getByRole("button", { name: "Inspect null-props", exact: true }).click();
      await page.getByRole("heading", { name: "Record detail", exact: true }).waitFor();
      const nullPropsRegion = page.getByRole("region", { name: "Properties", exact: true });
      await nullPropsRegion.getByText("null", { exact: true }).waitFor();

      await page.getByRole("button", { name: "Canonical edges", exact: true }).click();
      await page.getByRole("button", { name: "Inspect prop-edge", exact: true }).click();
      await page.getByRole("heading", { name: "Record detail", exact: true }).waitFor();
      assert.equal(await page.locator("pre.content").textContent(), "plain edge text body");
      const edgePropsRegion = page.getByRole("region", { name: "Properties", exact: true });
      await edgePropsRegion.getByText("9007199254740993", { exact: true }).waitFor();
      await edgePropsRegion.getByRole("button", { name: "Copy value edge_big", exact: true }).click();
      await clipboardEquals(page, "9007199254740993");
      const edgeAssociations = page.getByRole("region", {
        name: "Recorded associations",
        exact: true,
      });
      await edgeAssociations.getByText(/weight_tag \(string · edge\)/).waitFor();
      assert.match(await edgeAssociations.textContent(), /heavy/);

      assert.equal(await page.locator("img").count(), 0, "hostile img must not inject DOM");
      assert.equal(await page.locator("svg").count(), 0, "hostile svg must not inject DOM");
      assert.equal(
        await page.evaluate(() => document.querySelectorAll("script").length),
        1,
        "only the app module script may exist",
      );
      assert.equal(await page.evaluate(() => window.__xss), undefined);
      assert.deepEqual(dialogs, [], "hostile properties must trigger zero alerts");
      assert.deepEqual(pageErrors, [], "rendering hostile properties must trigger zero page errors");
    }, "properties");
  },
);

test(
  "properties tree threads field through recursive Expand, Open full properties, and exact 9007199254740993123456789 clipboard copy",
  { timeout: 45000 },
  async () => {
    await workspace(async (page, _child, _iter, meta) => {
      const pageErrors = [];
      const dialogs = [];
      page.on("pageerror", (err) => pageErrors.push(String(err)));
      page.on("dialog", async (dialog) => {
        dialogs.push(dialog.message());
        await dialog.dismiss();
      });
      await meta.context.grantPermissions(["clipboard-read", "clipboard-write"], {
        origin: new URL(page.url()).origin,
      });

      await page.getByRole("button", { name: "Inspect prop-raw-node", exact: true }).click();
      await page.getByRole("heading", { name: "Record detail", exact: true }).waitFor();
      assert.equal(await page.locator("pre.content").textContent(), "plain text body without json");

      const propsRegion = page.getByRole("region", { name: "Properties", exact: true });
      await propsRegion.waitFor();
      await propsRegion.getByText("9007199254740993123456789", { exact: true }).waitFor();
      await propsRegion.getByRole("button", { name: "Copy value huge", exact: true }).click();
      await clipboardEquals(page, "9007199254740993123456789");

      await propsRegion.getByRole("button", { name: "Expand nested", exact: true }).click();
      await propsRegion.getByText('"<img src=x onerror=alert(1)>"', { exact: true }).waitFor();
      await propsRegion.getByRole("button", { name: "Copy value img", exact: true }).click();
      await clipboardEquals(page, "<img src=x onerror=alert(1)>");

      const expectedNodeProps =
        '{"huge":9007199254740993123456789,"nested":{"nil":null,"empty":"","img":"<img src=x onerror=alert(1)>"},"script":"<script>window.__xss=1</script>","\\"><svg onload=alert(1)>":"hostile-key-val"}';
      await propsRegion.getByRole("button", { name: "Copy full properties", exact: true }).click();
      await clipboardEquals(page, expectedNodeProps);

      await propsRegion.getByRole("button", { name: "Open full properties", exact: true }).click();
      await page.getByRole("heading", { name: "Full field: properties", exact: true }).waitFor();
      assert.equal(await page.locator("pre.content").textContent(), expectedNodeProps);
      await page.getByRole("button", { name: "Copy value huge", exact: true }).click();
      await clipboardEquals(page, "9007199254740993123456789");
      await page.getByRole("button", { name: "Back", exact: true }).click();
      await page.getByRole("heading", { name: "Record detail", exact: true }).waitFor();

      await page.getByRole("button", { name: "Canonical edges", exact: true }).click();
      await page.getByRole("button", { name: "Inspect prop-raw-edge", exact: true }).click();
      await page.getByRole("heading", { name: "Record detail", exact: true }).waitFor();
      const edgePropsRegion = page.getByRole("region", { name: "Properties", exact: true });
      await edgePropsRegion.getByText("9007199254740993123456789", { exact: true }).waitFor();
      await edgePropsRegion.getByRole("button", { name: "Copy value edge_huge", exact: true }).click();
      await clipboardEquals(page, "9007199254740993123456789");

      assert.equal(await page.locator("img").count(), 0);
      assert.equal(await page.locator("svg").count(), 0);
      assert.equal(await page.evaluate(() => window.__xss), undefined);
      assert.deepEqual(dialogs, []);
      assert.deepEqual(pageErrors, []);
    }, "properties-raw");
  },
);
