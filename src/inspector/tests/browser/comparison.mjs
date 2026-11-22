import {installedWorkspace} from "./installed-harness.mjs";
import test from 'node:test';
import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {mkdtemp,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createInterface} from 'node:readline';
const root=resolve(import.meta.dirname,'../../../..');
const {chromium}=await import(pathToFileURL(process.env.INSPECTOR_PLAYWRIGHT??'/home/coreyt/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs'));
async function workspace(action,fixture='lexical',flags=[]){
 if(process.env.INSPECTOR_QUALIFY_BINARY)return installedWorkspace(action,fixture,flags,false);
 const dir=await mkdtemp(`${tmpdir()}/inspector60-browser-`);let child,browser;
 try{
  const db=fixture==='person'?execFileSync(`${root}/.venv/bin/python`,[`${root}/src/inspector/tools/vector-fixture.py`,dir,'person'],{encoding:'utf8'}).trim():execFileSync(`${root}/target/debug/examples/fixture`,[dir,fixture],{encoding:'utf8'}).trim();
  child=spawn(`${root}/target/debug/examples/driver`,[db,`${root}/target/debug/fathomdb-inspector`,...flags],{stdio:['pipe','pipe','pipe']});
  const iter=createInterface({input:child.stdout})[Symbol.asyncIterator]();const info=JSON.parse((await iter.next()).value);
  browser=await chromium.launch({executablePath:process.env.INSPECTOR_CHROMIUM??'/home/coreyt/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome',headless:true,args:['--no-sandbox','--disable-background-networking']});
  const page=await browser.newPage();page.setDefaultTimeout(fixture==='person'?30000:5000);
  await page.goto(pathToFileURL(info.bootstrap).href);await page.waitForURL(`http://${info.address}/`);await page.getByRole('button',{name:'Attach database',exact:true}).click();await page.getByRole('heading',{name:'Database inventory',exact:true}).waitFor();
  await action(page,child,iter);
 }finally{if(browser)await browser.close();if(child&&child.exitCode===null){child.stdin.end('close\n');await new Promise(r=>child.on('exit',r));}await rm(dir,{recursive:true,force:true});}
}
async function orchid(page){
 await page.getByRole('button',{name:'Text retrieval',exact:true}).click();await page.getByLabel('Lexical route',{exact:true}).selectOption('property');await page.getByLabel('Property name',{exact:true}).fill('biography');await page.getByLabel('Query text',{exact:true}).fill('orchid');await page.getByRole('button',{name:'Run lexical query',exact:true}).click();await page.waitForFunction(()=>document.querySelectorAll('.lexical-hit').length===3);
}
const click=(page,name)=>page.getByRole('button',{name,exact:true}).click();

test('actual A/B pins retain original requests and own ranks without rerunning; keyboard union/intersection/origin/Back', {timeout:30000},async()=>workspace(async page=>{
 let queries=0;page.on('request',r=>{if(r.url().endsWith('/api')&&r.postDataJSON()?.request?.op==='lexical')queries++;});
 await orchid(page);const original=await page.locator('pre.query-context').textContent();
 assert.equal(await page.getByRole('button',{name:'Pin A',exact:true}).count(),1,'completed actual observation must expose Pin A');
 await page.getByRole('button',{name:'Pin A',exact:true}).focus();await page.keyboard.press('Enter');await page.getByText('A pinned.',{exact:true}).waitFor();
 await page.getByLabel('Query text',{exact:true}).fill('stone');await click(page,'Run lexical query');await page.waitForFunction(()=>document.querySelectorAll('.lexical-hit').length===1);await click(page,'Pin B');await page.getByText('B pinned.',{exact:true}).waitFor();
 await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();
 assert.equal(await page.locator('.comparison-row').count(),4);assert.match(await page.locator('.comparison-side-A pre').textContent(),/"query": "orchid"/);assert.match(await page.locator('.comparison-side-B pre').textContent(),/"query": "stone"/);
 assert.match(await page.locator('.comparison-row').first().textContent(),/negated_sqlite_bm25/);assert.match(await page.locator('.comparison-row').first().textContent(),/not returned in this bounded observation/);
 await page.getByLabel('Comparison membership',{exact:true}).selectOption('intersection');await page.waitForFunction(()=>document.querySelectorAll('.comparison-row').length===0);assert.equal(await page.locator('.comparison-row').count(),0);await page.getByLabel('Comparison membership',{exact:true}).selectOption('union');await page.waitForFunction(()=>document.querySelectorAll('.comparison-row').length===4);
 const origin=page.getByRole('button',{name:'Open A origin lexical-a',exact:true});await origin.focus();await page.keyboard.press('Enter');await page.getByRole('heading',{name:'Record detail',exact:true}).waitFor();await click(page,'Back');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();assert.equal(await page.evaluate(()=>document.activeElement?.textContent),'Open A origin lexical-a');
 await click(page,'Open A original observation');await page.getByRole('heading',{name:'Text retrieval',exact:true}).waitFor();assert.equal(await page.locator('pre.query-context').textContent(),original);assert.equal(await page.getByLabel('Query text',{exact:true}).inputValue(),'orchid');assert.equal(queries,2);
 await click(page,'Unpin A');await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();assert.equal(await page.locator('.comparison-row').count(),1);assert.match(await page.locator('.comparison-side-A').textContent(),/not pinned/);assert.equal(queries,2);
}));

