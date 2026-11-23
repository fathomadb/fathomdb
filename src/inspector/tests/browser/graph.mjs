import {installedWorkspace} from "./installed-harness.mjs";
import { boundedExecFileSync, clipboardEquals, closeBrowserBounded, stopChildBounded } from "./helpers.mjs";
import { DatabaseSync } from "node:sqlite";
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
async function workspace(action, fixture = "graph", flags = []) {
 if(process.env.INSPECTOR_QUALIFY_BINARY)return installedWorkspace(action,fixture,flags,true);
  const dir = await mkdtemp(`${tmpdir()}/inspector-workspace-`);
  let child, browser;
  try {
    const db = fixture==="vector"
      ? boundedExecFileSync(`${root}/.venv/bin/python`,[`${root}/src/inspector/tools/vector-fixture.py`,dir],{encoding:"utf8"},120000).trim()
      : boundedExecFileSync(`${root}/target/debug/examples/fixture`, [dir, fixture], { encoding: "utf8" }, 120000).trim();
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
    await action(page, child, iter, { db });
  } finally {
    await closeBrowserBounded(browser, 5000);
    await stopChildBounded(child);
    await rm(dir, { recursive: true, force: true });
  }
}
test("directed stored graph keyboard selection, full edge detail, history and exact Back", {timeout:30000}, async()=>{
  await workspace(async(page)=>{
    await page.getByRole("button",{name:"Inspect derived",exact:true}).last().click();
    const graph=page.getByRole("button",{name:"Stored graph",exact:true});
    await graph.waitFor({timeout:1000});await graph.focus();await page.keyboard.press("Enter");
    await page.getByRole("heading",{name:"Stored directed graph",exact:true}).waitFor();
    assert.equal(await page.locator("svg.stored-graph").count(),1);
    assert.equal(await page.locator("svg.stored-graph path.graph-edge").count(),5);
    const context=await page.locator("pre.graph-context").textContent();
    const select=page.getByRole("button",{name:"Select edge parallel-a",exact:true});await select.focus();await page.keyboard.press("Enter");
    await page.getByRole("button",{name:"Open selected record",exact:true}).focus();await page.keyboard.press("Enter");
    await page.getByRole("heading",{name:"Record detail",exact:true}).waitFor();
    assert.match(await page.locator("pre.content").textContent(),/full edge é🦀<img/);
    assert.equal(await page.locator("img").count(),0);
    await page.getByRole("button",{name:"Back",exact:true}).click();
    assert.equal(await page.locator("pre.graph-context").textContent(),context);
    assert.equal(await page.evaluate(()=>document.activeElement?.textContent),"Open selected record");
    const node=page.getByRole("button",{name:/^Select node derived .*derived-r2/});await node.focus();await page.keyboard.press("Enter");
    await page.getByRole("button",{name:"Collapse selected node",exact:true}).click();
    await page.waitForFunction(()=>document.querySelectorAll("svg.stored-graph path.graph-edge").length===0);
    await page.getByRole("button",{name:"Expand selected node",exact:true}).click();
    await page.waitForFunction(()=>document.querySelectorAll("svg.stored-graph path.graph-edge").length===5);
    await page.getByLabel("Graph direction",{exact:true}).selectOption("outgoing");
    await page.getByLabel("Edge kind filter",{exact:true}).fill("supports");
    await page.getByRole("button",{name:"Apply graph filters",exact:true}).click();
    await page.waitForFunction(()=>document.querySelectorAll("svg.stored-graph path.graph-edge").length===3);
    await page.getByRole("button",{name:"Run native graph traversal",exact:true}).click();
    await page.getByRole("heading",{name:"Actual frozen graph evidence",exact:true}).waitFor();
    const nativeRequest=JSON.parse(await page.locator("pre.native-graph-context").textContent()).request;
    assert.equal(nativeRequest?.seed_logical_id,"derived","actual native graph seed must be displayed with its retained result");
    assert.deepEqual([nativeRequest.direction,nativeRequest.kind,nativeRequest.depth],["outgoing","supports",3]);
    const nativeContext=await page.locator("pre.native-graph-context").textContent();
    await page.getByRole("button",{name:/^Select node alpha .*alpha-r1/}).click();
    assert.equal(await page.locator("pre.native-graph-context").textContent(),nativeContext,"later selection must not reinterpret the original native request");
    await page.getByRole("button",{name:/^Select node derived .*derived-r2/}).click();
    assert.equal(await page.locator("pre.graph-native-payload").count(),1,"original frozen graph bytes must remain separate from hydrated records");
    assert.equal(await page.locator("pre.graph-native-payload").textContent(),"native graph alpha target");
    await page.context().grantPermissions(["clipboard-read","clipboard-write"],{origin:new URL(page.url()).origin});
    await page.getByRole("button",{name:"Copy observed graph payload window",exact:true}).click();await clipboardEquals(page,"native graph alpha target");
    assert.match(await page.locator(".graph-native").textContent(),/alpha-r1/);
    assert.match(await page.locator(".graph-native").textContent(),/parallel-a-r1/);
    assert.match(await page.locator(".graph-native").textContent(),/undetermined/);
    await page.getByRole("button",{name:"Investigate selected record",exact:true}).click();
    await page.getByRole("heading",{name:"Provenance and retained history",exact:true}).waitFor();
    const revision=page.getByRole("button",{name:"Open retained revision derived-r1",exact:true});await revision.focus();await page.keyboard.press("Enter");
    await page.getByRole("heading",{name:"Record detail",exact:true}).waitFor();
    assert.match(await page.locator("pre.content").textContent(),/derived body/);
    await page.getByRole("button",{name:"Provenance and history",exact:true}).click();
    assert.match(await page.getByRole("region",{name:"Recorded investigation links",exact:true}).textContent(),/registered dependency to source/);
    assert.equal(await page.locator("img").count(),0);
  });
});

