import assert from "node:assert/strict";
import {spawn,execFileSync} from "node:child_process";
import {mkdtemp,mkdir,copyFile,readFile,readdir,rm} from "node:fs/promises";
import {tmpdir} from "node:os";
import {resolve,join} from "node:path";
import {pathToFileURL} from "node:url";
import {createInterface} from "node:readline";
import {setTimeout as delay} from "node:timers/promises";
assert.equal(process.env.AGENT_LONG,"1","AGENT_LONG=1 required for F06 graph qualification");
const root=resolve(import.meta.dirname,"../../..");
const scratch=await mkdtemp(join(tmpdir(),"inspector-graph-F06-"));
const ownedWorkers=new Set();let child,browser,monitor,stopping=false,peak=0,idle=0,measurementError=null;
const report={qualification:"slice50_real_F06_graph",synthetic_pause:false};
async function childPids() {
  const ids = new Set();
  for (const tid of await readdir(`/proc/${child.pid}/task`)) {
    try {
      for (const pid of (await readFile(`/proc/${child.pid}/task/${tid}/children`, "utf8"))
        .trim()
        .split(/\s+/)
        .filter(Boolean))
        ids.add(Number(pid));
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
  }
  for (const pid of ids) ownedWorkers.add(pid);
  return [...ids];
}
async function rss(pid) {
  try {
    const status = await readFile(`/proc/${pid}/status`, "utf8"),
      match = status.match(/^VmRSS:\s+(\d+)/m);
    if (!match) {
      assert.match(status, /^State:\s+Z/m, "RSS missing for a non-zombie process");
      return 0;
    }
    return Number(match[1]) * 1024;
  } catch (error) {
    if (error.code === "ENOENT") return 0;
    throw error;
  }
}
async function gone(pid) {
  try {
    await readFile(`/proc/${pid}/status`);
    return false;
  } catch (error) {
    if (error.code === "ENOENT") return true;
    throw error;
  }
}
async function cpu(pid) {
  const stat = await readFile(`/proc/${pid}/task/${pid}/stat`, "utf8");
  const fields = stat.slice(stat.lastIndexOf(")") + 2).split(" ");
  return Number(fields[11]) + Number(fields[12]);
}
async function before(promise, deadline, message) {
  let timer;
  try {
    return await Promise.race([
      promise,
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(Error(message)), Math.max(1, deadline - performance.now()));
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}
async function until(predicate, deadline, message) {
  while (true) {
    assert.ok(performance.now() < deadline, message);
    if (await predicate()) return;
    await delay(Math.min(5, Math.max(0, deadline - performance.now())));
  }
}
function lockFree(path, deadline = performance.now() + 5000) {
  assert.ok(performance.now() < deadline, "lock check must fit shared deadline");
  execFileSync(
    join(root, ".venv/bin/python"),
    [
      "-c",
      'import fcntl,sys\nf=open(sys.argv[1],"r+")\nfcntl.flock(f,fcntl.LOCK_EX|fcntl.LOCK_NB)',
      `${path}.lock`,
    ],
    { timeout: Math.max(1, Math.ceil(deadline - performance.now())) },
  );
}
try {
 const producer=join(scratch,"producer");await mkdir(producer);
 const base=execFileSync(join(root,"target/debug/examples/fixture"),[producer,"scale"],{encoding:"utf8",timeout:600000}).trim();
 const manifest=JSON.parse(execFileSync(join(root,".venv/bin/python"),["-c",'import sqlite3,json,sys\nc=sqlite3.connect(sys.argv[1]);print(json.dumps(dict(nodes=c.execute("select count(*) from canonical_nodes").fetchone()[0],edges=c.execute("select count(*) from canonical_edges").fetchone()[0],logs=c.execute("select count(*) from operational_mutations").fetchone()[0],large_bytes=c.execute("select max(length(cast(body as blob))) from canonical_nodes").fetchone()[0])))',base],{encoding:"utf8"}));
 assert.ok(manifest.nodes>=100000);assert.equal(manifest.edges,10000);assert.equal(manifest.logs,100000);assert.ok(manifest.large_bytes>=10*1024*1024);report.fixture=manifest;
 const paths={};for(const phase of ["positive","cancel","fault","detach","browser"]){const dir=join(scratch,phase);await mkdir(dir);paths[phase]=join(dir,"fixture.sqlite");await copyFile(base,paths[phase]);await copyFile(`${base}.lock`,`${paths[phase]}.lock`);}
 child=spawn(join(root,"target/debug/examples/driver"),[paths.browser,join(root,"target/debug/fathomdb-inspector")],{cwd:root,stdio:["pipe","pipe","pipe"]});child.stdin.on("error",()=>{});
 const iter=createInterface({input:child.stdout})[Symbol.asyncIterator]();const info=JSON.parse((await before(iter.next(),performance.now()+10000,"driver readiness")).value);
 const origin=`http://${info.address}`,secret=(await readFile(info.bootstrap,"utf8")).match(/name="secret" value="([a-f0-9]+)"/)[1];
 const boot=await fetch(`${origin}/bootstrap`,{method:"POST",headers:{Origin:"null","Content-Type":"application/x-www-form-urlencoded"},body:`secret=${secret}`,redirect:"manual"});assert.equal(boot.status,303);const cookie=boot.headers.get("set-cookie").split(";")[0];
 const handshake=await(await fetch(`${origin}/handshake`,{headers:{Cookie:cookie}})).json();const pair={build:handshake.build,protocol:handshake.protocol,launch:handshake.launch,attachment:handshake.attachment};
 async function api(request){const response=await fetch(`${origin}/api`,{method:"POST",headers:{Cookie:cookie,Origin:origin,"Content-Type":"application/json"},body:JSON.stringify({...pair,request}),signal:AbortSignal.timeout(35000)});const data=await response.json();if("attachment" in data)pair.attachment=data.attachment;return data;}
 async function prepare(path){assert.equal((await api({op:"attach",path})).status,"complete");const p=await api({op:"page",class:"node",after:"0",limit:1,filter:{identity:"alpha"}});const h=p.items[0].handle;return{op:"graph",handle:h,direction:"both",kind:null,depth:3,expanded:[h]};}
 monitor=(async()=>{while(!stopping){const pids=await childPids();peak=Math.max(peak,(await rss(child.pid))+(await Promise.all(pids.map(rss))).reduce((a,b)=>a+b,0));await delay(10);}})().catch(e=>measurementError=e);
 let request=await prepare(paths.positive);const pids=await childPids();idle=(await rss(child.pid))+(await rss(pids[0]));let began=performance.now();
 const g=await api(request);assert.equal(g.status,"partial");assert.ok(g.edges.length>0);assert.ok(g.nodes.length+g.edges.length<=200);assert.ok(g.scope.examined_links<=1000);assert.ok(g.scope.work_units<=1000);assert.ok(g.scope.examined_canonical_rows<=100000);assert.ok(Buffer.byteLength(JSON.stringify(g))<=2*1024*1024);report.positive={ms:performance.now()-began,objects:g.nodes.length+g.edges.length,examined_links:g.scope.examined_links,work_units:g.scope.work_units,scanned_rows:g.scope.examined_canonical_rows,bytes:Buffer.byteLength(JSON.stringify(g))};assert.equal((await api({op:"detail",handle:g.seed.handle})).status,"complete");await api({op:"detach"});lockFree(paths.positive);
 async function active(path){const request=await prepare(path),pids=await childPids();assert.equal(pids.length,1);const pid=pids[0],start=await cpu(pid);let done=false;const pending=api(request).then(v=>{done=true;return v;});await until(async()=>done||(await cpu(pid))>=start+1,performance.now()+5000,"owned graph dispatch CPU progress");assert.equal(done,false,"graph must still be executing at interruption");return{request,pid,pending};}
 let run=await active(paths.cancel);const queued=Array.from({length:10},()=>api(run.request));began=performance.now();let deadline=began+30000;
 const ack=await before(api({op:"cancel"}),deadline,"cancel ACK");const ackMs=performance.now()-began;assert.equal(ack.status,"cancelled");assert.ok(ackMs<=250,`cancel ACK ${ackMs}ms`);const late=await before(run.pending,deadline,"cancelled graph");assert.equal(late.status,"cancelled");assert.equal(late.edges,undefined);const waiting=await before(Promise.all(queued),deadline,"queued cancellation");assert.ok(waiting.every(v=>["cancelled","busy","expired"].includes(v.status)));await until(()=>gone(run.pid),deadline,"cancel worker quiescence");lockFree(paths.cancel,deadline);report.cancel={ack_ms:ackMs,quiescence_ms:performance.now()-began,queued:waiting.length,busy:waiting.filter(v=>v.status==="busy").length,no_late_graph:true};
 run=await active(paths.fault);began=performance.now();deadline=began+30000;assert.ok(ownedWorkers.has(run.pid),"signal only this recorded driver's direct owned child");process.kill(run.pid,"SIGKILL");const fault=await before(run.pending,deadline,"fault graph reply");assert.equal(fault.error,"worker_disconnected");assert.equal(fault.edges,undefined);await until(()=>gone(run.pid),deadline,"fault reap");lockFree(paths.fault,deadline);report.fault_ms=performance.now()-began;
 run=await active(paths.detach);began=performance.now();deadline=began+30000;assert.equal((await before(api({op:"detach"}),deadline,"detach")).status,"complete");assert.ok(["complete","partial"].includes((await before(run.pending,deadline,"graph completion")).status));await until(()=>gone(run.pid),deadline,"detach reap");lockFree(paths.detach,deadline);report.detach_ms=performance.now()-began;
 const {chromium}=await import(pathToFileURL(process.env.INSPECTOR_PLAYWRIGHT??"/home/coreyt/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright/index.mjs"));browser=await chromium.launch({executablePath:process.env.INSPECTOR_CHROMIUM??"/home/coreyt/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome",headless:true,args:["--no-sandbox","--disable-background-networking"]});const context=await browser.newContext();const [name,value]=cookie.split("=");await context.addCookies([{name,value,url:origin}]);const page=await context.newPage();const external=[];page.on("request",r=>{if(!r.url().startsWith(origin))external.push(r.url());});await page.goto(origin);const cdp=await context.newCDPSession(page);const heap=async()=>{const h=await cdp.send("Runtime.getHeapUsage");return h.usedSize+(h.embedderHeapUsedSize??0)+(h.backingStorageSize??0);};const shellHeap=await heap();let heapPeak=shellHeap;
 await page.getByRole("button",{name:"Attach database",exact:true}).click();await page.getByRole("heading",{name:"Database inventory",exact:true}).waitFor({timeout:35000});await page.getByRole("button",{name:"Canonical nodes",exact:true}).click();await page.getByRole("button",{name:"Inspect alpha",exact:true}).click();const entry=page.getByRole("button",{name:"Stored graph",exact:true});await entry.focus();await page.keyboard.press("Enter");await page.getByRole("heading",{name:"Stored directed graph",exact:true}).waitFor();
 for(let i=0;i<6;i++){assert.ok(await page.locator(".graph-edge").count()>0);heapPeak=Math.max(heapPeak,await heap());await page.getByRole("button",{name:"Collapse selected node",exact:true}).click();await page.waitForFunction(()=>document.querySelectorAll(".graph-edge").length===0);await page.getByRole("button",{name:"Expand selected node",exact:true}).click();await page.waitForFunction(()=>document.querySelectorAll(".graph-edge").length>0);}
 assert.ok(heapPeak-shellHeap<=512*1024*1024);assert.deepEqual(external,[]);await page.getByRole("button",{name:"Back",exact:true}).click();await page.getByRole("heading",{name:"Record detail",exact:true}).waitFor();assert.equal(await page.evaluate(()=>document.activeElement?.textContent),"Stored graph");await page.getByRole("button",{name:"Detach database",exact:true}).click();await page.getByRole("heading",{name:"Attach a local database",exact:true}).waitFor();lockFree(paths.browser);report.browser={cycles:6,heap_peak:heapPeak,heap_growth:heapPeak-shellHeap,no_external_requests:true,keyboard_back:true};
 stopping=true;await monitor;if(measurementError)throw measurementError;report.memory={attached_idle:idle,sampled_total_peak:peak,growth:Math.max(0,peak-idle),sample_ms:10};assert.ok(peak-idle<=1024*1024*1024);report.execution_witness="recorded owned worker main-dispatch CPU progress while real graph request pending; no synthetic pause";
 console.log(JSON.stringify(report,null,2));
} finally {
 stopping=true;if(monitor)await monitor;if(browser)await browser.close();if(child){const ended=new Promise(r=>child.once("exit",r));child.stdin.end("close\n");await before(ended,performance.now()+30000,"driver cleanup");}for(const pid of ownedWorkers)assert.ok(await gone(pid),`owned worker ${pid} not reaped`);await rm(scratch,{recursive:true,force:true});
}
