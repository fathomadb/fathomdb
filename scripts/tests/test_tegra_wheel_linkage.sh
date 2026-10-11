#!/usr/bin/env bash
# Extracted-wheel CUDA linkage must fail closed on findings and tool failures.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECKER="$SCRIPT_DIR/../release/check-tegra-wheel-linkage.sh"
TMPROOT="$(mktemp -d)"
trap 'rm -rf "$TMPROOT"' EXIT
mkdir -p "$TMPROOT/bin"
: > "$TMPROOT/extension.so"

cat > "$TMPROOT/bin/nm" <<'MOCK_NM'
#!/usr/bin/env bash
cat "$MOCK_NM_OUTPUT"
exit "${MOCK_NM_STATUS:-0}"
MOCK_NM
cat > "$TMPROOT/bin/readelf" <<'MOCK_READELF'
#!/usr/bin/env bash
cat "$MOCK_READELF_OUTPUT"
exit "${MOCK_READELF_STATUS:-0}"
MOCK_READELF
chmod +x "$TMPROOT/bin/nm" "$TMPROOT/bin/readelf"

expect_status() {
  local label="$1" expected="$2" actual
  set +e
  PATH="$TMPROOT/bin:$PATH" MOCK_NM_OUTPUT="$TMPROOT/nm.out" \
    MOCK_READELF_OUTPUT="$TMPROOT/readelf.out" \
    bash "$CHECKER" "$TMPROOT/extension.so" > "$TMPROOT/check.out" 2>&1
  actual=$?
  set -e
  if { [ "$expected" = pass ] && [ "$actual" -eq 0 ]; } || \
    { [ "$expected" = fail ] && [ "$actual" -ne 0 ]; }; then
    printf 'PASS  %s\n' "$label"
  else
    printf 'FAIL  %s: exit %s\n' "$label" "$actual" >&2
    cat "$TMPROOT/check.out" >&2
    exit 1
  fi
}

: > "$TMPROOT/nm.out"
printf ' 0x0000000000000001 (NEEDED) Shared library: [libc.so.6]\n' > "$TMPROOT/readelf.out"
expect_status 'ordinary system dependency passes' pass

printf '                 U cudaMalloc\n' > "$TMPROOT/nm.out"
expect_status 'unresolved ordinary CUDA Runtime symbol fails' fail

printf '                 U __cudaRegisterFatBinary\n' > "$TMPROOT/nm.out"
expect_status 'unresolved CUDA registration symbol fails' fail

: > "$TMPROOT/nm.out"
printf ' 0x0000000000000001 (NEEDED) Shared library: [libcublas.so.12]\n' > "$TMPROOT/readelf.out"
expect_status 'dynamic cuBLAS dependency fails' fail

printf ' 0x0000000000000001 (NEEDED) Shared library: [libnvidia-ml.so.1]\n' > "$TMPROOT/readelf.out"
expect_status 'dynamic NVIDIA dependency fails' fail

printf ' 0x0000000000000001 (NEEDED) Shared library: [libc.so.6]\n' > "$TMPROOT/readelf.out"
MOCK_NM_STATUS=42 expect_status 'nm inspection error fails closed' fail
MOCK_READELF_STATUS=43 expect_status 'readelf inspection error fails closed' fail
