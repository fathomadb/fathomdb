import assert from "node:assert/strict";
import {spawn,execFileSync} from "node:child_process";
import {mkdtemp,mkdir,copyFile,readFile,readdir,rm} from "node:fs/promises";
import {tmpdir} from "node:os";
import {resolve,join} from "node:path";
import {createInterface} from "node:readline";
import {setTimeout as delay} from "node:timers/promises";
assert.equal(process.env.AGENT_LONG,"1","AGENT_LONG=1 required for native deadline qualification");
const root=resolve(import.meta.dirname,"../../..");
const scratch=await mkdtemp(join(tmpdir(),"inspector-lexical-native-"));
const ownedWorkers=new Set();let child,monitor,stopping=false,peak=0,idle=0,measurementError=null;
const report={qualification:"slice30_real_native_lexical",synthetic_pause:false};
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
  const base=execFileSync(join(root,"target/debug/examples/fixture"),[producer,"lexical-native"],{encoding:"utf8",timeout:120000}).trim();
  const paths={};
  for(const phase of ["cancel","fault","close","timeout"]) {
    const dir=join(scratch,phase);await mkdir(dir);paths[phase]=join(dir,"fixture.sqlite");
    await copyFile(base,paths[phase]);await copyFile(`${base}.lock`,`${paths[phase]}.lock`);
  }
  child=spawn(join(root,"target/debug/examples/driver"),[paths.cancel,join(root,"target/debug/fathomdb-inspector")],{cwd:root,stdio:["pipe","pipe","pipe"]});child.stdin.on("error",()=>{});
  const iter=createInterface({input:child.stdout})[Symbol.asyncIterator]();
  const info=JSON.parse((await before(iter.next(),performance.now()+10000,"driver readiness")).value);
  const origin=`http://${info.address}`;
  const secret=(await readFile(info.bootstrap,"utf8")).match(/name="secret" value="([a-f0-9]+)"/)[1];
  const boot=await fetch(`${origin}/bootstrap`,{method:"POST",headers:{Origin:"null","Content-Type":"application/x-www-form-urlencoded"},body:`secret=${secret}`,redirect:"manual"});assert.equal(boot.status,303);
  const cookie=boot.headers.get("set-cookie").split(";")[0];
  const h=await(await fetch(`${origin}/handshake`,{headers:{Cookie:cookie}})).json();
  const pair={build:h.build,protocol:h.protocol,launch:h.launch,attachment:h.attachment};
  async function api(request) {
    const response=await fetch(`${origin}/api`,{method:"POST",headers:{Cookie:cookie,Origin:origin,"Content-Type":"application/json"},body:JSON.stringify({...pair,request}),signal:AbortSignal.timeout(35000)});
    const value=await response.json();if("attachment" in value)pair.attachment=value.attachment;return value;
  }
  const query=text=>({op:"lexical",route:"bm25f",query:text,filter:{},view:{},limit:100});
  assert.equal((await api({op:"attach",path:paths.cancel})).status,"complete");
  idle=(await rss(child.pid))+(await Promise.all((await childPids()).map(rss))).reduce((a,b)=>a+b,0);
  monitor=(async()=>{while(!stopping){const value=(await rss(child.pid))+(await Promise.all((await childPids()).map(rss))).reduce((a,b)=>a+b,0);peak=Math.max(peak,value);await delay(10);}})().catch(error=>{measurementError=error;stopping=true;});
  async function nativeRead(path,attached=false) {
    if(!attached)assert.equal((await api({op:"attach",path})).status,"complete");
    const pids=await childPids();assert.equal(pids.length,1);const pid=pids[0],start=await cpu(pid);let completed=false;
    const pending=api(query("q1")).then(value=>{completed=true;return value;});
    await until(async()=>completed||(await cpu(pid))>=start+3,performance.now()+5000,"native lexical dispatch CPU progress");
    assert.equal(completed,false,"real lexical request must remain active at interruption");return{pid,pending};
  }
  let active=await nativeRead(paths.cancel,true),began=performance.now(),deadline=began+30000;
  const queued=Array.from({length:9},()=>api({op:"page",class:"node",after:"0",limit:1,filter:{}}));
  await delay(20);
  const ack=await before(api({op:"cancel"}),deadline,"cancel ack"),ackMs=performance.now()-began;
  assert.equal(ack.status,"cancelled");assert.ok(ackMs<=250,`cancel ACK ${ackMs}ms`);
  const late=await before(active.pending,deadline,"cancelled lexical reply");assert.equal(late.status,"cancelled");assert.equal(late.items,undefined);
  const waiting=await Promise.all(queued);assert.ok(waiting.some(v=>v.status==="busy"),"queue overflow must refuse excess work");
  assert.ok(waiting.every(v=>["busy","cancelled","expired"].includes(v.status)));
  await until(()=>gone(active.pid),deadline,"cancel quiescence");lockFree(paths.cancel,deadline);
  report.cancel={ack_ms:ackMs,quiescence_and_lock_ms:performance.now()-began,busy:waiting.filter(v=>v.status==="busy").length,no_late_payload:true};
  active=await nativeRead(paths.fault);began=performance.now();deadline=began+30000;process.kill(active.pid,"SIGKILL");
  const fault=await before(active.pending,deadline,"fault reply");assert.equal(fault.error,"worker_disconnected");assert.equal(fault.items,undefined);assert.equal(fault.attachment,null);
  await until(()=>gone(active.pid),deadline,"fault quiescence");lockFree(paths.fault,deadline);report.fault={elapsed_ms:performance.now()-began,no_late_payload:true};
  active=await nativeRead(paths.close);began=performance.now();deadline=began+30000;
  const detach=api({op:"detach"});const positive=await before(active.pending,deadline,"native positive completion");
  assert.ok(["complete","partial"].includes(positive.status));assert.equal(positive.scope.score_family,"native_bm25f");assert.equal(positive.scope.native_returned,3500);assert.equal(positive.items.length,100);
  assert.equal((await before(detach,deadline,"detach reply")).status,"complete");await until(()=>gone(active.pid),deadline,"detach quiescence");lockFree(paths.close,deadline);
  report.detach={elapsed_ms:performance.now()-began,native_returned:positive.scope.native_returned,output_returned:positive.items.length};
  assert.equal((await api({op:"attach",path:paths.timeout})).status,"complete");
  const timeoutPid=(await childPids())[0],terms=Array.from({length:800},(_,n)=>`q${n}`).join(" ");assert.ok(Buffer.byteLength(terms)<=4096);
  began=performance.now();const timed=await api(query(terms));const elapsed=performance.now()-began;
  assert.equal(timed.status,"budget_exhausted","real native workload must reach the unchanged request deadline");assert.equal(timed.items,undefined);assert.equal(timed.attachment,null);
  await until(()=>gone(timeoutPid),performance.now()+5000,"timed-out native worker quiescence");lockFree(paths.timeout);
  assert.ok(elapsed>=29000&&elapsed<=32000,`request deadline ${elapsed}ms`);report.timeout={elapsed_ms:elapsed,error:timed.error,no_late_payload:true};
  if(measurementError)throw measurementError;
  assert.ok(peak-idle<=1024*1024*1024);report.memory={attached_idle_rss:idle,sampled_peak_total_rss:peak,growth:peak-idle,sample_ms:10};
  report.execution_witness="owned main dispatch-thread CPU progress with real lexical reply pending; no synthetic pause";
  report.composition="paired default/debug None-model artifact; installed qualification remains70";
} finally {
  stopping=true;if(monitor)await before(monitor,performance.now()+6000,"monitor cleanup");
  if(child&&child.exitCode===null) {const exited=new Promise(r=>child.once("exit",r));child.stdin.end("close\n");await before(exited,performance.now()+30000,"driver cleanup");}
  for(const pid of ownedWorkers)await until(()=>gone(pid),performance.now()+5000,"owned worker cleanup");
  await rm(scratch,{recursive:true,force:true});
}
report.cleanup="owned workers and lock released; owned scratch removed";
console.log(JSON.stringify(report));
