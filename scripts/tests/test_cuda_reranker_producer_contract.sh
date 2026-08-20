#!/usr/bin/env bash
# Slice 71: the candidate v3 route must produce real reranker evidence, not
# feature metadata that a fixture can imitate.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PREFLIGHT="$ROOT/scripts/release/cuda-preflight.sh"
SMOKE="$ROOT/scripts/release/cuda-package-rehearsal-smoke.sh"

need() {
  local file="$1" text="$2"
  grep -Fq -- "$text" "$file" || {
    printf 'FAIL  %s must contain %s\n' "${file#"$ROOT"/}" "$text" >&2
    exit 1
  }
}

need "$PREFLIGHT" 'FATHOMDB_CUDA_PREFLIGHT_RERANKER_CACHE'
need "$PREFLIGHT" 'DEFAULT_RERANKER_CACHE_ROOT='
need "$PREFLIGHT" 'dst=/fathomdb-reranker-cache-root,readonly'
need "$PREFLIGHT" 'FATHOMDB_RERANKER_CACHE=/fathomdb-reranker-cache-root'
need "$PREFLIGHT" 'reranker-cache-manifest.json'
need "$PREFLIGHT" 'reranker-python-cpu-smoke.json'
need "$PREFLIGHT" 'reranker-napi-cpu-smoke.json'
need "$PREFLIGHT" 'forced-reranker-python.json'
need "$PREFLIGHT" 'forced-reranker-napi.json'
need "$PREFLIGHT" 'FATHOMDB_RERANK_DEVICE'
need "$PREFLIGHT" 'from fathomdb import rerank'
need "$PREFLIGHT" 'await engine.drain(30_000);'
need "$SMOKE" 'reranker-cli-doctor.json'
need "$SMOKE" 'doctor reranker-gpu --json'
need "$SMOKE" 'FATHOMDB_RERANK_DEVICE'

printf 'CUDA reranker producer contract passed\n'
