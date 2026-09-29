#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TOOL_MANIFEST="$REPO_ROOT/dev/tools/module-boundary-gate/Cargo.toml"
GATE="$REPO_ROOT/dev/tools/module-boundary-gate/target/debug/fathomdb-module-boundary-gate"

cargo test --quiet --manifest-path "$TOOL_MANIFEST"
"$REPO_ROOT/scripts/check-module-boundaries.sh"

fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT

# The wrapper must build a missing binary, reuse a fresh one, and rebuild after
# any parser source changes. A fake Cargo keeps this contract test offline.
cache_fixture="$fixture/cache"
mkdir -p "$cache_fixture/scripts" "$cache_fixture/dev/tools/module-boundary-gate/src"
cp "$REPO_ROOT/scripts/check-module-boundaries.sh" "$cache_fixture/scripts/"
cp "$TOOL_MANIFEST" "$REPO_ROOT/dev/tools/module-boundary-gate/Cargo.lock" \
  "$cache_fixture/dev/tools/module-boundary-gate/"
printf 'fn main() {}\n' >"$cache_fixture/dev/tools/module-boundary-gate/src/main.rs"
fake_cargo="$fixture/fake-cargo"
cat >"$fake_cargo" <<'FAKE_CARGO'
#!/usr/bin/env bash
set -euo pipefail
printf 'build\n' >>"${FAKE_CARGO_LOG:?}"
tool_dir="$(dirname "${@: -1}")"
mkdir -p "$tool_dir/target/debug"
cat >"$tool_dir/target/debug/fathomdb-module-boundary-gate" <<'FAKE_GATE'
#!/usr/bin/env bash
exit 0
FAKE_GATE
chmod +x "$tool_dir/target/debug/fathomdb-module-boundary-gate"
FAKE_CARGO
chmod +x "$fake_cargo" "$cache_fixture/scripts/check-module-boundaries.sh"
export FAKE_CARGO_LOG="$fixture/cargo.log"
FATHOMDB_MODULE_BOUNDARY_CARGO="$fake_cargo" "$cache_fixture/scripts/check-module-boundaries.sh"
FATHOMDB_MODULE_BOUNDARY_CARGO="$fake_cargo" "$cache_fixture/scripts/check-module-boundaries.sh"
build_count="$(wc -l <"$FAKE_CARGO_LOG")"
if [ "$build_count" -ne 1 ]; then
  printf 'fresh module-boundary binary was rebuilt\n' >&2
  exit 1
fi
sleep 1
touch "$cache_fixture/dev/tools/module-boundary-gate/src/main.rs"
FATHOMDB_MODULE_BOUNDARY_CARGO="$fake_cargo" "$cache_fixture/scripts/check-module-boundaries.sh"
build_count="$(wc -l <"$FAKE_CARGO_LOG")"
if [ "$build_count" -ne 2 ]; then
  printf 'stale module-boundary binary was not rebuilt\n' >&2
  exit 1
fi

mkdir -p "$fixture/src/rust/crates/fathomdb-engine" "$fixture/dev/tools"
cp -R "$REPO_ROOT/src/rust/crates/fathomdb-engine/src" "$fixture/src/rust/crates/fathomdb-engine/"
cp "$REPO_ROOT/src/rust/crates/fathomdb-engine/Cargo.toml" "$fixture/src/rust/crates/fathomdb-engine/"
cp "$REPO_ROOT/dev/tools/module-boundary-policy.txt" "$fixture/dev/tools/"
cp "$fixture/dev/tools/module-boundary-policy.txt" "$fixture/module-boundary-policy.clean"

expect_failure() {
  local label="$1" expected="$2"
  shift 2
  local output
  if output="$("$@" 2>&1)"; then
    printf 'expected %s mutant to fail\n' "$label" >&2
    exit 1
  fi
  if ! grep -Fq "$expected" <<<"$output"; then
    printf '%s mutant missed diagnostic %s\n%s\n' "$label" "$expected" "$output" >&2
    exit 1
  fi
}

expect_success() {
  local label="$1"
  shift
  local output
  if ! output="$("$@" 2>&1)"; then
    printf 'expected %s positive fixture to pass\n%s\n' "$label" "$output" >&2
    exit 1
  fi
}

