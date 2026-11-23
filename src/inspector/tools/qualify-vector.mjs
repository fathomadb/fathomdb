import assert from "node:assert/strict";
import {spawn,execFileSync} from "node:child_process";
import {mkdtemp,mkdir,copyFile,readFile,readdir,rm} from "node:fs/promises";
import {tmpdir} from "node:os";
import {resolve,join} from "node:path";
import {createInterface} from "node:readline";
import {setTimeout as delay} from "node:timers/promises";
import {stopChildBounded} from "../tests/browser/helpers.mjs";
assert.equal(process.env.AGENT_LONG,"1","AGENT_LONG=1 required for native deadline qualification");
const root=resolve(import.meta.dirname,"../../..");
const scratch=await mkdtemp(join(tmpdir(),"inspector-vector-native-"));
const ownedWorkers=new Set();let child,monitor,stopping=false,peak=0,idle=0,measurementError=null;
const report={qualification:"slice40_real_native_vector",synthetic_pause:false};
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
  const stat = await readFile(`/proc/${pid}/stat`, "utf8");
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
  const base=execFileSync(join(root,".venv/bin/python"),[join(root,"src/inspector/tools/vector-fixture.py"),producer],{encoding:"utf8",timeout:120000}).trim();
  const paths={};
  for(const phase of ["cancel","fault","close","positive"]) {
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
  const query=text=>({op:"serving",query:text,limit:100});
  assert.equal((await api({op:"attach",path:paths.cancel})).status,"complete");
  idle=(await rss(child.pid))+(await Promise.all((await childPids()).map(rss))).reduce((a,b)=>a+b,0);
  monitor=(async()=>{while(!stopping){const value=(await rss(child.pid))+(await Promise.all((await childPids()).map(rss))).reduce((a,b)=>a+b,0);peak=Math.max(peak,value);await delay(10);}})().catch(error=>{measurementError=error;stopping=true;});
  async function nativeRead(path,attached=false) {
    if(!attached)assert.equal((await api({op:"attach",path})).status,"complete");
    const pids=await childPids();assert.equal(pids.length,1);const pid=pids[0],start=await cpu(pid);let completed=false;
    const pending=api(query("puppy ".repeat(128))).then(value=>{completed=true;return value;});
    await until(async()=>completed||(await cpu(pid))>=start+1,performance.now()+5000,"native CPU embedding thread progress");
    assert.equal(completed,false,"real vector request must remain active at interruption");return{pid,pending};
  }
  let active=await nativeRead(paths.cancel,true),began=performance.now(),deadline=began+30000;
  const queued=Array.from({length:9},()=>api({op:"page",class:"node",after:"0",limit:1,filter:{}}));
  await delay(20);
  const ack=await before(api({op:"cancel"}),deadline,"cancel ack"),ackMs=performance.now()-began;
  assert.equal(ack.status,"cancelled");assert.ok(ackMs<=250,`cancel ACK ${ackMs}ms`);
  const late=await before(active.pending,deadline,"cancelled vector reply");assert.equal(late.status,"cancelled");assert.equal(late.items,undefined);
  const waiting=await Promise.all(queued);assert.ok(waiting.some(v=>v.status==="busy"),"queue overflow must refuse excess work");
  assert.ok(waiting.every(v=>["busy","cancelled","expired"].includes(v.status)));
  await until(()=>gone(active.pid),deadline,"cancel quiescence");lockFree(paths.cancel,deadline);
  report.cancel={ack_ms:ackMs,quiescence_and_lock_ms:performance.now()-began,busy:waiting.filter(v=>v.status==="busy").length,no_late_payload:true};
  active=await nativeRead(paths.fault);began=performance.now();deadline=began+30000;process.kill(active.pid,"SIGKILL");
  const fault=await before(active.pending,deadline,"fault reply");assert.equal(fault.error,"worker_disconnected");assert.equal(fault.items,undefined);assert.equal(fault.attachment,null);
  await until(()=>gone(active.pid),deadline,"fault quiescence");lockFree(paths.fault,deadline);report.fault={elapsed_ms:performance.now()-began,no_late_payload:true};
  active=await nativeRead(paths.close);began=performance.now();deadline=began+30000;
  const detach=api({op:"detach"});const positive=await before(active.pending,deadline,"native positive completion");
  assert.ok(["complete","partial"].includes(positive.status));assert.equal(positive.scope.score_family,"engine_rrf");assert.ok(positive.query_context.vector_hits>0);assert.equal(positive.query_context.text_hits,0);assert.equal(positive.items.length,2);assert.ok(positive.items.every(item=>Number.isInteger(item.evidence.vector_rank)));
  assert.equal((await before(detach,deadline,"detach reply")).status,"complete");await until(()=>gone(active.pid),deadline,"detach quiescence");lockFree(paths.close,deadline);
  report.detach={elapsed_ms:performance.now()-began,native_returned:positive.scope.native_returned,output_returned:positive.items.length};
  assert.equal((await api({op:"attach",path:paths.positive})).status,"complete");
  const positivePid=(await childPids())[0],residency=await rss(positivePid);assert.ok(residency<=2*1024*1024*1024,"attached total RSS conservatively bounds model residency");
  began=performance.now();const ordinary=await api(query("puppy"));const elapsed=performance.now()-began;
  assert.equal(ordinary.status,"complete");assert.ok(ordinary.query_context.vector_hits>0);assert.equal(ordinary.query_context.text_hits,0);assert.ok(elapsed<30000);
  report.positive={elapsed_ms:elapsed,vector_hits:ordinary.query_context.vector_hits,text_hits:ordinary.query_context.text_hits,attached_worker_rss_upper_bound_model_residency:residency};
  assert.equal((await api({op:"detach"})).status,"complete");await until(()=>gone(positivePid),performance.now()+5000,"positive teardown");lockFree(paths.positive);
  console.log(JSON.stringify({phase:"native_controls",...report}));
  const scaleProducer=join(scratch,"scale-producer");await mkdir(scaleProducer);
  await copyFile(base,join(scaleProducer,"fixture.sqlite"));await copyFile(`${base}.lock`,join(scaleProducer,"fixture.sqlite.lock"));
  const scalePath=execFileSync(join(root,"target/debug/examples/fixture"),[scaleProducer,"scale"],{encoding:"utf8",timeout:120000}).trim();
  const facts=JSON.parse(execFileSync(join(root,".venv/bin/python"),["-c",'import json,sqlite3,sys\nc=sqlite3.connect(sys.argv[1]);print(json.dumps({"nodes":c.execute("select count(*) from canonical_nodes").fetchone()[0],"mutations":c.execute("select count(*) from operational_mutations").fetchone()[0],"edges":c.execute("select count(*) from canonical_edges").fetchone()[0],"large_bytes":c.execute("select max(octet_length(body)) from canonical_nodes").fetchone()[0],"dog_rowid":c.execute("select rowid from canonical_nodes where logical_id=?",("dog",)).fetchone()[0]}))',scalePath],{encoding:"utf8",timeout:30000}));
  assert.ok(facts.nodes>=100000&&facts.mutations>=100000&&facts.edges>=10000&&facts.large_bytes>=10*1024*1024);
  assert.equal((await api({op:"attach",path:scalePath})).status,"complete");const scalePid=(await childPids())[0];
  const selected=await api({op:"page",class:"node",after:String(facts.dog_rowid-1),limit:10,filter:{identity:"dog"}});assert.equal(selected.items.length,1);const handle=selected.items[0].handle;
  const vectors=await api({op:"vectors",handle});assert.equal(vectors.items.length,1);began=performance.now();const neighbors=await api({op:"neighborhood",handle,vector_rowid:vectors.items[0].vector_rowid,after:"0",limit:10});assert.equal(neighbors.status,"complete");assert.equal(neighbors.items.length,2);const storedMs=performance.now()-began;
  began=performance.now();const limited=await api(query("puppy"));const refusalMs=performance.now()-began;assert.equal(limited.status,"budget_exhausted");assert.equal(limited.items,undefined);assert.ok(refusalMs<=30000);assert.equal((await api({op:"vectors",handle})).status,"complete");
  assert.equal((await api({op:"detach"})).status,"complete");await until(()=>gone(scalePid),performance.now()+5000,"F06 teardown");lockFree(scalePath);
  report.F06={...facts,stored_positive_ms:storedMs,stored_vectors_compared:neighbors.scope.eligible_compared,native_scope:"admission_refused_not_serving_success",native_refusal_ms:refusalMs,next_exact_read:"complete"};
  if(measurementError)throw measurementError;
  assert.ok(peak-idle<=1024*1024*1024);report.memory={attached_idle_rss:idle,sampled_peak_total_rss:peak,growth:peak-idle,sample_ms:10};
  report.deadline={actual_vector_positives_within_30s:true,natural_native_timeout_not_reached:true,inherited_strict30s_timeout:"qualified on native lexical work in Slice30; no synthetic vector pause"};
  report.execution_witness="owned process CPU progress with actual128-token CPU embedding query pending; no synthetic pause";
  report.composition="paired cached-cpu/debug sealed cached CPU artifact; installed qualification remains70";
} finally {
  stopping=true;if(monitor)await before(monitor,performance.now()+6000,"monitor cleanup").catch(()=>{});
  await stopChildBounded(child,{gracefulMs:10000,sigtermMs:2000,sigkillMs:5000,extraPids:[...ownedWorkers]});
  for(const pid of ownedWorkers)await until(()=>gone(pid),performance.now()+5000,"owned worker cleanup");
  await rm(scratch,{recursive:true,force:true});
}
report.cleanup="owned workers and lock released; owned scratch removed";
console.log(JSON.stringify(report));