test('pinned result survives independent source eviction and opens original without requery',{timeout:30000},async()=>workspace(async page=>{
 await orchid(page);const original=await page.locator('pre.query-context').textContent();await click(page,'Pin A');await page.getByText('A pinned.',{exact:true}).waitFor();
 for(let n=0;n<9;n++){const response=page.waitForResponse(r=>r.url().endsWith('/api')&&r.request().postDataJSON().request.op==='page');await click(page,'Canonical nodes');await response;}
 await click(page,'Compare observations');await click(page,'Open A original observation');await page.getByRole('heading',{name:'Text retrieval',exact:true}).waitFor();assert.equal(await page.locator('pre.query-context').textContent(),original);assert.equal(await page.locator('.lexical-hit').count(),3);assert.match(await page.locator('.lexical-workspace').textContent(),/Original source observation is no longer retained/);
}));

test('detach scrubs every retained native/compare DOM consumer and pin slots',{timeout:30000},async()=>workspace(async page=>{
 await orchid(page);await click(page,'Pin A');await page.getByText('A pinned.',{exact:true}).waitFor();await page.evaluate(()=>window.oldNative=document.querySelector('.lexical-workspace'));
 await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();await page.evaluate(()=>window.oldCompare=document.querySelector('.comparison-workspace'));await click(page,'Open A original observation');await page.getByRole('heading',{name:'Text retrieval',exact:true}).waitFor();await page.evaluate(()=>window.otherNative=document.querySelector('.lexical-workspace'));
 await click(page,'Detach database');await page.getByRole('button',{name:'Attach database',exact:true}).waitFor();
 assert.deepEqual(await page.evaluate(()=>[window.oldNative,window.oldCompare,window.otherNative].map(n=>n.textContent)),['','','']);
 await click(page,'Attach database');await page.getByRole('heading',{name:'Database inventory',exact:true}).waitFor();await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();assert.equal(await page.locator('.comparison-row').count(),0);assert.match(await page.locator('.comparison-side-A').textContent(),/not pinned/);
}));

test('source-only eviction scrubs detached indexed DOM while keeping its pinned result',{timeout:30000},async()=>workspace(async page=>{
 await click(page,'Canonical nodes');await click(page,'Inspect lexical-a');await click(page,'Indexed text');await page.getByLabel('Lexical route',{exact:true}).selectOption('property');await page.getByLabel('Property name',{exact:true}).fill('biography');await page.getByLabel('Query text',{exact:true}).fill('orchid');await click(page,'Run lexical query');await page.waitForFunction(()=>document.querySelectorAll('.lexical-hit').length===3);await click(page,'Pin A');await page.getByText('A pinned.',{exact:true}).waitFor();
 await page.evaluate(()=>window.originalIndexed=document.querySelector('[aria-label="Stored indexed text"]'));
 for(let n=0;n<9;n++){const response=page.waitForResponse(r=>r.url().endsWith('/api')&&r.request().postDataJSON().request.op==='page');await click(page,'Canonical nodes');await response;}
 assert.equal(await page.evaluate(()=>window.originalIndexed.textContent),'','source FIFO eviction must scrub its detached indexed DOM as well as its JSON');
 await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();await click(page,'Open A original observation');await page.getByRole('heading',{name:'Text retrieval',exact:true}).waitFor();assert.equal(await page.locator('.lexical-hit').count(),3);
}));

