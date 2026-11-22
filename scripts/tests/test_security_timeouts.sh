#!/usr/bin/env bash
# Regression tests for bounded strace/unshare timeouts in AC-036 and AC-037
# security scripts (P7-03). Uses sleeping shims + FATHOMDB_SECURITY_*_TIMEOUT_SECS=1
# so the entire suite completes in <5s.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECK_NO_LISTEN="$REPO_ROOT/scripts/security/check-no-listen.sh"
CHECK_NETNS="$REPO_ROOT/scripts/security/check-netns-deny-egress.sh"
CHECK_CATCH="$REPO_ROOT/scripts/security/check-netns-deny-egress-catch.sh"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

mkdir -p "$WORK/bin" "$WORK/target/debug/examples"
cat >"$WORK/target/debug/examples/security_cycle" <<'SHIM'
#!/usr/bin/env bash
exit 0
SHIM
chmod +x "$WORK/target/debug/examples/security_cycle"

# 1. check-no-listen.sh: hanging strace must be killed by timeout -> exit 1 (VIOLATION),
#    never misclassified as ptrace blocker (exit 2).
cat >"$WORK/bin/strace" <<'SHIM'
#!/usr/bin/env bash
sleep 10
SHIM
chmod +x "$WORK/bin/strace"

set +e
out="$(timeout 4s env \
  PATH="$WORK/bin:$PATH" \
  CARGO_TARGET_DIR="$WORK/target" \
  FATHOMDB_SECURITY_TIMEOUT_SECS=1 \
  bash "$CHECK_NO_LISTEN" 2>&1)"
rc=$?
set -e
if [ "$rc" -ne 1 ] || [[ "$out" != *'timed out'* ]] || [[ "$out" == *'ptrace-capable'* ]]; then
  printf 'FAIL check-no-listen timeout: rc=%s output=%s\n' "$rc" "$out" >&2
  exit 1
fi
printf 'PASS check-no-listen times out with VIOLATION exit 1\n'

# 2. check-netns-deny-egress.sh: hanging unshare/strace must be killed by timeout -> exit 1.
cat >"$WORK/bin/unshare" <<'SHIM'
#!/usr/bin/env bash
if [ "${1:-}" = "-rUn" ] && [ "${2:-}" = "true" ]; then
  exit 0
fi
sleep 10
SHIM
chmod +x "$WORK/bin/unshare"

set +e
out="$(timeout 4s env \
  PATH="$WORK/bin:$PATH" \
  CARGO_TARGET_DIR="$WORK/target" \
  FATHOMDB_SECURITY_TIMEOUT_SECS=1 \
  bash "$CHECK_NETNS" 2>&1)"
rc=$?
set -e
if [ "$rc" -ne 1 ] || [[ "$out" != *'timed out'* ]]; then
  printf 'FAIL check-netns-deny-egress timeout: rc=%s output=%s\n' "$rc" "$out" >&2
  exit 1
fi
printf 'PASS check-netns-deny-egress times out with VIOLATION exit 1\n'

# 3. check-netns-deny-egress-catch.sh: hanging live probe must fail loudly (exit 1),
#    never be masked by `|| true`.
set +e
out="$(timeout 4s env \
  PATH="$WORK/bin:$PATH" \
  FATHOMDB_SECURITY_CATCH_TIMEOUT_SECS=1 \
  bash "$CHECK_CATCH" 2>&1)"
rc=$?
set -e
if [ "$rc" -ne 1 ] || [[ "$out" != *'timed out'* ]]; then
  printf 'FAIL check-netns-deny-egress-catch timeout: rc=%s output=%s\n' "$rc" "$out" >&2
  exit 1
fi
printf 'PASS check-netns-deny-egress-catch times out loudly with VIOLATION exit 1\n'