test("late stored graph response cannot replace newer record selection",{timeout:30000},async()=>{
 await workspace(async(page)=>{
  await page.getByRole("button",{name:"Inspect derived",exact:true}).last().click();
  let release,captured;const released=new Promise(r=>release=r),held=new Promise(r=>captured=r);
  try{
   await page.route("**/api",async route=>{if(route.request().postDataJSON().request.op!=="graph")return route.continue();const response=await route.fetch();captured();await released;await route.fulfill({response});});
   await page.getByRole("button",{name:"Stored graph",exact:true}).click();await held;
   await page.getByRole("button",{name:"Canonical nodes",exact:true}).click();await page.getByRole("button",{name:"Inspect beta",exact:true}).click();
   await page.getByRole("heading",{name:"Record detail",exact:true}).waitFor();
   const delivered=page.waitForResponse(r=>r.url().endsWith("/api")&&r.request().postDataJSON().request.op==="graph");release();await delivered;await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
   assert.equal(await page.locator("svg.stored-graph").count(),0);assert.equal(await page.locator("pre.content").textContent(),"original text");
  }finally{release?.();}
 });
});

test("expanded non-root revision keeps collapse ownership after fresh handle minting",{timeout:30000},async()=>{
 await workspace(async page=>{
  await page.getByRole("button",{name:"Inspect derived",exact:true}).last().click();await page.getByRole("button",{name:"Stored graph",exact:true}).click();
  await page.getByRole("heading",{name:"Stored directed graph",exact:true}).waitFor();
  const node=page.getByRole("button",{name:/^Select node alpha .*alpha-r1/});await node.click();
  const response=page.waitForResponse(r=>r.url().endsWith("/api")&&r.request().postDataJSON().request.op==="graph");
  await page.getByRole("button",{name:"Expand selected node",exact:true}).click();await response;
  await page.waitForFunction(()=>document.querySelector(".graph-selection")?.textContent.includes("alpha-r1"));
  assert.equal(await page.getByRole("button",{name:"Collapse selected node",exact:true}).isEnabled(),true,"expansion ownership uses exact class/cursor, not newly minted opaque handles");
  await page.getByRole("button",{name:"Collapse selected node",exact:true}).click();
  await page.waitForFunction(()=>document.querySelectorAll("svg.stored-graph path.graph-edge").length===5);
  await page.waitForFunction(()=>[...document.querySelectorAll("button")].some(b=>b.textContent==="Expand selected node"&&!b.disabled));
  assert.equal(await page.getByRole("button",{name:"Expand selected node",exact:true}).isEnabled(),true);
 });
});