test('canonical page pin releases detached membership controls on detach',{timeout:30000},async()=>workspace(async page=>{
 await click(page,'Canonical nodes');await click(page,'Pin A');await page.getByText('A pinned.',{exact:true}).waitFor();await page.evaluate(()=>window.originalPage=document.querySelector('[role="list"]').closest('section.panel'));
 await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();await click(page,'Detach database');await page.getByRole('button',{name:'Attach database',exact:true}).waitFor();assert.equal(await page.evaluate(()=>window.originalPage.textContent),'','page membership DOM must clear through the common owner cleanup');
}));


test('real person investigation spans full biography, indexed match, head vector/L2/CPU, edge/source/history and exact original Back',{timeout:120000},async()=>workspace(async page=>{
 let textQueries=0;page.on('request',r=>{if(r.url().endsWith('/api')&&r.postDataJSON()?.request?.op==='lexical')textQueries++;});
 await click(page,'Canonical nodes');await page.getByRole('heading',{name:'node records',exact:true}).waitFor();
 assert.equal(await page.getByRole('button',{name:'Inspect person-a',exact:true}).count(),2,'person investigation must include two retained exact revisions');
 await page.getByRole('button',{name:'Inspect person-a',exact:true}).last().click();await page.getByRole('heading',{name:'Record detail',exact:true}).waitFor();assert.match(await page.locator('pre.content').textContent(),/München 🦀/);assert.match(await page.locator('pre.content').textContent(),/END OF COMPLETE BIOGRAPHY/);assert.match(await page.locator('pre.content').textContent(),/9007199254740993/);
 await click(page,'Indexed text');await page.getByRole('heading',{name:'Text retrieval',exact:true}).waitFor();assert.match(await page.locator('[aria-label="Stored indexed text"]').textContent(),/orchid/);
 await page.getByLabel('Query text',{exact:true}).fill('orchid');await click(page,'Run lexical query');await page.locator('.native-snippet mark').first().waitFor();const original=await page.locator('pre.query-context').textContent();await click(page,'Pin A');await page.getByText('A pinned.',{exact:true}).waitFor();
 await page.getByRole('button',{name:'Open exact origin person-a',exact:true}).last().click();await page.getByRole('heading',{name:'Record detail',exact:true}).waitFor();await click(page,'Stored vectors');await page.locator('.stored-vector').first().waitFor();assert.match(await page.locator('.stored-vector').first().textContent(),/window 0/);assert.match(await page.locator('.stored-vector').first().textContent(),/byte start0/);assert.equal(await page.locator('.vector-values').first().evaluate(n=>JSON.parse(n.textContent).length),384);
 await page.getByRole('button',{name:/^Find similar vector row /}).first().click();await page.getByRole('heading',{name:'Stored L2 neighborhood',exact:true}).waitFor();const neighborhood=JSON.parse(await page.locator('pre.vector-context').textContent());assert.equal(neighborhood.scope.direction,'lower_nearer');assert.ok(neighborhood.scope.eligible_compared>=3);await click(page,'Pin B');await page.getByText('B pinned.',{exact:true}).waitFor();
 await page.getByLabel('Semantic query text',{exact:true}).fill('software engineer who grows orchids');await click(page,'Run native hybrid search');await page.getByRole('heading',{name:'Native hybrid results',exact:true}).waitFor();assert.ok(await page.locator('.serving-hit').count()>0);assert.match(await page.locator('pre.serving-context').textContent(),/native_frozen/);
 await click(page,'Back');await page.getByRole('heading',{name:'Record detail',exact:true}).waitFor();await click(page,'Stored graph');await page.getByRole('heading',{name:'Stored directed graph',exact:true}).waitFor();await page.getByRole('button',{name:'Select edge person-knows',exact:true}).click();await click(page,'Open selected record');await page.getByRole('heading',{name:'Record detail',exact:true}).waitFor();assert.match(await page.locator('pre.content').textContent(),/met at the orchid laboratory/);await click(page,'Back');await page.getByRole('heading',{name:'Stored directed graph',exact:true}).waitFor();
 await page.getByRole('button',{name:/^Select node person-a .*person-a-r2/}).click();await click(page,'Investigate selected record');await page.getByRole('heading',{name:'Provenance and retained history',exact:true}).waitFor();assert.match(await page.locator('[aria-label="Recorded investigation links"]').textContent(),/registered dependency to source/);await click(page,'Open recorded source people-source-r1');await page.getByRole('heading',{name:'Record detail',exact:true}).waitFor();assert.match(await page.locator('pre.content').textContent(),/original authored people source/);await click(page,'Back');await page.getByRole('heading',{name:'Provenance and retained history',exact:true}).waitFor();await click(page,'Open retained revision person-a-r1');await page.getByRole('heading',{name:'Record detail',exact:true}).waitFor();assert.match(await page.locator('pre.content').textContent(),/retained early biography/);
 for(const heading of ['Provenance and retained history','Stored directed graph','Record detail','Text retrieval']){await click(page,'Back');await page.getByRole('heading',{name:heading,exact:true}).waitFor();}
 assert.equal(await page.locator('pre.query-context').textContent(),original);assert.equal(await page.getByLabel('Query text',{exact:true}).inputValue(),'orchid');assert.equal(await page.evaluate(()=>document.activeElement?.textContent),'Open exact origin person-a');assert.equal(textQueries,1);
 await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();assert.match(await page.locator('.comparison-side-A').textContent(),/orchid/);assert.match(await page.locator('.comparison-side-B').textContent(),/uncentered_f32_euclidean_l2/);assert.equal(await page.locator('img').count(),0);
},'person'));

