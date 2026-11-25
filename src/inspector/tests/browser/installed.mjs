import test from 'node:test';
import {createHash} from 'node:crypto';
import {request as httpRequest} from 'node:http';
import {DatabaseSync} from 'node:sqlite';
import assert from 'node:assert/strict';
import {readFile,writeFile,copyFile,access,mkdtemp,mkdir,symlink,readdir,lstat,readlink,rename,rm} from 'node:fs/promises';
import {setTimeout as delay} from 'node:timers/promises';
import {installedWorkspace} from './installed-harness.mjs';
import {boundedExecFileSync,clipboardEquals} from './helpers.mjs';
assert.ok(process.env.INSPECTOR_QUALIFY_BINARY,'this suite requires an extracted installed target');
const click=(page,name)=>page.getByRole('button',{name,exact:true}).click();
async function client(page,context){
 const origin=new URL(page.url()).origin;
 const cookie=(await context.cookies()).map(c=>`${c.name}=${c.value}`).join('; ');
 const handshake=await(await fetch(`${origin}/handshake`,{headers:{Cookie:cookie}})).json();
 const pair={build:handshake.build,protocol:handshake.protocol,launch:handshake.launch,attachment:handshake.attachment};
 async function api(request,override={},headers={}){
  const response=await fetch(`${origin}/api`,{method:'POST',headers:{Cookie:cookie,Origin:origin,'Content-Type':'application/json',...headers},body:JSON.stringify({...pair,...override,request}),signal:AbortSignal.timeout(35000)});
  const bytes=await response.text();const value=JSON.parse(bytes);assert.ok(Buffer.byteLength(bytes)<=2*1024*1024,'actual installed response envelope cap');
  if(response.status===200&&'attachment'in value)pair.attachment=value.attachment;
  return {response,value,bytes};
 }
 return {api,pair,origin,cookie};
}
async function until(action,ms=30000){const end=performance.now()+ms;while(performance.now()<end){if(await action())return;await delay(5);}throw Error('installed native progress/cleanup deadline');}
async function gone(pid){try{await access(`/proc/${pid}`);return false;}catch{return true;}}
async function cpu(pid){const stat=await readFile(`/proc/${pid}/task/${pid}/stat`,'utf8');const f=stat.slice(stat.lastIndexOf(')')+2).split(' ');return Number(f[11])+Number(f[12]);}
async function worker(meta){for(const pid of await meta.tree()){try{if((await readFile(`/proc/${pid}/cmdline`,'utf8')).includes('--session-worker'))return pid;}catch{}}throw Error('actual owned native worker required');}
function lockFree(db){boundedExecFileSync('flock',['--nonblock',`${db}.lock`,'true'],{},10000);}

test('installed structured document/tree, exact copy, hostile content and keyboard Back',{timeout:45000},async()=>installedWorkspace(async(page,_child,_iter,meta)=>{
 await meta.context.grantPermissions(['clipboard-read','clipboard-write'],{origin:new URL(page.url()).origin});
 const inspect=page.getByRole('button',{name:'Inspect structured',exact:true});await inspect.focus();await page.keyboard.press('Enter');
 await page.getByRole('heading',{name:'Record detail',exact:true}).waitFor();await page.getByText('9007199254740993123456789',{exact:true}).waitFor();
 await page.getByRole('button',{name:'Expand nested',exact:true}).focus();await page.keyboard.press('Enter');await page.getByText('null',{exact:true}).last().waitFor();
 await click(page,'Copy exact identity');await clipboardEquals(page,'structured');await click(page,'Copy value integer');await clipboardEquals(page,'9007199254740993123456789');assert.equal(await page.locator('img').count(),0);
 await click(page,'Back');await page.getByRole('button',{name:'Inspect structured',exact:true}).waitFor();assert.equal(await page.evaluate(()=>document.activeElement.textContent),'Inspect structured');
 await click(page,'Detach database');await page.getByRole('button',{name:'Attach database',exact:true}).waitFor();lockFree(meta.db);
},'default'));

