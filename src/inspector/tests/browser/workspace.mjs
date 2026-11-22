import {installedWorkspace} from "./installed-harness.mjs";
import test from "node:test";
import assert from "node:assert/strict";
import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { createInterface } from "node:readline";
const root = resolve(import.meta.dirname, "../../../..");
const { chromium } = await import(pathToFileURL(process.env.INSPECTOR_PLAYWRIGHT ?? "/home/coreyt/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs"));
async function workspace(action, fixture = "linked", flags = []) {
 if(process.env.INSPECTOR_QUALIFY_BINARY)return installedWorkspace(action,fixture,flags,true);
  const dir = await mkdtemp(`${tmpdir()}/inspector-workspace-`);
  let child, browser;
  try {
    const db = execFileSync(`${root}/target/debug/examples/fixture`, [dir, fixture], { encoding: "utf8" }).trim();
    child = spawn(`${root}/target/debug/examples/driver`, [db, `${root}/target/debug/fathomdb-inspector`, ...flags], { stdio: ["pipe", "pipe", "pipe"] });
    const lines = createInterface({ input: child.stdout });
    const iter = lines[Symbol.asyncIterator]();
    const info = JSON.parse((await iter.next()).value);
    browser = await chromium.launch({ executablePath: process.env.INSPECTOR_CHROMIUM ?? "/home/coreyt/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome", headless: true, args: ["--no-sandbox", "--disable-background-networking"] });
    const page = await browser.newPage();
    await page.goto(pathToFileURL(info.bootstrap).href);
    await page.waitForURL(`http://${info.address}/`);
    await page.getByRole("button", { name: "Attach database", exact: true }).click();
    await page.getByRole("heading", { name: "Database inventory", exact: true }).waitFor();
    await page.getByRole("button", { name: "Canonical nodes", exact: true }).click();
    await action(page, child, iter);
  } finally {
    if (browser) await browser.close();
    if (child) {
      child.stdin.end("close\n");
      await new Promise((r) => child.on("exit", r));
    }
    await rm(dir, { recursive: true, force: true });
  }
}
test("record workspace exposes scoped facts and an actionable exact materialization origin", { timeout: 30000 }, async () => {
  await workspace(async (page) => {
    await page.getByRole("button", { name: "Inspect derived", exact: true }).click();
    await page.getByRole("heading", { name: "Observed record facts", exact: true }).waitFor({ timeout: 1000 });
    const facts = await page.locator(".record-facts").textContent();
    assert.match(facts, /canonical_document/);
    assert.match(facts, /retrieval.*unknown/s);
    assert.match(facts, /inspector_session/);
    const attribute = page.getByRole("button", { name: "Open materialized attribute title", exact: true });
    await attribute.focus();
    await page.keyboard.press("Enter");
    await page.getByText("attribute", { exact: true }).waitFor();
    assert.equal(await page.locator("pre.content").textContent(), "derived-title");
    const origin = page.getByRole("button", { name: "Open verified canonical origin", exact: true });
    await origin.waitFor({ timeout: 1000 });
    await origin.focus();
    await page.keyboard.press("Space");
    await page.waitForFunction(() => document.querySelector("pre.content")?.textContent.includes("derived body"));
    await page.getByText("derived-r1", { exact: true }).waitFor();
    assert.match(await page.locator("pre.content").textContent(), /derived body/);
  });
});

test("Back restores the original tree and association observations with initiating keyboard focus", { timeout: 30000 }, async () => {
  await workspace(async (page) => {
    let associationReads = 0, treeReads = 0;
    page.on("request", (request) => {
      if (!request.url().endsWith("/api")) return;
      const op = request.postDataJSON().request.op;
      if (op === "associations") associationReads++;
      if (op === "tree") treeReads++;
    });
    await page.getByRole("button", { name: "Inspect structured", exact: true }).click();
    const expand = page.getByRole("button", { name: "Expand nested", exact: true });
    await expand.click();
    await page.getByRole("button", { name: "Collapse nested", exact: true }).waitFor();
    const associations = page.getByRole("region", { name: "Recorded associations", exact: true });
    await associations.locator("pre.scope").waitFor();
    const originalAssociation = await associations.locator("pre.scope").textContent();
    const originalReads = { associationReads, treeReads };
    const fullField = page.getByRole("button", { name: "Open full logical_id", exact: true });
    await fullField.focus();
    await page.keyboard.press("Enter");
    await page.getByRole("heading", { name: "Full field: logical_id", exact: true }).waitFor();
    await page.getByRole("button", { name: "Back", exact: true }).click();
    await page.getByRole("heading", { name: "Record detail", exact: true }).waitFor();
    await associations.locator("pre.scope").waitFor();
    assert.equal(associationReads, originalReads.associationReads, "Back must not hydrate a new association snapshot");
    assert.equal(treeReads, originalReads.treeReads, "Back must retain expanded tree observations");
    assert.equal(await associations.locator("pre.scope").textContent(), originalAssociation);
    assert.equal(await page.getByRole("button", { name: "Collapse nested", exact: true }).count(), 1);
    assert.equal(await page.evaluate(() => document.activeElement?.textContent), "Open full logical_id");
  });
});

