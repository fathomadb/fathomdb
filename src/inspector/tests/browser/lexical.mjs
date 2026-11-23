import {installedWorkspace} from "./installed-harness.mjs";
import { boundedExecFileSync, clipboardEquals, closeBrowserBounded, stopChildBounded } from "./helpers.mjs";
import test from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { createInterface } from "node:readline";
const root = resolve(import.meta.dirname, "../../../..");
const { chromium } = await import(pathToFileURL(process.env.INSPECTOR_PLAYWRIGHT ?? "/home/coreyt/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs"));
async function workspace(action, fixture = "lexical", flags = []) {
 if(process.env.INSPECTOR_QUALIFY_BINARY)return installedWorkspace(action,fixture,flags,true);
  const dir = await mkdtemp(`${tmpdir()}/inspector-workspace-`);
  let child, browser;
  try {
    const db = boundedExecFileSync(`${root}/target/debug/examples/fixture`, [dir, fixture], { encoding: "utf8" }, 120000).trim();
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
    await closeBrowserBounded(browser, 5000);
    await stopChildBounded(child);
    await rm(dir, { recursive: true, force: true });
  }
}
test("native lexical workspace follows indexed text to query evidence and exact origin with keyboard Back", {timeout:30000}, async()=>{
  await workspace(async(page)=>{
    const external=[];
    page.on("request",r=>{if(!r.url().startsWith("http://127.0.0.1:")&&!r.url().startsWith("file:"))external.push(r.url());});
    await page.getByRole("button",{name:"Inspect lexical-a",exact:true}).click();
    const indexed=page.getByRole("button",{name:"Indexed text",exact:true});
    await indexed.waitFor({timeout:1000});
    await indexed.focus();await page.keyboard.press("Enter");
    await page.getByRole("heading",{name:"Text retrieval",exact:true}).waitFor();
    assert.match(await page.getByRole("region",{name:"Stored indexed text",exact:true}).textContent(),/orchid orchid/);
    await page.getByLabel("Query text",{exact:true}).fill("cafe");
    await page.getByRole("button",{name:"Run lexical query",exact:true}).focus();await page.keyboard.press("Space");
    const a=page.locator(".lexical-hit").filter({has:page.getByRole("heading",{name:/lexical-a/})});
    await a.waitFor();
    assert.equal(await a.locator("mark").first().textContent(),"Café");
    assert.match(await a.textContent(),/rrf/);
    assert.match(await a.textContent(),/Record identity: record_keyed_not_merged\./);
    assert.doesNotMatch(await a.textContent(),/Equal-body contributors: unknown/);
    const original=await page.locator("pre.query-context").textContent();
    const origin=a.getByRole("button",{name:"Open exact origin lexical-a",exact:true});
    await origin.focus();await page.keyboard.press("Enter");
    await page.getByRole("heading",{name:"Record detail",exact:true}).waitFor();
    assert.match(await page.locator("pre.content").textContent(),/Café runners/);
    await page.getByRole("button",{name:"Back",exact:true}).click();
    await page.getByRole("heading",{name:"Text retrieval",exact:true}).waitFor();
    assert.equal(await page.locator("pre.query-context").textContent(),original);
    assert.equal(await page.getByLabel("Query text",{exact:true}).inputValue(),"cafe");
    assert.equal(await page.evaluate(()=>document.activeElement?.textContent),"Open exact origin lexical-a");
    await page.getByLabel("Lexical route",{exact:true}).selectOption("property");
    await page.getByLabel("Property name",{exact:true}).fill("biography");
    await page.getByLabel("Query text",{exact:true}).fill("orchid");
    await page.getByRole("button",{name:"Run lexical query",exact:true}).click();
    await page.waitForFunction(()=>document.querySelectorAll(".lexical-hit").length===3);
    assert.match(await page.locator(".lexical-results").textContent(),/negated_sqlite_bm25/);
    assert.match(await page.locator(".lexical-results").textContent(),/no_native_snippet/);
    assert.equal(await page.locator("img").count(),0);
    assert.deepEqual(external,[]);
  });
});

