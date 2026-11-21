#!/usr/bin/env bash
set -euo pipefail
if [[ $# -lt 3 || "$1" != /* || "$2" != /* || "$3" != /* ]]; then
  echo 'usage: run-installed.sh ABSOLUTE_EXTRACTED_PACKAGE ABSOLUTE_FIXTURE_BASELINES ABSOLUTE_UBUNTU22_TARGET [node test arguments...]' >&2
  exit 2
fi
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd -P)"
package="$1"; fixtures="$2"; build_target="$3"; shift 3
node_command="$(command -v node)"
node_directory="$(dirname -- "$node_command")"
node_root="$(dirname -- "$node_directory")"
task_uid="$(id -u)"
task_gid="$(id -g)"
browser="${INSPECTOR_CHROMIUM_DIRECTORY:-$HOME/.cache/ms-playwright/chromium-1248/chrome-linux64}"
modules="${INSPECTOR_NODE_MODULES:-$HOME/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules}"
cache="${XDG_CACHE_HOME:-$HOME/.cache}"
if [[ $# -eq 0 ]]; then
  set -- --test --test-concurrency=1 src/inspector/tests/browser/comparison.mjs src/inspector/tests/browser/lexical.mjs src/inspector/tests/browser/vector.mjs src/inspector/tests/browser/graph.mjs src/inspector/tests/browser/workspace.mjs
fi
extra=()
if [[ -n "${INSPECTOR_EVIDENCE_DIRECTORY:-}" ]]; then
  if [[ "$INSPECTOR_EVIDENCE_DIRECTORY" != /* || ! -d "$INSPECTOR_EVIDENCE_DIRECTORY" ]]; then
    echo 'INSPECTOR_EVIDENCE_DIRECTORY must name an existing absolute directory' >&2
    exit 2
  fi
  extra+=(-e INSPECTOR_EVIDENCE_DIRECTORY=/evidence -v "$INSPECTOR_EVIDENCE_DIRECTORY:/evidence")
fi
if [[ -n "${INSPECTOR_TRACE_HANDOFF:-}" ]]; then
  extra+=(--name "$INSPECTOR_CONTAINER_NAME" --cap-add SYS_PTRACE -e INSPECTOR_TRACE_HANDOFF=1 -v "$INSPECTOR_TRACE_DIRECTORY:/trace")
fi
docker run "${extra[@]}" --rm --network none --security-opt seccomp=unconfined --shm-size=512m \
  --user "$task_uid:$task_gid" \
  -e HOME=/tmp/testhome -e DISPLAY=:91 -e XDG_CACHE_HOME=/cache \
  -e INSPECTOR_QUALIFY_BINARY=/package/fathomdb-inspector \
  -e INSPECTOR_FIXTURE_DIR=/fixtures \
  -e INSPECTOR_FIXTURE_BINARY=/build/x86_64-unknown-linux-gnu/release/examples/fixture \
  -e INSPECTOR_PLAYWRIGHT=/modules/node_modules/playwright/index.mjs \
  -e PATH=/node/bin:/usr/bin:/bin \
  -e HF_TOKEN=inspector70-parent-secret -e OPENAI_API_KEY=inspector70-parent-api-secret \
  -e FATHOMDB_PERF_EXPERIMENTS=1 -e FATHOMDB_PERF_WRITER_PRAGMAS=application_id=987654 \
  -e FATHOMDB_PERF_READER_PRAGMAS=query_only=0 -e FATHOMDB_PERF_SEARCH_LIMIT=1 \
  -e FATHOMDB_EMBED_DEVICE=cuda:999 \
  -v "$root:/work:ro" -v "$package:/package:ro" -v "$fixtures:/fixtures:ro" \
  -v "$build_target:/build:ro" -v "$node_root:/node:ro" -v "$browser:/browser:ro" \
  -v "$modules:/modules/node_modules:ro" -v "$cache/fathomdb:/cache/fathomdb:ro" \
  -w /work "${INSPECTOR_UBUNTU22_IMAGE:-inspector70-jammy:desktop}" bash -c '
    set -euo pipefail
    mkdir -p "$HOME/.cache/ms-playwright/chromium-1248/chrome-linux64"
    install -m 755 /work/src/inspector/tools/installed-browser.sh "$HOME/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome"
    Xvfb :91 -screen 0 1280x900x24 -nolisten tcp >/tmp/xvfb.log 2>&1 & display_pid=$!
    openbox >/tmp/openbox.log 2>&1 & wm_pid=$!
    trap '\''kill "$wm_pid" "$display_pid" 2>/dev/null || true'\'' EXIT
    cat > /tmp/landlock.c <<"C"
#include <stdio.h>
#include <unistd.h>
#include <sys/syscall.h>
int main(void) { long abi = syscall(SYS_landlock_create_ruleset, 0, 0, 1); printf("Landlock ABI %ld\n", abi); return abi < 3; }
C
    cc /tmp/landlock.c -o /tmp/landlock-abi
    /tmp/landlock-abi
    getconf GNU_LIBC_VERSION
    uname -r
    /browser/chrome --version
    node "$@"
  ' bash "$@"