cp "$fixture/src/rust/crates/fathomdb-engine/src/search.rs" "$fixture/search.rs.clean"
printf '\nuse crate::*;\n' >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure glob 'governed internal glob import search.rs' "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\nuse crate::reader_pool::ReaderRequest;\n' >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure reverse-edge 'forbidden dependency search -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

cp "$fixture/src/rust/crates/fathomdb-engine/src/read.rs" "$fixture/read.rs.clean"
printf '\nuse crate::reader_pool::ReaderRequest;\n' >>"$fixture/src/rust/crates/fathomdb-engine/src/read.rs"
expect_failure read-pool-cycle 'forbidden dependency read -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/read.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/read.rs"

cp "$fixture/src/rust/crates/fathomdb-engine/src/graph_expand/mod.rs" "$fixture/graph-expand.rs.clean"
printf '\nuse crate::reader_pool::ReaderRequest;\n' >>"$fixture/src/rust/crates/fathomdb-engine/src/graph_expand/mod.rs"
expect_failure graph-pool-cycle 'forbidden dependency graph_expand -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/graph-expand.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/graph_expand/mod.rs"

printf '\nuse crate::search_api as forbidden_search_api;\n' >>"$fixture/src/rust/crates/fathomdb-engine/src/graph_expand/mod.rs"
expect_failure graph-search-api-cycle 'forbidden dependency graph_expand -> search_api' "$GATE" --root "$fixture"
cp "$fixture/graph-expand.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/graph_expand/mod.rs"

printf '\nuse crate::search::SearchReaderWork;\n' >>"$fixture/src/rust/crates/fathomdb-engine/src/graph_expand/mod.rs"
expect_failure graph-search-cycle 'forbidden dependency graph_expand -> search' "$GATE" --root "$fixture"
cp "$fixture/graph-expand.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/graph_expand/mod.rs"

cp "$fixture/src/rust/crates/fathomdb-engine/src/fusion.rs" "$fixture/fusion.rs.clean"
printf '\npub(crate) fn slice85_reported_reverse() { let _ = crate::reader_pool::ReaderRequest::shutdown(); }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/fusion.rs"
expect_failure reported-return 'unexpected boundary edge source=fusion' "$GATE" --root "$fixture"
cp "$fixture/fusion.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/fusion.rs"

cp "$fixture/src/rust/crates/fathomdb-engine/src/reader_pool.rs" "$fixture/reader-pool.rs.clean"
printf '\npub(crate) fn slice85_glob_target() {}\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/reader_pool.rs"
printf '\nuse crate::reader_pool::*;\nfn slice85_glob_return() { slice85_glob_target(); }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/fusion.rs"
expect_failure outside-glob 'target_item=slice85_glob_target' "$GATE" --root "$fixture"
cp "$fixture/fusion.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/fusion.rs"
cp "$fixture/reader-pool.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/reader_pool.rs"

cp "$fixture/src/rust/crates/fathomdb-engine/src/lib.rs" "$fixture/lib.rs.clean"
printf '\nfn slice85_root_return() { let _ = reader_pool::ReaderRequest::shutdown(); }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/lib.rs"
printf '\nfn slice85_calls_root() { crate::slice85_root_return(); }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure root-return 'unexpected boundary edge source=root' "$GATE" --root "$fixture"
cp "$fixture/lib.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/lib.rs"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\nfn slice85_distinct_item() { let _ = crate::fusion::fuse_three_arms; }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure item-identity 'target_item=fuse_three_arms' "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\n#[cfg(test)] use crate::reader_pool::ReaderRequest as Slice85TestOnlyRequest;\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure test-configuration 'kind=import configurations=test at search.rs:' \
  "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\n#[cfg(not(target_os = "linux"))] use crate::reader_pool::ReaderRequest as Slice85NonLinuxRequest;\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure nonlinux-configuration 'nonlinux' "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\nfn validate_filter_attributes_on_snapshot() {}\nfn slice85_shadow_call() { validate_filter_attributes_on_snapshot(); }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure shadow 'local function shadows governed callable' "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\nfn slice85_hidden_macro() { crate::slice85_dependency!(); }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure macro 'unreviewed governed macro' "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\nmod slice85_unclassified_inline { fn helper() {} }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure inline-classification 'module classification missing search::slice85_unclassified_inline' \
  "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