test('installed chunk navigation and full original content copy',{timeout:45000},async()=>installedWorkspace(async(page,_child,_iter,meta)=>{
 await meta.context.grantPermissions(['clipboard-read','clipboard-write'],{origin:new URL(page.url()).origin});
 await click(page,'Inspect chunked');await page.waitForFunction(()=>document.querySelector('pre.content')?.textContent==='A'.repeat(65536));
 await click(page,'Next content chunk');await page.waitForFunction(()=>document.querySelector('pre.content')?.textContent==='B'.repeat(65536));await click(page,'Next content chunk');await page.waitForFunction(()=>document.querySelector('pre.content')?.textContent==='C'.repeat(1200));
 await click(page,'Previous content chunk');await page.waitForFunction(()=>document.querySelector('pre.content')?.textContent==='B'.repeat(65536));await click(page,'Copy content');await clipboardEquals(page,'A'.repeat(65536)+'B'.repeat(65536)+'C'.repeat(1200));
},'chunked'));

test('installed authentication/origin/Host/build/session/control and candidate refusals',{timeout:45000},async()=>installedWorkspace(async(page,_child,_iter,meta)=>{
 const {api,origin,pair,cookie}=await client(page,meta.context);
 const pid=await worker(meta);const env=await readFile(`/proc/${pid}/environ`,'utf8');assert.doesNotMatch(env,/inspector70-parent|HF_TOKEN|OPENAI_API_KEY|FATHOMDB_PERF_/);assert.match(env,/FATHOMDB_EMBED_DEVICE=cpu/);const oracle=new DatabaseSync(meta.db,{readOnly:true});try{assert.notEqual(oracle.prepare('PRAGMA application_id').get().application_id,987654);}finally{oracle.close();}

 for(const headers of [{Cookie:''},{Origin:'https://evil.invalid'}])assert.equal((await api({op:'catalog'},{},headers)).response.status,403);
 const wrongHost=await new Promise((resolve,reject)=>{const req=httpRequest(`${origin}/handshake`,{headers:{Cookie:cookie,Host:'evil.invalid'}},res=>{res.resume();res.on('end',()=>resolve(res.statusCode));});req.on('error',reject);req.end();});assert.equal(wrongHost,403);
 for(const override of [{build:'0'.repeat(64)},{protocol:99},{launch:'0'.repeat(64)},{attachment:'0'.repeat(64)}])assert.equal((await api({op:'catalog'},override)).response.status,409);
 assert.equal((await api({op:'fixture',control:'pause_next'})).value.status,'unsupported');
 assert.equal((await api({op:'catalog',origin:'fixture',control:{control:'pause_next'}})).value.status,'unsupported');
 assert.equal((await fetch(`${origin}/fixture`,{headers:{Cookie:cookie}})).status,403);
 assert.equal((await api({op:'detach'})).value.status,'complete');lockFree(meta.db);
 const bytes=await readFile(meta.db);const refused=`${meta.dir}/older.sqlite`;const old=Buffer.from(bytes);old.writeUInt32BE(37,60);await writeFile(refused,old);await writeFile(`${refused}.lock`,'existing sentinel');
 const response=(await api({op:'attach',path:refused})).value;assert.equal(response.status,'refused');assert.equal(response.error,'unsupported_schema');assert.equal(response.observed_schema,37);assert.equal(response.supported_schema,39);assert.deepEqual(await readFile(refused),old);
 assert.equal((await api({op:'attach',path:meta.db})).value.status,'complete');
 assert.equal(pair.build,JSON.parse(boundedExecFileSync(meta.binary,['--build-description'],{encoding:'utf8'},15000)).build);
},'default',[],false));

