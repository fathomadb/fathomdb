import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import crypto from 'node:crypto';
import {pathToFileURL} from 'node:url';
import {createRequire} from 'node:module';
const env=process.env, mode=env.FATHOMDB_COORDINATED_RESET;
assert.equal(env.FATHOMDB_REQUIRE_LIVE,'1');
assert(['reset-refuse','recover'].includes(mode));
assert.equal(env.FATHOMDB_EMBED_DEVICE,'cuda:0');assert.equal(env.FATHOMDB_RERANK_DEVICE,'cuda:0');
assert.equal(env.CUDA_VISIBLE_DEVICES,env.FATHOMDB_SELECTED_UUID);
const pkg=env.FATHOMDB_PACKAGE_DIR;
const {Engine,embedBatchCls,rerank,releaseModels,CudaContextLostError}=await import(pathToFileURL(path.join(pkg,'dist/index.js')));
const require=createRequire(path.join(pkg,'package.json'));
const nativePath=require.resolve(env.FATHOMDB_NATIVE_PACKAGE), native=require(nativePath);
assert.equal(crypto.createHash("sha256").update(fs.readFileSync(nativePath)).digest("hex"),env.FATHOMDB_NATIVE_SHA256);
const root=env.FATHOMDB_CAPTURE_DIR;fs.mkdirSync(root,{recursive:true});
const db=path.join(root,'installed-coordinate.fdb');assert.equal(fs.existsSync(db),mode==='recover');
const Q='What is the capital of France?';
const P=['Paris.','Paris is the capital and most populous city of France.','France is a country in Western Europe; its capital and largest city is Paris, a major global center for art, fashion, and culture.'];
const passages=P.map((body,id)=>({id,body,score:1}));
const inputs=['the cat sat on the mat','a cat is on a mat','quantum mechanics dictates the behaviour of subatomic particles'];
let engine=await Engine.open(db,{useDefaultEmbedder:true});
const resolution=engine.openReport().embedderDeviceResolution;
assert.equal(resolution.effectiveDevice.kind,'cuda');assert.equal(resolution.selectedCudaUuid,env.FATHOMDB_SELECTED_UUID);
const vectors=[];for(const input of inputs)vectors.push(await engine.embed(input));
for(const v of vectors){assert.equal(v.length,384);assert(Math.abs(Math.sqrt(v.reduce((s,x)=>s+x*x,0))-1)<1e-5);}
const cos=(a,b)=>a.reduce((s,x,i)=>s+x*b[i],0), near=cos(vectors[0],vectors[1]);
assert(near-cos(vectors[0],vectors[2])>.15);assert(near-cos(vectors[1],vectors[2])>.15);
assert.equal((await embedBatchCls(inputs)).length,3);
{const scored=await rerank(Q,passages,3);assert.equal(scored.length,3);assert.deepEqual(scored.map(x=>x.id).sort(),[0,1,2]);assert(scored.every(x=>x.ceScore!==null&&Number.isFinite(x.ceScore)));}
if(mode!=='recover')await engine.write([{kind:'doc',body:'Paris is the capital of France.',sourceId:'coordinate-paris'},{kind:'doc',body:'France has its capital in Paris.',sourceId:'coordinate-france'}]);
await engine.drain(30000);
const hits=await engine.search(Q,undefined,2,undefined,undefined,undefined,undefined,{limit:2});
assert.equal(hits.results.length,2);assert(hits.results.every(x=>x.ceScore!==null&&Number.isFinite(x.ceScore)));
await engine.drain(30000);
if(mode==='recover'){
 const persisted=await engine.searchTextOnly('Paris');
 assert.deepEqual(persisted.results.map(x=>[x.sourceId,x.body]).sort(),[['coordinate-france','France has its capital in Paris.'],['coordinate-paris','Paris is the capital of France.']]);
 await releaseModels(); await releaseModels(); assert.equal((await embedBatchCls(inputs)).length,3); await releaseModels(); await engine.close();engine=null;console.log('INSTALLED-COORDINATE fresh-process-recovery post-drop');
}else{
 const before=JSON.parse(native.cudaPoolObservationForQualification());
 assert(['arm64','x64'].includes(os.arch()));if(os.arch()==='arm64')assert.equal(before.status,'private');
 if(before.status==='private'){assert.equal(BigInt(before.live_models_after),3n);assert(BigInt(before.used_current_bytes)<=BigInt(before.configured_max_size_bytes));assert.equal(BigInt(before.configured_max_size_bytes),3n<<30n);}
 console.log('INSTALLED-COORDINATE READY '+JSON.stringify({pid:process.pid,mode,pool:before,native:nativePath,native_sha256:crypto.createHash('sha256').update(fs.readFileSync(nativePath)).digest('hex')}));
 // No calls remain queued; the bridge retains only the already active primary,
 // synchronizes all its streams, and balances its own reference exactly once.
 assert.equal(crypto.createHash('sha256').update(fs.readFileSync(env.FATHOMDB_RESET_ACTOR)).digest('hex'),env.FATHOMDB_RESET_ACTOR_SHA256);
 if(before.status==='private')env.FATHOMDB_EXPECTED_CONTEXT_ID=String(before.context_id);
 else delete env.FATHOMDB_EXPECTED_CONTEXT_ID;
 const actor=require(env.FATHOMDB_RESET_ACTOR);
 const coordinated=JSON.parse(actor.coordinate(true));
 await releaseModels(); await releaseModels(); console.log('RELEASE_AFTER_LOSS_OK');
 assert.equal(coordinated.uuid,env.FATHOMDB_SELECTED_UUID);assert.equal(coordinated.mode,mode);assert.equal(coordinated.balanced_release,true);
 if(before.status==='private')assert.equal(coordinated.context_id,before.context_id);
 console.log('INSTALLED-COORDINATE coordinated '+JSON.stringify(coordinated));
 if(mode==='healthy'){
  assert.equal((await engine.embed(inputs[0])).length,384);
  assert.equal((await embedBatchCls(inputs)).length,3);
  {const scored=await rerank(Q,passages,3);assert.equal(scored.length,3);assert.deepEqual(scored.map(x=>x.id).sort(),[0,1,2]);assert(scored.every(x=>x.ceScore!==null&&Number.isFinite(x.ceScore)));}
 }else if(mode==='reset-refuse'){
  let first=null;
  for(const call of [()=>engine.embed('after reset'),()=>embedBatchCls(inputs),()=>rerank(Q,passages,3)]){
   let refused=false;
   try{await call();}catch(error){
    assert(error instanceof CudaContextLostError);refused=true;
    const cause=[error.recordedContextId,error.currentContextId,error.driverError];
    assert.equal(cause[0],coordinated.context_id);if(first===null)first=cause;assert.deepEqual(cause,first);
   }
   assert(refused,'lost generation unexpectedly forwarded');
  }
  await releaseModels(); await releaseModels(); let repeat=false; try{await embedBatchCls(inputs)}catch(error){assert(error instanceof CudaContextLostError); assert.deepEqual([error.recordedContextId,error.currentContextId,error.driverError],first); repeat=true} assert(repeat); console.log('INSTALLED-COORDINATE typed-refusal '+JSON.stringify(first));
 }
 await engine.close();engine=null;
 const after=JSON.parse(native.cudaPoolObservationForQualification());
 if(before.status==='private'){
  assert.equal(after.context_id,before.context_id);assert.equal(after.pool_id,before.pool_id);
  assert.equal(BigInt(after.live_models_after),0n);
  if(mode!=='healthy'){assert.equal(after.status,'lost');assert.equal(after.used_current_bytes,null);assert.equal(after.reserved_current_bytes,null);}
 }
 console.log('INSTALLED-COORDINATE POST-DROP '+JSON.stringify({mode,pool:after,released_model_owners:0}));
}
