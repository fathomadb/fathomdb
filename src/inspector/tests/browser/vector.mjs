import {installedWorkspace} from "./installed-harness.mjs";
import test from "node:test";
import assert from "node:assert/strict";
import {spawn,execFileSync} from "node:child_process";
import {mkdtemp,rm} from "node:fs/promises";
import {tmpdir} from "node:os";
import {resolve} from "node:path";
import {pathToFileURL} from "node:url";
import {createInterface} from "node:readline";
const root=resolve(import.meta.dirname,"../../../..");
const {chromium}=await import(pathToFileURL(process.env.INSPECTOR_PLAYWRIGHT??"/home/coreyt/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs"));
async function workspace(action,model=true){
 if(process.env.INSPECTOR_QUALIFY_BINARY)return installedWorkspace(action,model?"vector":"default",[],true);
 const dir=await mkdtemp(`${tmpdir()}/inspector-vector-ui-`);let child,browser;
 try{
  const db=model?execFileSync(`${root}/.venv/bin/python`,[`${root}/src/inspector/tools/vector-fixture.py`,dir],{encoding:"utf8"}).trim():execFileSync(`${root}/target/debug/examples/fixture`,[dir],{encoding:"utf8"}).trim();
  child=spawn(`${root}/target/debug/examples/driver`,[db,`${root}/target/debug/fathomdb-inspector`],{stdio:["pipe","pipe","pipe"]});
  const iter=createInterface({input:child.stdout})[Symbol.asyncIterator]();const info=JSON.parse((await iter.next()).value);
  browser=await chromium.launch({executablePath:process.env.INSPECTOR_CHROMIUM??"/home/coreyt/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome",headless:true,args:["--no-sandbox","--disable-background-networking"]});
  const page=await browser.newPage();await page.goto(pathToFileURL(info.bootstrap).href);await page.waitForURL(`http://${info.address}/`);
  await page.getByRole("button",{name:"Attach database",exact:true}).click();await page.getByRole("heading",{name:"Database inventory",exact:true}).waitFor();
  await page.getByRole("button",{name:"Canonical nodes",exact:true}).click();await action(page);
 }finally{if(browser)await browser.close();if(child){child.stdin.end("close\n");await new Promise(r=>child.on("exit",r));}await rm(dir,{recursive:true,force:true});}
}
test("stored vector keyboard workflow preserves numeric observation, exact origin and Back; cached native scores stay separate",{timeout:60000},async()=>{
 await workspace(async page=>{
  const external=[];page.on("request",r=>{if(!r.url().startsWith("http://127.0.0.1:")&&!r.url().startsWith("file:"))external.push(r.url());});
  await page.getByRole("button",{name:"Inspect dog",exact:true}).click();
  const vectors=page.getByRole("button",{name:"Stored vectors",exact:true});await vectors.waitFor({timeout:1000});await vectors.focus();await page.keyboard.press("Enter");
  await page.getByRole("heading",{name:"Vector inspection",exact:true}).waitFor();
  const stored=page.getByRole("region",{name:"Stored vector representations",exact:true});assert.match(await stored.textContent(),/384/);assert.match(await stored.textContent(),/window 0/);assert.match(await stored.textContent(),/stored_uncentered_f32/);
  const find=page.getByRole("button",{name:/^Find similar vector row /});await find.focus();await page.keyboard.press("Space");
  await page.getByRole("heading",{name:"Stored L2 neighborhood",exact:true}).waitFor();assert.equal(await page.locator(".vector-hit").count(),2);assert.equal(await page.locator(".vector-hit meter").count(),2);assert.match(await page.locator(".vector-results").textContent(),/lower is nearer/);
  const original=await page.locator("pre.vector-context").textContent();const planet=page.locator(".vector-hit").filter({has:page.getByRole("heading",{name:"planet",exact:true})});
  const origin=planet.getByRole("button",{name:"Open exact origin planet",exact:true});await origin.focus();await page.keyboard.press("Enter");await page.getByRole("heading",{name:"Record detail",exact:true}).waitFor();assert.match(await page.locator("pre.content").textContent(),/icy planet/);
  await page.getByRole("button",{name:"Back",exact:true}).click();await page.getByRole("heading",{name:"Vector inspection",exact:true}).waitFor();assert.equal(await page.locator("pre.vector-context").textContent(),original);assert.equal(await page.evaluate(()=>document.activeElement?.textContent),"Open exact origin planet");
  await page.getByLabel("Semantic query text",{exact:true}).fill("puppy");await page.getByRole("button",{name:"Run native hybrid search",exact:true}).focus();await page.keyboard.press("Enter");
  await page.getByRole("heading",{name:"Native hybrid results",exact:true}).waitFor();assert.equal(await page.locator(".serving-hit").count(),2);assert.match(await page.locator(".serving-results").textContent(),/engine_rrf/);assert.match(await page.locator(".serving-results").textContent(),/vector_rank/);assert.match(await page.locator(".serving-results").textContent(),/not_exposed/);assert.equal(await page.locator(".serving-hit meter").count(),0);assert.deepEqual(external,[]);
 });
});
test("late stored-neighborhood response cannot replace a newer record",{timeout:60000},async()=>{
 await workspace(async page=>{
  await page.getByRole("button",{name:"Inspect dog",exact:true}).click();await page.getByRole("button",{name:"Stored vectors",exact:true}).click();let release,captured;
  const gate=new Promise(r=>{release=r;});const seen=new Promise(r=>{captured=r;});
  await page.route("**/api",async route=>{if(route.request().postDataJSON().request?.op!=="neighborhood")return route.continue();const response=await route.fetch();captured();await gate;await route.fulfill({response});});
  await page.getByRole("button",{name:/^Find similar vector row /}).click();await seen;await page.getByRole("button",{name:"Canonical nodes",exact:true}).click();await page.getByRole("button",{name:"Inspect planet",exact:true}).click();release();await page.getByRole("heading",{name:"Record detail",exact:true}).waitFor();assert.match(await page.locator("pre.content").textContent(),/icy planet/);assert.equal(await page.locator(".vector-results").count(),0);
 });
});
test("model-free stored inspection stays usable and never claims native availability",{timeout:30000},async()=>{
 await workspace(async page=>{
  await page.getByRole("button",{name:"Inspect alpha",exact:true}).click();await page.getByRole("button",{name:"Stored vectors",exact:true}).click();await page.getByRole("heading",{name:"Vector inspection",exact:true}).waitFor();assert.match(await page.getByRole("region",{name:"Stored vector representations",exact:true}).textContent(),/No stored vector/);assert.equal(await page.getByRole("button",{name:"Run native hybrid search",exact:true}).isDisabled(),true);await page.getByRole("button",{name:"Canonical nodes",exact:true}).click();await page.getByRole("button",{name:"Inspect beta",exact:true}).click();assert.equal(await page.locator("pre.content").textContent(),"original text");
 },false);
});