cp "$fixture/src/rust/crates/fathomdb-engine/src/read_api.rs" "$fixture/read-api.rs.clean"
printf '\nfn forbidden_variant() { let _ = crate::reader_pool::ReaderRequest::Shutdown; }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/read_api.rs"
expect_failure request-variant 'direct ReaderRequest variant construction outside reader_pool' \
  "$GATE" --root "$fixture"
cp "$fixture/read-api.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/read_api.rs"

cp "$fixture/src/rust/crates/fathomdb-engine/src/errors.rs" "$fixture/errors.rs.clean"
printf '\nfn slice85_hidden_exec() { let _ = encode_graph_expand_result_v1; }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/errors.rs"
expect_failure root-glob-callable 'unexpected boundary edge source=errors' \
  "$GATE" --root "$fixture"
cp "$fixture/errors.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/errors.rs"

printf '\nfn slice85_scope_escape() { { let encode_graph_expand_result_v1 = 1usize; let _ = encode_graph_expand_result_v1; } let _ = encode_graph_expand_result_v1; }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/errors.rs"
expect_failure lexical-shadow-expiry 'unexpected boundary edge source=errors' \
  "$GATE" --root "$fixture"
cp "$fixture/errors.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/errors.rs"

printf '\nfn slice85_hidden_type(_: crate::reader_pool::ReaderRequest) {}\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure type-dependency 'forbidden dependency search -> reader_pool' \
  "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\nmacro_rules! slice85_hidden_boundary { () => {{ let _ = crate::reader_pool::ReaderRequest::Shutdown; }}; }\nfn slice85_macro_call() { slice85_hidden_boundary!(); }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure local-macro 'unreviewed local macro definition source=search' "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\nmacro_rules! slice85_admitted_hidden_boundary { () => {{ let _ = crate::graph_expand::encode_graph_expand_result_v1; }}; }\nfn slice85_admitted_macro_call() { slice85_admitted_hidden_boundary!(); }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/errors.rs"
expect_failure admitted-local-macro 'unreviewed local macro definition source=errors' \
  "$GATE" --root "$fixture"
cp "$fixture/errors.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/errors.rs"

cp "$fixture/src/rust/crates/fathomdb-engine/src/identity.rs" "$fixture/identity.rs.clean"
sed -i '/^                let value = value.into();$/a\                let _ = crate::reader_pool::ReaderRequest::Shutdown;' \
  "$fixture/src/rust/crates/fathomdb-engine/src/identity.rs"
expect_failure allowlisted-macro-body 'local macro definition fingerprint mismatch source=identity' \
  "$GATE" --root "$fixture"
cp "$fixture/identity.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/identity.rs"

printf 'local-macro errors slice85_missing deadbeef\n' \
  >>"$fixture/dev/tools/module-boundary-policy.txt"
expect_failure stale-local-macro 'stale local macro policy source=errors macro=slice85_missing' \
  "$GATE" --root "$fixture"
cp "$fixture/module-boundary-policy.clean" "$fixture/dev/tools/module-boundary-policy.txt"

printf '\n#[cfg(feature = "slice85-unknown")] use crate::reader_pool::ReaderRequest;\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure unknown-feature 'unsupported cfg predicate' "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\npub use search_types::GraphFrontierStats as Slice85LeakedStats;\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/lib.rs"
expect_failure root-contract 'unexpected boundary edge source=root' "$GATE" --root "$fixture"
cp "$fixture/lib.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/lib.rs"

cp "$fixture/src/rust/crates/fathomdb-engine/src/telemetry.rs" "$fixture/telemetry.rs.clean"
all_configs='all'
printf '\npub(crate) fn slice85_cycle_out() { crate::telemetry::slice85_cycle_in(); }\nfn slice85_cycle_return() { slice85_cycle_out(); }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
printf '\npub(crate) fn slice85_cycle_in() { crate::search::slice85_cycle_return(); }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/telemetry.rs"
printf 'edge search slice85_cycle_out telemetry slice85_cycle_in callable %s\n' "$all_configs" \
  >>"$fixture/dev/tools/module-boundary-policy.txt"
