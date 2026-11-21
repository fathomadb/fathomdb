import assert from 'node:assert/strict';
import path from 'node:path';
import {pathToFileURL} from 'node:url';
const {embedBatchCls,releaseModels,OverloadedError}=await import(pathToFileURL(path.join(process.env.FATHOMDB_PACKAGE_DIR,'dist/index.js')));
const inputs=Array(32).fill('the cat sat on the mat');
const expected=await embedBatchCls(inputs);
let done=false; const worker=embedBatchCls(inputs).finally(()=>{done=true});
const deadline=performance.now()+30000; let attempts=0,busy=false;
while(!done && performance.now()<deadline && attempts<10000){
 attempts++;
 try{await releaseModels()}catch(error){if(!(error instanceof OverloadedError))throw error; assert(!done,'worker finished before Busy observation'); busy=true; break}
}
assert(busy,`no concurrent typed Busy after ${attempts} attempts`);
assert.deepEqual(await worker,expected); await releaseModels(); assert.deepEqual(await embedBatchCls(inputs),expected); await releaseModels();
console.log(`BINDING_BUSY_OK attempts=${attempts}`);
