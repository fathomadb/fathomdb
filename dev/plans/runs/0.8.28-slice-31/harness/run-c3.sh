#!/usr/bin/env bash
# shellcheck disable=SC2312 # measurement harness: command substitutions only format records
# 0.8.28 Slice 31 S-C3 session steps (test-plan § 4.6, design § 9.3).
# Runs ON the GCP lab VM `fdb-lab-c3` (driver 580 only: no toolkit, no
# cuBLAS), from the pushed bundle directory:
#   c3/run-c3.sh  c3/bin/{slice31_cuda_library_major,candle_bge,embedder_lib,fathomdb-cli.tar.gz}
#   c3/models/{embedders,reranker}/<prefix>/  c3/wheels/pip-*.whl
# Usage: bash c3/run-c3.sh refusal|remedy|libcuda1
# Each step writes c3/out/<step>.log and appends one line per check to
# c3/out/summary.tsv (step, check, result, detail). Steps run in that order;
# libcuda1 is last because dpkg-divert changes the VM.
set -uo pipefail

STEP="${1:?usage: run-c3.sh refusal|remedy|libcuda1}"
C3="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUT="$C3/out"
BIN="$C3/bin"
VENV="$C3/venv"
mkdir -p "$OUT"
LOG="$OUT/$STEP.log"
# The original stdout stays on fd 3 for the closing summary.
exec 3>&1 >>"$LOG" 2>&1
LIBMAJOR="$BIN/slice31_cuda_library_major"
CANDLE_BGE="$BIN/candle_bge"
EMBEDDER_LIB="$BIN/embedder_lib"
CLI="$C3/cli/fathomdb"

record() { printf '%s\t%s\t%s\t%s\n' "$STEP" "$1" "$2" "${3:-}" >>"$OUT/summary.tsv"; echo "RESULT $STEP $1: $2 ${3:-}"; }
section() { printf '\n===== %s %s\n' "$(date -u +%FT%TZ)" "$*"; }

# Model files go where the loaders look (dirs::cache_dir() = ~/.cache), so no
# step downloads a model.
setup() {
  mkdir -p "$HOME/.cache/fathomdb/embedders" "$HOME/.cache/fathomdb/reranker"
  cp -rn "$C3/models/embedders/." "$HOME/.cache/fathomdb/embedders/"
  cp -rn "$C3/models/reranker/." "$HOME/.cache/fathomdb/reranker/"
  if [ ! -x "$CLI" ]; then
    mkdir -p "$C3/cli"
    tar -xzf "$BIN/fathomdb-cli.tar.gz" -C "$C3/cli" --strip-components=1
  fi
  chmod +x "$LIBMAJOR" "$CANDLE_BGE" "$EMBEDDER_LIB" "$CLI"
}

preconditions() {
  section "preconditions ($1)"
  echo "host: $(hostname) $(uname -srm)"
  grep PRETTY_NAME /etc/os-release
  ldd --version | sed -n 1p
  local smi
  smi="$(nvidia-smi --query-gpu=name,driver_version,compute_cap --format=csv,noheader 2>&1)"
  echo "nvidia-smi: $smi"
  case "$smi" in
    "Tesla T4, 580."*", 7.5") record "pre-gpu" PASS "$smi" ;;
    *) record "pre-gpu" FAIL "$smi" ;;
  esac
  local n
  n="$(ldconfig -p | grep -c libcublas || true)"
  echo "ldconfig -p | grep -c libcublas: $n"
  [ "$n" = 0 ] && record "pre-ldconfig-cublas" PASS "count=0" || record "pre-ldconfig-cublas" FAIL "count=$n"
  local found
  found="$(sudo find / -xdev -name 'libcublas*' 2>/dev/null)"
  echo "find / -xdev -name 'libcublas*':"; echo "${found:-(none)}"
  if [ "$1" = before-remedy ]; then
    [ -z "$found" ] && record "pre-find-cublas" PASS "empty" || record "pre-find-cublas" FAIL "$(echo "$found" | paste -sd' ')"
  else
    # After the remedy the venv's wheels are expected; nothing else may be.
    local other
    other="$(echo "$found" | grep -v "^$VENV/" | grep -v '^$' || true)"
    [ -z "$other" ] && record "pre-find-cublas" PASS "only the venv's wheel files" \
      || record "pre-find-cublas" FAIL "$(echo "$other" | paste -sd' ')"
  fi
  local cuda_dirs
  cuda_dirs="$(ls -d /usr/local/cuda* 2>/dev/null || true)"
  echo "/usr/local/cuda*: ${cuda_dirs:-(none)}"
  [ -z "$cuda_dirs" ] && record "pre-no-usr-local-cuda" PASS "none" || record "pre-no-usr-local-cuda" FAIL "$cuda_dirs"
  echo "LD_LIBRARY_PATH at entry: ${LD_LIBRARY_PATH-(unset)}"
  [ -z "${LD_LIBRARY_PATH+x}" ] && record "pre-ld-library-path" PASS "unset" \
    || record "pre-ld-library-path" FAIL "set: $LD_LIBRARY_PATH"
  echo "libcuda files:"; ls -l /usr/lib/x86_64-linux-gnu/libcuda.so* 2>&1
  dpkg-divert --list 2>/dev/null | grep -i libcuda || echo "(no libcuda diversion)"
  dpkg-query -W -f='${Package}=${Version}\n' '*nvidia*' 2>/dev/null
}