test('unpin refreshes native actions and late replacement validation cannot resurrect a slot',{timeout:30000},async()=>workspace(async page=>{
 await orchid(page);await click(page,'Pin A');await page.getByText('A pinned.',{exact:true}).waitFor();assert.equal(await page.getByRole('button',{name:'Replace A',exact:true}).count(),1,'occupied slot must expose explicit Replace A');await click(page,'Unpin A');
 assert.equal(await page.getByRole('button',{name:'Pin A',exact:true}).count(),1,'unpin must refresh cached native pin labels');
 await click(page,'Pin A');await page.getByText('A pinned.',{exact:true}).waitFor();await page.getByLabel('Query text',{exact:true}).fill('stone');await click(page,'Run lexical query');await page.waitForFunction(()=>document.querySelectorAll('.lexical-hit').length===1);
 let release,held;const gate=new Promise(r=>release=r),captured=new Promise(r=>held=r);let holding=true;
 try{
  await page.route('**/api',async route=>{if(!holding||route.request().postDataJSON().request.op!=='check')return route.continue();const response=await route.fetch();held();await gate;await route.fulfill({response});});
  await click(page,'Replace A');await captured;await click(page,'Unpin A');holding=false;release();await page.waitForFunction(()=>document.querySelector('#app').getAttribute('aria-busy')==='false');
  await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();assert.match(await page.locator('.comparison-side-A').textContent(),/not pinned/);assert.equal(await page.locator('.comparison-row').count(),0);
 }finally{holding=false;release?.();}
}));

test('stored and native graph pins preserve original membership, request and rankless values on replacement',{timeout:30000},async()=>workspace(async page=>{
 await click(page,'Canonical nodes');await page.getByRole('button',{name:'Inspect derived',exact:true}).last().click();await click(page,'Stored graph');await page.getByRole('heading',{name:'Stored directed graph',exact:true}).waitFor();const original=await page.locator('pre.graph-context').textContent();await click(page,'Pin A');await page.getByText('A pinned.',{exact:true}).waitFor();
 await click(page,'Run native graph traversal');await page.getByRole('heading',{name:'Actual frozen graph evidence',exact:true}).waitFor();await click(page,'Pin native graph B');await page.getByText('B pinned.',{exact:true}).waitFor();await page.getByLabel('Graph direction',{exact:true}).selectOption('incoming');await click(page,'Apply graph filters');await page.waitForFunction(()=>JSON.parse(document.querySelector('pre.scope').textContent).scope.direction==='incoming');
 await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();assert.match(await page.locator('.comparison-side-A').textContent(),/"direction": "both"/);assert.match(await page.locator('.comparison-side-B').textContent(),/actual_serving_graph_evidence/);assert.match(await page.locator('.comparison-row').first().textContent(),/Rank: not applicable/);assert.doesNotMatch(await page.locator('.comparison-row').first().textContent(),/Original rank: 0/);
 await click(page,'Open A original observation');await page.getByRole('heading',{name:'Stored directed graph',exact:true}).waitFor();assert.equal(await page.locator('pre.graph-context').textContent(),original);await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();await click(page,'Open B original observation');await page.getByRole('heading',{name:'Actual frozen graph evidence',exact:true}).waitFor();assert.match(await page.locator('pre.native-graph-context').textContent(),/derived/);
},'graph'));