test('installed aggregate F06 queue/cancel/retry/native fault/detach and bounded reads',{timeout:240000},async()=>installedWorkspace(async(page,_child,_iter,meta)=>{
 const {api}=await client(page,meta.context);const report={};const oracle=new DatabaseSync(meta.db,{readOnly:true});try{assert.ok(oracle.prepare('SELECT count(*) AS n FROM canonical_nodes').get().n>=100000);assert.ok(oracle.prepare('SELECT count(*) AS n FROM canonical_edges').get().n>=10000);assert.ok(oracle.prepare('SELECT count(*) AS n FROM operational_mutations').get().n>=100000);assert.ok(oracle.prepare('SELECT max(length(CAST(body AS BLOB))) AS n FROM canonical_nodes').get().n>10*1024*1024);}finally{oracle.close();}
 const catalog=(await api({op:'catalog'})).value;assert.ok(['partial','complete'].includes(catalog.status));assert.ok(catalog.scope.examined<=100000);assert.equal(catalog.scope.row_cap,100000);assert.equal(catalog.scope.exact,false);
 const selected=(await api({op:'page',class:'node',after:'0',limit:200,filter:{identity:'dog'}})).value;assert.equal(selected.items.length,1);const handle=selected.items[0].handle;
 const vectors=(await api({op:'vectors',handle})).value;assert.equal(vectors.items.length,1);
 const neighbors=(await api({op:'neighborhood',handle,vector_rowid:vectors.items[0].vector_rowid,after:'0',limit:10})).value;assert.equal(neighbors.items.length,2);
 assert.equal((await api({op:'serving',query:'puppy',limit:100})).value.status,'budget_exhausted');assert.equal((await api({op:'vectors',handle})).value.status,'complete');
 await meta.context.grantPermissions(['clipboard-read','clipboard-write'],{origin:new URL(page.url()).origin});
 await click(page,'Canonical nodes');await page.getByRole('heading',{name:'node records',exact:true}).waitFor();await page.getByLabel('Identity filter',{exact:true}).fill('scale-large');await click(page,'Apply filters');
 await page.getByText('No matches in the examined range; incomplete work or classification cannot prove absence.',{exact:true}).waitFor();await click(page,'Next page');const large=page.getByRole('button',{name:'Inspect scale-large',exact:true});await large.waitFor();await large.focus();await page.keyboard.press('Enter');await page.getByRole('heading',{name:'Record detail',exact:true}).waitFor();
 const expected='{"integer":9007199254740993123,"large":"'+'é🦀'.repeat(1800000)+'"}';await page.getByRole('button',{name:'Copy content',exact:true}).focus();await page.keyboard.press('Enter');await clipboardEquals(page,expected);
 await click(page,'Refresh this view');await page.getByRole('heading',{name:'Record detail',exact:true}).waitFor();await click(page,'Back');await page.getByRole('button',{name:'Refresh this view',exact:true}).waitFor();await click(page,'Refresh this view');await page.getByRole('heading',{name:'node records',exact:true}).waitFor();assert.equal(await page.getByLabel('Identity filter',{exact:true}).inputValue(),'scale-large');
 await click(page,'Canonical nodes');await page.getByRole('heading',{name:'node records',exact:true}).waitFor();await page.getByLabel('Identity filter',{exact:true}).fill('alpha');await click(page,'Apply filters');await click(page,'Inspect alpha');await page.getByRole('button',{name:'Stored graph',exact:true}).focus();await page.keyboard.press('Enter');await page.getByRole('heading',{name:'Stored directed graph',exact:true}).waitFor();
 for(let n=0;n<6;n++){assert.ok(await page.locator('.graph-edge').count()>0);assert.ok(await page.locator('.graph-edge,.graph-node').count()<=500);await click(page,'Collapse selected node');await page.waitForFunction(()=>document.querySelectorAll('.graph-edge').length===0);await click(page,'Expand selected node');await page.waitForFunction(()=>document.querySelectorAll('.graph-edge').length>0);}
 await click(page,'Back');await page.getByRole('heading',{name:'Record detail',exact:true}).waitFor();assert.equal(await page.evaluate(()=>document.activeElement?.textContent),'Stored graph');
 await click(page,'Canonical nodes');await page.getByRole('heading',{name:'node records',exact:true}).waitFor();await click(page,'Pin A');await page.getByText('A pinned.',{exact:true}).waitFor();
 for(let n=0;n<10;n++){const done=page.waitForResponse(r=>r.url().endsWith('/api')&&r.request().postDataJSON().request.op==='page');await click(page,'Canonical nodes');await done;}
 await click(page,'Compare observations');await page.getByRole('heading',{name:'Compare observations',exact:true}).waitFor();assert.equal(await page.locator('.comparison-row').count(),0);assert.match(await page.locator('.comparison-side-A').textContent(),/evicted or invalidated/);report.ui={full_10800042_byte_clipboard:true,keyboard_back_refresh:true,graph_cycles:6,pin_eviction:true};
 async function active(){const pid=await worker(meta),start=await cpu(pid);let completed=false;const pending=api({op:'catalog'}).then(r=>{completed=true;return r.value;});await until(async()=>completed||(await cpu(pid))>start,5000);assert.equal(completed,false,'real expensive operation must remain pending');return {pid,pending};}
 let current=await active();let began=performance.now();const queued=Array.from({length:9},()=>api({op:'page',class:'node',after:'0',limit:1,filter:{}}));await delay(20);
 const ack=(await api({op:'cancel'})).value;const ackMs=performance.now()-began;assert.equal(ack.status,'cancelled');assert.ok(ackMs<=250,`cancel ACK${ackMs}ms`);const late=await current.pending;assert.equal(late.status,'cancelled');assert.equal(late.counts,undefined);
 const waiting=await Promise.all(queued);assert.equal(waiting.filter(r=>r.value.status==='busy').length,1,'exactly one request exceeds eight waiting slots');assert.ok(waiting.every(r=>['busy','cancelled','expired'].includes(r.value.status)));await until(()=>gone(current.pid));lockFree(meta.db);assert.ok(performance.now()-began<30000);report.cancel={ack_ms:ackMs,quiescence_lock_ms:performance.now()-began};
 assert.equal((await api({op:'attach',path:meta.db})).value.status,'complete');current=await active();began=performance.now();assert.ok((await meta.tree()).includes(current.pid));process.kill(current.pid,'SIGKILL');const fault=await current.pending;assert.equal(fault.error,'worker_disconnected');assert.equal(fault.counts,undefined);await until(()=>gone(current.pid));lockFree(meta.db);assert.ok(performance.now()-began<30000);report.fault_ms=performance.now()-began;
 assert.equal((await api({op:'attach',path:meta.db})).value.status,'complete');current=await active();began=performance.now();const detached=api({op:'detach'});assert.ok(['partial','complete'].includes((await current.pending).status));assert.equal((await detached).value.status,'complete');await until(()=>gone(current.pid));lockFree(meta.db);assert.ok(performance.now()-began<30000);report.detach_ms=performance.now()-began;
 console.log(JSON.stringify({qualification:'installed_F06_native_controls',...report}));
},'f06',[],false));

