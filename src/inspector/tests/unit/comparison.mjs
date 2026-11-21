import test from 'node:test';
import assert from 'node:assert/strict';
import { entries, comparisonPage } from '../../assets/comparison.mjs';
const context={database:'db-é🦀',attachment:'session',operation:'1'};
const origin=(id,cursor,cls='node',revision='r1')=>({handle:`${cls}-${cursor}-${id}`,record:{class:cls,logical_id:id,write_cursor:cursor,revision_id:revision}});
const native=(space,value,cursor,origins=[])=>({class:'node',native_id:{space,value,complete:true},write_cursor:cursor,representative_branch:'text',rank:'7',score:'0.01639344262295082',payload:{text:'identical',complete:false},origin:{state:origins.length===1?'exact':'choices',targets:origins}});
const lexical=items=>({context,observation:1,items,scope:{score_family:'rrf',score_direction:'higher_better'},query_context:{route:'body'},request:{route:'body',query:'person',limit:10}});
const graph=records=>({context,observation:2,nodes:records,edges:[],scope:{route:'stored_adjacency'}});
const key=e=>JSON.stringify(e.identity);

test('typed entries preserve exact namespace/physical revision and separate zero/one/many origins',()=>{
 const a=origin('同じ 🦀','9007199254740992'),b=origin('同じ 🦀','9007199254740993');
 const items=[native('logical','x','9007199254740992',[]),native('content','x','9007199254740992',[a]),native('passage','x','9007199254740992',[a,b]),native('logical','x','9007199254740993',[a])];
 const rows=entries(lexical(items),{op:'lexical'});
 assert.equal(rows.length,4);assert.equal(new Set(rows.map(key)).size,4);
 assert.deepEqual(rows.map(r=>r.origins.length),[0,1,2,1]);
 assert.equal(rows[0].rank,'7');assert.equal(rows[0].value,'0.01639344262295082');
 assert.equal(rows[2].item,items[2],'entries reference the one admitted payload store');
 assert.match(key(rows[3]),/9007199254740993/);
});

test('graph membership keys use class/cursor/revision and never local endpoint keys or invented rank',()=>{
 const records=[origin('same','2','node','r1').record,origin('same','3','node','r2').record,origin('same','2','edge','r1').record,origin('same','2','node',null).record].map((r,i)=>({...r,key:`endpoint:${i}`,handle:`h${i}`}));
 const rows=entries(graph(records),{op:'graph'});
 assert.equal(new Set(rows.map(key)).size,4);
 assert.ok(rows.every(r=>r.rank===null&&r.value===null));
 const other=entries(graph(records.map(r=>({...r,key:'different-local-key'}))),{op:'graph'});
 assert.deepEqual(rows.map(key),other.map(key));
 assert.notEqual(key(rows[0]),key(entries({...graph(records),context:{...context,attachment:'other'}},{op:'graph'})[0]));
});

test('physical vector windows and frozen target/edge entries never collapse to canonical origin',()=>{
 const shared=origin('person','20');
 const vectors={context,items:[1,2].map(n=>({vector_rowid:String(n),source_cursor:'20',profile:'p',rank:String(n),distance:'0.125',origin:{targets:[shared]}})),scope:{metric:'uncentered_f32_euclidean_l2',direction:'lower_nearer'}};
 const rows=entries(vectors,{op:'neighborhood'});assert.equal(new Set(rows.map(key)).size,2);assert.equal(rows[0].metric,'uncentered_f32_euclidean_l2');assert.equal(rows[0].direction,'lower_nearer');
 const frozen={context,items:[1,2].map(n=>({target:shared.record,terminal_edge:origin('edge',String(n),'edge',`e${n}`).record,origin_records:[shared.record],payload:{text:'person'}})),scope:{route:'actual_serving_graph_evidence'}};
 const targets=entries(frozen,{op:'graph_serving'});assert.equal(new Set(targets.map(key)).size,2);assert.ok(targets.every(r=>r.rank===null));
});