printf 'edge telemetry slice85_cycle_in search slice85_cycle_return callable %s\n' "$all_configs" \
  >>"$fixture/dev/tools/module-boundary-policy.txt"
expect_failure local-helper-cycle 'unapproved governed cycle search <-> telemetry' \
  "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
cp "$fixture/telemetry.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/telemetry.rs"
cp "$fixture/module-boundary-policy.clean" "$fixture/dev/tools/module-boundary-policy.txt"

# Design review cycle 1, D-1: dependencies written inside std macro bodies are
# extracted with the same rules as ordinary expressions (P19/P20/P21).
cp "$fixture/src/rust/crates/fathomdb-engine/src/graph_expand/traversal.rs" "$fixture/traversal.rs.clean"
printf '\nfn slice85_macro_control() { let _ = crate::reader_pool::ReaderRequest::shutdown(); }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/read.rs"
expect_failure macro-control 'forbidden dependency read -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/read.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/read.rs"

printf '\nfn slice85_macro_hidden() { let _ = vec![crate::reader_pool::ReaderRequest::shutdown()]; }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/read.rs"
expect_failure macro-vec 'forbidden dependency read -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/read.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/read.rs"

printf '\nfn slice85_macro_repeat() { let _ = vec![crate::reader_pool::slice85_probe(); 2]; }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/read.rs"
expect_failure macro-vec-repeat 'forbidden dependency read -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/read.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/read.rs"

printf '\nfn slice85_macro_params() { let _ = rusqlite::params![crate::reader_pool::slice85_probe()]; }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/read.rs"
expect_failure macro-params 'forbidden dependency read -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/read.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/read.rs"

printf '\nfn slice85_macro_matches() { assert!(matches!(crate::search_api::slice85_probe(), 1)); }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/graph_expand/mod.rs"
expect_failure macro-assert-matches 'forbidden dependency graph_expand -> search_api' "$GATE" --root "$fixture"
cp "$fixture/graph-expand.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/graph_expand/mod.rs"

printf '\nfn slice85_macro_matches_capital() { assert!(matches!(crate::search_api::S85(), 1)); }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/graph_expand/mod.rs"
expect_failure macro-assert-matches-capital 'forbidden dependency graph_expand -> search_api' "$GATE" --root "$fixture"
cp "$fixture/graph-expand.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/graph_expand/mod.rs"

printf '\nfn slice85_macro_pattern(value: usize) -> bool { matches!(value, x if x == crate::search::slice85_probe()) }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/graph_expand/mod.rs"
expect_failure macro-matches-guard 'forbidden dependency graph_expand -> search' "$GATE" --root "$fixture"
cp "$fixture/graph-expand.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/graph_expand/mod.rs"

printf '\nfn slice85_macro_format() -> String { format!("{}", crate::reader_pool::slice85_probe()) }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure macro-format 'forbidden dependency search -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\nfn slice85_macro_write(f: &mut String) { let _ = write!(f, "{}", crate::search::slice85_probe()); }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/graph_expand/traversal.rs"
expect_failure macro-write 'forbidden dependency graph_expand::traversal -> search' "$GATE" --root "$fixture"
cp "$fixture/traversal.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/graph_expand/traversal.rs"

printf '\nfn slice85_macro_block() { slice85_block_macro! { let _ = crate::reader_pool::slice85_probe(); } }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure macro-statements 'forbidden dependency search -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\nfn slice85_macro_opaque() { slice85_opaque!(=> crate::reader_pool::slice85_probe); }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure macro-unparsed 'unparsed macro body source=search item=slice85_macro_opaque macro=slice85_opaque at search.rs:' \
  "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

# Design review cycle 1, D-2: module-qualified paths whose last segment is
# capitalised (constants, statics, variants, tuple/unit constructors) are edges
# (P1/P15/P16).
printf '\nfn slice85_const_probe() -> usize { crate::reader_pool::S85_PROBE_CONST }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure capital-const 'forbidden dependency search -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\nfn slice85_variant_probe() { let _ = crate::reader_pool::S85Enum::A; }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure capital-variant 'forbidden dependency search -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\nfn slice85_constructor_probe() { let _ = crate::reader_pool::S85Carrier(1); }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/read.rs"
expect_failure capital-constructor 'forbidden dependency read -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/read.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/read.rs"

