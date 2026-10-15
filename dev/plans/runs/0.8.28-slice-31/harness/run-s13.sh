#!/usr/bin/env bash
# shellcheck disable=SC2312 # measurement harness: command substitutions only format records
# S-13 session steps (test-plan § 4.7, cmds c-wsb-13000 and c-s13-run).
# Runs ON fdb-lab-s13 after vm-setup-cuda13.sh. Usage: bash s13/run-s13.sh setup|wsb02|s13run
set -uo pipefail
STEP="${1:?step}"
D="$HOME/s13"
OUT="$D/out"
SRC="$D/src"
mkdir -p "$OUT"
exec 3>&1 >>"$OUT/$STEP.log" 2>&1
record() { printf '%s\t%s\t%s\t%s\n' "$STEP" "$1" "$2" "${3:-}" >>"$OUT/summary.tsv"; echo "RESULT $STEP $1: $2 ${3:-}"; }
section() { printf '\n===== %s %s\n' "$(date -u +%FT%TZ)" "$*"; }
guard() { echo "guard $1: $(nvidia-smi --query-gpu=name,driver_version,compute_cap --format=csv,noheader) cuda=$(readlink -f /usr/local/cuda 2>/dev/null || echo none) nvcc=$(/usr/local/cuda-13.0/bin/nvcc --version | tail -1)"; }

env13() {
  export PATH="$HOME/.cargo/bin:$PATH"
  export CUDARC_CUDA_VERSION=13000 CUDA_PATH=/usr/local/cuda-13.0 CUDA_HOME=/usr/local/cuda-13.0 \
    PATH=/usr/local/cuda-13.0/bin:$PATH CUDA_COMPUTE_CAP=75
  cd "$SRC" || exit 2
}

run_with_maps() {
  local tag="$1"; shift
  local maps="$OUT/$tag.maps"
  : >"$maps"
  "$@" &
  local pid=$!
  while kill -0 "$pid" 2>/dev/null; do
    grep -E 'libcu|libnv' "/proc/$pid/maps" 2>/dev/null | awk '{print $6}' >>"$maps"
    sleep 0.2
  done
  wait "$pid"
  local rc=$?
  sort -u -o "$maps" "$maps"
  echo "--- CUDA libraries mapped by $tag:"; cat "$maps"
  return "$rc"
}

check_one_passed() {
  local check="$1" file="$2" rc="$3" line
  line="$(grep -E '^test result:' "$file" | tail -1)"
  if [ "$rc" -eq 0 ] && [[ "$line" == *"ok. 1 passed; 0 failed"* ]]; then record "$check" PASS "$line"
  else record "$check" FAIL "exit=$rc $line"; fi
}

step_setup() {
  mkdir -p "$SRC"
  tar -xf "$D/src-14ef6f4d04e6.tar" -C "$D"
  echo "commit (git archive): $(cat "$D/COMMIT")"
  mkdir -p "$HOME/.cargo"
  cp "$D/cargo-config.toml" "$HOME/.cargo/config.toml"
  find "$SRC" -maxdepth 1 | sort | sed -n 1,5p
  record setup PASS "source + cargo config in place"
}

build() {
  local name="$1"; shift
  section "build $name: $*"
  local t0=$SECONDS rc=0
  "$@" >"$OUT/build-$name.out" 2>"$OUT/build-$name.err" || rc=$?
  tail -5 "$OUT/build-$name.err"
  echo "exit=$rc secs=$((SECONDS - t0))"
  [ "$rc" -eq 0 ] && record "build-$name" PASS "exit=0 $((SECONDS - t0))s" || record "build-$name" FAIL "exit=$rc"
  return "$rc"
}

