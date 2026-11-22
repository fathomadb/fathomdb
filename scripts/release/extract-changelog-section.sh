#!/usr/bin/env bash
# Extract one version's section from CHANGELOG.md into an output file for
# GitHub Release notes (P7-08). Fails closed if the heading is absent or its
# body has no non-whitespace content.
set -euo pipefail

if [ "$#" -ne 3 ]; then
  printf 'usage: %s <version> <changelog-path> <output-path>\n' "$0" >&2
  exit 2
fi

VERSION="$1"
CHANGELOG_PATH="$2"
OUTPUT_PATH="$3"

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
  printf 'extract-changelog-section: invalid version "%s"\n' "$VERSION" >&2
  exit 2
fi

if [ ! -f "$CHANGELOG_PATH" ]; then
  printf 'extract-changelog-section: missing changelog file: %s\n' "$CHANGELOG_PATH" >&2
  exit 1
fi

python3 - "$VERSION" "$CHANGELOG_PATH" "$OUTPUT_PATH" <<'PY'
import re
import sys
from pathlib import Path

version, changelog_arg, output_arg = sys.argv[1:4]
lines = Path(changelog_arg).read_text(encoding="utf-8").splitlines()
heading_re = re.compile(rf"^##[ \t]+v?{re.escape(version)}(?:[ \t].*)?$")
next_heading_re = re.compile(r"^##[ \t]")

start = None
for idx, line in enumerate(lines):
    if heading_re.match(line):
        start = idx
        break

if start is None:
    print(f"extract-changelog-section: no section heading found for version {version}", file=sys.stderr)
    raise SystemExit(1)

end = len(lines)
for idx in range(start + 1, len(lines)):
    if next_heading_re.match(lines[idx]):
        end = idx
        break

section_lines = lines[start:end]
body_lines = [line for line in section_lines[1:] if line.strip()]
if not body_lines:
    print(f"extract-changelog-section: section for version {version} has an empty body", file=sys.stderr)
    raise SystemExit(1)

out = Path(output_arg)
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text("\n".join(section_lines).rstrip() + "\n", encoding="utf-8")
PY
