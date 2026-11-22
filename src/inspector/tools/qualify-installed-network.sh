#!/usr/bin/env bash
set -euo pipefail
if [[ $# -ne 4 || "$1" != /* || "$2" != /* || "$3" != /* || "$4" != /* || -e "$4" ]]; then
  echo 'usage: qualify-installed-network.sh ABSOLUTE_PACKAGE ABSOLUTE_FIXTURES ABSOLUTE_TARGET ABSOLUTE_NEW_TRACE_DIRECTORY' >&2
  exit 2
fi
command -v strace >/dev/null || { echo 'prerequisite: strace is missing' >&2; exit 2; }
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd -P)"
mkdir -m 700 -- "$4"
output="$4/runtime.log"
cache_root="${XDG_CACHE_HOME:-$HOME/.cache}/fathomdb"
rc=0
INSPECTOR_TRACE_OUTPUT="$4/native" \
  bash "$root/src/inspector/tools/run-installed.sh" "$1" "$2" "$3" \
    --test --test-name-pattern='real person investigation' \
    src/inspector/tests/browser/comparison.mjs >"$output" 2>&1 || rc=$?
cat "$output"
if [[ "$rc" -ne 0 ]]; then
  exit "$rc"
fi
node --input-type=module - "$4" "$1/fathomdb-inspector" "$cache_root" <<'JS'
import assert from 'node:assert/strict';
import {readdir,readFile} from 'node:fs/promises';
import {realpathSync} from 'node:fs';
const directory=process.argv[2],binary=realpathSync(process.argv[3]),cacheRoot=realpathSync(process.argv[4]);
const files=(await readdir(directory)).filter(n=>/^native\.\d+$/.test(n));
assert.ok(files.length>1,'actual traced supervisor and native descendants required');
const byPid=new Map();
for(const name of files){
 const pid=Number(name.slice('native.'.length));
 byPid.set(pid,await readFile(`${directory}/${name}`,'utf8'));
}
const supervisorPids=new Set();
const workerPids=new Set();
const browserPids=new Set();
for(const [pid,content] of byPid){
 const isBinary=content.includes(`execve("${binary}"`)||content.includes(`execve("${process.argv[3]}"`);
 if(isBinary&&content.includes('--session-worker')){
  workerPids.add(pid);
 }else if(isBinary){
  supervisorPids.add(pid);
 }else if(content.includes('chrome')&&content.includes('execve(')){
  browserPids.add(pid);
 }
}
assert.ok(supervisorPids.size>=1,'trace must include exact packaged supervisor');
assert.ok(workerPids.size>=1,'trace must include exact packaged --session-worker');
function expand(seed,exclude){
 const set=new Set(seed);
 let grew=true;
 while(grew){
  grew=false;
  for(const pid of [...set]){
   const content=byPid.get(pid)??'';
   for(const match of content.matchAll(/\b(?:clone3?|fork|vfork)\([^\n]*\)\s*=\s*(\d+)/g)){
    const child=Number(match[1]);
    if(byPid.has(child)&&!set.has(child)&&!exclude.has(child)){set.add(child);grew=true;}
   }
  }
 }
 return set;
}
const allWorkerPids=expand(workerPids,new Set());
const allBrowserPids=expand(browserPids,new Set([...supervisorPids,...allWorkerPids]));
const allSupervisorPids=expand(supervisorPids,new Set([...allWorkerPids,...allBrowserPids]));
const workerInternet=[];
for(const pid of allWorkerPids){
 for(const line of (byPid.get(pid)??'').split('\n')){
  if(/socket\(AF_INET6?\b|connect\([^\n]*sa_family=AF_INET6?\b/.test(line))workerInternet.push({pid,line});
 }
}
assert.deepEqual(workerInternet,[],'no Internet-family socket or connect attempts from installed native CPU worker tree');
const supervisorOutbound=[];
for(const pid of allSupervisorPids){
 for(const line of (byPid.get(pid)??'').split('\n')){
  if(/(?:connect|sendto|sendmsg)\([^\n]*sa_family=AF_INET6?\b|socket\(AF_INET6\b|socket\(AF_INET,\s*SOCK_DGRAM/.test(line))supervisorOutbound.push({pid,line});
 }
}
assert.deepEqual(supervisorOutbound,[],'no outbound Internet-family connect/send or UDP/IPv6 socket attempts from installed supervisor tree');
const opens=[];
const mutations=[];
const cachePrefixes=[cacheRoot,process.argv[4]].filter((v,i,a)=>a.indexOf(v)===i);
const hasCachePath=(line)=>cachePrefixes.some(p=>line.includes(`"${p}/`)||line.includes(`"${p}"`));
for(const [pid,content] of byPid){
 for(const line of content.split('\n')){
  if(line.startsWith('openat(')&&hasCachePath(line))opens.push(line);
  if(/^(mkdir|mkdirat|unlink|unlinkat|rename|renameat2?|chmod|fchmodat|chown|fchownat|truncate)\(/.test(line)&&hasCachePath(line))mutations.push({pid,line});
 }
}
for(const asset of ['config.json','tokenizer.json','model.safetensors']){
 assert.ok(opens.some(line=>line.includes(`/${asset}"`)&&line.includes('O_RDONLY')),`pinned source asset ${asset} opened read-only`);
}
assert.ok(opens.every(line=>!/(O_WRONLY|O_RDWR|O_CREAT|O_TRUNC|O_APPEND)/.test(line)),'source cache has no write opens');
assert.deepEqual(mutations,[],'source cache has no mutation calls');
console.log(JSON.stringify({
 qualification:'installed_cached_cpu_network_and_cache',
 traced_files:files.length,
 supervisor_traced_pids:allSupervisorPids.size,
 worker_traced_pids:allWorkerPids.size,
 worker_internet_family_attempts:workerInternet.length,
 supervisor_outbound_attempts:supervisorOutbound.length,
 source_asset_readonly_opens:opens.length,
 source_cache_mutations:mutations.length,
}));
JS