for(const variant of ['training','unseen'])test(`installed study ${variant} fixture has intended distinct revisions/relationships`,{timeout:45000},async()=>installedWorkspace(async page=>{
 const prefix=variant==='training'?'practice':'field';await click(page,'Canonical nodes');await page.getByRole('heading',{name:'node records',exact:true}).waitFor();assert.equal(await page.getByRole('button',{name:`Inspect ${prefix}-${variant==='training'?'b':'a'}`,exact:true}).count(),2);
 await page.getByRole('button',{name:`Inspect ${prefix}-a`,exact:true}).last().click();assert.match(await page.locator('pre.content').textContent(),/9007199254740993/);assert.match(await page.locator('pre.content').textContent(),/END OF COMPLETE FIELD NOTE/);
 await click(page,'Stored graph');await page.getByRole('heading',{name:'Stored directed graph',exact:true}).waitFor();assert.ok(await page.locator('svg').count()>0);
},`study-${variant}`,[],false));


test('installed actual lexical native deadline expires and releases worker without late data',{timeout:90000},async()=>installedWorkspace(async(page,_child,_iter,meta)=>{
 const {api}=await client(page,meta.context),pid=await worker(meta);const alphabet='abcdefghijklmnopqrstuvwxyz0123456789';const terms=[...alphabet,...[...alphabet].flatMap(a=>[...alphabet].map(b=>a+b))].join(' ');assert.ok(Buffer.byteLength(terms)<=4096);const began=performance.now();const result=(await api({op:'lexical',route:'bm25f',query:terms,filter:{},view:{},limit:100})).value;const elapsed=performance.now()-began;
 assert.equal(result.status,'budget_exhausted');assert.equal(result.items,undefined);assert.equal(result.attachment,null);assert.ok(elapsed>=29000&&elapsed<=32000,`real native deadline${elapsed}ms`);await until(()=>gone(pid));lockFree(meta.db);console.log(JSON.stringify({qualification:'installed_native_deadline',elapsed_ms:elapsed,error:result.error,no_late_payload:true}));
},'lexical-deadline',[],false));


