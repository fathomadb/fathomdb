#!/usr/bin/env bash
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir "$tmp/bin"
cat >"$tmp/bin/cargo" <<'EOF'
#!/usr/bin/env bash
if [[ "$*" == *--no-run* ]]; then exit 0; fi
printf 'AC013_TREATMENT_RECORD treatment=warm n=1\n'
printf 'AC013_TREATMENT_RECORD treatment=process_cold n=1\n'
EOF
chmod +x "$tmp/bin/cargo"
if PATH="$tmp/bin:$PATH" AC013_SCALE_TREATMENT=warm LOG_PATH="$tmp/log" bash "$root/scripts/perf-experiments/run-ac013.sh"; then
  echo 'multiple treatment records must fail' >&2
  exit 1
fi