test("every self and parallel edge has distinct pointer-selectable geometry",{timeout:30000},async()=>{
 await workspace(async page=>{
  await page.getByRole("button",{name:"Inspect derived",exact:true}).last().click();await page.getByRole("button",{name:"Stored graph",exact:true}).click();await page.getByRole("heading",{name:"Stored directed graph",exact:true}).waitFor();
  const paths=[];for(let i=0;i<10;i++){const key=await page.getByRole("button",{name:`Select edge visual-${i}`,exact:true}).getAttribute("data-graph-key");paths.push(await page.locator(`path.graph-edge[data-graph-key="${key}"]`).getAttribute("d"));}
  assert.equal(new Set(paths.slice(0,2)).size,2,"separate self edges must not share one loop");
  assert.equal(new Set(paths.slice(2)).size,8,"eight parallel edges must not repeat a seven-curve cycle");
  for(let i=0;i<10;i++){
    const key=await page.getByRole("button",{name:`Select edge visual-${i}`,exact:true}).getAttribute("data-graph-key");
    const path=page.locator(`path.graph-edge[data-graph-key="${key}"]`);await path.scrollIntoViewIfNeeded();
    const point=await path.evaluate(node=>{const point=node.getPointAtLength(node.getTotalLength()*0.5).matrixTransform(node.getScreenCTM());return{x:point.x,y:point.y};});
    await page.mouse.click(point.x,point.y);
    assert.match(await page.locator(".graph-selection").textContent(),new RegExp(`visual-${i} `));
  }
 },"graph-visual");
});

test("missing endpoints sharing a preview remain separate directed anchors",{timeout:30000},async()=>{
 await workspace(async page=>{
  await page.getByRole("button",{name:"Canonical edges",exact:true}).click();await page.getByRole("button",{name:"Inspect prefix-edge",exact:true}).click();await page.getByRole("button",{name:"Stored graph",exact:true}).click();await page.getByRole("heading",{name:"Stored directed graph",exact:true}).waitFor();
  assert.equal(await page.locator("svg.stored-graph rect.graph-unresolved").count(),2,"different full missing IDs cannot collapse into a self-loop");
  assert.match(await page.locator("path.graph-edge").getAttribute("d"),/ Q /);
 },"graph-missing-prefix");
});

test("lexical observation survives graph and retained source-history navigation with exact Back",{timeout:30000},async()=>{
 await workspace(async page=>{
  await page.getByRole("button",{name:"Text retrieval",exact:true}).click();await page.getByLabel("Query text",{exact:true}).fill("derived");await page.getByRole("button",{name:"Run lexical query",exact:true}).click();
  const origin=page.getByRole("button",{name:"Open exact origin derived",exact:true});await origin.waitFor();const original=await page.locator("pre.query-context").textContent();
  await origin.focus();await page.keyboard.press("Enter");await page.getByRole("button",{name:"Stored graph",exact:true}).click();await page.getByRole("heading",{name:"Stored directed graph",exact:true}).waitFor();
  await page.getByRole("button",{name:"Investigate selected record",exact:true}).click();await page.getByRole("heading",{name:"Provenance and retained history",exact:true}).waitFor();
  await page.getByRole("button",{name:"Open recorded source source-r1",exact:true}).click();await page.getByRole("heading",{name:"Record detail",exact:true}).waitFor();assert.match(await page.locator("pre.content").textContent(),/source body/);
  for(const heading of ["Provenance and retained history","Stored directed graph","Record detail","Text retrieval"]){await page.getByRole("button",{name:"Back",exact:true}).click();await page.getByRole("heading",{name:heading,exact:true}).waitFor();}
  assert.equal(await page.locator("pre.query-context").textContent(),original);assert.equal(await page.getByLabel("Query text",{exact:true}).inputValue(),"derived");assert.equal(await page.evaluate(()=>document.activeElement?.textContent),"Open exact origin derived");
 });
});