for(let bits=0;bits<8;bits++)test(`installed retained content/chunk/export and inactive gates ${bits}`,{timeout:45000},async()=>installedWorkspace(async(page,_child,_iter,meta)=>{
 const {api}=await client(page,meta.context);const rows=(await api({op:'page',class:'node',after:'0',limit:200,filter:{identity:'alpha'}})).value.items;assert.equal(rows.length,bits&1?1:2);const item=rows.at(-1),handle=item.handle;
 const detail=(await api({op:'detail',handle})).value,chunk=(await api({op:'chunk',handle,field:'body',offset:'0',limit:262144})).value;const propDetail=(await api({op:'detail',handle,field:'properties'})).value,propChunk=(await api({op:'chunk',handle,field:'properties',offset:'0',limit:262144})).value,propTree=(await api({op:'tree',handle,field:'properties',start:'0',after:null})).value;const destination=`${meta.dir}/retained.txt`;const exported=(await api({op:'export',handle,destination})).value;
 if(bits&2){assert.equal(item.properties_status,'available');assert.ok(item.properties_bytes>0);assert.match(item.properties,/ERASED_SECRET_PROP_VALUE/);assert.equal(detail.content.text,'current comparison needle');assert.equal(chunk.text,'current comparison needle');assert.match(propDetail.content.text,/ERASED_SECRET_PROP_VALUE/);assert.match(propChunk.text,/ERASED_SECRET_PROP_VALUE/);assert.equal(propTree.status,'complete');assert.equal(exported.status,'complete');const bytes=await readFile(destination,'utf8');assert.equal(bytes.slice(bytes.indexOf('\n')+1),'current comparison needle');assert.notEqual((await api({op:'export',handle,destination})).value.status,'complete');}
 else {assert.equal(item.properties_status,'refused');for(const k of ['properties','properties_bytes','properties_storage'])assert.equal(k in item,false);assert.equal(detail.status,'refused');assert.equal(chunk.status,'refused');assert.equal(propDetail.status,'refused');assert.equal(propChunk.status,'refused');assert.equal(propTree.status,'refused');assert.notEqual(exported.status,'complete');await assert.rejects(access(destination));}
},'comparison-policy',[...(bits&1?['--no-show-inactive']:[]),...(bits&2?['--show-erased']:[]),...(bits&4?['--show-payloads']:[])],false));

for(let bits=0;bits<8;bits++)test(`installed autonomous absence and independent telemetry gates ${bits}`,{timeout:45000},async()=>installedWorkspace(async(page,_child,_iter,meta)=>{
 const {api}=await client(page,meta.context);const oracle=new DatabaseSync(meta.db,{readOnly:true});try{assert.equal(oracle.prepare("SELECT count(*) AS n FROM canonical_nodes WHERE logical_id IN ('source','derived')").get().n,0);}finally{oracle.close();}
 for(const identity of ['source','derived'])assert.equal((await api({op:'page',class:'node',after:'0',limit:200,filter:{identity}})).value.items.length,0);
 const logs=(await api({op:'page',class:'log',after:'0',limit:200,filter:{collection:'erasure_pending_redaction'}})).value.items;assert.equal(logs.length,1);const handle=logs[0].handle;const detail=(await api({op:'detail',handle})).value,chunk=(await api({op:'chunk',handle,field:'body',offset:'0',limit:262144})).value;
 if(bits&4){assert.match(detail.content.text,/l:source/);assert.doesNotMatch(detail.content.text,/source body/);assert.match(chunk.text,/l:source/);}else{assert.equal(detail.status,'refused');assert.equal(chunk.status,'refused');}
},'erasure',[...(bits&1?['--no-show-inactive']:[]),...(bits&2?['--show-erased']:[]),...(bits&4?['--show-payloads']:[])],false));

