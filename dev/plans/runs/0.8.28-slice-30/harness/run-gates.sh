#!/usr/bin/env bash
# shellcheck disable=SC2312,SC2030,SC2031 # measurement harness: command substitutions only format diagnostics; settings are exported inside per-series subshells on purpose
# 0.8.28 Slice 30 qualification driver: the exact series behind each gate of
# plan section 4.2. One gate per call; every series holds the GPU lock.
#   run-gates.sh <g1|g2|g3|g4|g5|g6|g7|g8|g9> [n-override]
# Env (required): SCRATCH (work dir), NODE_PROD (consumer dir of the product
# addon), PY_W1 and PY_W2 (interpreters of the venvs holding the product
# Tegra wheel and the same wheel plus rerank-cuda). Optional: NODE_STUDY
# (consumer dir of the study P build, G9), QUAL_OUT (default $SCRATCH/series),
# FDB_CLI (the CLI built with tegra-pool, G1).
set -u
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
: "${SCRATCH:?}" "${NODE_PROD:?}" "${PY_W1:?}" "${PY_W2:?}"
out=${QUAL_OUT:-$SCRATCH/series}
lock=${GPU_LOCK:-/tmp/fathomdb-gpu.lock}
gate=$1
nover=${2:-}
soak_s=${SOAK_SECONDS:-1200}
mkdir -p "$out" "$SCRATCH/dbs"
export FATHOMDB_DB_SCRATCH="$SCRATCH/dbs"
export QUAL_QUIET_STRICT=${QUAL_QUIET_STRICT:-0}
export PATH=/usr/local/cuda/bin:$PATH

# Environment of one series: forced CUDA device policy, no pool settings.
base_env() {
  export FATHOMDB_EMBED_DEVICE=cuda:0 FATHOMDB_RERANK_DEVICE=cuda:0
  unset FATHOMDB_POOL_MODE FATHOMDB_POOL_MAXSIZE FATHOMDB_POOL_RELEASE_THRESHOLD FATHOMDB_CUDA_EARLY_INIT
  unset FATHOMDB_GPU_ALLOCATION_WITNESS FATHOMDB_POOL_VARIANT HEAP_OBJECTS HEAP_GROW_AFTER_OPEN IMPORT_ORDER
  unset OVERSIZE_BATCH CYCLES IDLE_AFTER_CLOSE_S EXPECT_PATH EXPECT_DEVICE CONSUMER_MODE QUAL_LABEL SOAK_SECONDS
}
use_node() {
  export CONSUMER_LANG=node FATHOMDB_MODULE="${1:-$NODE_PROD}/node_modules/fathomdb/dist/index.js"
  export ARTIFACT="${1:-$NODE_PROD}/node_modules/fathomdb-linux-arm64-gnu/fathomdb.linux-arm64-gnu.node"
}
use_py() {
  export CONSUMER_LANG=py PYTHON="$1"
  ARTIFACT=$(find "$(dirname "$1")/../lib" -name '_fathomdb*.so' | sed -n 1p)
  export ARTIFACT
}
# ser <label> <count> <dir> [node flags]: one series under the GPU lock.
ser() {
  local label=$1 count=$2 dir=$3
  shift 3
  QUAL_LABEL=$label flock "$lock" bash "$here/run-series.sh" "$label" "$count" "$out/$dir" "$@"
}
n() { echo "${nover:-$1}"; }

