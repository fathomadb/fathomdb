#!/usr/bin/env bash
# AC-036: an executor that denies strace's PTRACE_TRACEME is a tooling
# blocker, not evidence that the security_cycle application listened.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CHECK="$REPO_ROOT/scripts/security/check-no-listen.sh"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

mkdir -p "$WORK/bin"
cat >"$WORK/bin/strace" <<'SHIM'
#!/usr/bin/env bash
echo 'strace: test_ptrace_get_syscall_info: PTRACE_TRACEME: Operation not permitted' >&2
exit 1
SHIM
chmod +x "$WORK/bin/strace"

set +e
output="$(PATH="$WORK/bin:$PATH" bash "$CHECK" 2>&1)"
status=$?
set -e

if [ "$status" -ne 2 ]; then
    printf 'FAIL AC-036 ptrace denial returned %s, want blocker exit 2: %s\n' "$status" "$output" >&2
    exit 1
fi
if [[ "$output" != *'PTRACE_TRACEME'* ]] || \
   [[ "$output" != *'ptrace-capable'* ]] || \
   [[ "$output" == *'security_cycle exited non-zero under strace'* ]]; then
    printf 'FAIL AC-036 ptrace denial diagnostic was unsafe or incomplete: %s\n' "$output" >&2
    exit 1
fi

printf 'PASS AC-036 classifies sandbox ptrace denial as a tooling blocker\n'

cat >"$WORK/bin/strace" <<'SHIM'
#!/usr/bin/env bash
echo 'strace: traced application exited unexpectedly' >&2
exit 1
SHIM
chmod +x "$WORK/bin/strace"

set +e
output="$(PATH="$WORK/bin:$PATH" bash "$CHECK" 2>&1)"
status=$?
set -e

if [ "$status" -ne 1 ] || [[ "$output" != *'security_cycle exited non-zero under strace'* ]]; then
    printf 'FAIL AC-036 traced-program failure was not retained as violation: %s\n' "$output" >&2
    exit 1
fi

printf 'PASS AC-036 retains a non-ptrace traced-program failure as a violation\n'

cat >"$WORK/bin/strace" <<'SHIM'
#!/usr/bin/env bash
echo 'security_cycle: PTRACE_TRACEME marker from application diagnostics' >&2
exit 1
SHIM
chmod +x "$WORK/bin/strace"

set +e
output="$(PATH="$WORK/bin:$PATH" bash "$CHECK" 2>&1)"
status=$?
set -e

if [ "$status" -ne 1 ] || [[ "$output" == *'ptrace-capable unconfined executor'* ]]; then
    printf 'FAIL AC-036 accepted a target diagnostic as a strace blocker: %s\n' "$output" >&2
    exit 1
fi

printf 'PASS AC-036 requires an actual strace ptrace-denial diagnostic\n'
