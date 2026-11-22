import assert from 'node:assert/strict';
import {spawn,execFileSync} from 'node:child_process';
import {mkdtemp,mkdir,cp,readdir,readFile,rm} from 'node:fs/promises';
import {setTimeout as delay} from 'node:timers/promises';
import {tmpdir} from 'node:os';
import {resolve,join} from 'node:path';
import {pathToFileURL} from 'node:url';
async function identity(pid){try{const f=(await readFile(`/proc/${pid}/stat`,'utf8')).split(') ').slice(1).join(') ').split(' ');return {start:f[19],live:f[0]!=='Z'};}catch{return null;}}
const root=resolve(import.meta.dirname,'../../../..');
const {chromium}=await import(pathToFileURL(process.env.INSPECTOR_PLAYWRIGHT??'/home/coreyt/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs'));
async function until(action,ms=30000){const end=Date.now()+ms;let error;while(Date.now()<end){try{const result=await action();if(result)return result;}catch(e){error=e;}await delay(20);}throw error??new Error('installed launch deadline');}
async function children(pid){try{const result=[];for(const tid of await readdir(`/proc/${pid}/task`)){try{result.push(...(await readFile(`/proc/${pid}/task/${tid}/children`,'utf8')).trim().split(/\s+/).filter(Boolean).map(Number));}catch{}}return [...new Set(result)];}catch{return [];}}
async function tree(pid){return [pid,...(await Promise.all((await children(pid)).map(tree))).flat()];}
async function rss(pid){try{return Number((await readFile(`/proc/${pid}/status`,'utf8')).match(/^VmRSS:\s+(\d+)/m)?.[1]??0)*1024;}catch{return 0;}}
async function cmdline(pid){try{return (await readFile(`/proc/${pid}/cmdline`,'utf8')).split(/[\0\s]+/).filter(Boolean);}catch{return [];}}
async function findBrowserPid(supervisorPid,profilePath){
 const profileArg=`--user-data-dir=${profilePath}`;
 const pids=await tree(supervisorPid);
 for(const pid of pids){
  const argv=await cmdline(pid);
  if(argv.includes(profileArg)&&!argv.some(arg=>arg.startsWith('--type=')))return pid;
 }
 return null;
}
export async function installedWorkspace(action,fixture='linked',flags=[],nodes=true,environment={}){
 const binary=process.env.INSPECTOR_QUALIFY_BINARY;
 assert.ok(binary?.startsWith('/'),'installed binary must be explicit and absolute');
 const dir=await mkdtemp(join(tmpdir(),'i70-'));let child,supervisorPid,browser,exited,sample;let closed=false;const owned=new Map();
 try{
  let db;
  if(['person','vector','study-training','study-unseen','f06','lexical-deadline','review'].includes(fixture)){
   const directory=join(dir,'fixture');await mkdir(directory);await cp(join(process.env.INSPECTOR_FIXTURE_DIR,fixture,'work'),directory,{recursive:true});db=join(directory,'fixture.sqlite');
  }else db=execFileSync(process.env.INSPECTOR_FIXTURE_BINARY??join(root,'target/debug/examples/fixture'),fixture==='default'?[dir]:[dir,fixture],{encoding:'utf8',timeout:600000,env:{PATH:process.env.PATH,HOME:process.env.HOME,LANG:'C.UTF-8',LC_ALL:'C.UTF-8'}}).trim();
  const traceOutput=process.env.INSPECTOR_TRACE_OUTPUT;
  const spawnEnv={...process.env,...environment,TMPDIR:dir};
  delete spawnEnv.INSPECTOR_TRACE_OUTPUT;
  if(traceOutput){
   assert.ok(traceOutput.startsWith('/'),'INSPECTOR_TRACE_OUTPUT must be absolute');
   child=spawn('strace',['-ff','-e','trace=network,%file,process','-o',traceOutput,binary,...flags,db],{env:spawnEnv,stdio:['ignore','pipe','pipe']});
  }else{
   child=spawn(binary,[...flags,db],{env:spawnEnv,stdio:['ignore','pipe','pipe']});
  }
  exited=new Promise(r=>child.once('exit',(code,signal)=>r({code,signal})));let stderr='';child.stderr.on('data',b=>stderr+=b);
  const profile=await until(async()=>{assert.equal(child.exitCode,null,stderr);for(const entry of await readdir(dir)){const path=join(dir,entry,'browser');try{const port=await readFile(join(path,'DevToolsActivePort'),'utf8');return {path,port:Number(port.split('\n')[0])};}catch{}}});
  supervisorPid=traceOutput?await until(async()=>{const kids=await children(child.pid);for(const pid of kids){const argv=await cmdline(pid);if(argv[0]===binary)return pid;}return null;}):child.pid;
  browser=await chromium.connectOverCDP(`http://127.0.0.1:${profile.port}`);
  const context=browser.contexts()[0];const page=await until(async()=>context.pages().find(p=>p.url().startsWith('http://127.0.0.1:')));
  page.setDefaultTimeout(30000);const external=[];page.on('request',r=>{if(!r.url().startsWith(new URL(page.url()).origin))external.push(r.url());});
  const cdp=await context.newCDPSession(page);const heap=async()=>{const h=await cdp.send('Runtime.getHeapUsage');return h.usedSize+(h.embedderHeapUsedSize??0)+(h.backingStorageSize??0);};
  const heapStart=await heap();let heapPeak=heapStart,servicePeak=0,browserRssPeak=0,totalRssPeak=0,samples=0,sampling=false,measurementError;
  const browserPid=await until(()=>findBrowserPid(supervisorPid,profile.path));
  assert.ok((await tree(supervisorPid)).includes(browserPid),'native browser belongs to this supervisor');
  async function measure(){
   const all=await tree(supervisorPid),serviceTree=new Set(all),browserTree=new Set(await tree(browserPid));
   // Unique private launch paths also identify Chromium helpers that immediately reparent.
   for(const candidate of await readdir('/proc'))if(/^\d+$/.test(candidate)){try{if((await readFile(`/proc/${candidate}/cmdline`,'utf8')).includes(resolve(profile.path,'..')))all.push(Number(candidate));}catch{}}
   const ids=[...new Set(all)];const values=await Promise.all(ids.map(rss));let service=0,browserRss=0;
   for(let n=0;n<ids.length;n++){const id=await identity(ids[n]);if(id)owned.set(ids[n],id.start);if(browserTree.has(ids[n])||!serviceTree.has(ids[n]))browserRss+=values[n];else service+=values[n];}
   servicePeak=Math.max(servicePeak,service);browserRssPeak=Math.max(browserRssPeak,browserRss);totalRssPeak=Math.max(totalRssPeak,service+browserRss);heapPeak=Math.max(heapPeak,await heap());samples++;
  }
  sample=setInterval(async()=>{if(sampling)return;sampling=true;try{await measure();}catch(e){measurementError=e;}finally{sampling=false;}},25);
  await page.getByRole('button',{name:'Attach database',exact:true}).click();await page.getByRole('heading',{name:'Database inventory',exact:true}).waitFor();if(nodes)await page.getByRole('button',{name:'Canonical nodes',exact:true}).click();
  const browserBaselinePids=new Set(await tree(browserPid));const attachedTree=(await tree(supervisorPid)).filter(p=>!browserBaselinePids.has(p));const attachedBaseline=(await Promise.all(attachedTree.map(rss))).reduce((a,b)=>a+b,0);assert.ok(attachedBaseline<=2*1024*1024*1024,'attached service RSS conservatively bounds model residency');
  await action(page,child,undefined,{db,binary,dir,context,tree:()=>tree(supervisorPid),rss});
  assert.ok(samples>0,'actual process-tree RSS samples required');assert.equal(measurementError,undefined);assert.ok(servicePeak<=2*1024*1024*1024,'whole service tree conservatively bounds model residency throughout');assert.ok(servicePeak-attachedBaseline<=1024*1024*1024,'service RSS growth over attached baseline cap');
  heapPeak=Math.max(heapPeak,await heap());assert.ok(heapPeak-heapStart<=512*1024*1024,'installed browser heap growth cap');
  const preClose=await identity(browserPid);
  assert.ok(preClose?.live&&(await tree(supervisorPid)).includes(browserPid),'owned actual native browser exists before close');
  const closeStart=performance.now();
  await cdp.send('Browser.close').catch(()=>page.close());
  const result=await Promise.race([exited,delay(30000,undefined,{ref:false}).then(()=>{throw new Error('actual native window close deadline');})]);
  assert.deepEqual(result,{code:0,signal:null},stderr);clearInterval(sample);await until(async()=>{if(sampling)return false;for(const [pid,start]of owned){const current=await identity(pid);if(current?.live&&current.start===start)return false;}return true;},Math.max(1,30000-(performance.now()-closeStart)));assert.ok(performance.now()-closeStart<30000);closed=true;execFileSync('flock',['--nonblock',`${db}.lock`,'true']);
  const retained=(await readdir(dir)).filter(n=>n.startsWith('fdi-'));assert.deepEqual(retained,[],'private native launch resources released');
  assert.deepEqual(external,[],'no external UI requests');
  console.log(JSON.stringify({qualification:'installed_native_window',fixture,build:JSON.parse(execFileSync(binary,['--build-description'],{encoding:'utf8',env:{}})).build,native_window_closed:true,close_ms:performance.now()-closeStart,sample_ms:25,samples,attached_service_rss:attachedBaseline,service_sampled_peak_rss:servicePeak,browser_tree_sampled_peak_rss:browserRssPeak,total_tree_sampled_peak_rss:totalRssPeak,model_residency_conservative_upper_bound:servicePeak,owned_descendants_no_longer_live:true,browser_heap_growth:heapPeak-heapStart}));
 }finally{
  clearInterval(sample);
  if(browser)await browser.close().catch(()=>{});
  if(supervisorPid&&supervisorPid!==child?.pid){try{process.kill(supervisorPid,'SIGTERM');}catch{}}
  if(child&&child.exitCode===null){child.kill('SIGTERM');await Promise.race([exited,delay(5000,undefined,{ref:false})]);if(supervisorPid&&supervisorPid!==child.pid){try{process.kill(supervisorPid,'SIGKILL');}catch{}}if(child.exitCode===null){child.kill('SIGKILL');await exited;}}
  await rm(dir,{recursive:true,force:true});
  if(!closed)console.error('installed native window close NOT qualified for this failed run');
 }
}