step_wsb02() {
  env13
  section "WSB-02 environment"
  guard before-wsb02
  echo "host: $(hostname) $(uname -srm) vCPU=$(nproc)"
  /usr/local/cuda-13.0/bin/nvcc --version
  rustup show active-toolchain; cargo --version; rustc --version
  env | grep -E '^(CUDA|CUDARC|PATH=)' | sort
  sha256sum Cargo.lock
  local nv; nv="$(nvcc --version | tail -1)"
  [[ "$nv" == *"cuda_13.0"* ]] && record nvcc PASS "$nv" || record nvcc FAIL "$nv"
  build embedder cargo build --locked -p fathomdb-embedder --features embed-cuda,rerank-cuda || return 1
  build cli-release cargo build --locked --release -p fathomdb-cli --features embed-cuda,rerank-cuda || return 1
  build embedder-tests cargo test --locked --no-run -p fathomdb-embedder --features embed-cuda,rerank-cuda || return 1
  section "registry crates"
  cargo tree --locked -p fathomdb-embedder --features embed-cuda,rerank-cuda -e normal --prefix none \
    | grep -E '^(cudarc|candle)' | sort -u | tee "$OUT/wsb02-cuda-crates.txt"
  python3 -I - Cargo.lock <<'PY' | tee -a "$OUT/wsb02-cuda-crates.txt"
import re, sys
text = open(sys.argv[1]).read()
for block in text.split("[[package]]"):
    m = re.search(r'name = "([^"]+)"\nversion = "([^"]+)"(?:\nsource = "([^"]+)")?', block)
    if m and (m.group(1).startswith("cudarc") or m.group(1).startswith("candle")):
        print("lock:", m.group(1), m.group(2), m.group(3) or "path")
PY
  if grep -q '^cudarc-fathomdb v0.19.10' "$OUT/wsb02-cuda-crates.txt" && ! grep -q '^cudarc v' "$OUT/wsb02-cuda-crates.txt" \
     && grep -q 'candle-core-fathomdb v0.11.0' "$OUT/wsb02-cuda-crates.txt"; then
    record registry-crates PASS "cudarc-fathomdb 0.19.10 (registry), candle-*-fathomdb 0.11.0; no plain cudarc"
  else
    record registry-crates FAIL "see wsb02-cuda-crates.txt"
  fi
  section "c-s13-run test binaries (loader-test-hooks)"
  build s13-tests cargo test --locked --no-run -p fathomdb-embedder --features embed-cuda,rerank-cuda,loader-test-hooks --message-format=json || return 1
  python3 -I - "$OUT/build-s13-tests.out" <<'PY' | tee "$OUT/s13-exes.txt"
import json, sys
for line in open(sys.argv[1]):
    try: m = json.loads(line)
    except ValueError: continue
    if m.get("reason") == "compiler-artifact" and m.get("executable") and m["profile"]["test"]:
        name, kind = m["target"]["name"], m["target"]["kind"]
        if name == "candle_bge" or (name == "fathomdb_embedder" and "lib" in kind):
            print(name, m["executable"])
PY
  guard after-wsb02
}