printf '\nfn slice85_unit_probe() { let _ = crate::reader_pool::S85Unit; }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure capital-unit 'forbidden dependency search -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\nfn slice85_struct_literal_probe() { let _ = crate::reader_pool::S85Struct { value: 1 }; }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure capital-struct-literal 'forbidden dependency search -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\nfn slice85_pattern_probe(value: usize) { if let crate::reader_pool::S85Enum::B(_) = value {} }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/read.rs"
expect_failure capital-tuple-pattern 'forbidden dependency read -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/read.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/read.rs"

printf '\nfn slice85_struct_pattern_probe(value: usize) { let crate::reader_pool::S85Struct { .. } = value; }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/read.rs"
expect_failure capital-struct-pattern 'forbidden dependency read -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/read.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/read.rs"

printf '\nfn slice85_let_type_probe() { let _probe: Option<crate::reader_pool::ReaderRequest> = None; }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/read.rs"
expect_failure let-type-annotation 'forbidden dependency read -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/read.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/read.rs"

# Design review cycle 1, D-3: a non-root re-export resolves to its defining
# item, so laundering a forbidden owner through a reported module fails (P12).
cp "$fixture/src/rust/crates/fathomdb-engine/src/rerank.rs" "$fixture/rerank.rs.clean"
printf '\npub(crate) use crate::reader_pool::ReaderRequest as S85Laundered;\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/fusion.rs"
printf '\nuse crate::fusion::S85Laundered;\nfn slice85_launder(_: &S85Laundered) {}\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure reexport-laundering 'forbidden dependency search -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\npub(crate) use crate::fusion::S85Laundered as S85LaunderedTwice;\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/rerank.rs"
printf '\nfn slice85_launder_twice(_: &crate::rerank::S85LaunderedTwice) {}\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure reexport-laundering-chain 'forbidden dependency search -> reader_pool' "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
cp "$fixture/fusion.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/fusion.rs"
cp "$fixture/rerank.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/rerank.rs"

# Design review cycle 1, D-13: composition (re-export) and contract (type)
# edges are distinct kinds.
printf '\npub(crate) use crate::fusion::fuse_rrf as slice85_reexported;\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure reexport-kind 'target_item=fuse_rrf syntax=crate::fusion::fuse_rrf kind=reexport ' \
  "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\nfn slice85_type_only(_: crate::fusion::Slice85Probe) {}\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure type-kind 'target_item=Slice85Probe syntax=crate::fusion::Slice85Probe kind=type ' \
  "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

# Design review cycle 1, D-7: the frozen edge inventory covers edges with a
# governed or root endpoint; reported-to-reported edges are extracted for
# reachability but need no policy line (P13), and such a line is rejected.
printf '\nfn slice85_reported_only() { let _ = crate::rerank::rerank_passages; }\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/fusion.rs"
expect_success reported-to-reported "$GATE" --root "$fixture"
cp "$fixture/fusion.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/fusion.rs"

printf 'edge fusion slice85_reported_only rerank rerank_passages callable all\n' \
  >>"$fixture/dev/tools/module-boundary-policy.txt"
expect_failure out-of-scope-edge-line 'policy edge outside the frozen scope source=fusion' \
  "$GATE" --root "$fixture"
cp "$fixture/module-boundary-policy.clean" "$fixture/dev/tools/module-boundary-policy.txt"

# Design review cycle 1, D-6: configurations come from the engine manifest's
# feature table plus operator and debug_assertions axes and an all-features
# closure; nothing a shipped build compiles is invisible (P3/P4).
cp "$fixture/src/rust/crates/fathomdb-engine/Cargo.toml" "$fixture/Cargo.toml.clean"
printf '\n#[cfg(feature = "operator")] use crate::reader_pool::ReaderRequest as Slice85OperatorRequest;\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure operator-configuration 'forbidden dependency search -> reader_pool configuration=operator-linux' \
  "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\n#[cfg(not(debug_assertions))] use crate::reader_pool::ReaderRequest as Slice85ReleaseRequest;\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure release-configuration 'forbidden dependency search -> reader_pool configuration=default-linux-release' \
  "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf '\n#[cfg(feature = "default-reranker")] use crate::reader_pool::ReaderRequest as Slice85RerankRequest;\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure all-features-configuration 'forbidden dependency search -> reader_pool configuration=all-features-linux' \
  "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

