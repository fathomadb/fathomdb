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
 if(process.env.INSPECTOR_QUALIFY_BINARY)return installedWorkspace(action,fixture,flags,false);
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
const click=(page,name)=>page.getByRole('button',{name,exact:true}).click();
test('attachment status follows success failure and reattach independently of feedback',{timeout:30000},async()=>workspace(async page=>{
 assert.equal(await page.locator('#session-status').count(),1,'attached session requires its own durable status');
 assert.match(await page.locator('#session-status').textContent(),/Attached database/);
 await click(page,'Detach database'); await page.getByLabel('Database path',{exact:true}).waitFor();
 const original=await page.getByLabel('Database path',{exact:true}).inputValue();
 assert.match(await page.locator('#session-status').textContent(),/No database attached/);
 await page.getByLabel('Database path',{exact:true}).fill(original+'.missing');await click(page,'Attach database');await page.getByText('Select an existing database file.',{exact:true}).waitFor();
 await page.getByLabel('Database path',{exact:true}).fill(original);await click(page,'Attach database');await page.getByRole('heading',{name:'Database inventory',exact:true}).waitFor();
 assert.match(await page.locator('#session-status').textContent(),/Attached database/);
 assert.doesNotMatch(await page.locator('#notice').textContent(),/Select an existing|No database attached/);
}));

test('long native results stay inside default and narrow viewports with query and origin actions first',{timeout:30000},async()=>workspace(async page=>{
 await page.setViewportSize({width:1042,height:793});await click(page,'Text retrieval');
 await page.getByLabel('Lexical route',{exact:true}).selectOption('property');await page.getByLabel('Property name',{exact:true}).fill('biography');await page.getByLabel('Query text',{exact:true}).fill('orchid');await click(page,'Run lexical query');await page.waitForFunction(()=>document.querySelectorAll('.lexical-hit').length===3);
 for(const width of [1042,640]){
  await page.setViewportSize({width,height:793});
  const actual=await page.evaluate(()=>({page:document.documentElement.scrollWidth,viewport:innerWidth}));
  assert.ok(actual.page<=actual.viewport+1,`page-wide overflow ${actual.page} > ${actual.viewport}`);
 }
 const card=page.locator('.lexical-hit').first();
 assert.equal(await card.evaluate(n=>!!(n.querySelector('button')?.compareDocumentPosition(n.querySelector('.observed-payload'))&Node.DOCUMENT_POSITION_FOLLOWING)),true,'origin/copy actions must precede long raw payload');
 assert.equal(await page.locator('form[aria-label="Native lexical query"]').evaluate(n=>!!(n.compareDocumentPosition(document.querySelector('[aria-label="Stored indexed text"]'))&Node.DOCUMENT_POSITION_FOLLOWING)),true,'query must precede stored windows');
} ,'lexical'));

test('property controls use actual declarations and explain invalid input beside the query',{timeout:30000},async()=>workspace(async page=>{
 await click(page,'Text retrieval');await page.getByLabel('Lexical route',{exact:true}).selectOption('property');
 const property=page.getByLabel('Property name',{exact:true});
 assert.equal(await property.getAttribute('list')!==null,true,'declared properties must be offered by the actual control');
 assert.deepEqual(await page.locator('datalist option').evaluateAll(nodes=>nodes.map(n=>n.value)),['biography']);
 let queries=0;page.on('request',r=>{if(r.url().endsWith('/api')&&r.postDataJSON()?.request?.op==='lexical')queries++;});
 await property.fill('not-declared');await page.getByLabel('Query text',{exact:true}).fill('orchid');await click(page,'Run lexical query');
 await page.getByRole('status',{name:'Lexical query feedback'}).getByText(/Choose a declared searchable property/).waitFor();assert.equal(queries,0);
 await property.fill('biography');await click(page,'Run lexical query');await page.waitForFunction(()=>document.querySelectorAll('.lexical-hit').length===3);
 assert.equal(await page.getByRole('status',{name:'Lexical query feedback'}).textContent(),'');assert.equal(queries,1);
 await click(page,'Canonical nodes');await click(page,'Inspect lexical-a');await click(page,'Indexed text');await page.getByLabel('Lexical route',{exact:true}).selectOption('property');
 assert.deepEqual(await page.locator('datalist option').evaluateAll(nodes=>nodes.map(n=>n.value)),['biography']);
},'lexical'));

test('browse and detail expose admitted stored titles alongside exact identities',{timeout:30000},async()=>workspace(async page=>{
 await click(page,'Canonical nodes');await page.getByRole('heading',{name:'node records',exact:true}).waitFor();
 assert.equal(await page.locator('.record-list .record-display').filter({hasText:'derived-title'}).count(),1,'a stored title must be visible before opening a record');
 await click(page,'Inspect derived');await page.getByRole('heading',{name:'Record detail',exact:true}).waitFor();
 assert.match(await page.locator('.record-workspace>.record-display').textContent(),/derived-title/);
 assert.equal(await page.getByRole('button',{name:'Copy exact identity',exact:true}).count(),1);
}));