# Runs "$@" in the background, samples its /proc/<pid>/maps for CUDA
# libraries while it runs, and writes the union to $OUT/<tag>.maps.
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

summary_line() { grep -E '^test result:' "$1" | tail -1; }

check_one_passed() {
  local check="$1" file="$2" rc="$3" line
  line="$(summary_line "$file")"
  if [ "$rc" -eq 0 ] && [[ "$line" == *"ok. 1 passed; 0 failed"* ]]; then
    record "$check" PASS "$line"
  else
    record "$check" FAIL "exit=$rc $line"
  fi
}

step_refusal() {
  preconditions before-remedy
  section "C3 refusal: ignored test"
  local f="$OUT/refusal-test.txt" rc=0
  env FATHOMDB_REQUIRE_LIVE=1 "$LIBMAJOR" --ignored --exact \
    missing_cuda_12_cublas_is_a_typed_refusal_with_the_remedy --nocapture --test-threads=1 >"$f" 2>&1 || rc=$?
  cat "$f"
  check_one_passed "refusal-ignored-test" "$f" "$rc"

  section "C3 refusal: doctor cuda-libraries --json"
  rc=0
  "$CLI" doctor cuda-libraries --json >"$OUT/refusal-doctor-libs.json" 2>"$OUT/refusal-doctor-libs.stderr" || rc=$?
  cat "$OUT/refusal-doctor-libs.json"; echo "--- stderr:"; cat "$OUT/refusal-doctor-libs.stderr"; echo "exit=$rc"
  python3 -I - "$OUT/refusal-doctor-libs.json" "$rc" <<'PY' && record "refusal-doctor-libs" PASS "exit=70 reason=missing library=cublas" || record "refusal-doctor-libs" FAIL "exit=$rc; see refusal.log"
import json, sys
d = json.load(open(sys.argv[1])); rc = int(sys.argv[2])
ref = d.get("refusal") or {}
text = json.dumps(d)
ok = (rc == 70 and ref.get("reason") == "missing" and ref.get("library") == "cublas"
      and "pip install nvidia-cublas-cu12 nvidia-curand-cu12" in text and "LD_LIBRARY_PATH" in text)
print("refusal:", json.dumps(ref, indent=1))
sys.exit(0 if ok else 1)
PY

  for verb in gpu reranker-gpu; do
    section "C3 floor: doctor $verb --json"
    rc=0
    "$CLI" doctor "$verb" --json >"$OUT/refusal-doctor-$verb.json" 2>"$OUT/refusal-doctor-$verb.stderr" || rc=$?
    cat "$OUT/refusal-doctor-$verb.json"; echo "--- stderr:"; cat "$OUT/refusal-doctor-$verb.stderr"; echo "exit=$rc"
    if grep -q cuda_incompatible "$OUT/refusal-doctor-$verb.json" "$OUT/refusal-doctor-$verb.stderr"; then
      record "floor-doctor-$verb" FAIL "cuda_incompatible reported (exit=$rc)"
    else
      record "floor-doctor-$verb" PASS "no cuda_incompatible (exit=$rc, recorded)"
    fi
  done

  section "C3 refusal: auto live embed on CPU"
  f="$OUT/refusal-auto-embed.txt"; rc=0
  run_with_maps refusal-auto-embed env FATHOMDB_EMBED_DEVICE=auto FATHOMDB_REQUIRE_LIVE=1 \
    "$CANDLE_BGE" --exact embed_returns_unit_norm_vector --nocapture --test-threads=1 >"$f" 2>&1 || rc=$?
  cat "$f"
  check_one_passed "refusal-auto-embed-cpu" "$f" "$rc"
}

