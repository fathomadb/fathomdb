#!/usr/bin/env bash
# scripts/release/smoke/smoke-gh-release-mcp.sh — Verify a sealed GitHub Release
# fathomdb-mcp binary archive (.tar.gz or .zip + .sha256) and run the stdio
# MCP smoke against the extracted binary.
set -euo pipefail

usage() {
  printf 'usage: %s --archive <archive> --sha256 <sha256-file>\n' "$0" >&2
}

ARCHIVE=""
SHA_FILE=""
while [ "$#" -gt 0 ]; do
  case "$1" in
    --archive) ARCHIVE="${2:-}"; shift 2 ;;
    --sha256)  SHA_FILE="${2:-}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'smoke-gh-release-mcp: unknown argument %s\n' "$1" >&2; usage; exit 2 ;;
  esac
done

if [ -z "$ARCHIVE" ] || [ -z "$SHA_FILE" ]; then
  usage
  exit 2
fi
if [ ! -f "$ARCHIVE" ] || [ ! -f "$SHA_FILE" ]; then
  printf 'smoke-gh-release-mcp: archive or sha256 file missing\n' >&2
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
WORK="$(mktemp -d)"
smoke_complete=0
cleanup() {
  local status=$? cleanup_status=0
  trap - EXIT
  rm -rf "$WORK" || cleanup_status=$?
  if [ "$status" -eq 0 ]; then
    if [ "$smoke_complete" -ne 1 ]; then status=1; else status=$cleanup_status; fi
  fi
  exit "$status"
}
trap cleanup EXIT

python3 - "$ARCHIVE" "$SHA_FILE" "$WORK" <<'PY'
import hashlib
from pathlib import Path
import sys
import tarfile
from zipfile import ZipFile

archive = Path(sys.argv[1]).resolve()
sha_file = Path(sys.argv[2]).resolve()
extract_dir = Path(sys.argv[3]).resolve()

line = sha_file.read_text(encoding="utf-8").strip().splitlines()
if len(line) != 1:
    raise SystemExit(f"expected 1 line in {sha_file}, got {len(line)}")
parts = line[0].split()
if len(parts) != 2 or parts[1] != archive.name:
    raise SystemExit(f"unexpected sha256 line format: {line[0]!r}")
actual = hashlib.sha256(archive.read_bytes()).hexdigest()
if actual != parts[0].lower():
    raise SystemExit(f"sha256 mismatch: expected {parts[0]}, got {actual}")

if archive.name.endswith(".tar.gz"):
    with tarfile.open(archive, "r:gz") as tf:
        tf.extractall(extract_dir)
elif archive.name.endswith(".zip"):
    with ZipFile(archive, "r") as zf:
        zf.extractall(extract_dir)
else:
    raise SystemExit(f"unsupported archive format: {archive.name}")
PY

MCP_BIN="$(find "$WORK" -type f \( -name 'fathomdb-mcp' -o -name 'fathomdb-mcp.exe' \) -print -quit)"
if [ -z "$MCP_BIN" ]; then
  printf 'smoke-gh-release-mcp: extracted archive contains no fathomdb-mcp binary\n' >&2
  exit 1
fi
chmod +x "$MCP_BIN" 2>/dev/null || true

host_os="$(uname -s)"
if [ "$host_os" = "Linux" ]; then
  # shellcheck source=/dev/null
  . "$REPO_ROOT/scripts/release/glibc-floor-contract.sh"
  bash "$REPO_ROOT/scripts/check-glibc-floor.sh" --floor "$GLIBC_FLOOR" "$MCP_BIN"
fi

python3 "$REPO_ROOT/scripts/release/smoke/mcp-stdio-smoke.py" --binary "$MCP_BIN"

smoke_complete=1
printf 'smoke-gh-release-mcp: ok — %s verified and passed stdio smoke\n' "$(basename "$ARCHIVE")"
