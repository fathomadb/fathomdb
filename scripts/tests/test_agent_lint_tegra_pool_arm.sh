#!/usr/bin/env bash
# agent-lint.sh must compile the Tegra private-pool driver part: the
# workspace clippy leg never enables `tegra-pool`, so without its own arm the
# driver code is linted by nobody. The arm runs on aarch64 Linux with a CUDA
# toolkit and prints a skip notice everywhere else.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LINT="$SCRIPT_DIR/../agent-lint.sh"
fail=0

require() {
  local what="$1" pattern="$2"
  if ! grep -Eq -- "$pattern" "$LINT"; then
    printf 'FAIL  agent-lint.sh lacks %s (pattern: %s)\n' "$what" "$pattern" >&2
    fail=1
  fi
}

require 'the tegra-pool clippy arm' \
  'cargo clippy -p fathomdb-embedder --features embed-cuda,rerank-cuda,tegra-pool --all-targets( --quiet)? -- -D warnings'
require 'an aarch64 guard' 'host_arch="\$\(uname -m\)"'
require 'an aarch64 comparison' '"\$host_arch" = aarch64'
require 'a Linux guard' 'host_os="\$\(uname -s\)"'
require 'a Linux comparison' '"\$host_os" = Linux'
require 'a CUDA toolkit guard' 'bin/nvcc'
require 'a printed skip notice' "printf 'skip lint-rust-tegra-pool"

# RUSTFLAGS must not leak into any other cargo invocation: only the arm's own
# env prefix may set it.
if grep -nE '^[[:space:]]*(export[[:space:]]+)?RUSTFLAGS=' "$LINT"; then
  printf 'FAIL  agent-lint.sh sets RUSTFLAGS for the whole script\n' >&2
  fail=1
fi

if [ "$fail" -ne 0 ]; then
  exit 1
fi
printf 'PASS  agent-lint.sh lints the tegra-pool driver part on aarch64 Linux\n'