test('compact result evidence retains original request and separately timed later checks',{timeout:30000},async()=>workspace(async page=>{
 await click(page,'Text retrieval');await page.getByLabel('Lexical route',{exact:true}).selectOption('property');await page.getByLabel('Query text',{exact:true}).fill('orchid');await click(page,'Run lexical query');await page.waitForFunction(()=>document.querySelectorAll('.lexical-hit').length===3);
 assert.equal(await page.locator('.lexical-results .observation-summary').count(),1,'result needs a compact scoped summary');
 const summary=page.locator('.lexical-results .observation-summary');assert.match(await summary.textContent(),/orchid/);assert.match(await summary.textContent(),/Native retrieval time/);assert.match(await summary.textContent(),/Later policy\/origin read/);assert.match(await summary.textContent(),/20\d\d-\d\d-\d\dT/);
 await page.getByLabel('Query text',{exact:true}).fill('stone');assert.match(await summary.textContent(),/orchid/);assert.doesNotMatch(await summary.textContent(),/stone/);
},'lexical'));

test('bounded hub expansion retains the selected branch and provides a readable local viewport',{timeout:30000},async()=>workspace(async page=>{
 await page.setViewportSize({width:1042,height:793});await click(page,'Canonical nodes');await click(page,'Inspect ux-seed');await click(page,'Stored graph');await page.getByRole('heading',{name:'Stored directed graph',exact:true}).waitFor();
 assert.equal(await page.getByRole('button',{name:'Zoom in graph',exact:true}).count(),1,'dense graphs need local viewport navigation');
 await page.getByRole('button',{name:/^Select node ux-hub cursor/}).click();await click(page,'Expand selected node');await page.waitForFunction(()=>document.querySelector('.graph-summary')?.textContent.startsWith('partial'));
 assert.match(await page.locator('.graph-selection').textContent(),/ux-hub/);
 assert.equal(await page.getByRole('button',{name:'Collapse selected node',exact:true}).isEnabled(),true);
 await page.getByRole('button',{name:'Zoom in graph',exact:true}).focus();await page.keyboard.press('Enter');
 assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
 await click(page,'Collapse selected node');await page.waitForFunction(()=>document.querySelector('.graph-summary')?.textContent.includes('1 edges;'));
 assert.equal(await page.locator('.stored-graph .graph-edge').count(),1);
},'ux-hub'));

test('hub scope is adjacent to the diagram rather than below the entire relationship list',{timeout:30000},async()=>workspace(async page=>{
 await click(page,'Canonical nodes');await click(page,'Inspect ux-seed');await click(page,'Stored graph');await page.getByRole('heading',{name:'Stored directed graph',exact:true}).waitFor();
 assert.equal(await page.locator('.graph-summary').evaluate(n=>!!(n.compareDocumentPosition(document.querySelector('.graph-relationships'))&Node.DOCUMENT_POSITION_FOLLOWING)),true,'graph scope must precede the potentially long relationship list');
},'ux-hub'));

test('uninspected oversized and absent stored labels have distinct explicit identity fallbacks',{timeout:30000},async()=>{
 await workspace(async page=>{
  await click(page,'Canonical nodes');await page.getByRole('heading',{name:'node records',exact:true}).waitFor();
  assert.ok(await page.locator('.record-display-fallback').filter({hasText:'no stored title/name'}).count()>0,'missing display field must be labeled, not silently suppressed');
  await click(page,'Inspect alpha');await page.getByRole('heading',{name:'Record detail',exact:true}).waitFor();
  assert.match(await page.locator('.record-workspace>.record-display-fallback').textContent(),/no stored title\/name/);
 },'linked');
 await workspace(async page=>{
  await click(page,'Canonical nodes');await page.getByRole('heading',{name:'node records',exact:true}).waitFor();
  assert.ok(await page.locator('.record-display-fallback').filter({hasText:'body exceeds bounded preview limit'}).count()>0,'oversized body is uninspected, not untitled');
 },'chunked');
});

test('backend-refused lexical input keeps actionable feedback visible after observation invalidation',{timeout:30000},async()=>workspace(async page=>{
 await click(page,'Text retrieval');await page.getByLabel('Query text',{exact:true}).fill('orchid');await click(page,'Run lexical query');await page.locator('pre.query-context').waitFor();
 await page.getByLabel('Result limit',{exact:true}).fill('0');await click(page,'Run lexical query');
 await page.getByRole('status',{name:'Lexical query feedback'}).getByText(/Operation did not complete \(invalid_range\)/).waitFor({timeout:3000});
 assert.match(await page.locator('#app').textContent(),/Refresh explicitly/);
},'lexical'));