step_s13run() {
  env13
  guard before-s13run
  tar -I zstd -xf "$HOME/s13-models.tar.zst" -C "$D"
  mkdir -p "$HOME/.cache/fathomdb/embedders" "$HOME/.cache/fathomdb/reranker"
  cp -rn "$D/models/embedders/." "$HOME/.cache/fathomdb/embedders/"
  cp -rn "$D/models/reranker/." "$HOME/.cache/fathomdb/reranker/"
  ls -R "$HOME/.cache/fathomdb"
  local bge lib cli="$SRC/target/release/fathomdb"
  bge="$(awk '$1=="candle_bge"{print $2; exit}' "$OUT/s13-exes.txt")"
  lib="$(awk '$1=="fathomdb_embedder"{print $2; exit}' "$OUT/s13-exes.txt")"
  echo "CANDLE_BGE_BIN=$bge"; echo "EMBEDDER_LIB_BIN=$lib"; echo "LD_LIBRARY_PATH=${LD_LIBRARY_PATH-(unset)}"
  ldconfig -p | grep -E 'libcublas|libcurand|libcuda\.' ; cat /etc/ld.so.conf.d/*cuda* 2>/dev/null

  section "CUDA-08: doctor cuda-allocator"
  local rc=0
  FATHOMDB_EMBED_DEVICE=cuda:0 "$cli" doctor cuda-allocator --json >"$OUT/s13-doctor-allocator.json" 2>"$OUT/s13-doctor-allocator.stderr" || rc=$?
  cat "$OUT/s13-doctor-allocator.json" "$OUT/s13-doctor-allocator.stderr"; echo "exit=$rc"
  python3 -I - "$OUT/s13-doctor-allocator.json" <<'PY' && record cuda08-doctor-allocator PASS "built=false path=null reason=not_applicable module_load_init=ran (exit=$rc)" || record cuda08-doctor-allocator FAIL "exit=$rc; see s13run.log"
import json, sys
d = json.load(open(sys.argv[1]))
def find(o, k):
    if isinstance(o, dict):
        if k in o: return o[k]
        for v in o.values():
            r = find(v, k)
            if r is not None: return r
    return None
ok = find(d, "built") is False and d.get("path", find(d, "path")) is None and find(d, "reason") == "not_applicable" and find(d, "module_load_init") == "ran"
sys.exit(0 if ok else 1)
PY

  section "CMP-C5: doctor cuda-libraries"
  rc=0
  "$cli" doctor cuda-libraries --json >"$OUT/s13-doctor-libs.json" 2>"$OUT/s13-doctor-libs.stderr" || rc=$?
  cat "$OUT/s13-doctor-libs.json" "$OUT/s13-doctor-libs.stderr"; echo "exit=$rc"
  python3 -I - "$OUT/s13-doctor-libs.json" "$rc" <<'PY' && record c5-doctor-libs PASS "exit=0 built_major=13 cublas 13.x loaded from the 13.0 toolkit" || record c5-doctor-libs FAIL "exit=$rc; see s13run.log"
import json, os, sys
d = json.load(open(sys.argv[1])); rc = int(sys.argv[2])
cb = next((x for x in d["libraries"] if x["library"] == "cublas"), {})
print("cublas:", cb)
p = os.path.realpath(cb.get("path") or "")
ok = rc == 0 and d.get("built_major") == 13 and cb.get("status") == "loaded" and str(cb.get("version") or "").startswith("13.") and p.startswith("/usr/local/cuda-13")
sys.exit(0 if ok else 1)
PY

  section "CUDA-08: forced live embed on cuda:0"
  local f="$OUT/s13-embed.txt"; rc=0
  run_with_maps s13-embed env FATHOMDB_EMBED_DEVICE=cuda:0 FATHOMDB_REQUIRE_LIVE=1 \
    "$bge" --exact embed_returns_unit_norm_vector --nocapture --test-threads=1 >"$f" 2>&1 || rc=$?
  cat "$f"; check_one_passed cuda08-embed "$f" "$rc"

  section "CUDA-08: reranker on cuda:0"
  f="$OUT/s13-rerank.txt"; rc=0
  run_with_maps s13-rerank env FATHOMDB_REQUIRE_LIVE=1 \
    "$lib" --exact candle_reranker::gpu_tests::gpu_loads_and_scores_finite --nocapture --test-threads=1 >"$f" 2>&1 || rc=$?
  cat "$f"; check_one_passed cuda08-rerank "$f" "$rc"
  if grep -qE 'cuda-13' "$OUT/s13-embed.maps" && ! grep -qE 'libcublas\.so\.12|cuda-12' "$OUT/s13-embed.maps" "$OUT/s13-rerank.maps"; then
    record c5-maps PASS "$(grep cublas "$OUT/s13-embed.maps" | paste -sd' ')"
  else
    record c5-maps FAIL "$(paste -sd' ' "$OUT/s13-embed.maps")"
  fi
  guard after-s13run
}

section "S-13 step $STEP on $(hostname)"
case "$STEP" in
  setup) step_setup ;;
  wsb02) step_wsb02 ;;
  s13run) step_s13run ;;
  *) echo "unknown step" >&2; exit 2 ;;
esac
section "end of step $STEP"
grep -P "^$STEP\t" "$OUT/summary.tsv" >&3