test("a superseded lexical response cannot replace or invalidate a newer record", {timeout:30000}, async()=>{
  await workspace(async(page)=>{
    await page.getByRole("button",{name:"Text retrieval",exact:true}).click();
    await page.getByLabel("Query text",{exact:true}).fill("cafe");
    let release,captured;
    const released=new Promise(r=>{release=r;}),held=new Promise(r=>{captured=r;});
    try {
      await page.route("**/api",async route=>{
        if(route.request().postDataJSON().request.op!=="lexical")return route.continue();
        const response=await route.fetch();assert.equal((await response.json()).status,"complete");captured();await released;await route.fulfill({response});
      });
      await page.getByRole("button",{name:"Run lexical query",exact:true}).click();await held;
      await page.getByRole("button",{name:"Canonical nodes",exact:true}).click();
      await page.getByRole("button",{name:"Inspect lexical-b",exact:true}).click();
      await page.waitForFunction(()=>document.querySelector("pre.content")?.textContent.includes("runner blue"));
      const delivered=page.waitForResponse(r=>r.url().endsWith("/api")&&r.request().postDataJSON().request.op==="lexical");
      release();await delivered;await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
      assert.match(await page.locator("pre.content").textContent(),/runner blue/);
      assert.equal(await page.locator(".lexical-results").count(),0);
    } finally {release?.();}
  });
});

test("global native lexical controls distinguish empty, invalid and qualified BM25F observations", {timeout:30000}, async()=>{
  await workspace(async(page)=>{
    await page.getByRole("button",{name:"Text retrieval",exact:true}).click();
    await page.getByLabel("Query text",{exact:true}).fill("unfindableword");
    await page.getByRole("button",{name:"Run lexical query",exact:true}).click();
    await page.getByText("No permitted matches in this bounded native observation.",{exact:true}).waitFor();
    await page.getByLabel("Lexical route",{exact:true}).selectOption("bm25f");
    assert.equal(await page.getByLabel("Kind filter",{exact:true}).isDisabled(),true);
    assert.equal(await page.getByLabel("Include inactive in query",{exact:true}).isDisabled(),true);
    await page.getByLabel("Query text",{exact:true}).fill("orchid");
    await page.getByRole("button",{name:"Run lexical query",exact:true}).click();
    await page.getByRole("heading",{name:"Native results: native_bm25f (higher_better)",exact:true}).waitFor();
    assert.match(await page.locator(".lexical-results").textContent(),/later_exact_cursor_hydration/);
    assert.match(await page.locator("pre.scope").textContent(),/not_filtered_by_native_bm25f/);
    const previous=await page.locator("pre.query-context").textContent();
    await page.getByLabel("Query text",{exact:true}).fill(" ");
    await page.getByRole("button",{name:"Run lexical query",exact:true}).click();
    // Invalid input is reported; the old bounded observation is never relabeled as the failed request.
    await page.waitForFunction(()=>document.querySelector("#notice")?.textContent.includes("invalid_range")||document.body.textContent.includes("Operation did not complete (invalid_range)"));
    if(await page.locator("pre.query-context").count())assert.equal(await page.locator("pre.query-context").textContent(),previous);
  });
});


test("lexical indexed windows and original UTF8 payload/snippet copy to the real clipboard exactly", {timeout:30000}, async()=>{
  await workspace(async(page)=>{
    await page.context().grantPermissions(["clipboard-read","clipboard-write"],{origin:new URL(page.url()).origin});
    await page.getByRole("button",{name:"Inspect lexical-a",exact:true}).click();
    await page.getByRole("button",{name:"Indexed text",exact:true}).click();
    await page.getByRole("button",{name:"Copy indexed window property:biography",exact:true}).click();
    await clipboardEquals(page,"orchid orchid");
    await page.getByLabel("Query text",{exact:true}).fill("cafe");
    await page.getByRole("button",{name:"Run lexical query",exact:true}).click();
    const card=page.locator(".lexical-hit").filter({has:page.getByRole("heading",{name:/lexical-a/})});
    await card.waitFor();
    const expected='{"profile":{"biography":"orchid orchid"},"status":"open","text":"Café runners <img src=https://evil.invalid/x onerror=alert(1)>"}';
    assert.equal(await card.locator("pre.observed-payload").textContent(),expected);
    await card.getByRole("button",{name:/^Copy observed text /}).click();await clipboardEquals(page,expected);
    await card.getByRole("button",{name:/^Copy observed snippet /}).click();await clipboardEquals(page,expected);
  });
});