test('real bounded graph comparison paginates and cancellation clears its retained DOM',{timeout:30000},async()=>workspace(async page=>{
 await click(page,'Canonical nodes');await click(page,'Inspect alpha');await click(page,'Stored graph');await page.getByRole('heading',{name:'Stored directed graph',exact:true}).waitFor();await click(page,'Pin A');await page.getByText('A pinned.',{exact:true}).waitFor();await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();assert.equal(await page.locator('.comparison-row').count(),50);assert.match(await page.locator('.comparison-count').textContent(),/161 typed entries/);
 const identities=[];for(let i=0;i<4;i++){identities.push(...await page.locator('.comparison-identity').allTextContents());if(i<3){await click(page,'Next comparison page');await page.waitForFunction(start=>document.querySelector('.comparison-count').textContent.includes(`showing ${start}–`),i*50+51);}}
 assert.equal(new Set(identities).size,161);
 const cdp=await page.context().newCDPSession(page);await cdp.send('HeapProfiler.collectGarbage');const idle=(await cdp.send('Runtime.getHeapUsage')).usedSize;let peak=idle;
 for(let cycle=0;cycle<12;cycle++){
  await click(page,'Open A original observation');await page.getByRole('heading',{name:'Stored directed graph',exact:true}).waitFor();await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();assert.equal(await page.locator('.comparison-row').count(),50);peak=Math.max(peak,(await cdp.send('Runtime.getHeapUsage')).usedSize);
 }
 await cdp.send('HeapProfiler.collectGarbage');const retained=(await cdp.send('Runtime.getHeapUsage')).usedSize;
 assert.ok(peak-idle<=512*1024*1024,'actual comparison browser heap growth stays within512MiB');assert.ok(retained-idle<=20*1024*1024,'discarded derived owners do not retain an additional20MiB after twelve native/compare cycles');console.info(`comparison heap: idle=${idle} peak=${peak} retained=${retained};12 actual cycles`);await cdp.detach();
 await page.evaluate(()=>window.canceledCompare=document.querySelector('.comparison-workspace'));await click(page,'Cancel request');await page.getByRole('button',{name:'Attach database',exact:true}).waitFor();assert.equal(await page.evaluate(()=>window.canceledCompare.textContent),'');
},'graph-highdegree'));

test('pins have no FIFO entitlement or additional ten-observation budget',{timeout:30000},async()=>workspace(async page=>{
 await orchid(page);await click(page,'Pin A');await page.getByText('A pinned.',{exact:true}).waitFor();
 for(let n=0;n<10;n++){const done=page.waitForResponse(r=>r.url().endsWith('/api')&&r.request().postDataJSON().request.op==='page');await click(page,'Canonical nodes');await done;}
 await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();assert.equal(await page.locator('.comparison-row').count(),0);assert.match(await page.locator('.comparison-side-A').textContent(),/evicted or invalidated/);
}));

for(let bits=0;bits<8;bits++)test(`D01 comparison membership and metadata obey flags ${bits}`,{timeout:30000},async()=>workspace(async page=>{
 await click(page,'Canonical nodes');await page.getByRole('heading',{name:'node records',exact:true}).waitFor();
 assert.equal(await page.getByRole('button',{name:'Inspect alpha',exact:true}).count(),bits&1?1:2);assert.equal(await page.getByRole('button',{name:'Inspect pending',exact:true}).count(),bits&1?0:1);
 await click(page,'Pin A');await page.getByText('A pinned.',{exact:true}).waitFor();await click(page,'Operational current values');await page.getByRole('heading',{name:'current records',exact:true}).waitFor();await click(page,'Pin B');await page.getByText('B pinned.',{exact:true}).waitFor();await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();
 const alpha=page.locator('.comparison-row').filter({has:page.getByRole('heading',{name:'alpha',exact:true})});assert.equal(await alpha.count(),bits&1?1:2);assert.equal(await page.locator('.comparison-payload').count(),0,'browse/telemetry metadata observations never manufacture cached payloads');assert.doesNotMatch(await page.locator('.comparison-workspace').textContent(),/pending retained body|derived body|source body/);assert.match(await page.locator('.comparison-side-A').textContent(),new RegExp(`"no_show_inactive": ${!!(bits&1)}`));
},'comparison-policy',[...(bits&1?['--no-show-inactive']:[]),...(bits&2?['--show-erased']:[]),...(bits&4?['--show-payloads']:[])]));


