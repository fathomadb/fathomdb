#!/usr/bin/env bash
set -euo pipefail
if [[ $# -ne 4 || "$4" != /* || -e "$4" ]]; then
  echo 'usage: qualify-installed-network.sh ABSOLUTE_PACKAGE ABSOLUTE_FIXTURES ABSOLUTE_TARGET ABSOLUTE_NEW_TRACE_DIRECTORY' >&2
  exit 2
fi
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd -P)"
mkdir -m 700 -- "$4"
export INSPECTOR_TRACE_DIRECTORY="$4"
export INSPECTOR_TRACE_HANDOFF=1
export INSPECTOR_CONTAINER_NAME="inspector70-network-$$"
output="$4/runtime.log"
bash "$root/src/inspector/tools/run-installed.sh" "$1" "$2" "$3" --test --test-name-pattern='real person investigation' src/inspector/tests/browser/comparison.mjs >"$output" 2>&1 & runtime_pid=$!
# The test waits before Engine attachment; trace only its exact recorded production supervisor.
for ((n=0;n<1500;n++)); do
  if rg -q '^TRACE_SUPERVISOR=[0-9]+$' "$output"; then break; fi
  if ! kill -0 "$runtime_pid" 2>/dev/null; then cat "$output"; wait "$runtime_pid"; exit 1; fi
  sleep .02
done
supervisor="$(sed -n 's/^TRACE_SUPERVISOR=\([0-9]*\)$/\1/p' "$output")"
if [[ ! "$supervisor" =~ ^[0-9]+$ ]]; then cat "$output"; wait "$runtime_pid"; exit 1; fi
docker exec --user 0 "$INSPECTOR_CONTAINER_NAME" strace -ff -e trace=network,%file -o /trace/native -p "$supervisor" >"$4/tracer.log" 2>&1 & tracer_pid=$!
for ((n=0;n<500;n++)); do
  if rg -q 'Process .* attached' "$4/tracer.log"; then break; fi
  if ! kill -0 "$tracer_pid" 2>/dev/null; then cat "$4/tracer.log"; wait "$runtime_pid"; wait "$tracer_pid"; exit 1; fi
  sleep .02
done
rg -q 'Process .* attached' "$4/tracer.log"
docker exec --user 0 "$INSPECTOR_CONTAINER_NAME" touch "/tmp/inspector70-trace-ready-$supervisor"
wait "$runtime_pid"
wait "$tracer_pid"
cat "$output"
node --input-type=module - "$4" <<'JS'
import assert from 'node:assert/strict';
import {readdir,readFile} from 'node:fs/promises';
const directory=process.argv[2],files=(await readdir(directory)).filter(n=>/^native\.\d+$/.test(n));
assert.ok(files.length>1,'actual traced supervisor and native descendants required');
const traces=await Promise.all(files.map(n=>readFile(`${directory}/${n}`,'utf8'))),text=traces.join('\n');
assert.match(text,/execve\("\/package\/fathomdb-inspector", \[.*--session-worker/,'trace includes exact packaged worker');
const internet=text.split('\n').filter(line=>/socket\(AF_INET6?\b|connect\([^\n]*sa_family=AF_INET6?\b/.test(line));
assert.deepEqual(internet,[],'no Internet-family attempts from installed supervisor/native CPU tree');
const opens=text.split('\n').filter(line=>/openat\(.*"\/cache\/fathomdb\//.test(line));
for(const asset of ['config.json','tokenizer.json','model.safetensors'])assert.ok(opens.some(line=>line.includes(`/${asset}"`)&&line.includes('O_RDONLY')),'pinned source asset opened read-only');
assert.ok(opens.every(line=>!/(O_WRONLY|O_RDWR|O_CREAT|O_TRUNC|O_APPEND)/.test(line)),'source cache has no write opens');
assert.ok(!text.split('\n').some(line=>/^(mkdir|unlink|rename|chmod|chown|truncate).*"\/cache\/fathomdb\//.test(line)),'source cache has no mutation calls');
console.log(JSON.stringify({qualification:'installed_cached_cpu_network_and_cache',traced_files:files.length,internet_family_attempts:internet.length,source_asset_readonly_opens:opens.length}));
JS
