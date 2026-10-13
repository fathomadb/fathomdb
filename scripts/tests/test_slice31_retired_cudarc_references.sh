#!/usr/bin/env bash
# 0.8.28 Slice 31 (AC31-10, AC31-06; design § 3.3): the vendored cudarc tree
# and the private-pool marker feature are retired. Outside the historical
# records (dev/plans, dev/adr, CHANGELOG.md and the two ledgers) no tracked
# file names them, and the embedder's tegra-pool feature enables nothing.
# Each pattern is built from pieces so this file never matches itself.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

failed=0
pass() { printf 'PASS  %s\n' "$1"; }
fail() { printf 'FAIL  %s\n' "$1" >&2; failed=1; }

HISTORY=(':!dev/plans' ':!dev/adr' ':!CHANGELOG.md'
  ':!dev/todos-and-considerations-ledger.jsonl' ':!dev/steward/steward-ledger.jsonl')

# no_vendored_cudarc_reference_outside_history (AC31-10's git grep).
vendored_tree="third_party/""cudarc"
vendored_test="test_vendored""_cudarc"
vendored_text="vendored"" cudarc"
vendored_version="cudarc"" 0.19.7"
if matches="$(git grep -il -e "$vendored_tree" -e "$vendored_test" -e "$vendored_text" \
  -e "$vendored_version" -- . "${HISTORY[@]}")"; then
  fail "no_vendored_cudarc_reference_outside_history: $(printf '%s' "$matches" | wc -l | tr -d ' ') file(s) still name the vendored cudarc:
$matches"
else
  pass 'no_vendored_cudarc_reference_outside_history'
fi

# no_private_pool_marker_reference_outside_history (design § 3.3).
marker="fathomdb-private"
marker="$marker-pool"
if matches="$(git grep -n -e "$marker" -- . "${HISTORY[@]}")"; then
  fail "no_private_pool_marker_reference_outside_history: the marker feature is still named:
$matches"
else
  pass 'no_private_pool_marker_reference_outside_history'
fi

# the_tegra_pool_feature_is_empty (cmd c-tegra-pool-feature).
if out="$(python3 -I -c 'import tomllib; f=tomllib.load(open("src/rust/crates/fathomdb-embedder/Cargo.toml","rb"))["features"]; assert f["tegra-pool"] == [], f["tegra-pool"]' 2>&1)"; then
  pass 'the_tegra_pool_feature_is_empty'
else
  fail "the_tegra_pool_feature_is_empty: $out"
fi

exit "$failed"