case $gate in
  g1)
    # Decision: auto pool mode, auto device policy; every process must be private.
    (base_env; unset FATHOMDB_EMBED_DEVICE FATHOMDB_RERANK_DEVICE; use_node
      export CONSUMER_MODE=full EXPECT_DEVICE=cuda EXPECT_PATH=private; ser node-auto "$(n 20)" g1/node)
    (base_env; unset FATHOMDB_EMBED_DEVICE FATHOMDB_RERANK_DEVICE; use_py "$PY_W1"
      export CONSUMER_MODE=full EXPECT_DEVICE=cuda EXPECT_PATH=private; ser py-auto "$(n 20)" g1/py)
    ;;
  g2)
    for state in earlyoff:FATHOMDB_CUDA_EARLY_INIT=off modeoff:FATHOMDB_POOL_MODE=off invalid:FATHOMDB_POOL_MODE=bogus; do
      name=${state%%:*}
      kv=${state#*:}
      (base_env; use_node; export "${kv?}" CONSUMER_MODE=full; ser "node-$name" "$(n 5)" "g2/node-$name")
      (base_env; use_py "$PY_W1"; export "${kv?}" CONSUMER_MODE=full; ser "py-$name" "$(n 5)" "g2/py-$name")
    done
    # Late import at a 4M-object heap, device policy auto: product and S (pool off).
    (base_env; use_node; unset FATHOMDB_EMBED_DEVICE FATHOMDB_RERANK_DEVICE
      export IMPORT_ORDER=late HEAP_OBJECTS=4000000 CONSUMER_MODE=full EXPECT_DEVICE=any; ser node-late4m "$(n 5)" g2/node-late4m)
    (base_env; use_node; unset FATHOMDB_EMBED_DEVICE FATHOMDB_RERANK_DEVICE
      export IMPORT_ORDER=late HEAP_OBJECTS=4000000 CONSUMER_MODE=full EXPECT_DEVICE=any FATHOMDB_POOL_MODE=off; ser node-late4m-S "$(n 5)" g2/node-late4m-S)
    (base_env; use_py "$PY_W1"; export CONSUMER_MODE=fork; ser py-fork-hook "$(n 5)" g2/py-fork-hook)
    (base_env; use_py "$PY_W1"; export CONSUMER_MODE=fork FATHOMDB_CUDA_EARLY_INIT=off; ser py-fork-nohook "$(n 5)" g2/py-fork-nohook)
    ;;
  g3)
    (base_env; use_node; export CONSUMER_MODE=cycles CYCLES=100 IDLE_AFTER_CLOSE_S=10; ser node-cycles "$(n 10)" g3/node)
    (base_env; use_py "$PY_W1"; export CONSUMER_MODE=cycles CYCLES=50 IDLE_AFTER_CLOSE_S=10; ser py-cycles "$(n 10)" g3/py)
    ;;
  g4)
    (base_env; use_node; export CONSUMER_MODE=full OVERSIZE_BATCH=128 FATHOMDB_POOL_MAXSIZE=3221225472 EXPECT_PATH=private
      ser node-oversize "$(n 5)" g4/node)
    (base_env; use_py "$PY_W1"; export CONSUMER_MODE=oversize OVERSIZE_BATCH=128 FATHOMDB_POOL_MAXSIZE=3221225472 EXPECT_PATH=private
      ser py-oversize "$(n 5)" g4/py)
    ;;
  g5)
    # Equivalence references: the same artifacts with the pool off. The P side
    # is the G1 series (embedding hash, cls hash) and the G9 Python W2 series
    # (rerank scores).
    (base_env; unset FATHOMDB_EMBED_DEVICE FATHOMDB_RERANK_DEVICE; use_node
      export CONSUMER_MODE=full FATHOMDB_POOL_MODE=off; ser node-S "$(n 20)" g5/node-S)
    (base_env; unset FATHOMDB_EMBED_DEVICE FATHOMDB_RERANK_DEVICE; use_py "$PY_W1"
      export CONSUMER_MODE=full FATHOMDB_POOL_MODE=off; ser py-S "$(n 20)" g5/py-S)
    ;;
  g6)
    for heap in 4000000 8000000; do
      (base_env; use_node; export IMPORT_ORDER=early HEAP_OBJECTS=$heap HEAP_GROW_AFTER_OPEN=1000000 CONSUMER_MODE=full EXPECT_PATH=private
        ser "node-heap$heap" "$(n 20)" "g6/node-heap$heap")
    done
    ;;
  g7)
    for k in 2 4 8; do
      langs=()
      for _ in $(seq 1 "$k"); do langs+=(node); done
      (base_env; use_node; export CONSUMER_MODE=full EXPECT_PATH=private
        QUAL_LABEL=g7-k$k flock "$lock" bash "$here/concurrent-runner.sh" "$(n 5)" "$out/g7/k$k" "${langs[@]}")
    done
    ;;
  g8)
    # Soak: one Node and one Python process together, no-swap condition.
    (base_env; use_node; export PYTHON="$PY_W1" CONSUMER_MODE=soak SOAK_SECONDS="$soak_s" QUAL_RUN_TIMEOUT_S=$((soak_s + 600)) NO_SWAP=1 EXPECT_PATH=private
      QUAL_LABEL=g8 flock "$lock" bash "$here/concurrent-runner.sh" 1 "$out/g8" node py)
    ;;
  g9)
    # Performance: randomised interleaved blocks, no-swap condition, strict
    # quiet check. Node 25: product P, study P, S. Python (rerank-cuda wheel):
    # P, S (pool off and no early cuInit). Python import: P, S, and "I" (pool
    # off, early cuInit on) whose difference to S is the measured cuInit cost.
    export QUAL_QUIET_STRICT=${G9_STRICT:-1} NO_SWAP=1
    node_art="$NODE_PROD/node_modules/fathomdb-linux-arm64-gnu/fathomdb.linux-arm64-gnu.node"
    py_art=$(find "$(dirname "$PY_W2")/../lib" -name '_fathomdb*.so' | sed -n 1p)
    {
      echo "prodP node 25.9.0 $NODE_PROD $node_art CONSUMER_MODE=perf"
      if [ -n "${NODE_STUDY:-}" ]; then
        echo "studyP node 25.9.0 $NODE_STUDY $NODE_STUDY/node_modules/fathomdb-linux-arm64-gnu/fathomdb.linux-arm64-gnu.node CONSUMER_MODE=perf,FATHOMDB_POOL_VARIANT=P-first-use,FATHOMDB_POOL_RELEASE_THRESHOLD=0"
      fi
      echo "S node 25.9.0 $NODE_PROD $node_art CONSUMER_MODE=perf,FATHOMDB_POOL_MODE=off"
    } >"$SCRATCH/g9-node.tsv"
    {
      echo "pyP py 25.9.0 $PY_W2 $py_art CONSUMER_MODE=perf"
      echo "pyS py 25.9.0 $PY_W2 $py_art CONSUMER_MODE=perf,FATHOMDB_POOL_MODE=off,FATHOMDB_CUDA_EARLY_INIT=off"
    } >"$SCRATCH/g9-py.tsv"
    {
      echo "pyimpP py 25.9.0 $PY_W2 $py_art CONSUMER_MODE=import"
      echo "pyimpS py 25.9.0 $PY_W2 $py_art CONSUMER_MODE=import,FATHOMDB_POOL_MODE=off,FATHOMDB_CUDA_EARLY_INIT=off"
      echo "pyimpI py 25.9.0 $PY_W2 $py_art CONSUMER_MODE=import,FATHOMDB_POOL_MODE=off"
    } >"$SCRATCH/g9-pyimp.tsv"
    (base_env; flock "$lock" bash "$here/interleave.sh" "$SCRATCH/g9-node.tsv" "$(n 20)" 20261081 "$out/g9/node")
    (base_env; flock "$lock" bash "$here/interleave.sh" "$SCRATCH/g9-py.tsv" "$(n 20)" 20261082 "$out/g9/py")
    (base_env; flock "$lock" bash "$here/interleave.sh" "$SCRATCH/g9-pyimp.tsv" "$(n 20)" 20261083 "$out/g9/pyimp")
    ;;
  *)
    echo "unknown gate $gate" >&2
    exit 2
    ;;
esac