async function inventory(directory){const result={};async function walk(path){const stat=await lstat(path);const item={mode:stat.mode,uid:stat.uid,gid:stat.gid,ino:stat.ino,size:stat.size,mtime:stat.mtimeMs,ctime:stat.ctimeMs};if(stat.isFile())item.sha256=createHash('sha256').update(await readFile(path)).digest('hex');if(stat.isSymbolicLink())item.target=await readlink(path);result[path.slice(directory.length)]=item;if(stat.isDirectory())for(const name of (await readdir(path)).sort())await walk(`${path}/${name}`);}await walk(directory);return result;}
const sourceCache=process.env.INSPECTOR_SOURCE_CACHE??`${process.env.XDG_CACHE_HOME??`${process.env.HOME}/.cache`}/fathomdb/embedders/0b2926f8a9b1`;
for(const mode of ['missing','partial','corrupt','replaced','racing'])test(`installed cached asset ${mode} admission preserves cache and model-free inspection`,{timeout:45000},async()=>{
 const cache=await mkdtemp('/tmp/i70-cache-');const target=`${cache}/fathomdb/embedders/0b2926f8a9b1`;await mkdir(target,{recursive:true});let stop=false,racer;
 try{
  for(const name of ['config.json','tokenizer.json','model.safetensors'])if(mode!=='missing'&&(mode!=='partial'||name==='config.json'))await symlink(`${sourceCache}/${name}`,`${target}/${name}`);
  if(['corrupt','replaced'].includes(mode)){await writeFile(`${target}/replacement`,mode==='corrupt'?'partial':'{"hidden_size":384,"model_type":"bert"}');await rename(`${target}/replacement`,`${target}/config.json`);}
  const before=await inventory(cache);
  if(mode==='racing')racer=(async()=>{let n=0;const valid=await readFile(`${sourceCache}/config.json`);while(!stop){await writeFile(`${target}/replacement`,n++%2?valid:'{"hidden_size":384,"model_type":"bert"}');await rename(`${target}/replacement`,`${target}/config.json`);await delay(1);}})();
  await installedWorkspace(async(page,_child,_iter,meta)=>{const {api}=await client(page,meta.context);const catalog=(await api({op:'catalog'})).value;const available=catalog.database.model_runtime==='available_cached_cpu';if(mode!=='racing')assert.equal(available,false);const serving=(await api({op:'serving',query:'puppy',limit:10})).value;if(available){assert.ok(serving.query_context.vector_hits>0);assert.equal(serving.query_context.text_hits,0);}else assert.equal(serving.error,'model_unavailable');const lexical=(await api({op:'lexical',route:'body',query:'domestic',filter:{},view:{},limit:10})).value;assert.ok(lexical.items.length>0);},'vector',[],false,{XDG_CACHE_HOME:cache});
  stop=true;if(racer)await racer;if(mode!=='racing')assert.deepEqual(await inventory(cache),before);else assert.deepEqual((await readdir(target)).sort(),['config.json','model.safetensors','tokenizer.json']);
 }finally{stop=true;if(racer)await racer;await rm(cache,{recursive:true,force:true});}
});

test('installed keyboard cancellation of actual CPU work clears content and permits UI retry/detach',{timeout:45000},async()=>installedWorkspace(async(page,_child,_iter,meta)=>{
 await click(page,'Canonical nodes');await click(page,'Inspect dog');await page.getByRole('heading',{name:'Record detail',exact:true}).waitFor();await page.evaluate(()=>window.cancelledContent=document.querySelector('pre.content'));
 const {api}=await client(page,meta.context),pid=await worker(meta);const processCpu=async()=>{const stat=await readFile(`/proc/${pid}/stat`,'utf8'),fields=stat.slice(stat.lastIndexOf(')')+2).split(' ');return Number(fields[11])+Number(fields[12]);};const start=await processCpu();let completed=false,early;const pending=api({op:'serving',query:'puppy '.repeat(128),limit:100}).then(r=>{completed=true;early=r.value;return r.value;});
 await until(async()=>completed||(await processCpu())>start,5000);assert.equal(completed,false,`actual native CPU work remains pending before keyboard cancellation: ${JSON.stringify(early)}`);
 const response=page.waitForResponse(r=>r.url().endsWith('/api')&&r.request().postDataJSON()?.request?.op==='cancel');const began=performance.now();await page.getByRole('button',{name:'Cancel request',exact:true}).focus();await page.keyboard.press('Enter');const cancelled=await(await response).json();assert.equal(cancelled.status,'cancelled');assert.ok(performance.now()-began<=250);
 await page.getByRole('button',{name:'Attach database',exact:true}).waitFor();assert.equal(await page.locator('pre.content').count(),0);assert.equal(await page.evaluate(()=>window.cancelledContent.textContent),'');const late=await pending;assert.equal(late.status,'cancelled');assert.equal(late.items,undefined);await until(()=>gone(pid));lockFree(meta.db);
 await page.getByRole('button',{name:'Attach database',exact:true}).focus();await page.keyboard.press('Space');await page.getByRole('heading',{name:'Database inventory',exact:true}).waitFor();await click(page,'Detach database');await page.getByRole('button',{name:'Attach database',exact:true}).waitFor();lockFree(meta.db);
},'vector',[],false));
