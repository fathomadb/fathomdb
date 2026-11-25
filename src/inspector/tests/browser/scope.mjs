import { installedWorkspace } from "./installed-harness.mjs";
import { boundedExecFileSync, closeBrowserBounded, stopChildBounded } from "./helpers.mjs";
import test from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
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
  ),
);

async function workspace(action, fixture = "scoped", flags = []) {
  if (process.env.INSPECTOR_QUALIFY_BINARY) return installedWorkspace(action, fixture, flags, false);
  const dir = await mkdtemp(`${tmpdir()}/inspector-scope-`);
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
    const page = await browser.newPage();
    await page.goto(pathToFileURL(info.bootstrap).href);
    await page.waitForURL(`http://${info.address}/`);
    await page.getByRole("button", { name: "Attach database", exact: true }).click();
    await page.getByRole("heading", { name: "Database inventory", exact: true }).waitFor();
    await action(page, child, iter);
  } finally {
    await closeBrowserBounded(browser, 5000);
    await stopChildBounded(child);
    await rm(dir, { recursive: true, force: true });
  }
}

const click = (page, name) => page.getByRole("button", { name, exact: true }).click();

test("single-scope default store stays uncluttered without a multi-scope switcher", { timeout: 30000 }, async () =>
  workspace(async (page) => {
    assert.equal(await page.locator(".scope-switcher").count(), 0);
    await click(page, "Canonical nodes");
    await page.getByRole("heading", { name: "node records", exact: true }).waitFor();
    assert.equal(await page.getByLabel("Scope filter", { exact: true }).count(), 0);
  }, "linked"));

test("multi-scope switcher filters inventory, list, detail, graph, and lexical views without cross-scope leakage", { timeout: 45000 }, async () =>
  workspace(async (page) => {
    const switcher = page.getByLabel("Active scope", { exact: true });
    await switcher.waitFor();
    const options = await switcher.locator("option").evaluateAll((nodes) => nodes.map((n) => n.value));
    assert.deepEqual(options, [
      "__all__",
      "scope:",
      "scope:alpha",
      "scope:beta",
      "scope:hidden-inactive",
    ]);

    // Switch active scope to "alpha" in Inventory.
    await switcher.selectOption("scope:alpha");
    await page.waitForFunction(() => {
      const text = document.querySelector("#app")?.textContent ?? "";
      return text.includes("4 (exact); logical identities: 4");
    });

    // Navigate to Canonical nodes (pre-filtered to active scope "alpha").
    await click(page, "Canonical nodes");
    await page.getByRole("heading", { name: "node records", exact: true }).waitFor();
    const alphaListText = await page.locator(".record-list").textContent();
    assert.match(alphaListText, /Alpha Shared Title/);
    assert.doesNotMatch(alphaListText, /Beta Shared Title/);
    assert.doesNotMatch(alphaListText, /Default Shared Title/);
    assert.match(alphaListText, /scope alpha/);

    // Open colliding shared-node in scope "alpha" -> Detail -> Stored graph -> Native graph.
    await click(page, "Inspect shared-node");
    await page.getByRole("heading", { name: "Record detail", exact: true }).waitFor();
    assert.match(await page.locator(".record-scope").textContent(), /Scope: alpha/);
    assert.match(await page.locator("pre.content").textContent(), /Alpha Shared Title/);

    await click(page, "Stored graph");
    await page.getByRole("heading", { name: "Stored directed graph", exact: true }).waitFor();
    assert.match(await page.locator(".graph-scope").textContent(), /Scope: alpha/);
    assert.match(await page.locator(".graph-summary").textContent(), /2 retained nodes, 1 edges/);

    await click(page, "Run native graph traversal");
    await page.locator(".graph-native-payload").waitFor();
    assert.match(await page.locator(".graph-native-payload").textContent(), /Alpha Peer/);
    assert.doesNotMatch(await page.locator(".graph-native-payload").textContent(), /Beta Peer/);

    // Switch Scope filter in list view to "beta" and verify colliding shared-node resolves to Beta.
    await click(page, "Canonical nodes");
    await page.getByRole("heading", { name: "node records", exact: true }).waitFor();
    await page.getByLabel("Scope filter", { exact: true }).selectOption("scope:beta");
    await click(page, "Apply filters");
    await page.waitForFunction(() => {
      const text = document.querySelector(".record-list")?.textContent ?? "";
      return text.includes("Beta Shared Title") && !text.includes("Alpha Shared Title");
    });
    await click(page, "Inspect shared-node");
    await page.getByRole("heading", { name: "Record detail", exact: true }).waitFor();
    assert.match(await page.locator(".record-scope").textContent(), /Scope: beta/);
    assert.match(await page.locator("pre.content").textContent(), /Beta Shared Title/);

    // Text retrieval respects explicit scope selector across beta and alpha.
    await click(page, "Text retrieval");
    await page.getByRole("heading", { name: "Text retrieval", exact: true }).waitFor();
    await page.getByLabel("Scope filter", { exact: true }).selectOption("scope:beta");
    await page.getByLabel("Query text", { exact: true }).fill("orchid");
    await click(page, "Run lexical query");
    await page.locator(".lexical-hit").first().waitFor();
    const betaLexText = await page.locator(".lexical-results").textContent();
    assert.match(betaLexText, /Beta/);
    assert.doesNotMatch(betaLexText, /Alpha/);

    await page.getByLabel("Scope filter", { exact: true }).selectOption("scope:alpha");
    await click(page, "Run lexical query");
    await page.waitForFunction(() => {
      const text = document.querySelector(".lexical-results")?.textContent ?? "";
      return text.includes("Alpha") && !text.includes("Beta");
    });
  }, "scoped"));

test("D-01 hides inactive-only scopes from switcher, counts, and list under --no-show-inactive", { timeout: 30000 }, async () =>
  workspace(
    async (page) => {
      const switcher = page.getByLabel("Active scope", { exact: true });
      await switcher.waitFor();
      const options = await switcher.locator("option").evaluateAll((nodes) => nodes.map((n) => n.value));
      assert.deepEqual(options, ["__all__", "scope:", "scope:alpha", "scope:beta"]);
      const inventoryText = await page.locator("#app").textContent();
      assert.doesNotMatch(inventoryText, /hidden-inactive/);

      await click(page, "Canonical nodes");
      await page.getByRole("heading", { name: "node records", exact: true }).waitFor();
      const listText = await page.locator("#app").textContent();
      assert.doesNotMatch(listText, /hidden-inactive|secret-pending|secret-deleted/);
    },
    "scoped",
    ["--no-show-inactive"],
  ));
