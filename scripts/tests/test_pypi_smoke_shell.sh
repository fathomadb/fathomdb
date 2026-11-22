#!/usr/bin/env bash
# Shell control-flow regression; real registry consumers remain release proof.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/bin"
cat > "$fixture/bin/uname" <<'SH'
#!/usr/bin/env bash
printf 'Darwin\n'
SH
cat > "$fixture/bin/pip" <<'SH'
#!/usr/bin/env bash
if [ "$1" = install ] && [[ "$*" == *"fathomdb=="* ]]; then
  count=0
  if [ -f "$FIXTURE_PIP_COUNT" ]; then
    count="$(cat "$FIXTURE_PIP_COUNT")"
  fi
  count=$((count + 1))
  printf '%s' "$count" > "$FIXTURE_PIP_COUNT"
  if [ "${FIXTURE_CDN_SKEW_FAILS:-0}" -ge "$count" ]; then
    printf 'ERROR: Could not find a version that satisfies the requirement fathomdb==0.8.30\nERROR: No matching distribution found for fathomdb==0.8.30\n' >&2
    exit 1
  fi
fi
exit "${FIXTURE_INSTALL_EXIT:-0}"
SH
cat > "$fixture/bin/python3" <<'SH'
#!/usr/bin/env bash
set -eu
if [ "$1" = -m ]; then
  mkdir -p "$3/bin"
  printf ':\n' > "$3/bin/activate"
  if [ "${FIXTURE_FAIL_PHASE:-none}" = nounset ]; then
    printf 'unset SMOKE_FIXTURE_MISSING\n: "$SMOKE_FIXTURE_MISSING"\n' > "$3/bin/activate"
  fi
  cp "$FIXTURE_BIN/python3" "$3/bin/python"
  exit 0
fi
if [ "$1" = -I ]; then
  printf '%s argc=%s\n' "$5" "$#" >> "$FIXTURE_CALLS"
  if [ "$5" = "${FIXTURE_FAIL_PHASE:-none}" ]; then exit 43; fi
else
  printf 'trace\n' >> "$FIXTURE_CALLS"
  if [ "${FIXTURE_FAIL_PHASE:-none}" = trace ]; then exit 44; fi
fi
SH
cat > "$fixture/bin/rm" <<'SH'
#!/usr/bin/env bash
/bin/rm "$@"
exit "${FIXTURE_CLEANUP_EXIT:-0}"
SH
chmod +x "$fixture/bin/"*
failed=0
for phase in none write reopen trace install nounset write-cleanup cleanup cdn-skew-recover cdn-skew-exhaust; do
  : > "$fixture/calls"
  rm -f "$fixture/pip-count"
  status=0
  install_exit=0
  cleanup_exit=0
  cdn_skew_fails=0
  retry_attempts=3
  fail_phase="$phase"
  case "$phase" in
    write-cleanup) fail_phase="write"; cleanup_exit=45;;
    cleanup) fail_phase=none; cleanup_exit=45;;
    cdn-skew-recover) fail_phase=none; cdn_skew_fails=1;;
    cdn-skew-exhaust) fail_phase=none; cdn_skew_fails=5; retry_attempts=2;;
  esac
  if [ "$phase" = install ]; then install_exit=42; fi
  PATH="$fixture/bin:$PATH" FIXTURE_BIN="$fixture/bin" FIXTURE_CALLS="$fixture/calls" \
    FIXTURE_PIP_COUNT="$fixture/pip-count" FIXTURE_CDN_SKEW_FAILS="$cdn_skew_fails" \
    FATHOMDB_SMOKE_PYPI_INSTALL_ATTEMPTS="$retry_attempts" FATHOMDB_SMOKE_PYPI_RETRY_DELAY_SECS=0 \
    FIXTURE_FAIL_PHASE="$fail_phase" FIXTURE_INSTALL_EXIT="$install_exit" FIXTURE_CLEANUP_EXIT="$cleanup_exit" \
    bash "$ROOT/scripts/release/smoke/smoke-pypi-wheel.sh" 0.8.30 > "$fixture/log" 2>&1 || status=$?
  expected=0
  case "$phase" in write-cleanup) expected=43;; cleanup) expected=45;; nounset) expected=1;; install) expected=42;; write|reopen) expected=43;; trace) expected=44;; cdn-skew-exhaust) expected=1;; esac
  if [ "$status" -ne "$expected" ]; then
    printf 'FAIL phase=%s expected_exit=%s actual_exit=%s\n' "$phase" "$expected" "$status" >&2
    cat "$fixture/log" >&2
    failed=1
  fi
  expected_calls=''
  case "$phase" in
    none|trace|cleanup|cdn-skew-recover) expected_calls="$(printf 'write argc=5\nreopen argc=5\ntrace')";;
    write|write-cleanup) expected_calls='write argc=5';;
    reopen) expected_calls="$(printf 'write argc=5\nreopen argc=5')";;
  esac
  actual_calls="$(cat "$fixture/calls")"
  if [ "$actual_calls" != "$expected_calls" ]; then
    printf 'FAIL phase=%s: required consumer calls absent or extra arguments\n' "$phase" >&2
    cat "$fixture/calls" >&2
    failed=1
  fi
  pip_count="$(cat "$fixture/pip-count" 2>/dev/null || printf '0')"
  case "$phase" in
    install) [ "$pip_count" -eq 0 ] || [ "$pip_count" -eq 1 ] || { printf 'FAIL phase=install retried non-CDN failure: pip_count=%s\n' "$pip_count" >&2; failed=1; } ;;
    cdn-skew-recover) [ "$pip_count" -eq 2 ] || { printf 'FAIL phase=cdn-skew-recover expected 2 pip attempts, got %s\n' "$pip_count" >&2; failed=1; } ;;
    cdn-skew-exhaust) [ "$pip_count" -eq 2 ] || { printf 'FAIL phase=cdn-skew-exhaust expected 2 pip attempts, got %s\n' "$pip_count" >&2; failed=1; } ;;
  esac
  if [ "$status" -ne 0 ] && [ "$phase" != cleanup ] && grep -q 'smoke-pypi-wheel: ok' "$fixture/log"; then
    printf 'FAIL phase=%s: false success output\n' "$phase" >&2
    failed=1
  fi
done
[ "$failed" -eq 0 ]
printf 'PASS PyPI shell write/reopen/trace execution and original failure exits (Bash %s)\n' "$BASH_VERSION"
