#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TOOL_DIR="$REPO_ROOT/dev/tools/module-boundary-gate"
TOOL_MANIFEST="$TOOL_DIR/Cargo.toml"
# The wrapper builds into this fixed target dir whatever CARGO_TARGET_DIR is,
# so every mutant below runs the gate built from the current source.
GATE="$TOOL_DIR/target/debug/fathomdb-module-boundary-gate"

cargo test --quiet --locked --target-dir "$TOOL_DIR/target" --manifest-path "$TOOL_MANIFEST"
"$REPO_ROOT/scripts/check-module-boundaries.sh"
stale_gate_source="$(find "$TOOL_DIR/src" "$TOOL_MANIFEST" "$TOOL_DIR/Cargo.lock" -newer "$GATE" -print -quit)"
if [ -n "$stale_gate_source" ]; then
  printf 'module-boundary gate binary is older than %s\n' "$stale_gate_source" >&2
  exit 1
fi

# Carrier ownership invariants.
engine_src="$REPO_ROOT/src/rust/crates/fathomdb-engine/src"
if grep -nE '^impl .*GraphExpansionError(ReasonV1|V1)\b' "$engine_src/graph_expand/execution.rs"; then
  printf 'GraphExpansionError impls must be colocated with graph_expand::types\n' >&2
  exit 1
fi
for impl_header in 'impl GraphExpansionErrorReasonV1 {' 'impl GraphExpansionErrorV1 {' \
  'impl Display for GraphExpansionErrorV1 {' 'impl std::error::Error for GraphExpansionErrorV1 {}'; do
  if ! grep -Fqx "$impl_header" "$engine_src/graph_expand/types.rs"; then
    printf 'graph_expand/types.rs is missing %s\n' "$impl_header" >&2
    exit 1
  fi
done
if awk '/^pub\(crate\) struct GraphExpandRetentionCountersForTest \{/{inside=1; next}
  inside && /^\}/{inside=0}
  inside && /^[[:space:]]*pub/{found=1}
  END{exit !found}' "$engine_src/graph_expand/execution.rs"; then
  printf 'GraphExpandRetentionCountersForTest fields must stay private\n' >&2
  exit 1
fi

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
# Honours --target-dir, then CARGO_TARGET_DIR, like cargo; the built gate
# prints which build produced it.
set -euo pipefail
printf 'build\n' >>"${FAKE_CARGO_LOG:?}"
build_number="$(wc -l <"$FAKE_CARGO_LOG")"
manifest=""
target_dir=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --manifest-path) manifest="$2"; shift 2 ;;
    --target-dir) target_dir="$2"; shift 2 ;;
    *) shift ;;
  esac
done
tool_dir="$(dirname "${manifest:?}")"
target_dir="${target_dir:-${CARGO_TARGET_DIR:-$tool_dir/target}}"
# FAKE_CARGO_NOOP models cargo judging an existing binary fresh: no relink.
if [ -n "${FAKE_CARGO_NOOP:-}" ] && [ -x "$target_dir/debug/fathomdb-module-boundary-gate" ]; then
  exit 0
fi
mkdir -p "$target_dir/debug"
printf '#!/usr/bin/env bash\nprintf "fake-gate-build-%s\\n"\n' "$build_number" \
  >"$target_dir/debug/fathomdb-module-boundary-gate"
chmod +x "$target_dir/debug/fathomdb-module-boundary-gate"
FAKE_CARGO
chmod +x "$fake_cargo" "$cache_fixture/scripts/check-module-boundaries.sh"
export FAKE_CARGO_LOG="$fixture/cargo.log"
env -u CARGO_TARGET_DIR FATHOMDB_MODULE_BOUNDARY_CARGO="$fake_cargo" \
  "$cache_fixture/scripts/check-module-boundaries.sh" >/dev/null
env -u CARGO_TARGET_DIR FATHOMDB_MODULE_BOUNDARY_CARGO="$fake_cargo" \
  "$cache_fixture/scripts/check-module-boundaries.sh" >/dev/null
build_count="$(wc -l <"$FAKE_CARGO_LOG")"
if [ "$build_count" -ne 1 ]; then
  printf 'fresh module-boundary binary was rebuilt\n' >&2
  exit 1
fi
sleep 1
touch "$cache_fixture/dev/tools/module-boundary-gate/src/main.rs"
env -u CARGO_TARGET_DIR FATHOMDB_MODULE_BOUNDARY_CARGO="$fake_cargo" \
  "$cache_fixture/scripts/check-module-boundaries.sh" >/dev/null
build_count="$(wc -l <"$FAKE_CARGO_LOG")"
if [ "$build_count" -ne 2 ]; then
  printf 'stale module-boundary binary was not rebuilt\n' >&2
  exit 1
fi
# A CARGO_TARGET_DIR in the environment must not
# split the build from the exec; the rebuilt binary is the one that runs.
sleep 1
touch "$cache_fixture/dev/tools/module-boundary-gate/src/main.rs"
cache_output="$(CARGO_TARGET_DIR="$fixture/elsewhere-target" \
  FATHOMDB_MODULE_BOUNDARY_CARGO="$fake_cargo" "$cache_fixture/scripts/check-module-boundaries.sh")"
if [ "$cache_output" != 'fake-gate-build-3' ]; then
  printf 'module-boundary wrapper ran a stale binary under CARGO_TARGET_DIR: %s\n' "$cache_output" >&2
  exit 1
fi
# A manifest or lockfile edit that cargo does not
# relink for must not leave the binary looking stale; one no-op build settles
# it, so the next run neither rebuilds nor trips the stale-binary guard.
sleep 1
touch "$cache_fixture/dev/tools/module-boundary-gate/Cargo.toml"
for _ in 1 2; do
  env -u CARGO_TARGET_DIR FAKE_CARGO_NOOP=1 FATHOMDB_MODULE_BOUNDARY_CARGO="$fake_cargo" \
    "$cache_fixture/scripts/check-module-boundaries.sh" >/dev/null
done
build_count="$(wc -l <"$FAKE_CARGO_LOG")"
if [ "$build_count" -ne 4 ]; then
  printf 'manifest-only edit kept the module-boundary binary stale: %s builds\n' "$build_count" >&2
  exit 1
fi
newer_manifest="$(find "$cache_fixture/dev/tools/module-boundary-gate/Cargo.toml" \
  "$cache_fixture/dev/tools/module-boundary-gate/Cargo.lock" \
  -newer "$cache_fixture/dev/tools/module-boundary-gate/target/debug/fathomdb-module-boundary-gate")"
if [ -n "$newer_manifest" ]; then
  printf 'manifest-only edit left the module-boundary binary older than its manifest\n' >&2
  exit 1
fi

# Production mutation qualification is explicit, outside every routine tier.
printf 'ok    module-boundary-gate cheap fixtures\n'
