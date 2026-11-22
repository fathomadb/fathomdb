#!/usr/bin/env bash
# Write a test-owned Chromium wrapper that preserves production arguments under env_clear().
set -euo pipefail
if [[ $# -ne 2 || "$1" != /* || ! -x "$1" || "$2" != /* ]]; then
  echo 'usage: installed-browser.sh ABSOLUTE_CHROME_EXECUTABLE ABSOLUTE_WRAPPER_PATH' >&2
  exit 2
fi
mkdir -p -- "$(dirname -- "$2")"
escaped="${1//\'/\'\\\'\'}"
printf '#!/bin/sh\n# Instrument only this test-owned native Chromium; preserve production arguments.\nexec '\''%s'\'' --remote-debugging-port=0 "$@"\n' "$escaped" > "$2"
chmod 755 -- "$2"
