#!/usr/bin/env bash
set -euo pipefail
if [[ $# -lt 3 || "$1" != /* || "$2" != /* || "$3" != /* ]]; then
  echo 'usage: run-installed.sh ABSOLUTE_EXTRACTED_PACKAGE ABSOLUTE_FIXTURE_BASELINES ABSOLUTE_BUILD_TARGET [node test arguments...]' >&2
  exit 2
fi
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd -P)"
package="$1"
fixtures="$2"
build_target="$3"
shift 3
fail_prerequisite() { echo "prerequisite: $*" >&2; exit 2; }
for tool in node Xvfb flock python3; do
  command -v "$tool" >/dev/null || fail_prerequisite "$tool is missing"
done
[[ -x "$package/fathomdb-inspector" ]] || fail_prerequisite 'extracted fathomdb-inspector executable is missing'
[[ -d "$fixtures" ]] || fail_prerequisite 'fixtures directory is missing'
[[ -x "$build_target/x86_64-unknown-linux-gnu/release/examples/fixture" ]] || fail_prerequisite 'release fixture example binary is missing'
browser="${INSPECTOR_CHROMIUM_DIRECTORY:-$HOME/.cache/ms-playwright/chromium-1248/chrome-linux64}"
modules="${INSPECTOR_NODE_MODULES:-$HOME/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules}"
cache="${XDG_CACHE_HOME:-$HOME/.cache}"
[[ -x "$browser/chrome" ]] || fail_prerequisite 'installed Chromium executable is missing'
[[ -f "$modules/playwright/index.mjs" ]] || fail_prerequisite 'existing Playwright modules are missing'
source_cache="$cache/fathomdb/embedders/0b2926f8a9b1"
for asset in config.json tokenizer.json model.safetensors; do
  [[ -r "$source_cache/$asset" ]] || fail_prerequisite "pinned asset $source_cache/$asset is missing or unreadable"
done
if [[ -n "${INSPECTOR_EVIDENCE_DIRECTORY:-}" ]]; then
  if [[ "$INSPECTOR_EVIDENCE_DIRECTORY" != /* || ! -d "$INSPECTOR_EVIDENCE_DIRECTORY" ]]; then
    echo 'INSPECTOR_EVIDENCE_DIRECTORY must name an existing absolute directory' >&2
    exit 2
  fi
fi
if [[ $# -eq 0 ]]; then
  set -- --test --test-concurrency=1 \
    src/inspector/tests/browser/comparison.mjs \
    src/inspector/tests/browser/lexical.mjs \
    src/inspector/tests/browser/vector.mjs \
    src/inspector/tests/browser/graph.mjs \
    src/inspector/tests/browser/workspace.mjs
fi
runtime_tmp="$(mktemp -d "${TMPDIR:-/tmp}/inspector-installed.XXXXXX")"
display_pid=""
cleanup() {
  if [[ -n "$display_pid" ]]; then
    kill "$display_pid" 2>/dev/null || true
    wait "$display_pid" 2>/dev/null || true
  fi
  rm -rf -- "$runtime_tmp"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

testhome="$runtime_tmp/home"
mkdir -p -- "$testhome/.config"
bash "$root/src/inspector/tools/installed-browser.sh" \
  "$browser/chrome" \
  "$testhome/.cache/ms-playwright/chromium-1248/chrome-linux64/chrome"

display_file="$runtime_tmp/display"
Xvfb -displayfd 3 -screen 0 1280x900x24 -nolisten tcp 3>"$display_file" >"$runtime_tmp/xvfb.log" 2>&1 &
display_pid=$!
display_num=""
for ((n = 0; n < 250; n++)); do
  if [[ -s "$display_file" ]]; then
    display_num="$(tr -d '[:space:]' < "$display_file")"
    break
  fi
  if ! kill -0 "$display_pid" 2>/dev/null; then
    cat "$runtime_tmp/xvfb.log" >&2 || true
    exit 1
  fi
  sleep 0.02
done
if [[ ! "$display_num" =~ ^[0-9]+$ ]]; then
  cat "$runtime_tmp/xvfb.log" >&2 || true
  echo 'Xvfb failed to report a display number' >&2
  exit 1
fi

python3 - <<'PY'
import ctypes
abi = ctypes.CDLL(None).syscall(444, 0, 0, 1)
print(f"Landlock ABI {abi}")
assert abi >= 3, f"Landlock ABI {abi} < 3"
PY
getconf GNU_LIBC_VERSION
uname -r
"$browser/chrome" --version

cd -- "$root"
env \
  -u WAYLAND_DISPLAY \
  -u XAUTHORITY \
  -u DBUS_SESSION_BUS_ADDRESS \
  -u XDG_RUNTIME_DIR \
  HOME="$testhome" \
  XDG_CONFIG_HOME="$testhome/.config" \
  XDG_CACHE_HOME="$cache" \
  DISPLAY=":$display_num" \
  INSPECTOR_QUALIFY_BINARY="$package/fathomdb-inspector" \
  INSPECTOR_FIXTURE_DIR="$fixtures" \
  INSPECTOR_FIXTURE_BINARY="$build_target/x86_64-unknown-linux-gnu/release/examples/fixture" \
  INSPECTOR_PLAYWRIGHT="$modules/playwright/index.mjs" \
  INSPECTOR_CHROMIUM="$browser/chrome" \
  INSPECTOR_SOURCE_CACHE="$source_cache" \
  HF_TOKEN=inspector70-parent-secret \
  OPENAI_API_KEY=inspector70-parent-api-secret \
  FATHOMDB_PERF_EXPERIMENTS=1 \
  FATHOMDB_PERF_WRITER_PRAGMAS=application_id=987654 \
  FATHOMDB_PERF_READER_PRAGMAS=query_only=0 \
  FATHOMDB_PERF_SEARCH_LIMIT=1 \
  FATHOMDB_EMBED_DEVICE=cuda:999 \
  node "$@"
