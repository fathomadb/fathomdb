import assert from 'node:assert/strict';
import fs from 'node:fs';
import { pathToFileURL } from 'node:url';
const [tools, binary, root] = process.argv.slice(2);
const {Client} = await import(pathToFileURL(`${tools}/node_modules/@modelcontextprotocol/sdk/dist/esm/client/index.js`));
const {StdioClientTransport} = await import(pathToFileURL(`${tools}/node_modules/@modelcontextprotocol/sdk/dist/esm/client/stdio.js`));
const results=[];
for (const format of ['text','structured']) {
 const transport=new StdioClientTransport({command:binary,args:['--config',`${root}/${format}.json`],stderr:'pipe'});
 const client=new Client({name:'fathomdb-current-conformance',version:'1.0.0'});
 const transcript=[];
 transport.stderr.on('data',d=>fs.appendFileSync(`${root}/sdk-${format}.stderr`,d));
 let checks=0;
 function check(name,fn){fn();checks++;transcript.push({check:name,pass:true});}
 function payload(result){return format==='structured'?result.structuredContent:result.content[0].text;}
 function refs(result){return format==='structured'?result.structuredContent.results.map(h=>h.ref):result.content[0].text.split('\n').filter(l=>l.startsWith('"')).map(l=>JSON.parse(/^"(?:[^"\\]|\\.)*"/.exec(l)[0]));}
 function handle(result,name){return format==='structured'?result.structuredContent[name]:new RegExp(`(?:^| )${name}=([^ \\n]+)`).exec(result.content[0].text)?.[1];}
 async function call(name,args={},error=false){
   const r=await client.callTool({name,arguments:args});transcript.push({tool:name,args,result:r});
   assert.equal(r.isError===true,error);assert.equal(r.content.length,1);assert.equal(r.content[0].type,'text');
   if(format==='text'){
    assert.equal(r.structuredContent,undefined);
    const budget=/tokens=(\d+)\/(\d+)/.exec(r.content[0].text);
    if(budget){assert.equal(+budget[1],Math.ceil(Buffer.byteLength(r.content[0].text)/4));assert.ok(+budget[1]<=+budget[2]);}
   }
   else {assert.ok(r.structuredContent);assert.deepEqual(JSON.parse(r.content[0].text),r.structuredContent);}
   return r;
 }
 try {
  await client.connect(transport);
  check('tools only capability',()=>assert.deepEqual(client.getServerCapabilities(),{tools:{}}));
  check('ping',()=>{});assert.deepEqual(await client.ping(),{});
  const listed=await client.listTools();transcript.push({discovery:listed});
  check('all seven tool names',()=>assert.deepEqual(listed.tools.map(t=>t.name).sort(),['expand','feedback','fetch','list_stores','orient','search','use_store']));
  check('format discovery',()=>listed.tools.forEach(t=>{assert.equal(!!t.outputSchema,format==='structured');assert.equal(t.inputSchema.additionalProperties,false);}));
  const searchSchema=listed.tools.find(t=>t.name==='search').inputSchema;
  check('current flags and direct filters advertised',()=>['include_superseded','include_inactive','kind','source_type','status','created_after','attributes'].forEach(k=>assert.ok(searchSchema.properties[k])));
  await call('list_stores');await call('use_store',{store:'demo'});await call('orient');
  checks+=3;
  let page=await call('search',{query:'alpha',limit:1,as_of:1900000000,max_tokens:10000,highlights:true});
  const all=[];let pages=0;
  do {all.push(...refs(page));const h=handle(page,'continuation');pages++;if(!h)break;assert.ok(pages<10);page=await call('search',{continuation:h,max_tokens:10000});}while(true);
  check('unchanged state complete no-gap paging',()=>{assert.equal(all.length,4);assert.equal(new Set(all).size,4);assert.deepEqual([...all].sort(),['l:note:1','l:note:2','l:note:3','l:version']);});
  for(const [sup,inactive,count] of [[false,false,4],[true,false,5],[false,true,5],[true,true,6]]){
   const r=await call('search',{query:'alpha',include_superseded:sup,include_inactive:inactive,as_of:1900000000,limit:100,max_tokens:10000});
   check(`existence flags ${sup}/${inactive}`,()=>assert.equal(refs(r).length,count));
  }
  const filtered=await call('search',{query:'alpha',kind:'note',attributes:{group:'wanted'},as_of:1900000000,max_tokens:10000});
  check('direct text filtered selection',()=>assert.deepEqual(refs(filtered).sort(),['l:note:1','l:note:2']));
  const missingSource=await call('search',{query:'alpha',source_type:'nonexistent',max_tokens:10000});
  check('missing source metadata is not a match',()=>assert.deepEqual(refs(missingSource),[]));
  const fetched=await call('fetch',{refs:['l:note:1','l:missing','bad','l:note:1'],max_tokens:10000});
  check('ordered fetch duplicate and per-item errors',()=>{
   const p=payload(fetched);if(format==='structured'){assert.equal(p.items.length,4);assert.equal(p.items[1].error.class,'not_found');assert.equal(p.items[2].error.code,'bad_argument');assert.deepEqual(p.items[0],p.items[3]);}
   else {assert.equal((p.match(/^record "l:note:1"/gm)||[]).length,2);assert.match(p,/error "l:missing" not_found\//);assert.match(p,/error "bad" invalid_request\/bad_argument/);assert.match(p,/café 🧭/);}
  });
  const expanded=await call('expand',{refs:['l:note:1'],max_depth:2,max_tokens:10000});
  check('real graph expansion',()=>format==='structured'?assert.equal(expanded.structuredContent.targets[0].ref,'l:note:2'):assert.match(expanded.content[0].text,/"l:note:2".*hop=1/));
  const first=await call('search',{query:'alpha',limit:1,max_tokens:10000});
  await call('feedback',{response_id:handle(first,'response_id'),used:refs(first)});checks++;
  for(const args of [{store:'missing',query:'alpha'},{query:123},{query:'alpha',include_inactive:null},{query:'alpha',attributes:{undeclared:'x'}},{continuation:handle(first,'continuation'),include_superseded:false}]){
   const r=await call('search',args,true);check('typed format-correct tool error',()=>format==='structured'?assert.ok(r.structuredContent.error.code):assert.match(r.content[0].text,/^error /));
  }
  const hybrid=await call('search',{query:'alpha',mode:'hybrid',max_tokens:10000});
  check('None-provider exact degradation words',()=>{const p=payload(hybrid);if(format==='structured')assert.deepEqual(p.degradation,[{arm:'vector',reason:'embedder_absent'},{arm:'vector',reason:'embedder_unavailable'}]);else {assert.match(p,/embedder_absent/);assert.match(p,/embedder_unavailable/);}});
  await assert.rejects(client.callTool({name:'drop_store',arguments:{}}),e=>e.code===-32602);checks++;
  await assert.rejects(client.request({method:'unknown/method'},{parse:x=>x}),e=>e.code===-32601);checks++;
  await client.ping();checks++;
  results.push({format,checks,pass:true});
 } catch(e) {transcript.push({failure:{message:e.message,stack:e.stack}});results.push({format,checks,pass:false,error:e.message});}
 finally{await client.close();fs.writeFileSync(`${root}/sdk-${format}.json`,JSON.stringify(transcript,null,2));}
}
fs.writeFileSync(`${root}/sdk-summary.json`,JSON.stringify(results,null,2));console.log(results);if(results.some(r=>!r.pass))process.exitCode=1;
