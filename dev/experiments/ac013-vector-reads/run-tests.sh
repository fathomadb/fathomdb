#!/usr/bin/env bash
# Run the fathomdb-engine vector, projection and slice40 test targets (plus
# the filtered-KNN and perf_gates fast tests) under one candidate's knobs, so
# a prototype is shown not to break them. Not timed; takes no lock.
#
#   dev/experiments/ac013-vector-reads/run-tests.sh <label> [KEY=VALUE ...]
#
# Example:
#   run-tests.sh CHUNKED FATHOMDB_PERF_EXPERIMENTS=1 FATHOMDB_PERF_VECTOR_RERANK=chunked
#
# Env: OUT (log directory, default /tmp/ac013-vreads-tests), TARGETS
# (space-separated test targets; default: every tests/*.rs whose name
# contains vector, vec0, projection or slice40, plus pr_g10_filtered_knn and
# perf_gates, minus the exp_ probes and targets that need features other
# than test-hooks), FEATURES (cargo features, default test-hooks; with
# "test-hooks operator" the operator-gated targets rebuild_vec0,
# rebuild_projections and erasure_projection_registry are kept). Prints one line per target and a final
# `RUN_TESTS label= pass= fail=` line; exits non-zero if any target fails.
set -uo pipefail

label="${1:?label}"
shift
OUT="${OUT:-/tmp/ac013-vreads-tests}"
FEATURES="${FEATURES:-test-hooks}"
mkdir -p "$OUT"
repo="$(cd "$(dirname "$0")/../../.." && pwd)"
crate="$repo/src/rust/crates/fathomdb-engine"

if [ -z "${TARGETS:-}" ]; then
  TARGETS=""
  for f in "$crate"/tests/*.rs; do
    name="$(basename "$f" .rs)"
    case "$name" in
      exp_*) continue ;;
      *vector*|*vec0*|*projection*|slice40_*|pr_g10_filtered_knn|perf_gates) ;;
      *) continue ;;
    esac
    # Skip targets whose required-features go beyond test-hooks.
    feats="$(awk -v n="$name" '
      $0 ~ "^name = \"" n "\"" {found=1; next}
      found && /^required-features/ {print; exit}
      found && /^\[\[/ {exit}' "$crate/Cargo.toml")"
    allowed='^required-features = \["test-hooks"\]$'
    case "$FEATURES" in
      *operator*) allowed='^required-features = \[("test-hooks"|"operator"|, )+\]$' ;;
    esac
    if [ -n "$feats" ] && echo "$feats" | grep -qvE "$allowed"; then
      echo "skip $name ($feats)"
      continue
    fi
    TARGETS="$TARGETS $name"
  done
fi

pass=0
fail=0
for t in $TARGETS; do
  log="$OUT/$label.$t.log"
  if env "$@" cargo test --release -p fathomdb-engine --features "$FEATURES" --test "$t" \
      >"$log" 2>&1; then
    pass=$((pass + 1))
    echo "ok   $t $(grep -E '^test result:' "$log" | tail -1)"
  else
    fail=$((fail + 1))
    echo "FAIL $t (log $log)"
  fi
done
echo "RUN_TESTS label=$label pass=$pass fail=$fail"
[ "$fail" -eq 0 ]