test('bounded union/intersection keeps own values and multiset entries with no absent rank zero',()=>{
 const a=entries(lexical([native('logical','A','1'),native('logical','B','2'),native('logical','B','2')]),{op:'lexical'});
 const b=entries(lexical([native('logical','B','2'),native('logical','C','3')]),{op:'lexical'});
 const union=comparisonPage(a,b,{mode:'union',start:0,limit:50});
 assert.equal(union.total,4);assert.deepEqual(union.rows.map(r=>[!!r.A,!!r.B]),[[true,false],[true,true],[true,false],[false,true]]);
 assert.equal(union.rows[0].B,null);assert.equal(union.rows[1].A.value,'0.01639344262295082');
 assert.equal(comparisonPage(a,b,{mode:'intersection',start:0,limit:50}).total,1);
 const many=entries(lexical(Array.from({length:200},(_,i)=>native('logical',String(i),String(i)))),{op:'lexical'});
 const disjoint=entries(lexical(Array.from({length:200},(_,i)=>native('logical',String(i+200),String(i+200)))),{op:'lexical'});
 const seen=[];for(let start=0;start<400;start+=50){const page=comparisonPage(many,disjoint,{mode:'union',start,limit:50});assert.equal(page.total,400);assert.ok(page.rows.length<=50);seen.push(...page.rows.map(r=>key(r.A??r.B)));}assert.equal(new Set(seen).size,400);
});

test('unknown native identities cannot join by text or canonical origin; bounded membership properties',()=>{
 const unknown=native('content',null,'1',[origin('same','1')]);unknown.native_id.complete=false;
 const left=entries(lexical([unknown]),{op:'lexical'}),right=entries({...lexical([unknown]),observation:2},{op:'lexical'});
 assert.equal(comparisonPage(left,right,{mode:'intersection'}).total,0);
 for(let n=0;n<=20;n++)for(let m=0;m<=20;m++){
  const a=entries(lexical(Array.from({length:n},(_,i)=>native('logical',String(i),String(i)))),{op:'lexical'});
  const b=entries(lexical(Array.from({length:m},(_,i)=>native('logical',String(i+5),String(i+5)))),{op:'lexical'});
  const intersection=Math.max(0,Math.min(n,5+m)-5);
  assert.equal(comparisonPage(a,b,{mode:'union'}).total,n+m-intersection);
  assert.equal(comparisonPage(a,b,{mode:'intersection'}).total,intersection);
 }
});

test('operational comparison identities retain collection and exact key alongside cursor/entry',()=>{
 const data={context,items:[{class:'current',collection_name:'é',record_key:'9007199254740993',write_cursor:'8',payload_status:'refused',handle:'h'}]};
 const row=entries(data,{op:'page'})[0];
 assert.ok(row.identity.includes('9007199254740993'),'operational physical identity must retain its exact record key');
 assert.equal(row.origins[0].metadataOnly,true);
 assert.equal(row.item,data.items[0]);
});

test('representative retrieval arms do not change one native physical identity',()=>{
 const hit=native('logical','same-é🦀','9007199254740993',[origin('same-é🦀','9007199254740993')]);
 const a=entries(lexical([hit]),{op:'lexical'});
 const vector={...hit,representative_branch:'Vector',rank:'2',score:'0.03225806451612903'};
 const b=entries({...lexical([vector]),observation:2,scope:{score_family:'engine_rrf',score_direction:'higher_better'}},{op:'serving'});
 const result=comparisonPage(a,b,{mode:'intersection'});
 assert.equal(result.total,1,'same class/IdSpace/value/cursor must overlap across representative arms');
 assert.equal(result.rows[0].A.item.representative_branch,'text');assert.equal(result.rows[0].B.item.representative_branch,'Vector');
 assert.equal(result.rows[0].A.rank,'7');assert.equal(result.rows[0].B.rank,'2');
});
