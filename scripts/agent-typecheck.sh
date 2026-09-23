#!/usr/bin/env bash
# Type-check all language surfaces.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib/agent-output.sh
. "$SCRIPT_DIR/lib/agent-output.sh"
cd_repo_root

# Rust: cargo check is the type-only gate (clippy already does this in lint, but check is cheaper).
run_capped typecheck-rust cargo check --workspace --quiet

# `cargo check --workspace` above only checks lib+bins, not test targets, so
# it cannot catch a debug-only `Engine::*_for_test` hook drifting out of sync
# with its caller's cfg. Pin both engine test-build surfaces that broke
# independently of each other in 0.8.27 slice 40: the release-profile test
# build (debug_assertions off) and the default-feature debug test build.
#
# The default-feature debug build has zero warnings at baseline (every
# `Engine::*_for_test` hook's sole caller is compiled in under
# `debug_assertions`), so `-D warnings` is safe here with no allowlist.
run_capped typecheck-rust-engine-default-tests \
  env RUSTFLAGS=-Dwarnings cargo check -p fathomdb-engine --all-targets

# The release-profile build cannot use a blanket `-D warnings`: two
# `dead_code` warnings on this branch predate 0.8.27 slice 40 and are
# themselves an artifact of the same pattern this gate exists to catch
# (`ReaderWorkerPool::worker_count`/`live_count` in src/lib.rs and
# `SubscriberRegistry::dispatch_stress_failure` in src/lifecycle.rs are each
# reachable only from a `#[cfg(debug_assertions)]`-gated `Engine::*_for_test`
# caller, so they go dead specifically under `--release`). Tracked, not
# fixed, here — fixing them is a one-line `#[cfg(debug_assertions)]` per
# item, but it's out of this gate's scope. Allow exactly those two; fail on
# any other compiler warning.
typecheck_rust_engine_release_tests() {
  local verb="typecheck-rust-engine-release-tests"
  local spill="/tmp/fathomdb-agent-${verb}-$$.log"
  if ! cargo check --release -p fathomdb-engine --lib --tests >"$spill" 2>&1; then
    printf 'FAIL %s (compile error)\n' "$verb"
    printf -- '----\n'
    head -n 200 "$spill"
    printf -- '----\n'
    printf 'full log: %s\n' "$spill"
    return 1
  fi
  local unexpected
  # shellcheck disable=SC2016  # backticks below are literal cargo diagnostic
  # text to match, not command substitution.
  unexpected="$(grep -E '^warning: ' "$spill" \
    | grep -v 'generated [0-9]* warning' \
    | grep -v 'methods `worker_count` and `live_count` are never used' \
    | grep -v 'method `dispatch_stress_failure` is never used' \
    || true)"
  if [ -n "$unexpected" ]; then
    printf 'FAIL %s: new compiler warning(s) in the release-profile engine test build (only the 2 pre-existing dead_code warnings are allowed):\n' "$verb"
    printf '%s\n' "$unexpected"
    printf -- '----\n'
    printf 'full log: %s\n' "$spill"
    return 1
  fi
  rm -f "$spill"
  return 0
}
typecheck_rust_engine_release_tests

# Python preflight: use the project's exact pinned version, so local prework
# cannot report a false green from version drift or an absent type checker.
readonly PYRIGHT_VERSION="1.1.410"
pyright_bin=""
if [ -x .venv/bin/pyright ]; then
  pyright_bin=".venv/bin/pyright"
elif command -v pyright >/dev/null 2>&1; then
  pyright_bin="$(command -v pyright)"
fi

if [ -z "$pyright_bin" ]; then
  printf 'FAIL typecheck-python: Pyright %s is required but not installed. Run scripts/bootstrap.sh in a clean non-worktree checkout.\n' "$PYRIGHT_VERSION" >&2
  exit 1
fi

if ! pyright_version_output="$("$pyright_bin" --version 2>&1)"; then
  printf 'FAIL typecheck-python: could not read the installed Pyright version. Run scripts/bootstrap.sh in a clean non-worktree checkout.\n' >&2
  exit 1
fi

# Pyright's canonical version line comes first; later lines may be update
# notices, but must not be allowed to mask an incompatible first line.
pyright_version_line="${pyright_version_output%%$'\n'*}"

if [ "$pyright_version_line" != "pyright $PYRIGHT_VERSION" ]; then
  printf 'FAIL typecheck-python: Pyright %s is required; selected %s. Run scripts/bootstrap.sh in a clean non-worktree checkout.\n' "$PYRIGHT_VERSION" "$pyright_version_output" >&2
  exit 1
fi

run_capped typecheck-python "$pyright_bin" -p src/python

# TypeScript: tsc --noEmit if installed
if [ -d src/ts/node_modules ]; then
  run_capped typecheck-ts bash -c 'cd src/ts && npm run --silent typecheck'
else
  skip_notice typecheck-ts "src/ts/node_modules not installed"
fi
