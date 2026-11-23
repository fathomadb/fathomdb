import test from 'node:test';
import assert from 'node:assert/strict';
import * as comparison from '../../assets/comparison.mjs';
const origin=(cursor='9007199254740993',revision='r1',cls='node')=>({handle:'unrelated-handle',record:{class:cls,write_cursor:cursor,revision_id:revision,logical_id:'same label'}});
const entry=(native,origins=[origin()],session='session',database='db')=>({key:native,identity:[database,session,native],origins,item:{},rank:'7',value:'0.5'});
test('exact canonical correspondence is separate from native membership and preserves independent entries',()=>{
 const A=[entry('body')],B=[entry('bm25f')];
 const groups=comparison.canonicalCorrespondence?.(A,B)??new Map();
 assert.equal(groups.size,1,'same admitted exact canonical revision must expose a separate correspondence');
 const group=[...groups.values()][0];assert.equal(group.A[0],A[0]);assert.equal(group.B[0],B[0]);
 assert.equal(comparison.comparisonPage(A,B,{mode:'intersection'}).total,0);
 assert.equal(comparison.comparisonPage(A,B).total,2);
 for(const other of [entry('different',[origin('9007199254740992')]),entry('revision',[origin(undefined,'r2')]),entry('class',[origin(undefined,'r1','edge')]),entry('session',[origin()],'other'),entry('database',[origin()],'session','other'),entry('unknown',[origin(undefined,null)]),entry('many',[origin(),origin('2')]),entry('withheld',[{...origin(),metadataOnly:true}]),entry('missing',[])])assert.equal(comparison.canonicalCorrespondence(A,[other]).size,0);
 for(let n=1;n<=24;n++){
  const many=Array.from({length:n},(_,i)=>entry('window-'+i));const result=comparison.canonicalCorrespondence(many,B);
  assert.equal(result.size,1);assert.equal([...result.values()][0].A.length,n);assert.equal(comparison.comparisonPage(many,B).total,n+1);
 }
});

import {propertyChoices} from '../../assets/presentation.mjs';
test('property choices accept default tokenizer and exact CSV roles but refuse incomplete or edge-only declarations',()=>{
 const p={name:'title',name_bytes:'5',name_complete:true,entity_class:'node',roles:'filterable,searchable',roles_bytes:'21',recorded_tokenizer:'',tokenizer_bytes:'0'};
 assert.deepEqual(propertyChoices({projections:[p]}),['title']);
 const legacy={name:'legacy',name_bytes:'6',name_complete:true,roles:'filterable,searchable',roles_bytes:'21',recorded_tokenizer:'',tokenizer_bytes:'0'};
 assert.deepEqual(propertyChoices({projections:[legacy]}),['legacy']);
 for(const change of [{entity_class:'edge'},{recorded_tokenizer:null,tokenizer_bytes:null},{name_complete:false},{name_bytes:'5000'},{roles_bytes:'4'},{roles:'filterable',roles_bytes:'10'},{tokenizer_bytes:'20'}])assert.deepEqual(propertyChoices({projections:[{...p,...change}]}),[]);
 assert.deepEqual(propertyChoices({evicted:true,projections:[p]}),[]);
});

import * as presentation from '../../assets/presentation.mjs';
test('UTC display uses exact declared epoch units and leaves unknown values unformatted',()=>{
 assert.equal(presentation.epochUTC?.('1700000000')??null,'2023-11-14T22:13:20.000Z');
 assert.equal(presentation.epochUTC('1700000000123','milliseconds'),'2023-11-14T22:13:20.123Z');
 assert.equal(presentation.epochUTC('0'),'1970-01-01T00:00:00.000Z');
 assert.equal(presentation.epochUTC('-1'),'1969-12-31T23:59:59.000Z');
 for(const value of ['9007199254740993','8640000000001','1.2','unknown','',null])assert.equal(presentation.epochUTC(value),null);
 for(let value=-3000;value<=3000;value+=37)assert.equal(Date.parse(presentation.epochUTC(String(value))),value*1000);
});