test("hydrated chunks retain their separate read context and Back position", { timeout: 30000 }, async () => {
  await workspace(async (page) => {
    await page.getByRole("button", { name: "Inspect chunked", exact: true }).click();
    await page.getByRole("button", { name: "Next content chunk", exact: true }).click();
    await page.waitForFunction(() => document.querySelector("pre.content")?.textContent === "B".repeat(65536));
    const context = page.locator("pre.chunk-context");
    await context.waitFor({ timeout: 1000 });
    const original = await context.textContent();
    assert.equal(JSON.parse(original).snapshot, "this_request_only");
    const detailScope = JSON.parse(await page.locator(".record-workspace > pre.scope").textContent());
    assert.notEqual(JSON.parse(original).operation, detailScope.observation.operation);
    await page.getByRole("button", { name: "Open full logical_id", exact: true }).click();
    await page.getByRole("heading", { name: "Full field: logical_id", exact: true }).waitFor();
    await page.getByRole("button", { name: "Back", exact: true }).click();
    await page.getByRole("heading", { name: "Record detail", exact: true }).waitFor();
    assert.equal(await page.locator("pre.content").textContent(), "B".repeat(65536));
    assert.equal(await context.textContent(), original);
  }, "chunked");
});

test("a superseded real detail reply cannot replace a newer record selection", { timeout: 30000 }, async () => {
  await workspace(async (page) => {
    let release, captured, first = true;
    const released = new Promise((r) => { release = r; });
    const held = new Promise((r) => { captured = r; });
    try {
      await page.route("**/api", async (route) => {
        if (route.request().postDataJSON().request.op !== "detail" || !first) return route.continue();
        first = false;
        const response = await route.fetch();
        assert.match((await response.json()).content.text, /derived body/);
        captured();
        await released;
        await route.fulfill({ response });
      });
      await page.getByRole("button", { name: "Inspect derived", exact: true }).click();
      await held;
      await page.getByRole("button", { name: "Inspect source", exact: true }).click();
      await page.waitForFunction(() => document.querySelector("pre.content")?.textContent.includes("source body"));
      const delivered = page.waitForResponse((response) => response.url().endsWith("/api") && response.request().postDataJSON().request.op === "detail");
      release();
      await delivered;
      await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
      assert.match(await page.locator("pre.content").textContent(), /source body/, "late selection must not replace the current exact record");
      assert.doesNotMatch(await page.locator("pre.content").textContent(), /derived body/);
    } finally { release?.(); }
  });
});

test("a failed superseded policy check cannot invalidate the newer exact workspace", { timeout: 30000 }, async () => {
  await workspace(async (page) => {
    let release, captured, first = true;
    const released = new Promise((r) => { release = r; });
    const held = new Promise((r) => { captured = r; });
    try {
      await page.getByRole("button", { name: "Inspect derived", exact: true }).click();
      const source = page.getByRole("button", { name: "Open recorded source source", exact: true });
      await source.waitFor();
      await page.route("**/api", async (route) => {
        if (route.request().postDataJSON().request.op !== "check" || !first) return route.continue();
        first = false;
        const response = await route.fetch();
        assert.equal((await response.json()).status, "complete");
        captured();
        await released;
        await route.abort("failed");
      });
      await held;
      await source.click();
      await page.waitForFunction(() => document.querySelector("pre.content")?.textContent.includes("source body"));
      const failed = page.waitForEvent("requestfailed", (request) => request.url().endsWith("/api") && request.postDataJSON().request.op === "check");
      release();
      await failed;
      await page.evaluate(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))));
      assert.equal(await page.locator("pre.content").count(), 1, "failed old check must not clear the newly selected workspace");
      assert.match(await page.locator("pre.content").textContent(), /source body/);
    } finally { release?.(); }
  });
});