test("stored vector observation survives graph-history navigation and returns without requery",{timeout:120000},async()=>{
 await workspace(async page=>{
  await page.getByRole("button",{name:"Inspect dog",exact:true}).click();await page.getByRole("button",{name:"Stored vectors",exact:true}).click();await page.getByRole("button",{name:/^Find similar vector row /}).click();await page.getByRole("heading",{name:"Stored L2 neighborhood",exact:true}).waitFor();
  const original=await page.locator("pre.vector-context").textContent();const origin=page.getByRole("button",{name:"Open exact origin planet",exact:true});await origin.focus();await page.keyboard.press("Enter");
  await page.getByRole("button",{name:"Stored graph",exact:true}).click();await page.getByRole("heading",{name:"Stored directed graph",exact:true}).waitFor();await page.getByRole("button",{name:"Investigate selected record",exact:true}).click();await page.getByRole("heading",{name:"Provenance and retained history",exact:true}).waitFor();
  await page.getByRole("button",{name:/^Open retained revision /}).click();await page.getByRole("heading",{name:"Record detail",exact:true}).waitFor();assert.match(await page.locator("pre.content").textContent(),/icy planet/);
  for(const heading of ["Provenance and retained history","Stored directed graph","Record detail","Vector inspection"]){await page.getByRole("button",{name:"Back",exact:true}).click();await page.getByRole("heading",{name:heading,exact:true}).waitFor();}
  assert.equal(await page.locator("pre.vector-context").textContent(),original);assert.equal(await page.evaluate(()=>document.activeElement?.textContent),"Open exact origin planet");
 },"vector");
});

test("collapse clears the detached graph workspace after child Back focus retention",{timeout:30000},async()=>{
 await workspace(async page=>{
  await page.getByRole("button",{name:"Inspect derived",exact:true}).last().click();await page.getByRole("button",{name:"Stored graph",exact:true}).click();await page.getByRole("heading",{name:"Stored directed graph",exact:true}).waitFor();
  await page.getByRole("button",{name:"Open selected record",exact:true}).click();await page.getByRole("heading",{name:"Record detail",exact:true}).waitFor();await page.getByRole("button",{name:"Back",exact:true}).click();await page.getByRole("heading",{name:"Stored directed graph",exact:true}).waitFor();
  await page.evaluate(()=>window.detachedGraphForCheck=document.querySelector("svg.stored-graph").closest(".panel"));
  await page.getByRole("button",{name:"Collapse selected node",exact:true}).click();await page.waitForFunction(()=>document.querySelectorAll("svg.stored-graph path.graph-edge").length===0);
  assert.equal(await page.evaluate(()=>window.detachedGraphForCheck.textContent),"","replaced graph must release hidden DOM text even when Back retained a control focus reference");
  await page.evaluate(()=>delete window.detachedGraphForCheck);
 });
});

test("high-degree bounded graph keeps every arrow within its SVG viewport",{timeout:30000},async()=>{
 await workspace(async page=>{
  await page.getByRole("button",{name:"Inspect alpha",exact:true}).click();await page.getByRole("button",{name:"Stored graph",exact:true}).click();await page.getByRole("heading",{name:"Stored directed graph",exact:true}).waitFor();
  assert.equal(await page.locator("path.graph-edge").count(),160);
  assert.equal(await page.locator("svg.stored-graph").evaluate(svg=>{const v=svg.viewBox.baseVal;return [...svg.querySelectorAll("path.graph-edge")].every(path=>{const b=path.getBBox();return b.x>=v.x&&b.y>=v.y&&b.x+b.width<=v.x+v.width&&b.y+b.height<=v.y+v.height;});}),true,"declared graph scope must not silently clip high-degree arrows outside its viewport");
 },"graph-highdegree");
});