remedy_env() {
  # LD_LIBRARY_PATH exactly as the product's remedy text, with the venv's python3.
  export PATH="$VENV/bin:$PATH"
  # shellcheck disable=SC2155
  export LD_LIBRARY_PATH="$(python3 -c 'import nvidia.cublas as m; print(m.__path__[0])')/lib:$(python3 -c 'import nvidia.curand as m; print(m.__path__[0])')/lib:${LD_LIBRARY_PATH:-}"
  echo "LD_LIBRARY_PATH=$LD_LIBRARY_PATH"
}

step_remedy() {
  preconditions before-remedy
  section "C3 remedy: venv"
  if [ ! -x "$VENV/bin/pip" ]; then
    if ! python3 -m venv "$VENV"; then
      echo "python3 -m venv failed (no ensurepip on this image); bootstrapping pip from the pushed wheel"
      rm -rf "$VENV"
      python3 -m venv --without-pip "$VENV"
      local pipwhl
      pipwhl="$(find "$C3"/wheels -name 'pip-*.whl' | sort | sed -n 1p)"
      "$VENV/bin/python3" "$pipwhl/pip" install --no-index "$pipwhl"
    fi
  fi
  local rc=0
  "$VENV/bin/pip" install nvidia-cublas-cu12==12.6.4.1 nvidia-curand-cu12==10.3.7.77 || rc=$?
  [ "$rc" -eq 0 ] && record "remedy-pip-install" PASS "nvidia-cublas-cu12==12.6.4.1 nvidia-curand-cu12==10.3.7.77" \
    || record "remedy-pip-install" FAIL "exit=$rc"
  "$VENV/bin/pip" freeze | tee "$OUT/remedy-pip-freeze.txt"
  remedy_env
  local cublas_dir curand_dir
  cublas_dir="$(python3 -c 'import nvidia.cublas as m; print(m.__path__[0])')/lib"
  curand_dir="$(python3 -c 'import nvidia.curand as m; print(m.__path__[0])')/lib"
  ls -l "$cublas_dir" "$curand_dir"

  section "C3 remedy: doctor cuda-libraries --json"
  rc=0
  "$CLI" doctor cuda-libraries --json >"$OUT/remedy-doctor-libs.json" 2>"$OUT/remedy-doctor-libs.stderr" || rc=$?
  cat "$OUT/remedy-doctor-libs.json"; echo "--- stderr:"; cat "$OUT/remedy-doctor-libs.stderr"; echo "exit=$rc"
  python3 -I - "$OUT/remedy-doctor-libs.json" "$rc" "$cublas_dir" "$curand_dir" <<'PY' && record "remedy-doctor-libs" PASS "exit=0 cublas+curand loaded from the venv" || record "remedy-doctor-libs" FAIL "exit=$rc; see remedy.log"
import json, os, sys
d = json.load(open(sys.argv[1])); rc = int(sys.argv[2])
cublas_dir, curand_dir = (os.path.realpath(p) for p in sys.argv[3:5])
libs = {x.get("library"): x for x in d.get("libraries", [])}
def under(entry, directory):
    p = (entry or {}).get("path") or ""
    return os.path.realpath(p).startswith(directory + "/")
cb, cr = libs.get("cublas"), libs.get("curand")
for name, e in (("cublas", cb), ("curand", cr)):
    print(name, json.dumps(e))
ok = (rc == 0 and not d.get("refusal")
      and cb and cb.get("status") == "loaded" and under(cb, cublas_dir)
      and cr and cr.get("status") == "loaded" and under(cr, curand_dir) and str(cr.get("version") or "").startswith("10"))
sys.exit(0 if ok else 1)
PY

  section "C3 remedy: forced live embed on cuda:0"
  local f="$OUT/remedy-embed.txt"; rc=0
  run_with_maps remedy-embed env FATHOMDB_EMBED_DEVICE=cuda:0 FATHOMDB_REQUIRE_LIVE=1 \
    "$CANDLE_BGE" --exact embed_returns_unit_norm_vector --nocapture --test-threads=1 >"$f" 2>&1 || rc=$?
  cat "$f"
  check_one_passed "remedy-forced-embed" "$f" "$rc"

  section "C3 remedy: forced reranker on cuda:0"
  f="$OUT/remedy-rerank.txt"; rc=0
  run_with_maps remedy-rerank env FATHOMDB_REQUIRE_LIVE=1 \
    "$EMBEDDER_LIB" --exact candle_reranker::gpu_tests::gpu_loads_and_scores_finite --nocapture --test-threads=1 >"$f" 2>&1 || rc=$?
  cat "$f"
  check_one_passed "remedy-forced-rerank" "$f" "$rc"
}

