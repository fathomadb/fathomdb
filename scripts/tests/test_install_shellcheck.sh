#!/usr/bin/env bash
# Regression tests for the pinned ShellCheck installer. These use only shims:
# no test downloads a release or relaxes the production SHA verification.
#
# 0.8.23 Slice 80.2 (AC80-4): the installer's `--print-target` mode is the
# single source of truth for each (os, arch)'s release slug and SHA-256 —
# this test reads it rather than duplicating the table, and drives BOTH
# linux.x86_64 and linux.aarch64 explicitly (via a shimmed `uname`) so both
# architectures are exercised on every host, not only whichever one this
# test happens to run on.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
INSTALLER="$REPO_ROOT/scripts/install-shellcheck.sh"
# shellcheck source=../lib/shellcheck-version.sh
. "$REPO_ROOT/scripts/lib/shellcheck-version.sh"

FIX="$(mktemp -d)"
trap 'rm -rf "$FIX"' EXIT

fail() { printf 'FAIL  %s\n' "$1" >&2; exit 1; }
pass() { printf 'PASS  %s\n' "$1"; }

make_shims() {
  local dir="$1"
  mkdir -p "$dir"
  cat >"$dir/uname" <<'UNAME'
#!/usr/bin/env bash
case "$1" in
  -s) printf '%s\n' "$SHELLCHECK_TEST_UNAME_OS" ;;
  -m) printf '%s\n' "$SHELLCHECK_TEST_UNAME_ARCH" ;;
  *) exit 2 ;;
esac
UNAME
  cat >"$dir/curl" <<'CURL'
#!/usr/bin/env bash
printf 'curl invoked\n' >>"$SHELLCHECK_TEST_MARKERS"
printf 'offline test transport\n' >&2
exit 28
CURL
  cat >"$dir/sha256sum" <<'SHA'
#!/usr/bin/env bash
if [ "$1" != '-c' ] || [ "$2" != '-' ]; then
  printf 'unexpected sha256sum invocation: %s\n' "$*" >&2
  exit 2
fi
input="$(cat)"
printf 'sha256sum invoked: %s\n' "$input" >>"$SHELLCHECK_TEST_MARKERS"
case "${SHELLCHECK_TEST_SHA_MODE:-pass}" in
  pass)
    case "$input" in
      *"$SHELLCHECK_TEST_SHA"*) ;;
      *) exit 1 ;;
    esac
    exit 0
    ;;
  fail) exit 1 ;;
  *) exit 2 ;;
esac
SHA
  cat >"$dir/tar" <<'TAR'
#!/usr/bin/env bash
dest=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    -C) shift; dest="$1" ;;
  esac
  shift
done
[ -n "$dest" ] || exit 2
mkdir -p "$dest/shellcheck-v$SHELLCHECK_VERSION"
cat >"$dest/shellcheck-v$SHELLCHECK_VERSION/shellcheck" <<BIN
#!/usr/bin/env bash
printf 'ShellCheck - shell script analysis tool\nversion: $SHELLCHECK_VERSION\n'
BIN
chmod +x "$dest/shellcheck-v$SHELLCHECK_VERSION/shellcheck"
printf 'tar invoked\n' >>"$SHELLCHECK_TEST_MARKERS"
TAR
  chmod +x "$dir/uname" "$dir/curl" "$dir/sha256sum" "$dir/tar"
}

print_target() {
  local os="$1" arch="$2"
  SHELLCHECK_TEST_UNAME_OS="$os" SHELLCHECK_TEST_UNAME_ARCH="$arch" \
    PATH="$FIX/bin:/usr/bin:/bin" bash "$INSTALLER" --print-target
}

run_installer() {
  local home="$1" cache="$2" markers="$3" mode="$4" sha="$5" os="$6" arch="$7"
  set +e
  OUT="$(HOME="$home" SHELLCHECK_CACHE_DIR="$cache" \
    SHELLCHECK_TEST_MARKERS="$markers" SHELLCHECK_TEST_SHA="$sha" \
    SHELLCHECK_TEST_SHA_MODE="$mode" SHELLCHECK_VERSION="$SHELLCHECK_VERSION" \
    SHELLCHECK_TEST_UNAME_OS="$os" SHELLCHECK_TEST_UNAME_ARCH="$arch" \
    PATH="$FIX/bin:/usr/bin:/bin" bash "$INSTALLER" 2>&1)"
  RC=$?
  set -e
}

make_shims "$FIX/bin"

