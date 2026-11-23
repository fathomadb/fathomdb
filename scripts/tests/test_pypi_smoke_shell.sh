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
if [ "$1" = install ] && [[ "$*" == *"fathomdb[cli]=="* ]]; then
  count=0
  if [ -f "$FIXTURE_PIP_COUNT" ]; then
    count="$(cat "$FIXTURE_PIP_COUNT")"
  fi
  count=$((count + 1))
  printf '%s' "$count" > "$FIXTURE_PIP_COUNT"
  if [ "${FIXTURE_CDN_SKEW_FAILS:-0}" -ge "$count" ]; then
    skew_pkg="${FIXTURE_CDN_SKEW_PKG:-fathomdb}"
    printf 'ERROR: Could not find a version that satisfies the requirement %s==0.8.30\nERROR: No matching distribution found for %s==0.8.30\n' "$skew_pkg" "$skew_pkg" >&2
    exit 1
  fi
fi
exit "${FIXTURE_INSTALL_EXIT:-0}"
SH
cat > "$fixture/bin/fathomdb-stub" <<'SH'
#!/usr/bin/env bash
set -eu
if [ "$1" = "--version" ]; then
  printf 'cli-version\n' >> "$FIXTURE_CALLS"
  printf 'fathomdb 0.8.30\n'
  if [ "${FIXTURE_FAIL_PHASE:-none}" = cli-version ]; then exit 46; fi
elif [ "$1" = "doctor" ] && [ "$2" = "platform" ] && [ "$3" = "--json" ]; then
  printf 'cli-platform\n' >> "$FIXTURE_CALLS"
  printf '{"schema_version":"fathomdb.doctor.platform.v1","platform_class":"non_aarch64"}\n'
  if [ "${FIXTURE_FAIL_PHASE:-none}" = cli-platform ]; then exit 47; fi
else
  printf 'unexpected fathomdb args: %s\n' "$*" >&2
  exit 99
fi
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
  cp "$FIXTURE_BIN/fathomdb-stub" "$3/bin/fathomdb"
  exit 0
fi
if [ "$1" = -c ]; then
  printf 'coinstall\n' >> "$FIXTURE_CALLS"
  printf '0.8.30\n'
  if [ "${FIXTURE_FAIL_PHASE:-none}" = coinstall ]; then exit 48; fi
elif [ "$1" = -I ]; then
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
for phase in none coinstall cli-version cli-platform write reopen trace install nounset write-cleanup cleanup cdn-skew-recover cdn-skew-cli-recover cdn-skew-exhaust; do
  : > "$fixture/calls"
  rm -f "$fixture/pip-count"
  status=0
  install_exit=0
  cleanup_exit=0
  cdn_skew_fails=0
  cdn_skew_pkg="fathomdb"
  retry_attempts=3
  fail_phase="$phase"
  case "$phase" in
    write-cleanup) fail_phase="write"; cleanup_exit=45;;
    cleanup) fail_phase=none; cleanup_exit=45;;
    cdn-skew-recover) fail_phase=none; cdn_skew_fails=1; cdn_skew_pkg="fathomdb";;
    cdn-skew-cli-recover) fail_phase=none; cdn_skew_fails=1; cdn_skew_pkg="fathomdb-cli";;
    cdn-skew-exhaust) fail_phase=none; cdn_skew_fails=5; retry_attempts=2;;
  esac
  if [ "$phase" = install ]; then install_exit=42; fi
  PATH="$fixture/bin:$PATH" FIXTURE_BIN="$fixture/bin" FIXTURE_CALLS="$fixture/calls" \
    FIXTURE_PIP_COUNT="$fixture/pip-count" FIXTURE_CDN_SKEW_FAILS="$cdn_skew_fails" \
    FIXTURE_CDN_SKEW_PKG="$cdn_skew_pkg" \
    FATHOMDB_SMOKE_PYPI_INSTALL_ATTEMPTS="$retry_attempts" FATHOMDB_SMOKE_PYPI_RETRY_DELAY_SECS=0 \
    FIXTURE_FAIL_PHASE="$fail_phase" FIXTURE_INSTALL_EXIT="$install_exit" FIXTURE_CLEANUP_EXIT="$cleanup_exit" \
    bash "$ROOT/scripts/release/smoke/smoke-pypi-wheel.sh" 0.8.30 > "$fixture/log" 2>&1 || status=$?
  expected=0
  case "$phase" in
    write-cleanup) expected=43;;
    cleanup) expected=45;;
    nounset) expected=1;;
    install) expected=42;;
    coinstall) expected=48;;
    cli-version) expected=46;;
    cli-platform) expected=47;;
    write|reopen) expected=43;;
    trace) expected=44;;
    cdn-skew-exhaust) expected=1;;
  esac
  if [ "$status" -ne "$expected" ]; then
    printf 'FAIL phase=%s expected_exit=%s actual_exit=%s\n' "$phase" "$expected" "$status" >&2
    cat "$fixture/log" >&2
    failed=1
  fi
  expected_calls=''
  case "$phase" in
    none|trace|cleanup|cdn-skew-recover|cdn-skew-cli-recover)
      expected_calls="$(printf 'coinstall\ncli-version\ncli-platform\nwrite argc=5\nreopen argc=5\ntrace')"
      ;;
    coinstall) expected_calls='coinstall';;
    cli-version) expected_calls="$(printf 'coinstall\ncli-version')";;
    cli-platform) expected_calls="$(printf 'coinstall\ncli-version\ncli-platform')";;
    write|write-cleanup) expected_calls="$(printf 'coinstall\ncli-version\ncli-platform\nwrite argc=5')";;
    reopen) expected_calls="$(printf 'coinstall\ncli-version\ncli-platform\nwrite argc=5\nreopen argc=5')";;
  esac
  actual_calls="$(cat "$fixture/calls")"
  if [ "$actual_calls" != "$expected_calls" ]; then
    printf 'FAIL phase=%s: required consumer calls absent or extra arguments\nexpected:\n%s\nactual:\n%s\n' "$phase" "$expected_calls" "$actual_calls" >&2
    failed=1
  fi
  pip_count="$(cat "$fixture/pip-count" 2>/dev/null || printf '0')"
  case "$phase" in
    install) [ "$pip_count" -eq 0 ] || [ "$pip_count" -eq 1 ] || { printf 'FAIL phase=install retried non-CDN failure: pip_count=%s\n' "$pip_count" >&2; failed=1; } ;;
    cdn-skew-recover|cdn-skew-cli-recover) [ "$pip_count" -eq 2 ] || { printf 'FAIL phase=%s expected 2 pip attempts, got %s\n' "$phase" "$pip_count" >&2; failed=1; } ;;
    cdn-skew-exhaust) [ "$pip_count" -eq 2 ] || { printf 'FAIL phase=cdn-skew-exhaust expected 2 pip attempts, got %s\n' "$pip_count" >&2; failed=1; } ;;
  esac
  if [ "$status" -ne 0 ] && [ "$phase" != cleanup ] && grep -q 'smoke-pypi-wheel: ok' "$fixture/log"; then
    printf 'FAIL phase=%s: false success output\n' "$phase" >&2
    failed=1
  fi
done
[ "$failed" -eq 0 ]
printf 'PASS PyPI shell coinstall/cli/write/reopen/trace execution and original failure exits (Bash %s)\n' "$BASH_VERSION"