step_libcuda1() {
  preconditions after-remedy
  section "CUDA-09: divert the unversioned libcuda.so"
  ls -l /usr/lib/x86_64-linux-gnu/libcuda.so* 2>&1
  sudo dpkg-divert --rename --add /usr/lib/x86_64-linux-gnu/libcuda.so
  ls -l /usr/lib/x86_64-linux-gnu/libcuda.so* 2>&1
  dpkg-divert --list | grep -i libcuda
  if [ -e /usr/lib/x86_64-linux-gnu/libcuda.so ]; then
    record "libcuda1-divert" FAIL "libcuda.so still present"
  else
    record "libcuda1-divert" PASS "only libcuda.so.1 (and its target) remain"
  fi
  remedy_env
  section "CUDA-09: doctor cuda-libraries --json (the D-18 report names the driver file opened)"
  local drc=0
  "$CLI" doctor cuda-libraries --json >"$OUT/libcuda1-doctor-libs.json" 2>"$OUT/libcuda1-doctor-libs.stderr" || drc=$?
  cat "$OUT/libcuda1-doctor-libs.json"; echo "--- stderr:"; cat "$OUT/libcuda1-doctor-libs.stderr"; echo "exit=$drc"
  python3 -I - "$OUT/libcuda1-doctor-libs.json" "$drc" <<'PY' && record "libcuda1-doctor-driver" PASS "exit=0 driver opened_as libcuda.so.1" || record "libcuda1-doctor-driver" FAIL "exit=$drc; see libcuda1.log"
import json, sys
d = json.load(open(sys.argv[1]))
drv = next((x for x in d.get("libraries", []) if x.get("library") == "driver"), {})
print("driver:", json.dumps(drv))
sys.exit(0 if int(sys.argv[2]) == 0 and drv.get("opened_as") == "libcuda.so.1" and drv.get("status") == "loaded" else 1)
PY
  section "CUDA-09: forced live embed with only libcuda.so.1"
  local f="$OUT/libcuda1-embed.txt" rc=0
  run_with_maps libcuda1-embed env FATHOMDB_EMBED_DEVICE=cuda:0 FATHOMDB_REQUIRE_LIVE=1 \
    LD_DEBUG=files LD_DEBUG_OUTPUT="$OUT/libcuda1-lddebug" \
    "$CANDLE_BGE" --exact embed_returns_unit_norm_vector --nocapture --test-threads=1 >"$f" 2>&1 || rc=$?
  cat "$f"
  check_one_passed "libcuda1-forced-embed" "$f" "$rc"
  echo "--- LD_DEBUG=files: libcuda names opened"
  cat "$OUT"/libcuda1-lddebug.* 2>/dev/null | grep -E 'file=libcuda' | sed 's/^ *[0-9]*: *//' | sort | uniq -c
  local tried_unversioned tried_so1
  tried_unversioned="$(cat "$OUT"/libcuda1-lddebug.* 2>/dev/null | grep -c 'file=libcuda\.so \[' || true)"
  tried_so1="$(cat "$OUT"/libcuda1-lddebug.* 2>/dev/null | grep -c 'file=libcuda\.so\.1 \[' || true)"
  echo "ld.so lookups: libcuda.so=$tried_unversioned libcuda.so.1=$tried_so1"
  if grep -q 'libcuda.so.1\|libcuda.so.580' "$OUT/libcuda1-embed.maps" && ! grep -qE '/libcuda\.so$' "$OUT/libcuda1-embed.maps"; then
    record "libcuda1-maps" PASS "$(grep libcuda "$OUT/libcuda1-embed.maps" | paste -sd' ') (ld.so lookups libcuda.so=$tried_unversioned, libcuda.so.1=$tried_so1)"
  else
    record "libcuda1-maps" FAIL "$(grep libcuda "$OUT/libcuda1-embed.maps" | paste -sd' ')"
  fi
}

setup
section "S-C3 step $STEP on $(hostname)"
case "$STEP" in
  refusal) step_refusal ;;
  remedy) step_remedy ;;
  libcuda1) step_libcuda1 ;;
  *) echo "unknown step $STEP" >&2; exit 2 ;;
esac
section "end of step $STEP"
grep -P "^$STEP\t" "$OUT/summary.tsv" >&3