for(const back of [false,true])test(`standalone unpinned frozen graph rechecks wall-clock expiry${back?' on Back':''}`,{timeout:30000},async()=>workspace(async page=>{
 await click(page,'Canonical nodes');await page.getByRole('button',{name:'Inspect derived',exact:true}).last().click();await click(page,'Stored graph');await page.getByRole('heading',{name:'Stored directed graph',exact:true}).waitFor();
 const nativeResponse=page.waitForResponse(r=>r.url().endsWith('/api')&&r.request().postDataJSON().request.op==='graph_serving');await click(page,'Run native graph traversal');const original=await (await nativeResponse).json();await page.getByRole('heading',{name:'Actual frozen graph evidence',exact:true}).waitFor();const target=original.items.find(item=>item.target.revision_id==='alpha-expiring-r1');assert.ok(target,'actual frozen graph must select the authored expiring target');
 const boundary=Number(target.target.valid_until)*1000;assert.ok(boundary>Date.now(),'positive begins before the real validity boundary');
 await click(page,'Pin native graph A');await page.getByText('A pinned.',{exact:true}).waitFor();await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();await click(page,'Open A original observation');await page.getByRole('heading',{name:'Actual frozen graph evidence',exact:true}).waitFor();await click(page,'Unpin A');assert.match(await page.locator('#app').textContent(),/alpha expires after unpin/);
 if(back){await click(page,'Open native origin parallel-a-r1');await page.getByRole('heading',{name:'Record detail',exact:true}).waitFor();}
 await page.waitForFunction(boundary=>Date.now()>boundary+700,boundary,{timeout:12000});
 if(back){await click(page,'Back');await page.waitForFunction(()=>document.querySelector('#app').getAttribute('aria-busy')==='false');await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));}
 assert.equal((await page.locator('#app').textContent()).includes('alpha expires after unpin'),false,'unpinned frozen owner must withdraw expired content without a generation change');
},'comparison-expiry',['--no-show-inactive']));


test('actual large observations refuse derived comparison within the shared20MiB budget and keep originals usable',{timeout:60000},async()=>workspace(async page=>{
 let queries=0;page.on('request',r=>{if(r.url().endsWith('/api')&&r.postDataJSON()?.request?.op==='lexical')queries++;});
 await click(page,'Text retrieval');await page.getByLabel('Query text',{exact:true}).fill('needle');await page.getByLabel('Result limit',{exact:true}).fill('100');
 const reads=[];
 for(const side of ['A','B']){
  const response=page.waitForResponse(r=>r.url().endsWith('/api')&&r.request().postDataJSON().request.op==='lexical');await click(page,'Run lexical query');const data=await (await response).json();
  assert.equal(data.scope.native_returned,100,'fixture authors exactly100matching physical records');assert.ok(data.items.length>0&&data.items.length<=200);assert.ok(Buffer.byteLength(JSON.stringify(data))<=2*1024*1024,'each actual response retains the2MiB contract');reads.push(data);
  await page.waitForFunction(count=>document.querySelectorAll('.lexical-hit').length===count,data.items.length);await click(page,`Pin ${side}`);await page.getByText(`${side} pinned.`,{exact:true}).waitFor();
 }
 assert.ok(reads.reduce((n,d)=>n+Buffer.byteLength(JSON.stringify(d)),0)>3*1024*1024,'real source bytes reach the derived reservation boundary');
 await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();await page.getByRole('alert').waitFor();assert.match(await page.getByRole('alert').textContent(),/shared observation byte budget exhausted/);assert.equal(await page.locator('.comparison-row').count(),0);
 await click(page,'Open A original observation');await page.getByRole('heading',{name:'Text retrieval',exact:true}).waitFor();assert.equal(await page.locator('.lexical-hit').count(),reads[0].items.length);assert.equal(queries,2,'refusal and original return never rerun the query');
},'comparison-envelope'));

test('each side keeps the actual native admission/population and index metadata',{timeout:30000},async()=>workspace(async page=>{
 const response=page.waitForResponse(r=>r.url().endsWith('/api')&&r.request().postDataJSON().request.op==='lexical');await orchid(page);const observed=await (await response).json();assert.ok(observed.admission,'native query has its own measured admission evidence');
 await click(page,'Pin A');await page.getByText('A pinned.',{exact:true}).waitFor();await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();const side=JSON.parse(await page.locator('.comparison-side-A pre').textContent());assert.deepEqual(side.admission,observed.admission,'comparison must preserve the actual source population/admission facts');assert.deepEqual(side.metadata,observed.metadata,'effective index metadata keeps its original observation');
}));