run_target_arms() {
  local label="$1" os="$2" arch="$3"

  local target slug sha
  target="$(print_target "$os" "$arch")" || fail "$label: --print-target failed: $target"
  slug="${target%% *}"
  sha="${target#* }"
  [ -n "$slug" ] && [ -n "$sha" ] && [ "$slug" != "$sha" ] \
    || fail "$label: --print-target did not emit 'slug sha256': $target"

  # Arm A: a cache hit does not touch the network, but it STILL proves the
  # cache archive against the production SHA before extraction and uses the
  # pinned installed binary afterward.
  local home_a="$FIX/home-a-$label" cache_a="$FIX/cache-a-$label" markers_a="$FIX/markers-a-$label"
  local archive_a="$cache_a/v$SHELLCHECK_VERSION/$slug-$sha/shellcheck.tar.xz"
  mkdir -p "$(dirname "$archive_a")" "$home_a"
  printf 'cached fixture; the SHA shim asserts the production digest is supplied\n' >"$archive_a"
  run_installer "$home_a" "$cache_a" "$markers_a" pass "$sha" "$os" "$arch"
  printf -- '---- %s cache-hit output ----\n%s\nexit=%d\n' "$label" "$OUT" "$RC"
  [ "$RC" -eq 0 ] || fail "$label: cache hit did not install the pinned ShellCheck: $OUT"
  grep -Fq 'Using verified shellcheck archive cache' <<<"$OUT" \
    || fail "$label: cache hit was not reported"
  if [ -e "$markers_a" ] && grep -Fq 'curl invoked' "$markers_a"; then
    fail "$label: cache hit invoked curl instead of using the local archive"
  fi
  grep -Fq 'sha256sum invoked' "$markers_a" \
    || fail "$label: cache hit did not re-verify the archive SHA-256"
  [ -x "$home_a/.local/bin/shellcheck" ] || fail "$label: cache hit did not install shellcheck"
  local installed_version
  installed_version="$(HOME="$home_a" "$home_a/.local/bin/shellcheck" --version | sed -n 's/^version: //p')"
  [ "$installed_version" = "$SHELLCHECK_VERSION" ] \
    || fail "$label: cache hit installed $installed_version, not $SHELLCHECK_VERSION"
  pass "$label: cached archive avoids curl but is SHA-256-verified and installs the pin"

  # Arm B: an offline cache miss is a loud failure. It must not become a
  # skipped shell gate or a fake successful install.
  local home_b="$FIX/home-b-$label" cache_b="$FIX/cache-b-$label" markers_b="$FIX/markers-b-$label"
  mkdir -p "$home_b"
  run_installer "$home_b" "$cache_b" "$markers_b" pass "$sha" "$os" "$arch"
  printf -- '---- %s offline-miss output ----\n%s\nexit=%d\n' "$label" "$OUT" "$RC"
  [ "$RC" -ne 0 ] || fail "$label: offline cache miss exited 0"
  grep -Fq 'download failed' <<<"$OUT" || fail "$label: offline cache miss did not name its download failure"
  grep -Fq 'curl invoked' "$markers_b" || fail "$label: offline cache miss did not attempt the bounded download"
  [ ! -e "$home_b/.local/bin/shellcheck" ] || fail "$label: offline cache miss left an installed binary behind"
  pass "$label: offline cache miss fails loudly rather than weakening the shell gate"

  # Arm C: a corrupt cache is NOT silently redownloaded or accepted.
  # Rechecking the digest on every hit is what makes the action cache an
  # acceleration, not a trust boundary.
  local home_c="$FIX/home-c-$label" cache_c="$FIX/cache-c-$label" markers_c="$FIX/markers-c-$label"
  local archive_c="$cache_c/v$SHELLCHECK_VERSION/$slug-$sha/shellcheck.tar.xz"
  mkdir -p "$(dirname "$archive_c")" "$home_c"
  printf 'corrupt cache fixture\n' >"$archive_c"
  run_installer "$home_c" "$cache_c" "$markers_c" fail "$sha" "$os" "$arch"
  printf -- '---- %s corrupt-cache output ----\n%s\nexit=%d\n' "$label" "$OUT" "$RC"
  [ "$RC" -ne 0 ] || fail "$label: corrupt cache exited 0"
  grep -Fq 'SHA-256 check' <<<"$OUT" || fail "$label: corrupt cache did not name SHA-256 verification"
  if [ -e "$markers_c" ] && grep -Fq 'curl invoked' "$markers_c"; then
    fail "$label: corrupt cache redownloaded instead of failing its checksum loudly"
  fi
  grep -Fq 'sha256sum invoked' "$markers_c" \
    || fail "$label: corrupt cache did not attempt SHA-256 verification"
  pass "$label: corrupt cached archive fails its SHA-256 check without network fallback"
}

run_target_arms linux-x86_64 Linux x86_64
run_target_arms linux-aarch64 Linux aarch64

printf 'ShellCheck installer cache tests passed\n'