test("historical as_of graph controls validate locally, query stored and native snapshots, render properties safely, and survive Back",{timeout:45000},async()=>{
 await workspace(async(page,_child,_iter,meta)=>{
  const oracle=new DatabaseSync(meta.db,{readOnly:true});
  let c1,t1;
  try{
   const row=oracle.prepare("SELECT write_cursor AS c1, committed_at_ms AS t1 FROM _fathomdb_commit_timestamps WHERE write_cursor >= (SELECT write_cursor FROM canonical_nodes WHERE logical_id='hist-root' ORDER BY write_cursor ASC LIMIT 1) ORDER BY write_cursor ASC LIMIT 1").get();
   c1=String(row.c1);t1=String(row.t1);
  }finally{oracle.close();}
  let apiCalls=0;
  page.on("request",r=>{if(r.url().endsWith("/api")&&["graph","graph_serving"].includes(r.postDataJSON()?.request?.op))apiCalls++;});
  await page.getByRole("button",{name:"Inspect hist-root",exact:true}).last().click();
  await page.getByRole("button",{name:"Stored graph",exact:true}).click();
  await page.getByRole("heading",{name:"Stored directed graph",exact:true}).waitFor();
  const callsBeforeInvalid=apiCalls;
  await page.getByLabel("Snapshot",{exact:true}).selectOption("write_cursor");
  await page.getByLabel("Snapshot value",{exact:true}).fill("not-an-int");
  await page.getByRole("button",{name:"Apply graph filters",exact:true}).click();
  assert.match(await page.getByRole("status",{name:"Graph query feedback",exact:true}).textContent(),/Invalid snapshot write cursor/);
  assert.equal(apiCalls,callsBeforeInvalid,"invalid as_of input must validate locally before api() to avoid cache eviction");
  assert.equal(await page.locator("svg.stored-graph").count(),1,"cached graph observation must remain intact after invalid input");

  await page.getByLabel("Snapshot value",{exact:true}).fill(c1);
  await page.getByRole("button",{name:"Apply graph filters",exact:true}).click();
  await page.waitForFunction(cursor=>{const pre=document.querySelector("pre.scope");return pre&&JSON.parse(pre.textContent).scope?.as_of?.resolved_as_of_write_cursor===cursor;},c1);
  assert.equal(await page.getByRole("button",{name:"Select edge e-early",exact:true}).count(),1);
  assert.equal(await page.getByRole("button",{name:"Select edge e-late",exact:true}).count(),0);

  await page.getByRole("button",{name:"Run native graph traversal",exact:true}).click();
  await page.getByRole("heading",{name:"Actual frozen graph evidence",exact:true}).waitFor();
  const nativeMeta=JSON.parse(await page.locator("pre.native-graph-context").textContent());
  assert.equal(nativeMeta.request.as_of.mode,"write_cursor");
  assert.equal(nativeMeta.request.as_of.resolved_as_of_write_cursor,c1);
  assert.match(await page.locator(".graph-native").textContent(),/<img src=x onerror=alert\(1\)>/);
  assert.match(await page.locator(".graph-native").textContent(),/"><svg onload=alert\(1\)>/);
  assert.equal(await page.locator("img").count(),0);
  assert.equal(await page.locator("svg").count(),1,"only the stored-graph svg may exist; hostile svg payload must render as text");

  await page.getByRole("button",{name:"Open selected record",exact:true}).click();
  await page.getByRole("heading",{name:"Record detail",exact:true}).waitFor();
  await page.getByRole("button",{name:"Back",exact:true}).click();
  await page.getByRole("heading",{name:"Stored directed graph",exact:true}).waitFor();
  assert.equal(await page.getByLabel("Snapshot",{exact:true}).inputValue(),"write_cursor");
  assert.equal(await page.getByLabel("Snapshot value",{exact:true}).inputValue(),c1);

  await page.getByLabel("Snapshot",{exact:true}).selectOption("epoch_ms");
  await page.getByLabel("Snapshot value",{exact:true}).fill(t1);
  await page.getByRole("button",{name:"Apply graph filters",exact:true}).click();
  await page.waitForFunction(ms=>{const pre=document.querySelector("pre.scope");return pre&&JSON.parse(pre.textContent).scope?.as_of?.mode==="epoch_ms"&&JSON.parse(pre.textContent).scope?.as_of?.requested_as_of_epoch_ms===ms;},t1);
  assert.equal(await page.getByRole("button",{name:"Select edge e-early",exact:true}).count(),1);
  assert.equal(await page.getByRole("button",{name:"Select edge e-late",exact:true}).count(),0);
 },"graph-history");
});