sed -i 's/^\[features\]$/[features]\nslice85-manifest-only = []/' \
  "$fixture/src/rust/crates/fathomdb-engine/Cargo.toml"
printf '\n#[cfg(feature = "slice85-manifest-only")] use crate::reader_pool::ReaderRequest as Slice85ManifestRequest;\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure manifest-derived-feature 'forbidden dependency search -> reader_pool configuration=all-features-linux' \
  "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
cp "$fixture/Cargo.toml.clean" "$fixture/src/rust/crates/fathomdb-engine/Cargo.toml"

sed -i '/^default-reranker = /d' "$fixture/src/rust/crates/fathomdb-engine/Cargo.toml"
expect_failure manifest-feature-removed 'unsupported cfg predicate' "$GATE" --root "$fixture"
cp "$fixture/Cargo.toml.clean" "$fixture/src/rust/crates/fathomdb-engine/Cargo.toml"

sed -i '/^operator = \[\]$/d' "$fixture/src/rust/crates/fathomdb-engine/Cargo.toml"
expect_failure manifest-axis-removed 'configuration feature operator is not declared' "$GATE" --root "$fixture"
cp "$fixture/Cargo.toml.clean" "$fixture/src/rust/crates/fathomdb-engine/Cargo.toml"

printf '\n#[cfg(all(test, not(test)))] use crate::fusion::fuse_rrf as slice85_never_compiled;\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/search.rs"
expect_failure empty-configuration 'edge has no evaluated configuration source=search' \
  "$GATE" --root "$fixture"
cp "$fixture/search.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/search.rs"

printf 'configuration-feature slice85-undeclared undeclared\n' >>"$fixture/dev/tools/module-boundary-policy.txt"
expect_failure undeclared-axis 'configuration feature slice85-undeclared is not declared' \
  "$GATE" --root "$fixture"
cp "$fixture/module-boundary-policy.clean" "$fixture/dev/tools/module-boundary-policy.txt"

# Design review cycle 1, D-10: mutually exclusive cfg twins of an Engine
# method may live in different owners (P14); overlapping ones still fail.
cp "$fixture/src/rust/crates/fathomdb-engine/src/read_api.rs" "$fixture/read-api.rs.clean"
cp "$fixture/src/rust/crates/fathomdb-engine/src/graph_api.rs" "$fixture/graph-api.rs.clean"
printf '\n#[cfg(target_os = "linux")]\nimpl Engine {\n    pub(crate) fn slice85_platform(&self) {}\n}\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/read_api.rs"
printf '\n#[cfg(not(target_os = "linux"))]\nimpl Engine {\n    pub(crate) fn slice85_platform(&self) {}\n}\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/graph_api.rs"
expect_success exclusive-engine-method-twins "$GATE" --root "$fixture"
cp "$fixture/graph-api.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/graph_api.rs"

printf '\n#[cfg(feature = "test-hooks")]\nimpl Engine {\n    pub(crate) fn slice85_platform(&self) {}\n}\n' \
  >>"$fixture/src/rust/crates/fathomdb-engine/src/graph_api.rs"
expect_failure overlapping-engine-method-twins \
  'Engine method slice85_platform has multiple defining modules in configuration hooks-linux' \
  "$GATE" --root "$fixture"
cp "$fixture/graph-api.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/graph_api.rs"
cp "$fixture/read-api.rs.clean" "$fixture/src/rust/crates/fathomdb-engine/src/read_api.rs"

printf 'fn stray() {}\n' >"$fixture/src/rust/crates/fathomdb-engine/src/stray.rs"
expect_failure undeclared-module 'Rust source file is not declared from lib.rs stray' \
  "$GATE" --root "$fixture"
rm "$fixture/src/rust/crates/fathomdb-engine/src/stray.rs"

sed -i '/^governed telemetry$/d' "$fixture/dev/tools/module-boundary-policy.txt"
expect_failure classification 'module classification missing telemetry' "$GATE" --root "$fixture"

printf 'ok    module-boundary-gate fixtures\n'
